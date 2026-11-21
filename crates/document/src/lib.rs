//! Versioned document and feature dependency primitives.
//!
//! This crate stores design intent and dependency order. It does not evaluate
//! features into CAD geometry; geometry evaluation belongs to the kernel layer.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct FeatureId(u64);

impl FeatureId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureNode {
    id: FeatureId,
    dependencies: BTreeSet<FeatureId>,
}

impl FeatureNode {
    pub fn id(&self) -> FeatureId {
        self.id
    }
    pub fn dependencies(&self) -> &BTreeSet<FeatureId> {
        &self.dependencies
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeatureGraph {
    nodes: BTreeMap<FeatureId, FeatureNode>,
}

impl FeatureGraph {
    pub fn insert(
        &mut self,
        id: FeatureId,
        dependencies: impl IntoIterator<Item = FeatureId>,
    ) -> Result<(), DocumentError> {
        if self.nodes.contains_key(&id) {
            return Err(DocumentError::DuplicateFeature(id));
        }
        let dependencies: BTreeSet<_> = dependencies.into_iter().collect();
        if dependencies.contains(&id) {
            return Err(DocumentError::DependencyCycle);
        }
        for dependency in &dependencies {
            if !self.nodes.contains_key(dependency) {
                return Err(DocumentError::MissingFeature(*dependency));
            }
        }
        self.nodes.insert(id, FeatureNode { id, dependencies });
        Ok(())
    }

    pub fn set_dependencies(
        &mut self,
        id: FeatureId,
        dependencies: impl IntoIterator<Item = FeatureId>,
    ) -> Result<(), DocumentError> {
        if !self.nodes.contains_key(&id) {
            return Err(DocumentError::MissingFeature(id));
        }
        let dependencies: BTreeSet<_> = dependencies.into_iter().collect();
        for dependency in &dependencies {
            if !self.nodes.contains_key(dependency) {
                return Err(DocumentError::MissingFeature(*dependency));
            }
        }
        let previous = self
            .nodes
            .get(&id)
            .map(|node| node.dependencies.clone())
            .unwrap_or_default();
        self.nodes
            .get_mut(&id)
            .expect("node checked above")
            .dependencies = dependencies;
        if self.topological_order().is_err() {
            self.nodes
                .get_mut(&id)
                .expect("node checked above")
                .dependencies = previous;
            return Err(DocumentError::DependencyCycle);
        }
        Ok(())
    }

    /// Returns deterministic dependency-first order, or an error for cycles.
    pub fn topological_order(&self) -> Result<Vec<FeatureId>, DocumentError> {
        let mut remaining: BTreeMap<_, _> = self
            .nodes
            .iter()
            .map(|(id, node)| (*id, node.dependencies.clone()))
            .collect();
        let mut order = Vec::with_capacity(remaining.len());
        while !remaining.is_empty() {
            let ready = remaining
                .iter()
                .find_map(|(id, deps)| deps.is_empty().then_some(*id));
            let Some(ready) = ready else {
                return Err(DocumentError::DependencyCycle);
            };
            remaining.remove(&ready);
            for dependencies in remaining.values_mut() {
                dependencies.remove(&ready);
            }
            order.push(ready);
        }
        Ok(order)
    }

    pub fn get(&self, id: FeatureId) -> Option<&FeatureNode> {
        self.nodes.get(&id)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    schema_version: u32,
    revision: u64,
    features: FeatureGraph,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: DOCUMENT_SCHEMA_VERSION,
            revision: 0,
            features: FeatureGraph::default(),
        }
    }
}

impl Document {
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub fn features(&self) -> &FeatureGraph {
        &self.features
    }

    pub fn add_feature(
        &mut self,
        id: FeatureId,
        dependencies: impl IntoIterator<Item = FeatureId>,
    ) -> Result<(), DocumentError> {
        self.features.insert(id, dependencies)?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionOverflow)?;
        Ok(())
    }

    pub fn set_dependencies(
        &mut self,
        id: FeatureId,
        dependencies: impl IntoIterator<Item = FeatureId>,
    ) -> Result<(), DocumentError> {
        self.features.set_dependencies(id, dependencies)?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentError {
    DuplicateFeature(FeatureId),
    MissingFeature(FeatureId),
    DependencyCycle,
    RevisionOverflow,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFeature(id) => write!(f, "feature {} already exists", id.value()),
            Self::MissingFeature(id) => write!(f, "feature {} does not exist", id.value()),
            Self::DependencyCycle => write!(f, "feature dependencies must not contain cycles"),
            Self::RevisionOverflow => write!(f, "document revision counter overflowed"),
        }
    }
}

impl std::error::Error for DocumentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_orders_dependencies_before_consumers() {
        let mut graph = FeatureGraph::default();
        let base = FeatureId::new(10);
        let shoulder = FeatureId::new(20);
        let setting = FeatureId::new(30);
        graph.insert(base, []).unwrap();
        graph.insert(shoulder, [base]).unwrap();
        graph.insert(setting, [base, shoulder]).unwrap();
        assert_eq!(
            graph.topological_order().unwrap(),
            [base, shoulder, setting]
        );
    }

    #[test]
    fn rejects_missing_and_duplicate_features() {
        let mut graph = FeatureGraph::default();
        let id = FeatureId::new(1);
        assert_eq!(
            graph.insert(id, [FeatureId::new(2)]),
            Err(DocumentError::MissingFeature(FeatureId::new(2)))
        );
        graph.insert(id, []).unwrap();
        assert_eq!(
            graph.insert(id, []),
            Err(DocumentError::DuplicateFeature(id))
        );
    }

    #[test]
    fn rejects_dependency_cycles_without_mutating_graph() {
        let mut graph = FeatureGraph::default();
        let a = FeatureId::new(1);
        let b = FeatureId::new(2);
        graph.insert(a, []).unwrap();
        graph.insert(b, [a]).unwrap();
        assert_eq!(
            graph.set_dependencies(a, [b]),
            Err(DocumentError::DependencyCycle)
        );
        assert_eq!(graph.topological_order().unwrap(), [a, b]);
    }

    #[test]
    fn document_revisions_advance_for_successful_edits_only() {
        let mut document = Document::default();
        let first = FeatureId::new(1);
        assert_eq!(document.revision(), 0);
        document.add_feature(first, []).unwrap();
        assert_eq!(document.revision(), 1);
        assert!(document.add_feature(first, []).is_err());
        assert_eq!(document.revision(), 1);
        assert_eq!(document.schema_version(), DOCUMENT_SCHEMA_VERSION);
    }
}
