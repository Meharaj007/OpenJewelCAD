# Open Jewel CAD architecture

**Status: initial implementation direction.** The first desktop workflow now runs in Rust with eframe/egui; this document describes that direction and the boundaries still to validate at product scale.

## 1. Goals and constraints

Open Jewel CAD targets jewellery designers and manufacturers who need accurate editable models. The architecture should prioritize geometric correctness, explicit units and tolerances, predictable regeneration, recoverable documents, and a usable Windows desktop workflow.

The initial stack is a Rust application and eframe/egui desktop shell, plus a C++ geometry library backed by OCCT. This avoids a separate Qt toolchain and runtime for the first editor. Keep the CAD core headless-capable so geometry checks do not need to launch the GUI.

## 2. System boundaries

```text
Rust desktop shell (eframe/egui)
       │
       └── Rust application and project services
                    │
                    └── safe Rust wrapper ── stable C ABI ── C++ geometry library (OCCT)
```

The interface direction is intentionally narrow. The desktop sends typed parameter values to Rust application code; Rust validates them and calls the safe kernel wrapper; geometry requests cross the C ABI into the C++ kernel. The C++ library returns owned result handles or structured errors. No arbitrary module calls across language boundaries.

### Responsibilities

- **Rust desktop (eframe/egui):** menus, panels, property editing, user input, preview presentation, and UI-thread event handling.
- **Rust application core:** document lifecycle, commands, parameter values, dependency scheduling, persistence orchestration, undo/redo coordination, background work, and plugin/AI policy.
- **C++ CAD library:** OCCT shape construction and operations, validation, unit-aware geometry helpers, meshing requests, import/export adapters, and jewellery-specific geometry algorithms.
- **Persistence:** versioned project archive/manifest and a transactional metadata/library database. Geometry serialization format remains a decision to verify; do not assume that serializing native OCCT objects is a stable interchange format.

The current editor uses Rust/egui for a compact native window and the safe Rust wrapper for kernel operations. Its shaded previews are generated from primitive parameters; they are not yet OCCT-meshed viewport geometry. Keep presentation code separate from kernel algorithms, and revisit renderer and UI choices when the full feature-history workflow is designed.

## 3. Suggested repository layout

```text
.
├── apps/
│   └── cli/                     # desktop editor, CLI commands, and project-file handling
├── crates/
│   ├── app-core/                # orchestration and application services
│   ├── document/                # document model and schema versions
│   ├── commands/                # typed commands and history coordination
│   ├── persistence/             # project archive and database adapters
│   └── cad-ffi/                 # safe Rust wrappers around C ABI
├── cpp/
│   ├── cad-api/                 # exported C ABI, handles, error mapping
│   ├── cad-kernel/              # OCCT-backed geometry and validation
│   ├── jewellery/               # reusable jewellery algorithms
│   └── viewport/                # future OCCT visualization and tessellation helpers
├── include/openjewelcad/          # public C ABI headers
├── docs/                        # architecture and user/developer documentation
├── tests/
│   ├── integration/
│   └── fixtures/
├── scripts/                     # reproducible developer/CI helpers
├── .github/workflows/           # build and test automation
├── CMakeLists.txt
├── Cargo.toml
└── vcpkg.json                   # only if vcpkg is selected after evaluation
```

Keep this layout provisional until the initial build and ABI spike confirms the toolchain and packaging approach. Avoid creating empty placeholder modules for every future product feature.

## 4. Geometry object model

### Kernel ownership

The C++ layer is the sole owner of OCCT objects. Public Rust code sees opaque, reference-counted shape handles, stable identifiers, metadata, and serialized values; it never sees `TopoDS_*` types. Handle destruction must call back into the same library that allocated it. Avoid sharing mutable OCCT objects across threads.

### Geometry values

- Coordinates are right-handed Cartesian values in millimetres.
- Geometry inputs include an explicit unit at import/API boundaries; conversion to internal millimetres happens once and is recorded where round-tripping needs it.
- Linear and angular tolerances are explicit operation inputs or policy values with documented defaults. Do not use arbitrary global floating-point epsilons.
- A shape result includes validity state, topology summary, bounding box, and operation diagnostics as appropriate. Expensive measurements should be requested or cached deliberately.
- Invalid results are errors or explicitly marked diagnostic outputs; never silently treat them as manufacturable solids.

### Stable C ABI shape

Use opaque handles and fixed-layout structs with an ABI version. All exported functions return an error code; a separate owned error/message result carries machine-readable category and human-readable detail. Functions accept plain scalars, fixed arrays, and caller-owned buffers with documented lengths. C++ exceptions are caught at every exported boundary. Define allocation/free functions for returned buffers and handles. Include cancellation tokens for long-running operations.

## 5. Document architecture

A document is a versioned graph of stable object IDs, typed feature definitions, parameter values, dependency edges, and derived geometry references. Geometry is derived state wherever regeneration is deterministic; the document must retain sufficient source parameters and feature inputs to rebuild it. Cache data is disposable and versioned independently.

The project envelope should contain a manifest (schema version, units, application version, metadata), document data, and optional preview/cache assets. Writes should be transactional: write a new complete project representation, flush, then atomically replace the prior file where the platform permits. Maintain autosave/recovery snapshots separately from the last confirmed save. Schema migrations must be explicit and tested.

SQLite is suitable for a searchable local asset catalogue and metadata if needed; it should not become the only copy of the active model. Select a journal and backup strategy and test crash recovery before relying on it for critical data.

## 6. Feature history and dependency system

Represent edits as typed features with immutable input snapshots and parameter maps. A directed acyclic dependency graph determines regeneration order. Reject cycles with a user-visible diagnostic. Each feature has a stable ID, implementation/schema version, input references, parameters, and generated outputs.

Regeneration is incremental: mark downstream nodes dirty when an input changes, then recompute in topological order. Cache keys include feature type/version, normalized parameters, input geometry identities, and relevant tolerance policy. A failed feature preserves the last valid document state and reports which node failed; do not replace valid geometry with partial output.

Undo/redo should operate at the application command/document transaction layer. Commands record reversible document deltas or snapshots, while geometry results are regenerated or reused from safe caches. Avoid relying on opaque OCCT mutation history as the user-facing history model.

## 7. Threading and memory model

- eframe/egui widgets and viewport presentation state stay on the GUI thread.
- Rust owns task scheduling, cancellation, progress, and document transaction sequencing.
- CPU-heavy independent geometry operations may run on workers only after the OCCT operation and data-sharing constraints are verified. Treat kernel shapes as immutable across worker boundaries unless an operation explicitly clones them.
- A document transaction commits only if its captured input revision is still current; stale asynchronous results are discarded.
- C ABI calls must specify whether they block. Long operations use job handles with poll/progress/cancel/release operations rather than blocking the UI.
- No callback may outlive its registered context. Shutdown drains or cancels jobs before library handles and UI state are destroyed.

## 8. Risks and open questions

1. **Windows dependency integration:** eframe/egui, OCCT, MSVC runtime, and Rust target compatibility must be proven in clean builds and packaging.
2. **UI and viewport scale:** the current egui preview is a simple projected box. A future tessellated, selectable OCCT viewport may require a dedicated rendering adapter; keep this separate from document and kernel ownership.
3. **OCCT licensing and distribution:** review the selected release's license, third-party notices, and obligations with the product's distribution model before shipping.
4. **Parametric stability:** topology references can become unstable after edits. Use semantic feature references where possible and design recovery behavior for failed downstream features.
5. **Manufacturing trust:** kernel validity does not prove castability or printability. Manufacturing checks need documented domain rules and independent regression fixtures.
6. **File compatibility:** STEP/STL/3DM capabilities and licenses vary. Treat each format as a separately tested adapter with round-trip limitations documented.
7. **AI and plugins:** constrain both to validated typed commands and explicit permissions; neither may mutate kernel memory or execute arbitrary code in-process.

## 9. Dependency and license policy

The initial stack includes Rust 1.95+, eframe 0.37.0 (MIT OR Apache-2.0), rfd 0.18.0 (MIT), CMake, OCCT 8.0.1, and MSVC. Cargo.lock records the Rust dependency resolution. SQLite, openNURBS, and additional test frameworks remain unselected. Record license texts and distribution obligations before shipping builds; do not add dependencies solely because they appear in the original wishlist.
