mod gui;

use openjewelcad_cad_ffi::{Kernel, Shape};
use openjewelcad_geometry::LengthMm;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PrimitiveKind {
    Box,
    Cylinder,
    Cone,
    TruncatedCone,
    Ellipsoid,
    Pipe,
    Sphere,
    Torus,
    RingBand,
    RoundBrilliant,
    LineCurve,
    LineSurface,
    OpenPolylineCurve,
    OpenPolylineSurface,
    BezierCurve,
    BezierSurface,
    RevolvedSurface,
    RevolvedBezierSurface,
    PolylineCurve,
    ArcCurve,
    ArcSurface,
    CircleCurve,
    EllipseCurve,
    PlanarSurface,
    PlanarCircleSurface,
    PlanarEllipseSurface,
    ExtrudedPolygon,
    ExtrudedCircle,
    ExtrudedEllipse,
    RevolvedSolid,
}

impl PrimitiveKind {
    fn name(self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Cylinder => "cylinder",
            Self::Cone => "cone",
            Self::TruncatedCone => "truncated-cone",
            Self::Ellipsoid => "ellipsoid",
            Self::Pipe => "pipe",
            Self::Sphere => "sphere",
            Self::Torus => "torus",
            Self::RingBand => "ring-band",
            Self::RoundBrilliant => "round-brilliant",
            Self::LineCurve => "line-curve",
            Self::LineSurface => "line-surface",
            Self::OpenPolylineCurve => "open-polyline-curve",
            Self::OpenPolylineSurface => "open-polyline-surface",
            Self::BezierCurve => "bezier-curve",
            Self::BezierSurface => "bezier-surface",
            Self::RevolvedSurface => "revolved-surface",
            Self::RevolvedBezierSurface => "bezier-revolved-surface",
            Self::PolylineCurve => "polyline-curve",
            Self::ArcCurve => "arc-curve",
            Self::ArcSurface => "arc-surface",
            Self::CircleCurve => "circle-curve",
            Self::EllipseCurve => "ellipse-curve",
            Self::PlanarSurface => "planar-surface",
            Self::PlanarCircleSurface => "planar-circle",
            Self::PlanarEllipseSurface => "planar-ellipse",
            Self::ExtrudedPolygon => "extruded-polygon",
            Self::ExtrudedCircle => "extruded-circle",
            Self::ExtrudedEllipse => "extruded-ellipse",
            Self::RevolvedSolid => "revolved-solid",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "box" => Some(Self::Box),
            "cylinder" => Some(Self::Cylinder),
            "cone" => Some(Self::Cone),
            "truncated-cone" => Some(Self::TruncatedCone),
            "ellipsoid" => Some(Self::Ellipsoid),
            "pipe" => Some(Self::Pipe),
            "sphere" => Some(Self::Sphere),
            "torus" => Some(Self::Torus),
            "ring-band" => Some(Self::RingBand),
            "round-brilliant" => Some(Self::RoundBrilliant),
            "line-curve" => Some(Self::LineCurve),
            "line-surface" => Some(Self::LineSurface),
            "open-polyline-curve" => Some(Self::OpenPolylineCurve),
            "open-polyline-surface" => Some(Self::OpenPolylineSurface),
            "bezier-curve" => Some(Self::BezierCurve),
            "bezier-surface" => Some(Self::BezierSurface),
            "revolved-surface" => Some(Self::RevolvedSurface),
            "bezier-revolved-surface" => Some(Self::RevolvedBezierSurface),
            "polyline-curve" => Some(Self::PolylineCurve),
            "arc-curve" => Some(Self::ArcCurve),
            "arc-surface" => Some(Self::ArcSurface),
            "circle-curve" => Some(Self::CircleCurve),
            "ellipse-curve" => Some(Self::EllipseCurve),
            "planar-surface" => Some(Self::PlanarSurface),
            "planar-circle" => Some(Self::PlanarCircleSurface),
            "planar-ellipse" => Some(Self::PlanarEllipseSurface),
            "extruded-polygon" => Some(Self::ExtrudedPolygon),
            "extruded-circle" => Some(Self::ExtrudedCircle),
            "extruded-ellipse" => Some(Self::ExtrudedEllipse),
            "revolved-solid" => Some(Self::RevolvedSolid),
            _ => None,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Cylinder => "Cylinder",
            Self::Cone => "Cone",
            Self::TruncatedCone => "Truncated cone",
            Self::Ellipsoid => "Ellipsoid",
            Self::Pipe => "Pipe",
            Self::Sphere => "Sphere",
            Self::Torus => "Torus",
            Self::RingBand => "Ring band",
            Self::RoundBrilliant => "Round brilliant",
            Self::LineCurve => "Line curve",
            Self::LineSurface => "Line surface",
            Self::OpenPolylineCurve => "Open polyline curve",
            Self::OpenPolylineSurface => "Open polyline surface",
            Self::BezierCurve => "Cubic Bezier curve",
            Self::BezierSurface => "Bezier surface",
            Self::RevolvedSurface => "Revolved surface",
            Self::RevolvedBezierSurface => "Revolved Bezier surface",
            Self::PolylineCurve => "Closed polyline curve",
            Self::ArcCurve => "Arc curve",
            Self::ArcSurface => "Arc surface",
            Self::CircleCurve => "Circle curve",
            Self::EllipseCurve => "Ellipse curve",
            Self::PlanarSurface => "Planar surface",
            Self::PlanarCircleSurface => "Planar circle surface",
            Self::PlanarEllipseSurface => "Planar ellipse surface",
            Self::ExtrudedPolygon => "Extruded profile",
            Self::ExtrudedCircle => "Extruded circle",
            Self::ExtrudedEllipse => "Extruded ellipse",
            Self::RevolvedSolid => "Revolved solid",
        }
    }

    fn parameter_names(self) -> &'static [&'static str] {
        match self {
            Self::Box => &["width_mm", "depth_mm", "height_mm"],
            Self::Cylinder => &["radius_mm", "height_mm"],
            Self::Cone => &["radius_mm", "height_mm"],
            Self::TruncatedCone => &["base_radius_mm", "top_radius_mm", "height_mm"],
            Self::Ellipsoid => &["radius_x_mm", "radius_y_mm", "radius_z_mm"],
            Self::Pipe => &["outer_radius_mm", "inner_radius_mm", "length_mm"],
            Self::Sphere => &["radius_mm"],
            Self::Torus => &["major_radius_mm", "tube_radius_mm"],
            Self::RingBand => &["inner_diameter_mm", "wall_thickness_mm", "band_width_mm"],
            Self::RoundBrilliant => &["diameter_mm", "height_mm", "table_diameter_mm"],
            Self::LineCurve
            | Self::OpenPolylineCurve
            | Self::BezierCurve
            | Self::PolylineCurve
            | Self::PlanarSurface => &[],
            Self::LineSurface | Self::OpenPolylineSurface | Self::BezierSurface => &["height_mm"],
            Self::RevolvedSurface | Self::RevolvedBezierSurface => &[],
            Self::ArcCurve => &["radius_mm"],
            Self::ArcSurface => &["radius_mm", "height_mm"],
            Self::CircleCurve | Self::PlanarCircleSurface => &["radius_mm"],
            Self::EllipseCurve | Self::PlanarEllipseSurface => &["radius_x_mm", "radius_y_mm"],
            Self::ExtrudedPolygon => &["height_mm"],
            Self::ExtrudedCircle => &["radius_mm", "height_mm"],
            Self::ExtrudedEllipse => &["radius_x_mm", "radius_y_mm", "height_mm"],
            Self::RevolvedSolid => &[],
        }
    }

    fn parameter_labels(self) -> &'static [&'static str] {
        match self {
            Self::Box => &["Width", "Depth", "Height"],
            Self::Cylinder => &["Radius", "Height"],
            Self::Cone => &["Radius", "Height"],
            Self::TruncatedCone => &["Base radius", "Top radius", "Height"],
            Self::Ellipsoid => &["X radius", "Y radius", "Z radius"],
            Self::Pipe => &["Outer radius", "Inner radius", "Length"],
            Self::Sphere => &["Radius"],
            Self::Torus => &["Major radius", "Tube radius"],
            Self::RingBand => &["Inner diameter", "Wall thickness", "Band width"],
            Self::RoundBrilliant => &["Girdle diameter", "Total height", "Table diameter"],
            Self::LineCurve
            | Self::OpenPolylineCurve
            | Self::BezierCurve
            | Self::PolylineCurve
            | Self::PlanarSurface => &[],
            Self::LineSurface | Self::OpenPolylineSurface | Self::BezierSurface => {
                &["Extrusion height"]
            }
            Self::RevolvedSurface | Self::RevolvedBezierSurface => &[],
            Self::ArcCurve => &["Radius"],
            Self::ArcSurface => &["Radius", "Extrusion height"],
            Self::CircleCurve | Self::PlanarCircleSurface => &["Radius"],
            Self::EllipseCurve | Self::PlanarEllipseSurface => &["X radius", "Y radius"],
            Self::ExtrudedPolygon => &["Extrusion height"],
            Self::ExtrudedCircle => &["Radius", "Extrusion height"],
            Self::ExtrudedEllipse => &["X radius", "Y radius", "Extrusion height"],
            Self::RevolvedSolid => &[],
        }
    }

    fn defaults(self) -> [f64; 3] {
        match self {
            Self::Box => [10.0, 10.0, 2.0],
            Self::Cylinder => [5.0, 10.0, 0.0],
            Self::Cone => [5.0, 10.0, 0.0],
            Self::TruncatedCone => [6.0, 3.0, 10.0],
            Self::Ellipsoid => [5.0, 3.5, 2.5],
            Self::Pipe => [3.0, 1.5, 10.0],
            Self::Sphere => [5.0, 0.0, 0.0],
            Self::Torus => [8.0, 2.0, 0.0],
            Self::RingBand => [18.2, 1.5, 4.0],
            Self::RoundBrilliant => [6.0, 3.6, 3.3],
            Self::LineCurve
            | Self::OpenPolylineCurve
            | Self::BezierCurve
            | Self::PolylineCurve
            | Self::PlanarSurface => [0.0, 0.0, 0.0],
            Self::LineSurface | Self::OpenPolylineSurface | Self::BezierSurface => [5.0, 0.0, 0.0],
            Self::RevolvedSurface | Self::RevolvedBezierSurface => [0.0; 3],
            Self::ArcCurve => [5.0, 0.0, 0.0],
            Self::ArcSurface => [5.0, 5.0, 0.0],
            Self::CircleCurve | Self::PlanarCircleSurface => [5.0, 0.0, 0.0],
            Self::EllipseCurve | Self::PlanarEllipseSurface => [5.0, 3.0, 0.0],
            Self::ExtrudedPolygon => [5.0, 0.0, 0.0],
            Self::ExtrudedCircle => [5.0, 5.0, 0.0],
            Self::ExtrudedEllipse => [5.0, 3.0, 5.0],
            Self::RevolvedSolid => [0.0; 3],
        }
    }

    fn uses_profile(self) -> bool {
        matches!(
            self,
            Self::PolylineCurve | Self::PlanarSurface | Self::ExtrudedPolygon | Self::RevolvedSolid
        )
    }

    fn is_solid(self) -> bool {
        !matches!(
            self,
            Self::PolylineCurve
                | Self::LineCurve
                | Self::OpenPolylineCurve
                | Self::BezierCurve
                | Self::LineSurface
                | Self::OpenPolylineSurface
                | Self::BezierSurface
                | Self::RevolvedSurface
                | Self::RevolvedBezierSurface
                | Self::ArcCurve
                | Self::ArcSurface
                | Self::CircleCurve
                | Self::EllipseCurve
                | Self::PlanarSurface
                | Self::PlanarCircleSurface
                | Self::PlanarEllipseSurface
        )
    }

    fn is_curve(self) -> bool {
        matches!(
            self,
            Self::LineCurve
                | Self::OpenPolylineCurve
                | Self::BezierCurve
                | Self::PolylineCurve
                | Self::ArcCurve
                | Self::CircleCurve
                | Self::EllipseCurve
        )
    }

    fn is_surface(self) -> bool {
        matches!(
            self,
            Self::PlanarSurface
                | Self::PlanarCircleSurface
                | Self::PlanarEllipseSurface
                | Self::LineSurface
                | Self::OpenPolylineSurface
                | Self::BezierSurface
                | Self::ArcSurface
                | Self::RevolvedSurface
                | Self::RevolvedBezierSurface
        )
    }

    fn uses_arc(self) -> bool {
        matches!(self, Self::ArcCurve | Self::ArcSurface)
    }

    fn uses_line(self) -> bool {
        matches!(self, Self::LineCurve | Self::LineSurface)
    }

    fn uses_vector_extrusion(self) -> bool {
        matches!(
            self,
            Self::LineSurface
                | Self::OpenPolylineSurface
                | Self::BezierSurface
                | Self::ArcSurface
                | Self::ExtrudedPolygon
                | Self::ExtrudedCircle
                | Self::ExtrudedEllipse
        )
    }

    fn vector_extrusion_project_version(self) -> Option<u32> {
        match self {
            Self::LineSurface | Self::OpenPolylineSurface | Self::ArcSurface => Some(17),
            Self::BezierSurface => Some(19),
            Self::ExtrudedPolygon | Self::ExtrudedCircle | Self::ExtrudedEllipse => Some(18),
            _ => None,
        }
    }

    fn uses_open_polyline(self) -> bool {
        matches!(
            self,
            Self::OpenPolylineCurve
                | Self::OpenPolylineSurface
                | Self::BezierCurve
                | Self::BezierSurface
                | Self::RevolvedSurface
                | Self::RevolvedBezierSurface
        )
    }

    fn is_circle_stage(self) -> bool {
        matches!(
            self,
            Self::CircleCurve | Self::PlanarCircleSurface | Self::ExtrudedCircle
        )
    }

    fn is_ellipse_stage(self) -> bool {
        matches!(
            self,
            Self::EllipseCurve | Self::PlanarEllipseSurface | Self::ExtrudedEllipse
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ArcFeature {
    start_angle_deg: f64,
    sweep_angle_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LineFeature {
    start_mm: [f64; 2],
    end_mm: [f64; 2],
}

impl LineFeature {
    fn new(start_mm: [f64; 2], end_mm: [f64; 2]) -> Result<Self, Box<dyn Error>> {
        if start_mm
            .into_iter()
            .chain(end_mm)
            .any(|value| !value.is_finite() || value.abs() > 1e7)
            || (end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]) <= 1e-9
        {
            return Err("line endpoints must be finite, within 10,000,000 mm, and distinct".into());
        }
        Ok(Self { start_mm, end_mm })
    }
}

impl Default for LineFeature {
    fn default() -> Self {
        Self {
            start_mm: [-5.0, 0.0],
            end_mm: [5.0, 0.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenPolylineFeature {
    point_count: u8,
    points_mm: [[f64; 2]; MAX_PROFILE_POINTS],
}

impl OpenPolylineFeature {
    fn bezier_default() -> Self {
        Self::from_points(&[[-5.0, 0.0], [-2.0, 6.0], [2.0, -6.0], [5.0, 0.0]])
            .expect("the default cubic Bezier control points are valid")
    }

    fn revolved_surface_default() -> Self {
        Self::from_points(&[[5.0, -2.0], [5.0, 2.0]])
            .expect("the default revolved profile is valid")
    }

    fn revolved_bezier_default() -> Self {
        Self::from_points(&[[5.0, -2.0], [6.0, -1.0], [6.0, 1.0], [5.0, 2.0]])
            .expect("the default revolved Bezier control points are valid")
    }

    fn from_points(points: &[[f64; 2]]) -> Result<Self, Box<dyn Error>> {
        if !(2..=MAX_PROFILE_POINTS).contains(&points.len()) {
            return Err(format!("open polyline needs 2–{MAX_PROFILE_POINTS} vertices").into());
        }
        if points
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || value.abs() > 1e7)
        {
            return Err(
                "open polyline coordinates must be finite millimetre values within ±10,000,000"
                    .into(),
            );
        }
        for pair in points.windows(2) {
            let dx = pair[1][0] - pair[0][0];
            let dy = pair[1][1] - pair[0][1];
            if dx * dx + dy * dy <= 1e-16 {
                return Err("open polyline segments need distinct consecutive vertices".into());
            }
        }
        let mut profile = Self {
            point_count: points.len() as u8,
            points_mm: [[0.0; 2]; MAX_PROFILE_POINTS],
        };
        profile.points_mm[..points.len()].copy_from_slice(points);
        Ok(profile)
    }

    fn points(&self) -> &[[f64; 2]] {
        &self.points_mm[..usize::from(self.point_count)]
    }

    fn reverse(&mut self) {
        self.points_mm[..usize::from(self.point_count)].reverse();
    }

    fn set_point_unchecked(&mut self, index: usize, point: [f64; 2]) {
        if index < usize::from(self.point_count) {
            self.points_mm[index] = point;
        }
    }

    fn add_point(&mut self) -> Result<(), Box<dyn Error>> {
        let count = usize::from(self.point_count);
        if count >= MAX_PROFILE_POINTS {
            return Err(
                format!("open polyline can contain at most {MAX_PROFILE_POINTS} vertices").into(),
            );
        }
        let last = self.points_mm[count - 1];
        let mut points = self.points().to_vec();
        points.push([last[0] + 5.0, last[1]]);
        *self = Self::from_points(&points)?;
        Ok(())
    }

    fn remove_point(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        let count = usize::from(self.point_count);
        if count <= 2 {
            return Err("an open polyline needs at least two vertices".into());
        }
        if index >= count {
            return Err("open polyline vertex index is out of range".into());
        }
        let mut points = self.points().to_vec();
        points.remove(index);
        *self = Self::from_points(&points)?;
        Ok(())
    }
}

impl Default for OpenPolylineFeature {
    fn default() -> Self {
        Self::from_points(&[[-5.0, 0.0], [0.0, 5.0], [5.0, 0.0]])
            .expect("the default open polyline is valid")
    }
}

impl ArcFeature {
    fn new(start_angle_deg: f64, sweep_angle_deg: f64) -> Result<Self, Box<dyn Error>> {
        if !start_angle_deg.is_finite()
            || !(-360.0..=360.0).contains(&start_angle_deg)
            || !sweep_angle_deg.is_finite()
            || sweep_angle_deg <= 0.0
            || sweep_angle_deg >= 360.0
        {
            return Err("arc start must be in [-360, 360]° and sweep must be in (0, 360)°".into());
        }
        Ok(Self {
            start_angle_deg,
            sweep_angle_deg,
        })
    }
}

impl Default for ArcFeature {
    fn default() -> Self {
        Self {
            start_angle_deg: 0.0,
            sweep_angle_deg: 90.0,
        }
    }
}

const MAX_PROFILE_POINTS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
struct PolygonProfile {
    point_count: u8,
    points_mm: [[f64; 2]; MAX_PROFILE_POINTS],
}

impl Default for PolygonProfile {
    fn default() -> Self {
        Self::from_points(&[[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]])
            .expect("the default square profile is valid")
    }
}

impl PolygonProfile {
    fn from_points(points: &[[f64; 2]]) -> Result<Self, Box<dyn Error>> {
        validate_polygon_profile(points)?;
        let mut profile = Self {
            point_count: points.len() as u8,
            points_mm: [[0.0; 2]; MAX_PROFILE_POINTS],
        };
        profile.points_mm[..points.len()].copy_from_slice(points);
        Ok(profile)
    }

    fn points(&self) -> &[[f64; 2]] {
        &self.points_mm[..usize::from(self.point_count)]
    }

    fn signed_area(&self) -> f64 {
        polygon_signed_area(self.points())
    }

    fn set_point_unchecked(&mut self, index: usize, point: [f64; 2]) {
        if index < usize::from(self.point_count) {
            self.points_mm[index] = point;
        }
    }

    fn add_vertex(&mut self) -> Result<(), Box<dyn Error>> {
        let count = usize::from(self.point_count);
        if count >= MAX_PROFILE_POINTS {
            return Err(
                format!("profile can contain at most {MAX_PROFILE_POINTS} vertices").into(),
            );
        }
        let points = self.points();
        let (edge, _) = (0..count)
            .map(|index| {
                let next = (index + 1) % count;
                let dx = points[next][0] - points[index][0];
                let dy = points[next][1] - points[index][1];
                (index, dx * dx + dy * dy)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .expect("validated profiles contain an edge");
        let next = (edge + 1) % count;
        let midpoint = [
            (self.points_mm[edge][0] + self.points_mm[next][0]) * 0.5,
            (self.points_mm[edge][1] + self.points_mm[next][1]) * 0.5,
        ];
        let insertion = if next == 0 { count } else { next };
        for index in (insertion..count).rev() {
            self.points_mm[index + 1] = self.points_mm[index];
        }
        self.points_mm[insertion] = midpoint;
        self.point_count += 1;
        Ok(())
    }

    fn remove_vertex(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        let count = usize::from(self.point_count);
        if count <= 3 {
            return Err("a closed polygon needs at least three vertices".into());
        }
        if index >= count {
            return Err("profile vertex index is out of range".into());
        }
        for current in index..count - 1 {
            self.points_mm[current] = self.points_mm[current + 1];
        }
        self.point_count -= 1;
        Ok(())
    }
}

fn rectangle_profile(width_mm: f64, height_mm: f64) -> Result<PolygonProfile, Box<dyn Error>> {
    if !width_mm.is_finite()
        || !height_mm.is_finite()
        || width_mm <= 0.0
        || height_mm <= 0.0
        || width_mm > 2e7
        || height_mm > 2e7
    {
        return Err(
            "rectangle width and height must be finite positive millimetres within 20,000,000"
                .into(),
        );
    }
    let half_width = width_mm * 0.5;
    let half_height = height_mm * 0.5;
    PolygonProfile::from_points(&[
        [-half_width, -half_height],
        [half_width, -half_height],
        [half_width, half_height],
        [-half_width, half_height],
    ])
}

fn regular_polygon_profile(
    sides: usize,
    circumradius_mm: f64,
) -> Result<PolygonProfile, Box<dyn Error>> {
    if !(3..=MAX_PROFILE_POINTS).contains(&sides) {
        return Err(format!("regular polygon needs 3–{MAX_PROFILE_POINTS} sides").into());
    }
    if !circumradius_mm.is_finite() || circumradius_mm <= 0.0 || circumradius_mm > 1e7 {
        return Err(
            "polygon radius must be a finite positive millimetre value within 10,000,000".into(),
        );
    }
    let points = (0..sides)
        .map(|index| {
            let angle =
                std::f64::consts::TAU * index as f64 / sides as f64 - std::f64::consts::FRAC_PI_2;
            [circumradius_mm * angle.cos(), circumradius_mm * angle.sin()]
        })
        .collect::<Vec<_>>();
    PolygonProfile::from_points(&points)
}

fn revolved_profile(
    inner_radius_mm: f64,
    outer_radius_mm: f64,
    height_mm: f64,
) -> Result<PolygonProfile, Box<dyn Error>> {
    if !inner_radius_mm.is_finite()
        || !outer_radius_mm.is_finite()
        || !height_mm.is_finite()
        || inner_radius_mm <= 0.0
        || outer_radius_mm <= inner_radius_mm
        || height_mm <= 0.0
        || outer_radius_mm > 1e7
        || height_mm > 2e7
    {
        return Err("revolve section needs a positive inner radius, a larger outer radius, and a positive height".into());
    }
    let half_height = height_mm * 0.5;
    PolygonProfile::from_points(&[
        [inner_radius_mm, -half_height],
        [outer_radius_mm, -half_height],
        [outer_radius_mm, half_height],
        [inner_radius_mm, half_height],
    ])
}

fn default_revolved_profile() -> PolygonProfile {
    revolved_profile(3.0, 5.0, 5.0).expect("the default revolve section is valid")
}

fn closed_profile_from_open(
    feature: OpenPolylineFeature,
) -> Result<PolygonProfile, Box<dyn Error>> {
    let mut points = feature.points().to_vec();
    if points.len() > 3 {
        let first = points[0];
        let last = points[points.len() - 1];
        if (last[0] - first[0]).hypot(last[1] - first[1]) <= 1e-8 {
            points.pop();
        }
    }
    PolygonProfile::from_points(&points)
}

fn polygon_signed_area(points: &[[f64; 2]]) -> f64 {
    (0..points.len())
        .map(|index| {
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            current[0] * next[1] - next[0] * current[1]
        })
        .sum::<f64>()
        * 0.5
}

fn validate_extrusion_direction(direction: [f64; 3]) -> Result<[f64; 3], Box<dyn Error>> {
    if direction
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1e7)
        || direction[0].hypot(direction[1]).hypot(direction[2]) <= 1e-12
    {
        return Err("extrusion direction needs finite, non-zero XYZ components".into());
    }
    Ok(direction)
}

fn polygon_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn polygon_point_on_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    const EPSILON: f64 = 1e-9;
    polygon_cross(a, b, point).abs() <= EPSILON
        && point[0] >= a[0].min(b[0]) - EPSILON
        && point[0] <= a[0].max(b[0]) + EPSILON
        && point[1] >= a[1].min(b[1]) - EPSILON
        && point[1] <= a[1].max(b[1]) + EPSILON
}

fn polygon_segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    const EPSILON: f64 = 1e-9;
    let ab_c = polygon_cross(a, b, c);
    let ab_d = polygon_cross(a, b, d);
    let cd_a = polygon_cross(c, d, a);
    let cd_b = polygon_cross(c, d, b);
    let sign = |value: f64| {
        if value > EPSILON {
            1
        } else if value < -EPSILON {
            -1
        } else {
            0
        }
    };
    let (first, second, third, fourth) = (sign(ab_c), sign(ab_d), sign(cd_a), sign(cd_b));
    (first != second && third != fourth)
        || (first == 0 && polygon_point_on_segment(c, a, b))
        || (second == 0 && polygon_point_on_segment(d, a, b))
        || (third == 0 && polygon_point_on_segment(a, c, d))
        || (fourth == 0 && polygon_point_on_segment(b, c, d))
}

fn validate_polygon_profile(points: &[[f64; 2]]) -> Result<(), Box<dyn Error>> {
    if !(3..=MAX_PROFILE_POINTS).contains(&points.len()) {
        return Err(format!("profile needs 3–{MAX_PROFILE_POINTS} vertices").into());
    }
    if points
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e7)
    {
        return Err(
            "profile coordinates must be finite millimetre values within ±10,000,000".into(),
        );
    }
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let dx = next[0] - current[0];
        let dy = next[1] - current[1];
        if dx * dx + dy * dy <= 1e-16 {
            return Err("profile edges need distinct consecutive vertices".into());
        }
    }
    if polygon_signed_area(points).abs() <= 1e-12 {
        return Err("profile must enclose a non-zero area".into());
    }
    for first in 0..points.len() {
        let first_next = (first + 1) % points.len();
        for second in first + 1..points.len() {
            let second_next = (second + 1) % points.len();
            if first_next == second || second_next == first {
                continue;
            }
            if polygon_segments_intersect(
                points[first],
                points[first_next],
                points[second],
                points[second_next],
            ) {
                return Err("profile outline must not cross or touch itself".into());
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransformParams {
    translation_mm: [f64; 3],
    rotation_deg: [f64; 3],
    uniform_scale: f64,
    mirror_axes: [bool; 3],
}

impl TransformParams {
    const IDENTITY: Self = Self {
        translation_mm: [0.0; 3],
        rotation_deg: [0.0; 3],
        uniform_scale: 1.0,
        mirror_axes: [false; 3],
    };

    fn validate(self) -> Result<Self, Box<dyn Error>> {
        if self
            .translation_mm
            .into_iter()
            .any(|value| !value.is_finite())
        {
            return Err("translation values must be finite millimetre values".into());
        }
        if self
            .rotation_deg
            .into_iter()
            .any(|value| !value.is_finite())
        {
            return Err("rotation values must be finite degrees".into());
        }
        if !self.uniform_scale.is_finite() || self.uniform_scale <= 0.0 {
            return Err("uniform scale must be a finite positive value".into());
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BooleanOperation {
    Union,
    Difference,
    Intersection,
}

impl BooleanOperation {
    fn name(self) -> &'static str {
        match self {
            Self::Union => "union",
            Self::Difference => "difference",
            Self::Intersection => "intersection",
        }
    }

    fn parse(name: &str) -> Result<Self, Box<dyn Error>> {
        match name {
            "union" => Ok(Self::Union),
            "difference" => Ok(Self::Difference),
            "intersection" => Ok(Self::Intersection),
            _ => Err("boolean operation must be union, difference, or intersection".into()),
        }
    }

    fn kernel_operation(self) -> openjewelcad_cad_ffi::BooleanOperation {
        match self {
            Self::Union => openjewelcad_cad_ffi::BooleanOperation::Union,
            Self::Difference => openjewelcad_cad_ffi::BooleanOperation::Difference,
            Self::Intersection => openjewelcad_cad_ffi::BooleanOperation::Intersection,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BooleanFeature {
    operation: BooleanOperation,
    tool_kind: PrimitiveKind,
    tool_parameters_mm: [LengthMm; 3],
    tool_transform: TransformParams,
}

impl BooleanFeature {
    fn new(
        operation: BooleanOperation,
        tool_kind: PrimitiveKind,
        values_mm: [f64; 3],
        transform: TransformParams,
    ) -> Result<Self, Box<dyn Error>> {
        if !tool_kind.is_solid() || tool_kind.uses_profile() {
            return Err("curve, surface, and custom-profile features are not supported as boolean tools yet".into());
        }
        let tool = PrimitiveModel::new(tool_kind, values_mm)?.with_transform(transform)?;
        Ok(Self {
            operation,
            tool_kind,
            tool_parameters_mm: tool.parameters_mm,
            tool_transform: tool.transform,
        })
    }

    fn values_mm(self) -> [f64; 3] {
        self.tool_parameters_mm.map(LengthMm::as_mm)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternMode {
    Linear,
    PolarZ,
}

impl PatternMode {
    fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::PolarZ => "polar-z",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PatternFeature {
    mode: PatternMode,
    copies: u32,
    step_mm: [f64; 3],
    angle_step_deg: f64,
}

impl PatternFeature {
    fn linear(copies: u32, step_mm: [f64; 3]) -> Result<Self, Box<dyn Error>> {
        if !(2..=64).contains(&copies)
            || step_mm.iter().any(|value| !value.is_finite())
            || step_mm.iter().all(|value| *value == 0.0)
            || step_mm
                .iter()
                .any(|value| !(*value * f64::from(copies - 1)).is_finite())
        {
            return Err("linear array needs 2–64 copies and a finite non-zero XYZ step".into());
        }
        Ok(Self {
            mode: PatternMode::Linear,
            copies,
            step_mm,
            angle_step_deg: 0.0,
        })
    }

    fn polar_z(copies: u32, angle_step_deg: f64) -> Result<Self, Box<dyn Error>> {
        if !(2..=64).contains(&copies)
            || !angle_step_deg.is_finite()
            || angle_step_deg == 0.0
            || angle_step_deg.abs() >= 360.0
            || !(angle_step_deg * f64::from(copies - 1)).is_finite()
        {
            return Err("polar array needs 2–64 copies and a non-zero angle below 360°".into());
        }
        Ok(Self {
            mode: PatternMode::PolarZ,
            copies,
            step_mm: [0.0; 3],
            angle_step_deg,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PrimitiveModel {
    kind: PrimitiveKind,
    parameters_mm: [LengthMm; 3],
    profile: PolygonProfile,
    arc: Option<ArcFeature>,
    line: Option<LineFeature>,
    open_polyline: Option<OpenPolylineFeature>,
    extrusion_direction: [f64; 3],
    transform: TransformParams,
    material_density_g_cm3: Option<f64>,
    boolean: Option<BooleanFeature>,
    pattern: Option<PatternFeature>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MeshExportFormat {
    BinaryStl,
    Obj,
}

#[derive(Clone, Copy, Debug)]
struct MeshExportSummary {
    pub vertices: usize,
    pub triangles: usize,
}

impl PrimitiveModel {
    fn new(kind: PrimitiveKind, values: [f64; 3]) -> Result<Self, Box<dyn Error>> {
        let names = kind.parameter_names();
        for (index, name) in names.iter().enumerate() {
            if !values[index].is_finite() || values[index] <= 0.0 {
                return Err(
                    format!("{name} must be a finite positive number of millimetres").into(),
                );
            }
        }
        if kind == PrimitiveKind::Torus && values[0] <= values[1] {
            return Err("torus major_radius_mm must be greater than tube_radius_mm".into());
        }
        if kind == PrimitiveKind::TruncatedCone && values[0] == values[1] {
            return Err("truncated cone radii must differ; use a cylinder for equal radii".into());
        }
        if kind == PrimitiveKind::Pipe && values[1] >= values[0] {
            return Err("pipe inner_radius_mm must be smaller than outer_radius_mm".into());
        }
        if kind == PrimitiveKind::RoundBrilliant && values[2] >= values[0] {
            return Err(
                "round brilliant table_diameter_mm must be smaller than diameter_mm".into(),
            );
        }

        let mut parameters_mm = [LengthMm::zero(); 3];
        for (index, _) in names.iter().enumerate() {
            parameters_mm[index] = LengthMm::new(values[index])?;
        }
        Ok(Self {
            kind,
            parameters_mm,
            profile: if kind == PrimitiveKind::RevolvedSolid {
                default_revolved_profile()
            } else {
                PolygonProfile::default()
            },
            arc: kind.uses_arc().then(ArcFeature::default),
            line: kind.uses_line().then(LineFeature::default),
            open_polyline: if kind == PrimitiveKind::RevolvedSurface {
                Some(OpenPolylineFeature::revolved_surface_default())
            } else if kind == PrimitiveKind::RevolvedBezierSurface {
                Some(OpenPolylineFeature::revolved_bezier_default())
            } else if matches!(
                kind,
                PrimitiveKind::BezierCurve | PrimitiveKind::BezierSurface
            ) {
                Some(OpenPolylineFeature::bezier_default())
            } else {
                kind.uses_open_polyline().then(OpenPolylineFeature::default)
            },
            extrusion_direction: [0.0, 0.0, 1.0],
            transform: TransformParams::IDENTITY,
            material_density_g_cm3: None,
            boolean: None,
            pattern: None,
        })
    }

    fn with_transform(mut self, transform: TransformParams) -> Result<Self, Box<dyn Error>> {
        self.transform = transform.validate()?;
        Ok(self)
    }

    fn with_extrusion_direction(mut self, direction: [f64; 3]) -> Result<Self, Box<dyn Error>> {
        if !self.kind.uses_vector_extrusion() {
            return Err("extrusion direction applies only to an extrusion feature".into());
        }
        self.extrusion_direction = validate_extrusion_direction(direction)?;
        if matches!(
            self.kind,
            PrimitiveKind::ExtrudedPolygon
                | PrimitiveKind::ExtrudedCircle
                | PrimitiveKind::ExtrudedEllipse
        ) && direction[2].abs() <= direction[0].hypot(direction[1]).hypot(direction[2]) * 1e-12
        {
            return Err(
                "closed-profile solid extrusion direction must include a Z component".into(),
            );
        }
        Ok(self)
    }

    fn with_extrusion_direction_if_supported(
        self,
        direction: [f64; 3],
    ) -> Result<Self, Box<dyn Error>> {
        if self.kind.uses_vector_extrusion() {
            self.with_extrusion_direction(direction)
        } else {
            Ok(self)
        }
    }

    fn with_profile(mut self, profile: PolygonProfile) -> Result<Self, Box<dyn Error>> {
        if self.kind.uses_profile() {
            validate_polygon_profile(profile.points())?;
            self.profile = profile;
        }
        Ok(self)
    }

    fn with_arc(mut self, arc: Option<ArcFeature>) -> Result<Self, Box<dyn Error>> {
        if self.kind.uses_arc() {
            self.arc = Some(arc.unwrap_or_default());
        } else if arc.is_some() {
            return Err("arc angles can only be stored on an arc curve or surface".into());
        }
        Ok(self)
    }

    fn with_line(mut self, line: Option<LineFeature>) -> Result<Self, Box<dyn Error>> {
        if self.kind.uses_line() {
            let line = line.unwrap_or_default();
            self.line = Some(LineFeature::new(line.start_mm, line.end_mm)?);
        } else if line.is_some() {
            return Err("line endpoints can only be stored on a line curve".into());
        }
        Ok(self)
    }

    fn with_open_polyline(
        mut self,
        open_polyline: Option<OpenPolylineFeature>,
    ) -> Result<Self, Box<dyn Error>> {
        if self.kind.uses_open_polyline() {
            let feature = open_polyline.unwrap_or_default();
            if matches!(
                self.kind,
                PrimitiveKind::BezierCurve
                    | PrimitiveKind::BezierSurface
                    | PrimitiveKind::RevolvedBezierSurface
            ) && feature.point_count != 4
            {
                return Err("a cubic Bezier feature requires exactly four control points".into());
            }
            let feature = OpenPolylineFeature::from_points(feature.points())?;
            if self.kind == PrimitiveKind::RevolvedBezierSurface
                && feature.points().iter().any(|point| point[0] <= 1e-9)
            {
                return Err("revolved Bezier control points must stay at positive X radius".into());
            }
            self.open_polyline = Some(feature);
        } else if open_polyline.is_some() {
            return Err(
                "open polyline points can only be stored on an open path curve or surface".into(),
            );
        }
        Ok(self)
    }

    fn with_material_density(mut self, density_g_cm3: Option<f64>) -> Result<Self, Box<dyn Error>> {
        if density_g_cm3.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err("material density must be a finite positive value in g/cm³".into());
        }
        self.material_density_g_cm3 = density_g_cm3;
        Ok(self)
    }

    fn with_boolean(mut self, boolean: Option<BooleanFeature>) -> Self {
        self.boolean = boolean;
        self
    }

    fn with_pattern(mut self, pattern: Option<PatternFeature>) -> Self {
        self.pattern = pattern;
        self
    }

    fn values(self) -> [f64; 3] {
        self.parameters_mm.map(LengthMm::as_mm)
    }

    fn serialize(self) -> String {
        let mut content = format!(
            "OPENJEWELCAD_PROJECT=20\nfeature_type={}\n",
            self.kind.name()
        );
        for (index, name) in self.kind.parameter_names().iter().enumerate() {
            content.push_str(&format!(
                "{name}={:.17}\n",
                self.parameters_mm[index].as_mm()
            ));
        }
        if let Some(arc) = self.arc {
            content.push_str(&format!(
                "arc_start_angle_deg={:.17}\narc_sweep_angle_deg={:.17}\n",
                arc.start_angle_deg, arc.sweep_angle_deg
            ));
        }
        if let Some(line) = self.line {
            content.push_str(&format!(
                "line_start_x_mm={:.17}\nline_start_y_mm={:.17}\nline_end_x_mm={:.17}\nline_end_y_mm={:.17}\n",
                line.start_mm[0], line.start_mm[1], line.end_mm[0], line.end_mm[1]
            ));
        }
        if self.kind.uses_vector_extrusion() {
            content.push_str(&format!(
                "extrusion_direction_x={:.17}\nextrusion_direction_y={:.17}\nextrusion_direction_z={:.17}\n",
                self.extrusion_direction[0],
                self.extrusion_direction[1],
                self.extrusion_direction[2]
            ));
        }
        if let Some(open_polyline) = self.open_polyline {
            content.push_str(&format!(
                "open_polyline_point_count={}\n",
                open_polyline.point_count
            ));
            for (index, point) in open_polyline.points().iter().enumerate() {
                content.push_str(&format!(
                    "open_polyline_{index}_x_mm={:.17}\nopen_polyline_{index}_y_mm={:.17}\n",
                    point[0], point[1]
                ));
            }
        }
        if self.kind.uses_profile() {
            content.push_str(&format!(
                "profile_point_count={}\n",
                self.profile.point_count
            ));
            for (index, point) in self.profile.points().iter().enumerate() {
                content.push_str(&format!(
                    "profile_{index}_x_mm={:.17}\nprofile_{index}_y_mm={:.17}\n",
                    point[0], point[1]
                ));
            }
        }
        for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
            content.push_str(&format!(
                "translation_{axis}_mm={:.17}\nrotation_{axis}_deg={:.17}\n",
                self.transform.translation_mm[index], self.transform.rotation_deg[index]
            ));
        }
        content.push_str(&format!(
            "uniform_scale={:.17}\n",
            self.transform.uniform_scale
        ));
        for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
            content.push_str(&format!(
                "mirror_{axis}={}\n",
                self.transform.mirror_axes[index]
            ));
        }
        if let Some(density) = self.material_density_g_cm3 {
            content.push_str(&format!("material_density_g_cm3={density:.17}\n"));
        }
        if let Some(boolean) = self.boolean {
            content.push_str(&format!(
                "boolean_operation={}\ntool_feature_type={}\n",
                boolean.operation.name(),
                boolean.tool_kind.name()
            ));
            for (index, name) in boolean.tool_kind.parameter_names().iter().enumerate() {
                content.push_str(&format!(
                    "tool_{name}={:.17}\n",
                    boolean.tool_parameters_mm[index].as_mm()
                ));
            }
            for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
                content.push_str(&format!(
                    "tool_translation_{axis}_mm={:.17}\ntool_rotation_{axis}_deg={:.17}\n",
                    boolean.tool_transform.translation_mm[index],
                    boolean.tool_transform.rotation_deg[index]
                ));
            }
            content.push_str(&format!(
                "tool_uniform_scale={:.17}\n",
                boolean.tool_transform.uniform_scale
            ));
            for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
                content.push_str(&format!(
                    "tool_mirror_{axis}={}\n",
                    boolean.tool_transform.mirror_axes[index]
                ));
            }
        }
        if let Some(pattern) = self.pattern {
            content.push_str(&format!(
                "array_mode={}\narray_copies={}\n",
                pattern.mode.name(),
                pattern.copies
            ));
            match pattern.mode {
                PatternMode::Linear => {
                    for (axis, value) in ["x", "y", "z"].into_iter().zip(pattern.step_mm) {
                        content.push_str(&format!("array_step_{axis}_mm={value:.17}\n"));
                    }
                }
                PatternMode::PolarZ => {
                    content.push_str(&format!(
                        "array_angle_step_deg={:.17}\n",
                        pattern.angle_step_deg
                    ));
                }
            }
        }
        content
    }

    fn parse(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut fields = BTreeMap::new();
        for line in content.lines() {
            let (key, value) = line.split_once('=').ok_or("project has a malformed line")?;
            if fields.insert(key, value).is_some() {
                return Err(format!("duplicate project field: {key}").into());
            }
        }
        let version = fields
            .remove("OPENJEWELCAD_PROJECT")
            .ok_or("unsupported or missing project version")?
            .parse::<u32>()
            .map_err(|_| "invalid project version")?;
        if !(1..=20).contains(&version) {
            return Err("unsupported or missing project version".into());
        }
        let kind_name = fields
            .remove("feature_type")
            .ok_or("project is missing feature_type")?;
        let kind = PrimitiveKind::from_name(kind_name)
            .ok_or_else(|| format!("unsupported feature type: {kind_name}"))?;
        if kind == PrimitiveKind::ExtrudedPolygon && version < 5 {
            return Err("extruded polygon projects require version 5 or newer".into());
        }
        if matches!(
            kind,
            PrimitiveKind::PolylineCurve | PrimitiveKind::PlanarSurface
        ) && version < 6
        {
            return Err("curve and surface projects require version 6 or newer".into());
        }
        if matches!(
            kind,
            PrimitiveKind::CircleCurve
                | PrimitiveKind::PlanarCircleSurface
                | PrimitiveKind::ExtrudedCircle
        ) && version < 7
        {
            return Err("circle profile projects require version 7 or newer".into());
        }
        if matches!(
            kind,
            PrimitiveKind::EllipseCurve
                | PrimitiveKind::PlanarEllipseSurface
                | PrimitiveKind::ExtrudedEllipse
        ) && version < 8
        {
            return Err("ellipse profile projects require version 8 or newer".into());
        }
        if kind == PrimitiveKind::LineCurve && version < 10 {
            return Err("line curve projects require version 10 or newer".into());
        }
        if kind == PrimitiveKind::OpenPolylineCurve && version < 11 {
            return Err("open polyline projects require version 11 or newer".into());
        }
        if kind == PrimitiveKind::RevolvedSolid && version < 12 {
            return Err("revolved solid projects require version 12 or newer".into());
        }
        if matches!(
            kind,
            PrimitiveKind::LineSurface | PrimitiveKind::OpenPolylineSurface
        ) && version < 13
        {
            return Err("open-curve surface projects require version 13 or newer".into());
        }
        if kind == PrimitiveKind::ArcCurve && version < 9 {
            return Err("arc curve projects require version 9 or newer".into());
        }
        if kind == PrimitiveKind::ArcSurface && version < 14 {
            return Err("arc surface projects require version 14 or newer".into());
        }
        if kind == PrimitiveKind::RevolvedSurface && version < 15 {
            return Err("revolved surface projects require version 15 or newer".into());
        }
        if matches!(
            kind,
            PrimitiveKind::BezierCurve | PrimitiveKind::BezierSurface
        ) && version < 19
        {
            return Err("Bezier projects require version 19 or newer".into());
        }
        if kind == PrimitiveKind::RevolvedBezierSurface && version < 20 {
            return Err("revolved Bezier surface projects require version 20 or newer".into());
        }
        let mut values = [0.0; 3];
        for (index, name) in kind.parameter_names().iter().enumerate() {
            let value = fields
                .remove(name)
                .ok_or_else(|| format!("project is missing {name}"))?;
            values[index] = value
                .parse::<f64>()
                .map_err(|_| format!("invalid {name}"))?;
        }
        let arc = if kind.uses_arc() {
            let start = fields
                .remove("arc_start_angle_deg")
                .ok_or("project is missing arc_start_angle_deg")?
                .parse::<f64>()
                .map_err(|_| "invalid arc_start_angle_deg")?;
            let sweep = fields
                .remove("arc_sweep_angle_deg")
                .ok_or("project is missing arc_sweep_angle_deg")?
                .parse::<f64>()
                .map_err(|_| "invalid arc_sweep_angle_deg")?;
            Some(ArcFeature::new(start, sweep)?)
        } else {
            None
        };
        let line = if kind.uses_line() {
            let mut point = |key: &str| -> Result<[f64; 2], Box<dyn Error>> {
                let x_key = format!("line_{key}_x_mm");
                let y_key = format!("line_{key}_y_mm");
                let x = fields
                    .remove(x_key.as_str())
                    .ok_or_else(|| format!("project is missing {x_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {x_key}"))?;
                let y = fields
                    .remove(y_key.as_str())
                    .ok_or_else(|| format!("project is missing {y_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {y_key}"))?;
                Ok([x, y])
            };
            Some(LineFeature::new(point("start")?, point("end")?)?)
        } else {
            None
        };
        let open_polyline = if kind.uses_open_polyline() {
            let count = fields
                .remove("open_polyline_point_count")
                .ok_or("project is missing open_polyline_point_count")?
                .parse::<usize>()
                .map_err(|_| "invalid open_polyline_point_count")?;
            if !(2..=MAX_PROFILE_POINTS).contains(&count) {
                return Err(
                    format!("open_polyline_point_count must be 2..={MAX_PROFILE_POINTS}").into(),
                );
            }
            if matches!(
                kind,
                PrimitiveKind::BezierCurve
                    | PrimitiveKind::BezierSurface
                    | PrimitiveKind::RevolvedBezierSurface
            ) && count != 4
            {
                return Err("Bezier projects must contain exactly four control points".into());
            }
            let mut points = Vec::with_capacity(count);
            for index in 0..count {
                let x_key = format!("open_polyline_{index}_x_mm");
                let y_key = format!("open_polyline_{index}_y_mm");
                let x = fields
                    .remove(x_key.as_str())
                    .ok_or_else(|| format!("project is missing {x_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {x_key}"))?;
                let y = fields
                    .remove(y_key.as_str())
                    .ok_or_else(|| format!("project is missing {y_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {y_key}"))?;
                points.push([x, y]);
            }
            Some(OpenPolylineFeature::from_points(&points)?)
        } else {
            None
        };
        let transform = if version >= 2 {
            parse_transform_fields(&mut fields, "", version)?
        } else {
            TransformParams::IDENTITY
        };
        let extrusion_direction = if let Some(direction_version) =
            kind.vector_extrusion_project_version()
        {
            let keys = [
                "extrusion_direction_x",
                "extrusion_direction_y",
                "extrusion_direction_z",
            ];
            let present = keys.map(|key| fields.contains_key(key));
            if version >= direction_version && present.iter().any(|present| !present) {
                return Err("project is missing an extrusion direction component".into());
            }
            if present.iter().any(|present| *present) && present.iter().any(|present| !present) {
                return Err("project has incomplete extrusion direction data".into());
            }
            if present[0] {
                let mut direction = [0.0; 3];
                for (index, key) in keys.into_iter().enumerate() {
                    direction[index] = fields
                        .remove(key)
                        .expect("direction keys were checked above")
                        .parse::<f64>()
                        .map_err(|_| format!("invalid {key}"))?;
                }
                let direction = validate_extrusion_direction(direction)?;
                if version < direction_version && direction != [0.0, 0.0, 1.0] {
                    return Err(format!(
                        "custom extrusion directions for {} require project version {direction_version}",
                        kind.name()
                    )
                    .into());
                }
                direction
            } else {
                [0.0, 0.0, 1.0]
            }
        } else {
            [0.0, 0.0, 1.0]
        };
        let density_g_cm3 = if version >= 2 {
            fields
                .remove("material_density_g_cm3")
                .map(|value| {
                    value
                        .parse::<f64>()
                        .map_err(|_| "invalid material_density_g_cm3")
                })
                .transpose()?
        } else {
            None
        };
        let boolean = if version >= 3 {
            fields
                .remove("boolean_operation")
                .map(|operation_name| {
                    let operation = BooleanOperation::parse(operation_name)?;
                    let tool_kind_name = fields
                        .remove("tool_feature_type")
                        .ok_or("project is missing tool_feature_type")?;
                    let tool_kind = PrimitiveKind::from_name(tool_kind_name).ok_or_else(|| {
                        format!("unsupported tool feature type: {tool_kind_name}")
                    })?;
                    let mut tool_values = [0.0; 3];
                    for (index, name) in tool_kind.parameter_names().iter().enumerate() {
                        let key = format!("tool_{name}");
                        tool_values[index] = fields
                            .remove(key.as_str())
                            .ok_or_else(|| format!("project is missing {key}"))?
                            .parse::<f64>()
                            .map_err(|_| format!("invalid {key}"))?;
                    }
                    let tool_transform = parse_transform_fields(&mut fields, "tool_", version)?;
                    BooleanFeature::new(operation, tool_kind, tool_values, tool_transform)
                })
                .transpose()?
        } else {
            None
        };
        if boolean.is_some() && !kind.is_solid() {
            return Err("boolean features require a solid base feature".into());
        }
        let pattern = if version >= 4 {
            fields
                .remove("array_mode")
                .map(|mode_name| {
                    let copies = fields
                        .remove("array_copies")
                        .ok_or("project is missing array_copies")?
                        .parse::<u32>()
                        .map_err(|_| "invalid array_copies")?;
                    match mode_name {
                        "linear" => {
                            let mut step = [0.0; 3];
                            for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
                                let key = format!("array_step_{axis}_mm");
                                step[index] = fields
                                    .remove(key.as_str())
                                    .ok_or_else(|| format!("project is missing {key}"))?
                                    .parse::<f64>()
                                    .map_err(|_| format!("invalid {key}"))?;
                            }
                            PatternFeature::linear(copies, step)
                        }
                        "polar-z" => {
                            let angle = fields
                                .remove("array_angle_step_deg")
                                .ok_or("project is missing array_angle_step_deg")?
                                .parse::<f64>()
                                .map_err(|_| "invalid array_angle_step_deg")?;
                            PatternFeature::polar_z(copies, angle)
                        }
                        _ => Err("array_mode must be linear or polar-z".into()),
                    }
                })
                .transpose()?
        } else {
            None
        };
        let profile = if kind.uses_profile() {
            let count = fields
                .remove("profile_point_count")
                .ok_or("project is missing profile_point_count")?
                .parse::<usize>()
                .map_err(|_| "invalid profile_point_count")?;
            if !(3..=MAX_PROFILE_POINTS).contains(&count) {
                return Err(format!("profile_point_count must be 3..={MAX_PROFILE_POINTS}").into());
            }
            let mut points = Vec::with_capacity(count);
            for index in 0..count {
                let x_key = format!("profile_{index}_x_mm");
                let y_key = format!("profile_{index}_y_mm");
                let x = fields
                    .remove(x_key.as_str())
                    .ok_or_else(|| format!("project is missing {x_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {x_key}"))?;
                let y = fields
                    .remove(y_key.as_str())
                    .ok_or_else(|| format!("project is missing {y_key}"))?
                    .parse::<f64>()
                    .map_err(|_| format!("invalid {y_key}"))?;
                points.push([x, y]);
            }
            PolygonProfile::from_points(&points)?
        } else {
            PolygonProfile::default()
        };
        if let Some(key) = fields.keys().next() {
            return Err(format!("unknown project field: {key}").into());
        }
        let model = Self::new(kind, values)?
            .with_transform(transform)?
            .with_extrusion_direction_if_supported(extrusion_direction)?
            .with_material_density(density_g_cm3)?
            .with_profile(profile)?
            .with_arc(arc)?
            .with_line(line)?
            .with_open_polyline(open_polyline)?;
        Ok(model.with_boolean(boolean).with_pattern(pattern))
    }

    fn create_shape(self, kernel: &Kernel) -> Result<Shape, openjewelcad_cad_ffi::KernelError> {
        let base = create_primitive_shape(
            self.kind,
            self.values(),
            self.transform,
            self.profile,
            self.arc,
            self.line,
            self.open_polyline,
            self.extrusion_direction,
            kernel,
        )?;
        let mut result = base;
        if let Some(boolean) = self.boolean {
            let tool = create_primitive_shape(
                boolean.tool_kind,
                boolean.values_mm(),
                boolean.tool_transform,
                PolygonProfile::default(),
                None,
                None,
                None,
                [0.0, 0.0, 1.0],
                kernel,
            )?;
            result = result.boolean_with(&tool, boolean.operation.kernel_operation(), 1e-7)?;
        }
        if let Some(pattern) = self.pattern {
            result = match pattern.mode {
                PatternMode::Linear => result.linear_array(pattern.copies, pattern.step_mm)?,
                PatternMode::PolarZ => {
                    result.polar_array_z(pattern.copies, pattern.angle_step_deg)?
                }
            };
        }
        Ok(result)
    }
}

fn parse_transform_fields(
    fields: &mut BTreeMap<&str, &str>,
    prefix: &str,
    version: u32,
) -> Result<TransformParams, Box<dyn Error>> {
    let mut transform = TransformParams::IDENTITY;
    for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
        let translation_key = format!("{prefix}translation_{axis}_mm");
        let rotation_key = format!("{prefix}rotation_{axis}_deg");
        transform.translation_mm[index] = fields
            .remove(translation_key.as_str())
            .ok_or_else(|| format!("project is missing {translation_key}"))?
            .parse::<f64>()
            .map_err(|_| format!("invalid {translation_key}"))?;
        transform.rotation_deg[index] = fields
            .remove(rotation_key.as_str())
            .ok_or_else(|| format!("project is missing {rotation_key}"))?
            .parse::<f64>()
            .map_err(|_| format!("invalid {rotation_key}"))?;
    }
    let scale_key = format!("{prefix}uniform_scale");
    transform.uniform_scale = fields
        .remove(scale_key.as_str())
        .ok_or_else(|| format!("project is missing {scale_key}"))?
        .parse::<f64>()
        .map_err(|_| format!("invalid {scale_key}"))?;
    for (index, axis) in ["x", "y", "z"].into_iter().enumerate() {
        let key = format!("{prefix}mirror_{axis}");
        transform.mirror_axes[index] = match fields.remove(key.as_str()) {
            Some("true") => {
                if version < 16 {
                    return Err(format!("{key} requires project version 16 or newer").into());
                }
                true
            }
            Some("false") => false,
            Some(_) => return Err(format!("invalid {key}; expected true or false").into()),
            None if version >= 16 => return Err(format!("project is missing {key}").into()),
            None => false,
        };
    }
    transform.validate()
}

fn create_primitive_shape(
    kind: PrimitiveKind,
    values: [f64; 3],
    transform: TransformParams,
    profile: PolygonProfile,
    arc: Option<ArcFeature>,
    line: Option<LineFeature>,
    open_polyline: Option<OpenPolylineFeature>,
    extrusion_direction: [f64; 3],
    kernel: &Kernel,
) -> Result<Shape, openjewelcad_cad_ffi::KernelError> {
    let shape = match kind {
        PrimitiveKind::Box => kernel.box_mm(values[0], values[1], values[2]),
        PrimitiveKind::Cylinder => kernel.cylinder_mm(values[0], values[1]),
        PrimitiveKind::Cone => kernel.cone_mm(values[0], values[1]),
        PrimitiveKind::TruncatedCone => kernel.truncated_cone_mm(values[0], values[1], values[2]),
        PrimitiveKind::Ellipsoid => kernel.ellipsoid_mm(values[0], values[1], values[2]),
        PrimitiveKind::Pipe => kernel.pipe_mm(values[0], values[1], values[2]),
        PrimitiveKind::Sphere => kernel.sphere_mm(values[0]),
        PrimitiveKind::Torus => kernel.torus_mm(values[0], values[1]),
        PrimitiveKind::RingBand => {
            let inner_radius = values[0] * 0.5;
            kernel.pipe_mm(inner_radius + values[1], inner_radius, values[2])
        }
        PrimitiveKind::RoundBrilliant => kernel.brilliant_gem_mm(values[0], values[1], values[2]),
        PrimitiveKind::LineCurve => {
            let line = line.unwrap_or_default();
            kernel.line_curve_mm(line.start_mm, line.end_mm)
        }
        PrimitiveKind::LineSurface => {
            let line = line.unwrap_or_default();
            kernel.extruded_open_polyline_surface_vector_mm(
                &[line.start_mm, line.end_mm],
                values[0],
                extrusion_direction,
            )
        }
        PrimitiveKind::OpenPolylineCurve => {
            let open_polyline = open_polyline.unwrap_or_default();
            kernel.open_polyline_curve_mm(open_polyline.points())
        }
        PrimitiveKind::OpenPolylineSurface => {
            let open_polyline = open_polyline.unwrap_or_default();
            kernel.extruded_open_polyline_surface_vector_mm(
                open_polyline.points(),
                values[0],
                extrusion_direction,
            )
        }
        PrimitiveKind::BezierCurve => {
            let bezier = open_polyline.unwrap_or_else(OpenPolylineFeature::bezier_default);
            kernel.bezier_curve_mm(bezier.points())
        }
        PrimitiveKind::BezierSurface => {
            let bezier = open_polyline.unwrap_or_else(OpenPolylineFeature::bezier_default);
            kernel.extruded_bezier_surface_vector_mm(
                bezier.points(),
                values[0],
                extrusion_direction,
            )
        }
        PrimitiveKind::RevolvedBezierSurface => {
            let bezier = open_polyline.unwrap_or_else(OpenPolylineFeature::revolved_bezier_default);
            kernel.revolved_bezier_surface_mm(bezier.points())
        }
        PrimitiveKind::RevolvedSurface => {
            let open_polyline = open_polyline.unwrap_or_default();
            kernel.revolved_open_polyline_surface_mm(open_polyline.points())
        }
        PrimitiveKind::PolylineCurve => kernel.closed_polyline_mm(profile.points()),
        PrimitiveKind::ArcCurve => {
            let arc = arc.unwrap_or_default();
            kernel.arc_curve_mm(values[0], arc.start_angle_deg, arc.sweep_angle_deg)
        }
        PrimitiveKind::ArcSurface => {
            let arc = arc.unwrap_or_default();
            kernel.extruded_arc_surface_vector_mm(
                values[0],
                arc.start_angle_deg,
                arc.sweep_angle_deg,
                values[1],
                extrusion_direction,
            )
        }
        PrimitiveKind::CircleCurve => kernel.circle_curve_mm(values[0]),
        PrimitiveKind::EllipseCurve => kernel.ellipse_curve_mm(values[0], values[1]),
        PrimitiveKind::PlanarSurface => kernel.planar_polygon_surface_mm(profile.points()),
        PrimitiveKind::PlanarCircleSurface => kernel.planar_circle_surface_mm(values[0]),
        PrimitiveKind::PlanarEllipseSurface => {
            kernel.planar_ellipse_surface_mm(values[0], values[1])
        }
        PrimitiveKind::ExtrudedPolygon => {
            kernel.extruded_polygon_vector_mm(profile.points(), values[0], extrusion_direction)
        }
        PrimitiveKind::ExtrudedCircle => {
            kernel.extruded_circle_vector_mm(values[0], values[1], extrusion_direction)
        }
        PrimitiveKind::ExtrudedEllipse => {
            kernel.extruded_ellipse_vector_mm(values[0], values[1], values[2], extrusion_direction)
        }
        PrimitiveKind::RevolvedSolid => kernel.revolved_polygon_mm(profile.points()),
    }?;
    let shape = if transform.translation_mm == [0.0; 3]
        && transform.rotation_deg == [0.0; 3]
        && transform.uniform_scale == 1.0
    {
        shape
    } else {
        shape.transformed_mm_degrees(
            transform.translation_mm,
            transform.rotation_deg,
            transform.uniform_scale,
        )?
    };
    let plane_mask = transform
        .mirror_axes
        .into_iter()
        .enumerate()
        .fold(0_u32, |mask, (axis, mirrored)| {
            mask | (u32::from(mirrored) << axis)
        });
    if plane_mask == 0 {
        Ok(shape)
    } else {
        shape.mirrored(plane_mask)
    }
}

fn save_model(path: &Path, model: PrimitiveModel) -> Result<(), Box<dyn Error>> {
    let temporary = temporary_path(path);
    fs::write(&temporary, model.serialize())?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(name)
}

fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // REPLACE_EXISTING | WRITE_THROUGH keeps saves atomic on the same volume.
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }
}

fn load_model(path: &Path) -> Result<PrimitiveModel, Box<dyn Error>> {
    PrimitiveModel::parse(&fs::read_to_string(path)?)
}

fn create_or_edit(
    kind: PrimitiveKind,
    action: &str,
    file: &str,
    values: [f64; 3],
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != kind {
            return Err(format!(
                "cannot edit a {} project with the {} command",
                existing.kind.name(),
                kind.name()
            )
            .into());
        }
        Some(existing)
    } else {
        None
    };
    let mut model = PrimitiveModel::new(kind, values)?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.material_density_g_cm3 = existing.material_density_g_cm3;
        model.boolean = existing.boolean;
        model.pattern = existing.pattern;
        model.profile = existing.profile;
        model.arc = existing.arc;
        model.line = existing.line;
        model.open_polyline = existing.open_polyline;
    }
    save_model(path, model)?;
    inspect(path)
}

fn create_or_edit_arc(
    action: &str,
    file: &str,
    radius_mm: f64,
    arc: ArcFeature,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != PrimitiveKind::ArcCurve {
            return Err("arc-curve edit can only update an arc curve project".into());
        }
        Some(existing)
    } else {
        None
    };
    let mut model =
        PrimitiveModel::new(PrimitiveKind::ArcCurve, [radius_mm, 0.0, 0.0])?.with_arc(Some(arc))?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.profile = existing.profile;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("arc parameters produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn create_or_edit_line(
    action: &str,
    file: &str,
    start_mm: [f64; 2],
    end_mm: [f64; 2],
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != PrimitiveKind::LineCurve {
            return Err("line-curve edit can only update a line curve project".into());
        }
        Some(existing)
    } else {
        None
    };
    let line = LineFeature::new(start_mm, end_mm)?;
    let mut model =
        PrimitiveModel::new(PrimitiveKind::LineCurve, [0.0; 3])?.with_line(Some(line))?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("line endpoints produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn create_or_edit_open_polyline(action: &str, file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != PrimitiveKind::OpenPolylineCurve {
            return Err("open-polyline-curve edit can only update an open polyline project".into());
        }
        Some(existing)
    } else {
        None
    };
    let mut model = PrimitiveModel::new(PrimitiveKind::OpenPolylineCurve, [0.0; 3])?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.open_polyline = existing.open_polyline;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err(
            "open polyline points produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, model)?;
    inspect(path)
}

fn create_or_edit_bezier(
    kind: PrimitiveKind,
    action: &str,
    file: &str,
    height_mm: f64,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != kind {
            return Err(format!(
                "{} edit can only update a matching Bezier project",
                kind.name()
            )
            .into());
        }
        Some(existing)
    } else {
        None
    };
    let values = if kind == PrimitiveKind::BezierSurface {
        [height_mm, 0.0, 0.0]
    } else {
        [0.0; 3]
    };
    let mut model = PrimitiveModel::new(kind, values)?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.open_polyline = existing.open_polyline;
        model.extrusion_direction = existing.extrusion_direction;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("Bezier feature produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_bezier_surface(file: &str, height_mm: f64) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let source = load_model(path)?;
    if source.kind != PrimitiveKind::BezierCurve {
        return Err("choose a cubic Bezier curve before creating its surface".into());
    }
    let mut surface = PrimitiveModel::new(PrimitiveKind::BezierSurface, [height_mm, 0.0, 0.0])?;
    surface.open_polyline = source.open_polyline;
    surface.transform = source.transform;
    surface.extrusion_direction = source.extrusion_direction;
    let kernel = Kernel::from_environment()?;
    if !surface.create_shape(&kernel)?.is_valid()? {
        return Err("Bezier extrusion produced invalid geometry; project was not changed".into());
    }
    save_model(path, surface)?;
    inspect(path)
}

fn create_or_edit_revolved_bezier(action: &str, file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != PrimitiveKind::RevolvedBezierSurface {
            return Err("revolved Bezier surface edit requires a matching project".into());
        }
        Some(existing)
    } else {
        None
    };
    let mut model = PrimitiveModel::new(PrimitiveKind::RevolvedBezierSurface, [0.0; 3])?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.open_polyline = existing.open_polyline;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err(
            "revolved Bezier profile produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_revolved_bezier_surface(file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let source = load_model(path)?;
    if source.kind != PrimitiveKind::BezierCurve {
        return Err("choose a cubic Bezier curve before revolving it into a surface".into());
    }
    let mut surface = PrimitiveModel::new(PrimitiveKind::RevolvedBezierSurface, [0.0; 3])?;
    surface.open_polyline = source.open_polyline;
    surface.transform = source.transform;
    let kernel = Kernel::from_environment()?;
    if !surface.create_shape(&kernel)?.is_valid()? {
        return Err(
            "revolved Bezier profile produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, surface)?;
    inspect(path)
}

fn update_open_polyline_project(
    file: &str,
    open_polyline: OpenPolylineFeature,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.uses_open_polyline() {
        return Err("open path editing requires an open polyline curve or surface project".into());
    }
    model = model.with_open_polyline(Some(open_polyline))?;
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("open polyline edit produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn set_open_polyline_point(
    file: &str,
    index: usize,
    point_mm: [f64; 2],
) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    let mut feature = model
        .open_polyline
        .ok_or("project does not contain an open polyline")?;
    if index >= usize::from(feature.point_count) {
        return Err("open polyline vertex index is out of range".into());
    }
    feature.set_point_unchecked(index, point_mm);
    update_open_polyline_project(file, feature)
}

fn add_open_polyline_point(file: &str) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    let mut feature = model
        .open_polyline
        .ok_or("project does not contain an open polyline")?;
    feature.add_point()?;
    update_open_polyline_project(file, feature)
}

fn remove_open_polyline_point(file: &str, index: usize) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    let mut feature = model
        .open_polyline
        .ok_or("project does not contain an open polyline")?;
    feature.remove_point(index)?;
    update_open_polyline_project(file, feature)
}

fn reverse_curve_project(file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    match model.kind {
        PrimitiveKind::LineCurve => {
            let line = model
                .line
                .as_mut()
                .ok_or("line curve project is missing its endpoints")?;
            std::mem::swap(&mut line.start_mm, &mut line.end_mm);
        }
        PrimitiveKind::OpenPolylineCurve => {
            model
                .open_polyline
                .as_mut()
                .ok_or("open polyline project is missing its path")?
                .reverse();
        }
        PrimitiveKind::PolylineCurve => {
            let reversed = model
                .profile
                .points()
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>();
            model.profile = PolygonProfile::from_points(&reversed)?;
        }
        _ => {
            return Err(
                "curve-reverse supports line, open polyline, and closed polyline curve projects"
                    .into(),
            )
        }
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err(
            "reversing the curve produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, model)?;
    inspect(path)
}

fn close_curve_project(file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if model.kind != PrimitiveKind::OpenPolylineCurve {
        return Err("curve-close requires an open polyline curve project".into());
    }
    let feature = model
        .open_polyline
        .ok_or("open polyline project is missing its path")?;
    model.profile = closed_profile_from_open(feature)?;
    model.kind = PrimitiveKind::PolylineCurve;
    model.open_polyline = None;
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("closing the curve produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn update_profile_project(file: &str, profile: PolygonProfile) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.uses_profile() {
        return Err(
            "profile editing is available for curve, surface, and extrusion projects".into(),
        );
    }
    validate_polygon_profile(profile.points())?;
    model.profile = profile;
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("profile edit produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn set_profile_point(file: &str, index: usize, point_mm: [f64; 2]) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    if !model.kind.uses_profile() || index >= usize::from(model.profile.point_count) {
        return Err("profile vertex index is out of range".into());
    }
    let mut points = model.profile.points().to_vec();
    points[index] = point_mm;
    update_profile_project(file, PolygonProfile::from_points(&points)?)
}

fn add_profile_point(file: &str) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    if !model.kind.uses_profile() {
        return Err(
            "profile editing is available for curve, surface, and extrusion projects".into(),
        );
    }
    let mut profile = model.profile;
    profile.add_vertex()?;
    update_profile_project(file, profile)
}

fn remove_profile_point(file: &str, index: usize) -> Result<(), Box<dyn Error>> {
    let model = load_model(Path::new(file))?;
    if !model.kind.uses_profile() {
        return Err(
            "profile editing is available for curve, surface, and extrusion projects".into(),
        );
    }
    let mut profile = model.profile;
    profile.remove_vertex(index)?;
    update_profile_project(file, profile)
}

fn transform_project(
    file: &str,
    translation_mm: [f64; 3],
    rotation_deg: [f64; 3],
    scale: f64,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    model.transform = TransformParams {
        translation_mm,
        rotation_deg,
        uniform_scale: scale,
        ..model.transform
    }
    .validate()?;
    save_model(path, model)?;
    inspect(path)
}

fn mirror_project(file: &str, plane: &str, state: &str) -> Result<(), Box<dyn Error>> {
    let plane_index = match plane.to_ascii_lowercase().as_str() {
        "yz" => 0,
        "xz" => 1,
        "xy" => 2,
        _ => return Err("mirror plane must be YZ, XZ, or XY".into()),
    };
    let enabled = match state.to_ascii_lowercase().as_str() {
        "on" | "true" => true,
        "off" | "false" => false,
        _ => return Err("mirror state must be on or off".into()),
    };
    let path = Path::new(file);
    let mut model = load_model(path)?;
    model.transform.mirror_axes[plane_index] = enabled;
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err("mirror transform produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn set_extrusion_direction_project(file: &str, direction: [f64; 3]) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.uses_vector_extrusion() {
        return Err(
            "extrusion direction can only be set on a surface or extruded solid feature".into(),
        );
    }
    model.extrusion_direction = validate_extrusion_direction(direction)?;
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err(
            "extrusion direction produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_profile_stage(
    file: &str,
    kind: PrimitiveKind,
    extrusion_height_mm: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if !kind.uses_profile() {
        return Err("target feature must use an editable profile".into());
    }
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.uses_profile() {
        return Err("open a profile curve or surface before converting it".into());
    }
    let values = if kind == PrimitiveKind::ExtrudedPolygon {
        [extrusion_height_mm.unwrap_or(5.0), 0.0, 0.0]
    } else {
        kind.defaults()
    };
    model.parameters_mm = PrimitiveModel::new(kind, values)?.parameters_mm;
    model.kind = kind;
    if !kind.is_solid() {
        model.boolean = None;
        model.pattern = None;
        model.material_density_g_cm3 = None;
    }
    validate_polygon_profile(model.profile.points())?;
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("profile conversion produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_open_curve_surface(
    file: &str,
    surface_kind: PrimitiveKind,
    height_mm: f64,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let source = load_model(path)?;
    let mut surface = match surface_kind {
        PrimitiveKind::LineSurface if source.kind == PrimitiveKind::LineCurve => {
            let mut surface = PrimitiveModel::new(surface_kind, [height_mm, 0.0, 0.0])?;
            surface.line = source.line;
            surface
        }
        PrimitiveKind::OpenPolylineSurface if source.kind == PrimitiveKind::OpenPolylineCurve => {
            let mut surface = PrimitiveModel::new(surface_kind, [height_mm, 0.0, 0.0])?;
            surface.open_polyline = source.open_polyline;
            surface
        }
        PrimitiveKind::LineSurface | PrimitiveKind::OpenPolylineSurface => {
            return Err(
                "choose a matching open line or open polyline curve before creating its surface"
                    .into(),
            );
        }
        _ => return Err("target must be an open-curve surface tool".into()),
    };
    surface.transform = source.transform;
    surface.extrusion_direction = source.extrusion_direction;
    let kernel = Kernel::from_environment()?;
    if !surface.create_shape(&kernel)?.is_valid()? {
        return Err(
            "open-curve extrusion produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, surface)?;
    inspect(path)
}

fn convert_arc_surface(file: &str, height_mm: f64) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let source = load_model(path)?;
    if source.kind != PrimitiveKind::ArcCurve {
        return Err("choose an arc curve before creating its surface".into());
    }
    let mut surface = PrimitiveModel::new(
        PrimitiveKind::ArcSurface,
        [source.parameters_mm[0].as_mm(), height_mm, 0.0],
    )?
    .with_arc(source.arc)?;
    surface.transform = source.transform;
    surface.extrusion_direction = source.extrusion_direction;
    let kernel = Kernel::from_environment()?;
    if !surface.create_shape(&kernel)?.is_valid()? {
        return Err("arc extrusion produced invalid geometry; project was not changed".into());
    }
    save_model(path, surface)?;
    inspect(path)
}

fn convert_revolved_surface(file: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let source = load_model(path)?;
    if source.kind != PrimitiveKind::OpenPolylineCurve {
        return Err("choose an open polyline curve before revolving it into a surface".into());
    }
    let mut surface = PrimitiveModel::new(PrimitiveKind::RevolvedSurface, [0.0; 3])?;
    surface.open_polyline = source.open_polyline;
    surface.transform = source.transform;
    let kernel = Kernel::from_environment()?;
    if !surface.create_shape(&kernel)?.is_valid()? {
        return Err(
            "revolved profile produced invalid surface geometry; project was not changed".into(),
        );
    }
    save_model(path, surface)?;
    inspect(path)
}

fn create_or_edit_arc_surface(
    action: &str,
    file: &str,
    radius_mm: f64,
    height_mm: f64,
    arc: ArcFeature,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let existing = if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != PrimitiveKind::ArcSurface {
            return Err("arc-surface edit can only update an arc surface project".into());
        }
        Some(existing)
    } else {
        None
    };
    let mut model = PrimitiveModel::new(PrimitiveKind::ArcSurface, [radius_mm, height_mm, 0.0])?
        .with_arc(Some(arc))?;
    if let Some(existing) = existing {
        model.transform = existing.transform;
        model.extrusion_direction = existing.extrusion_direction;
    }
    let kernel = Kernel::from_environment()?;
    if !model.create_shape(&kernel)?.is_valid()? {
        return Err(
            "arc surface parameters produced invalid geometry; project was not changed".into(),
        );
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_circle_stage(
    file: &str,
    kind: PrimitiveKind,
    extrusion_height_mm: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if !kind.is_circle_stage() {
        return Err("target feature must use a circular profile".into());
    }
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.is_circle_stage() {
        return Err("open a circle curve or surface before converting it".into());
    }
    let radius = model.parameters_mm[0].as_mm();
    let values = if kind == PrimitiveKind::ExtrudedCircle {
        [radius, extrusion_height_mm.unwrap_or(5.0), 0.0]
    } else {
        [radius, 0.0, 0.0]
    };
    model.parameters_mm = PrimitiveModel::new(kind, values)?.parameters_mm;
    model.kind = kind;
    if !kind.is_solid() {
        model.boolean = None;
        model.pattern = None;
        model.material_density_g_cm3 = None;
    }
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("circle conversion produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn convert_ellipse_stage(
    file: &str,
    kind: PrimitiveKind,
    extrusion_height_mm: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if !kind.is_ellipse_stage() {
        return Err("target feature must use an ellipse profile".into());
    }
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.is_ellipse_stage() {
        return Err("open an ellipse curve or surface before converting it".into());
    }
    let radii = [
        model.parameters_mm[0].as_mm(),
        model.parameters_mm[1].as_mm(),
    ];
    let values = if kind == PrimitiveKind::ExtrudedEllipse {
        [radii[0], radii[1], extrusion_height_mm.unwrap_or(5.0)]
    } else {
        [radii[0], radii[1], 0.0]
    };
    model.parameters_mm = PrimitiveModel::new(kind, values)?.parameters_mm;
    model.kind = kind;
    if !kind.is_solid() {
        model.boolean = None;
        model.pattern = None;
        model.material_density_g_cm3 = None;
    }
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("ellipse conversion produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn boolean_project(
    file: &str,
    operation: BooleanOperation,
    tool_kind: PrimitiveKind,
    tool_values_mm: [f64; 3],
    tool_translation_mm: [f64; 3],
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    if !model.kind.is_solid() {
        return Err("boolean operations require a solid base feature".into());
    }
    model.boolean = Some(BooleanFeature::new(
        operation,
        tool_kind,
        tool_values_mm,
        TransformParams {
            translation_mm: tool_translation_mm,
            ..TransformParams::IDENTITY
        },
    )?);
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("boolean operation produced an invalid solid; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn pattern_project(file: &str, pattern: PatternFeature) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    let mut model = load_model(path)?;
    model.pattern = Some(pattern);
    let kernel = Kernel::from_environment()?;
    let result = model.create_shape(&kernel)?;
    if !result.is_valid()? {
        return Err("array produced invalid geometry; project was not changed".into());
    }
    save_model(path, model)?;
    inspect(path)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => launch_gui(),
        [command] if command == "gui" => launch_gui(),
        [command] if command == "--help" || command == "help" => {
            print_help();
            Ok(())
        }
        [command, action, file, a, b, c]
            if command == "box" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::Box,
                action,
                file,
                [a.parse()?, b.parse()?, c.parse()?],
            )
        }
        [command, action, file, a, b, c]
            if command == "truncated-cone" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::TruncatedCone,
                action,
                file,
                [a.parse()?, b.parse()?, c.parse()?],
            )
        }
        [command, action, file, a, b, c]
            if command == "ring-band" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::RingBand,
                action,
                file,
                [a.parse()?, b.parse()?, c.parse()?],
            )
        }
        [command, action, file, a, b, c]
            if command == "round-brilliant" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::RoundBrilliant,
                action,
                file,
                [a.parse()?, b.parse()?, c.parse()?],
            )
        }
        [command, action, file, height]
            if command == "extruded-polygon" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::ExtrudedPolygon,
                action,
                file,
                [height.parse()?, 0.0, 0.0],
            )
        }
        [command, action, file, height]
            if command == "line-surface" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::LineSurface,
                action,
                file,
                [height.parse()?, 0.0, 0.0],
            )
        }
        [command, action, file, height] if command == "line-surface" && action == "from-curve" => {
            convert_open_curve_surface(file, PrimitiveKind::LineSurface, height.parse()?)
        }
        [command, action, file, height]
            if command == "open-polyline-surface" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::OpenPolylineSurface,
                action,
                file,
                [height.parse()?, 0.0, 0.0],
            )
        }
        [command, action, file, height]
            if command == "open-polyline-surface" && action == "from-curve" =>
        {
            convert_open_curve_surface(file, PrimitiveKind::OpenPolylineSurface, height.parse()?)
        }
        [command, action, file]
            if command == "bezier-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_bezier(PrimitiveKind::BezierCurve, action, file, 0.0)
        }
        [command, action, file, height]
            if command == "bezier-surface" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_bezier(PrimitiveKind::BezierSurface, action, file, height.parse()?)
        }
        [command, action, file, height]
            if command == "bezier-surface" && action == "from-curve" =>
        {
            convert_bezier_surface(file, height.parse()?)
        }
        [command, action, file]
            if command == "bezier-revolved-surface"
                && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_revolved_bezier(action, file)
        }
        [command, action, file]
            if command == "bezier-revolved-surface" && action == "from-curve" =>
        {
            convert_revolved_bezier_surface(file)
        }
        [command, action, file]
            if command == "revolved-surface" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(PrimitiveKind::RevolvedSurface, action, file, [0.0; 3])
        }
        [command, action, file] if command == "revolved-surface" && action == "from-curve" => {
            convert_revolved_surface(file)
        }
        [command, action, file]
            if command == "polyline-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(PrimitiveKind::PolylineCurve, action, file, [0.0; 3])
        }
        [command, action, file]
            if command == "open-polyline-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_open_polyline(action, file)
        }
        [command, action, file, radius, start, sweep]
            if command == "arc-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_arc(
                action,
                file,
                radius.parse()?,
                ArcFeature::new(start.parse()?, sweep.parse()?)?,
            )
        }
        [command, action, file, radius, start, sweep, height]
            if command == "arc-surface" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_arc_surface(
                action,
                file,
                radius.parse()?,
                height.parse()?,
                ArcFeature::new(start.parse()?, sweep.parse()?)?,
            )
        }
        [command, action, file, height] if command == "arc-surface" && action == "from-curve" => {
            convert_arc_surface(file, height.parse()?)
        }
        [command, action, file, start_x, start_y, end_x, end_y]
            if command == "line-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit_line(
                action,
                file,
                [start_x.parse()?, start_y.parse()?],
                [end_x.parse()?, end_y.parse()?],
            )
        }
        [command, action, file] if command == "planar-surface" && action == "new" => {
            create_or_edit(PrimitiveKind::PlanarSurface, "new", file, [0.0; 3])
        }
        [command, action, file] if command == "planar-surface" && action == "from-curve" => {
            convert_profile_stage(file, PrimitiveKind::PlanarSurface, None)
        }
        [command, action, file, radius]
            if command == "circle-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::CircleCurve,
                action,
                file,
                [radius.parse()?, 0.0, 0.0],
            )
        }
        [command, action, file, radius] if command == "planar-circle" && action == "new" => {
            create_or_edit(
                PrimitiveKind::PlanarCircleSurface,
                action,
                file,
                [radius.parse()?, 0.0, 0.0],
            )
        }
        [command, action, file] if command == "planar-circle" && action == "from-curve" => {
            convert_circle_stage(file, PrimitiveKind::PlanarCircleSurface, None)
        }
        [command, action, file, radius, height]
            if command == "extruded-circle" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::ExtrudedCircle,
                action,
                file,
                [radius.parse()?, height.parse()?, 0.0],
            )
        }
        [command, action, file, height]
            if command == "extruded-circle" && action == "from-profile" =>
        {
            convert_circle_stage(file, PrimitiveKind::ExtrudedCircle, Some(height.parse()?))
        }
        [command, action, file, radius_x, radius_y]
            if command == "ellipse-curve" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::EllipseCurve,
                action,
                file,
                [radius_x.parse()?, radius_y.parse()?, 0.0],
            )
        }
        [command, action, file, radius_x, radius_y]
            if command == "planar-ellipse" && action == "new" =>
        {
            create_or_edit(
                PrimitiveKind::PlanarEllipseSurface,
                action,
                file,
                [radius_x.parse()?, radius_y.parse()?, 0.0],
            )
        }
        [command, action, file] if command == "planar-ellipse" && action == "from-curve" => {
            convert_ellipse_stage(file, PrimitiveKind::PlanarEllipseSurface, None)
        }
        [command, action, file, radius_x, radius_y, height]
            if command == "extruded-ellipse" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::ExtrudedEllipse,
                action,
                file,
                [radius_x.parse()?, radius_y.parse()?, height.parse()?],
            )
        }
        [command, action, file, height]
            if command == "extruded-ellipse" && action == "from-profile" =>
        {
            convert_ellipse_stage(file, PrimitiveKind::ExtrudedEllipse, Some(height.parse()?))
        }
        [command, action, file, height]
            if command == "extruded-polygon" && action == "from-profile" =>
        {
            convert_profile_stage(file, PrimitiveKind::ExtrudedPolygon, Some(height.parse()?))
        }
        [command, action, file]
            if command == "revolved-solid" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(PrimitiveKind::RevolvedSolid, action, file, [0.0; 3])
        }
        [command, action, file] if command == "revolved-solid" && action == "from-profile" => {
            convert_profile_stage(file, PrimitiveKind::RevolvedSolid, None)
        }
        [command, action, file, a, b, c]
            if matches!(command.as_str(), "ellipsoid" | "pipe")
                && matches!(action.as_str(), "new" | "edit") =>
        {
            let kind = PrimitiveKind::from_name(command).expect("matched a known primitive");
            create_or_edit(kind, action, file, [a.parse()?, b.parse()?, c.parse()?])
        }
        [command, action, file, a, b]
            if matches!(command.as_str(), "cylinder" | "cone" | "torus")
                && matches!(action.as_str(), "new" | "edit") =>
        {
            let kind = PrimitiveKind::from_name(command).expect("matched a known primitive");
            create_or_edit(kind, action, file, [a.parse()?, b.parse()?, 0.0])
        }
        [command, action, file, a]
            if command == "sphere" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(PrimitiveKind::Sphere, action, file, [a.parse()?, 0.0, 0.0])
        }
        [command, file] if command == "inspect" => inspect(Path::new(file)),
        [command, file, tx, ty, tz, rx, ry, rz, scale] if command == "transform" => {
            transform_project(
                file,
                [tx.parse()?, ty.parse()?, tz.parse()?],
                [rx.parse()?, ry.parse()?, rz.parse()?],
                scale.parse()?,
            )
        }
        [command, file, plane, state] if command == "mirror" => mirror_project(file, plane, state),
        [command, file, direction_x, direction_y, direction_z]
            if matches!(
                command.as_str(),
                "surface-direction" | "extrusion-direction"
            ) =>
        {
            set_extrusion_direction_project(
                file,
                [
                    direction_x.parse()?,
                    direction_y.parse()?,
                    direction_z.parse()?,
                ],
            )
        }
        [command, file, copies, step_x, step_y, step_z] if command == "linear-array" => {
            pattern_project(
                file,
                PatternFeature::linear(
                    copies.parse()?,
                    [step_x.parse()?, step_y.parse()?, step_z.parse()?],
                )?,
            )
        }
        [command, file, copies, angle_step] if command == "polar-array" => pattern_project(
            file,
            PatternFeature::polar_z(copies.parse()?, angle_step.parse()?)?,
        ),
        [command, file, index, x, y] if command == "profile-point" => {
            set_profile_point(file, index.parse()?, [x.parse()?, y.parse()?])
        }
        [command, file, width, height] if command == "profile-rectangle" => {
            update_profile_project(file, rectangle_profile(width.parse()?, height.parse()?)?)
        }
        [command, file, sides, radius] if command == "profile-polygon" => update_profile_project(
            file,
            regular_polygon_profile(sides.parse()?, radius.parse()?)?,
        ),
        [command, file, inner, outer, height] if command == "profile-revolve-section" => {
            update_profile_project(
                file,
                revolved_profile(inner.parse()?, outer.parse()?, height.parse()?)?,
            )
        }
        [command, file, index, x, y] if command == "open-curve-point" => {
            set_open_polyline_point(file, index.parse()?, [x.parse()?, y.parse()?])
        }
        [command, file] if command == "curve-reverse" => reverse_curve_project(file),
        [command, file] if command == "curve-close" => close_curve_project(file),
        [command, file] if command == "profile-add" => add_profile_point(file),
        [command, file] if command == "open-curve-add" => add_open_polyline_point(file),
        [command, file, index] if command == "profile-remove" => {
            remove_profile_point(file, index.parse()?)
        }
        [command, file, index] if command == "open-curve-remove" => {
            remove_open_polyline_point(file, index.parse()?)
        }
        [command, file, output] if command == "export-stl" => export_project(
            Path::new(file),
            Path::new(output),
            MeshExportFormat::BinaryStl,
        ),
        [command, file, output] if command == "export-obj" => {
            export_project(Path::new(file), Path::new(output), MeshExportFormat::Obj)
        }
        [command, operation, file, tool_kind, a, b, c, tx, ty, tz] if command == "boolean" => {
            boolean_project(
                file,
                BooleanOperation::parse(operation)?,
                PrimitiveKind::from_name(tool_kind)
                    .ok_or_else(|| format!("unknown tool primitive: {tool_kind}"))?,
                [a.parse()?, b.parse()?, c.parse()?],
                [tx.parse()?, ty.parse()?, tz.parse()?],
            )
        }
        _ => {
            print_help();
            Err("invalid command; run with --help to see available commands".into())
        }
    }
}

fn launch_gui() -> Result<(), Box<dyn Error>> {
    gui::launch().map_err(|error| -> Box<dyn Error> {
        format!("could not start Open Jewel CAD: {error}").into()
    })
}

fn print_help() {
    println!(
        "Open Jewel CAD — parametric CAD editor\n\nCurve → surface → solid:\n  openjewelcad polyline-curve new <file>\n  openjewelcad profile-rectangle <file> <width_mm> <height_mm>\n  openjewelcad profile-polygon <file> <sides> <circumradius_mm>\n  openjewelcad planar-surface from-curve <file>\n  openjewelcad extruded-polygon from-profile <file> <height_mm>\n  openjewelcad profile-revolve-section <file> <inner_radius_mm> <outer_radius_mm> <height_mm>\n  openjewelcad revolved-solid from-profile <file>\n  openjewelcad circle-curve new <file> <radius_mm>\n  openjewelcad planar-circle new <file> <radius_mm>\n  openjewelcad planar-circle from-curve <file>\n  openjewelcad extruded-circle new <file> <radius_mm> <height_mm>\n  openjewelcad extruded-circle from-profile <file> <height_mm>\n  openjewelcad profile-point <file> <index> <x_mm> <y_mm>\n  openjewelcad profile-add <file>\n  openjewelcad profile-remove <file> <index>\n\nOther commands:\n  openjewelcad                 Open the desktop editor\n  openjewelcad gui             Open the desktop editor\n  openjewelcad box new <file> <width_mm> <depth_mm> <height_mm>\n  openjewelcad cylinder new <file> <radius_mm> <height_mm>\n  openjewelcad cone new <file> <radius_mm> <height_mm>\n  openjewelcad truncated-cone new <file> <base_radius_mm> <top_radius_mm> <height_mm>\n  openjewelcad ellipsoid new <file> <radius_x_mm> <radius_y_mm> <radius_z_mm>\n  openjewelcad pipe new <file> <outer_radius_mm> <inner_radius_mm> <length_mm>\n  openjewelcad ring-band new <file> <inner_diameter_mm> <wall_thickness_mm> <band_width_mm>\n  openjewelcad round-brilliant new <file> <girdle_diameter_mm> <height_mm> <table_diameter_mm>\n  openjewelcad extruded-polygon new <file> <height_mm>\n  openjewelcad revolved-solid new <file>\n  openjewelcad sphere new <file> <radius_mm>\n  openjewelcad torus new <file> <major_radius_mm> <tube_radius_mm>\n  openjewelcad transform <file> <tx_mm> <ty_mm> <tz_mm> <rx_deg> <ry_deg> <rz_deg> <scale>\n  openjewelcad boolean <union|difference|intersection> <file> <tool_type> <p1> <p2> <p3> <tx> <ty> <tz>\n  openjewelcad linear-array <file> <copies> <step_x_mm> <step_y_mm> <step_z_mm>\n  openjewelcad polar-array <file> <copies> <angle_step_deg>\n  Use edit instead of new to update an existing project of the same type.\n  openjewelcad inspect <file>\n  openjewelcad export-stl <file> <output.stl>\n  openjewelcad export-obj <file> <output.obj>\n\nLocal builds search build/Debug and build/Release automatically for the kernel."
    );
    println!(
        "\nEllipse profile commands:\n  openjewelcad ellipse-curve new <file> <radius_x_mm> <radius_y_mm>\n  openjewelcad planar-ellipse new <file> <radius_x_mm> <radius_y_mm>\n  openjewelcad planar-ellipse from-curve <file>\n  openjewelcad extruded-ellipse new <file> <radius_x_mm> <radius_y_mm> <height_mm>\n  openjewelcad extruded-ellipse from-profile <file> <height_mm>"
    );
    println!(
        "\nOpen curve commands:\n  openjewelcad line-curve new <file> <start_x_mm> <start_y_mm> <end_x_mm> <end_y_mm>\n  openjewelcad line-surface new <file> <height_mm>\n  openjewelcad line-surface from-curve <file> <height_mm>\n  openjewelcad open-polyline-curve new <file>\n  openjewelcad open-polyline-surface new <file> <height_mm>\n  openjewelcad open-polyline-surface from-curve <file> <height_mm>\n  openjewelcad bezier-curve new|edit <file>\n  openjewelcad bezier-surface new|edit <file> <height_mm>\n  openjewelcad bezier-surface from-curve <file> <height_mm>\n  openjewelcad revolved-surface new <file>\n  openjewelcad revolved-surface from-curve <file>\n  openjewelcad bezier-revolved-surface new|edit <file>\n  openjewelcad bezier-revolved-surface from-curve <file>\n  openjewelcad arc-curve new <file> <radius_mm> <start_angle_deg> <sweep_angle_deg>\n  openjewelcad arc-surface new <file> <radius_mm> <start_angle_deg> <sweep_angle_deg> <height_mm>\n  openjewelcad arc-surface from-curve <file> <height_mm>\n  openjewelcad open-curve-point <file> <index> <x_mm> <y_mm>\n  openjewelcad open-curve-add <file>\n  openjewelcad open-curve-remove <file> <index>\n  openjewelcad curve-reverse <file>  (line, open polyline, or closed polyline)\n  openjewelcad curve-close <file>  (open polyline curve)\n  A cubic Bezier has four editable control points; points 1 and 4 are endpoints.\n  Arc start is measured counter-clockwise from +X; sweep must be between 0° and 360°."
    );
    println!("\nTransform commands:\n  openjewelcad mirror <file> <YZ|XZ|XY> <on|off>\n  openjewelcad extrusion-direction <file> <direction_x> <direction_y> <direction_z> (surface-direction alias)");
}

fn export_project(
    input: &Path,
    output: &Path,
    format: MeshExportFormat,
) -> Result<(), Box<dyn Error>> {
    let model = load_model(input)?;
    let summary = export_model(model, output, format)?;
    println!(
        "Exported {} vertices and {} triangles to {} (millimetres).",
        summary.vertices,
        summary.triangles,
        output.display()
    );
    Ok(())
}

fn inspect(path: &Path) -> Result<(), Box<dyn Error>> {
    let model = load_model(path)?;
    let kernel = Kernel::from_environment()?;
    let shape = model.create_shape(&kernel)?;
    println!("Project: {}", path.display());
    println!("Feature: {}", model.kind.display_name());
    if model.kind.uses_vector_extrusion() {
        println!(
            "Extrusion direction: {:.3}, {:.3}, {:.3} (XYZ)",
            model.extrusion_direction[0],
            model.extrusion_direction[1],
            model.extrusion_direction[2]
        );
    }
    for (index, label) in model.kind.parameter_labels().iter().enumerate() {
        println!("{label}: {:.3} mm", model.parameters_mm[index].as_mm());
    }
    if model.kind.uses_profile() {
        for (index, point) in model.profile.points().iter().enumerate() {
            println!("Profile point {index}: {:.3}, {:.3} mm", point[0], point[1]);
        }
    }
    if let Some(arc) = model.arc {
        println!("Start angle: {:.3}°", arc.start_angle_deg);
        println!("Sweep angle: {:.3}°", arc.sweep_angle_deg);
    }
    if let Some(line) = model.line {
        println!(
            "Line start: {:.3}, {:.3} mm; end: {:.3}, {:.3} mm",
            line.start_mm[0], line.start_mm[1], line.end_mm[0], line.end_mm[1]
        );
    }
    if let Some(open_polyline) = model.open_polyline {
        for (index, point) in open_polyline.points().iter().enumerate() {
            println!(
                "{} {index}: {:.3}, {:.3} mm",
                if matches!(
                    model.kind,
                    PrimitiveKind::BezierCurve | PrimitiveKind::BezierSurface
                ) {
                    "Bezier control point"
                } else {
                    "Open curve point"
                },
                point[0],
                point[1]
            );
        }
    }
    if let Some(boolean) = model.boolean {
        println!(
            "Boolean: {} with {}",
            boolean.operation.name(),
            boolean.tool_kind.display_name()
        );
        for (index, label) in boolean.tool_kind.parameter_labels().iter().enumerate() {
            println!(
                "Tool {label}: {:.3} mm",
                boolean.tool_parameters_mm[index].as_mm()
            );
        }
    }
    if let Some(pattern) = model.pattern {
        match pattern.mode {
            PatternMode::Linear => println!(
                "Linear array: {} copies; step {:.3}, {:.3}, {:.3} mm",
                pattern.copies, pattern.step_mm[0], pattern.step_mm[1], pattern.step_mm[2]
            ),
            PatternMode::PolarZ => println!(
                "Polar array: {} copies; {:.3}° around Z per copy",
                pattern.copies, pattern.angle_step_deg
            ),
        }
    }
    if model.kind.is_curve() {
        println!("Curve length: {:.3} mm", shape.length_mm()?);
    } else {
        let properties = shape.mass_properties()?;
        if model.kind.is_solid() {
            println!("Volume: {:.3} mm³", properties[0]);
        }
        if model.kind.is_surface() {
            println!("Surface area: {:.3} mm²", properties[1]);
        }
        println!(
            "Center: {:.3}, {:.3}, {:.3} mm",
            properties[2], properties[3], properties[4]
        );
    }
    let bounds = shape.bounds_mm()?;
    println!(
        "Bounds: {:.3} × {:.3} × {:.3} mm",
        bounds[3] - bounds[0],
        bounds[4] - bounds[1],
        bounds[5] - bounds[2]
    );
    println!(
        "OCCT validity: {}",
        if shape.is_valid()? {
            "valid"
        } else {
            "invalid"
        }
    );
    Ok(())
}

fn export_model(
    model: PrimitiveModel,
    output: &Path,
    format: MeshExportFormat,
) -> Result<MeshExportSummary, Box<dyn Error>> {
    if !model.kind.is_solid() {
        return Err("mesh export is available after creating a solid".into());
    }
    let kernel = Kernel::from_environment()?;
    let shape = model.create_shape(&kernel)?;
    let mesh = shape.tessellate(0.05, 0.35)?;
    match format {
        MeshExportFormat::BinaryStl => write_binary_stl(&mesh, output)?,
        MeshExportFormat::Obj => write_obj(&mesh, output)?,
    }
    Ok(MeshExportSummary {
        vertices: mesh.vertices_mm.len(),
        triangles: mesh.triangles.len(),
    })
}

fn write_binary_stl(
    mesh: &openjewelcad_cad_ffi::TriangleMesh,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let triangle_count = u32::try_from(mesh.triangles.len())
        .map_err(|_| "STL export is limited to 4,294,967,295 triangles")?;
    let mut writer = BufWriter::new(fs::File::create(output)?);
    let mut header = [0_u8; 80];
    let label = b"Open Jewel CAD binary STL; units are millimetres";
    header[..label.len()].copy_from_slice(label);
    writer.write_all(&header)?;
    writer.write_all(&triangle_count.to_le_bytes())?;
    for triangle in &mesh.triangles {
        let a = mesh.vertices_mm[triangle[0] as usize];
        let b = mesh.vertices_mm[triangle[1] as usize];
        let c = mesh.vertices_mm[triangle[2] as usize];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let magnitude =
            (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if !magnitude.is_finite() || magnitude <= f64::EPSILON {
            return Err("mesh contains a degenerate triangle".into());
        }
        for coordinate in normal.map(|value| value / magnitude) {
            write_stl_float(&mut writer, coordinate)?;
        }
        for point in [a, b, c] {
            for coordinate in point {
                write_stl_float(&mut writer, coordinate)?;
            }
        }
        writer.write_all(&0_u16.to_le_bytes())?;
    }
    writer.flush()?;
    Ok(())
}

fn write_stl_float(writer: &mut impl Write, value: f64) -> Result<(), Box<dyn Error>> {
    let value = value as f32;
    if !value.is_finite() {
        return Err("mesh contains a coordinate outside the STL float range".into());
    }
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_obj(
    mesh: &openjewelcad_cad_ffi::TriangleMesh,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(fs::File::create(output)?);
    writeln!(
        writer,
        "# Open Jewel CAD triangle mesh; units are millimetres"
    )?;
    writeln!(writer, "o OpenJewelCAD")?;
    for point in &mesh.vertices_mm {
        if !point.iter().all(|value| value.is_finite()) {
            return Err("mesh contains a non-finite coordinate".into());
        }
        writeln!(writer, "v {:.9} {:.9} {:.9}", point[0], point[1], point[2])?;
    }
    for triangle in &mesh.triangles {
        let indices = triangle.map(|index| index as usize + 1);
        if indices.iter().any(|index| *index > mesh.vertices_mm.len()) {
            return Err("mesh contains an invalid triangle index".into());
        }
        writeln!(writer, "f {} {} {}", indices[0], indices[1], indices[2])?;
    }
    writer.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_constructors_create_valid_editable_closed_curves() {
        let rectangle = rectangle_profile(12.0, 8.0).unwrap();
        assert_eq!(rectangle.points().len(), 4);
        assert!((rectangle.signed_area() - 96.0).abs() < 1e-9);
        assert_eq!(rectangle.points()[0], [-6.0, -4.0]);
        assert_eq!(rectangle.points()[2], [6.0, 4.0]);

        let hexagon = regular_polygon_profile(6, 5.0).unwrap();
        assert_eq!(hexagon.points().len(), 6);
        assert!(hexagon.signed_area() > 0.0);
        assert!((hexagon.signed_area() - 3.0_f64.sqrt() * 37.5).abs() < 1e-9);
        assert!(regular_polygon_profile(2, 5.0).is_err());
        assert!(rectangle_profile(0.0, 5.0).is_err());
        assert!(revolved_profile(3.0, 5.0, 4.0).is_ok());
        assert!(revolved_profile(5.0, 3.0, 4.0).is_err());
    }

    #[test]
    fn box_project_round_trips_parameters() {
        for (kind, values) in [
            (PrimitiveKind::Box, [2.5, 3.0, 4.25]),
            (PrimitiveKind::Cylinder, [2.5, 4.25, 0.0]),
            (PrimitiveKind::Cone, [2.5, 4.25, 0.0]),
            (PrimitiveKind::TruncatedCone, [3.5, 2.0, 4.25]),
            (PrimitiveKind::Ellipsoid, [3.5, 2.0, 4.25]),
            (PrimitiveKind::Pipe, [3.5, 2.0, 4.25]),
            (PrimitiveKind::Sphere, [3.0, 0.0, 0.0]),
            (PrimitiveKind::Torus, [5.0, 1.25, 0.0]),
            (PrimitiveKind::RingBand, [18.0, 2.0, 4.0]),
            (PrimitiveKind::RoundBrilliant, [6.0, 3.6, 3.3]),
            (PrimitiveKind::LineCurve, [0.0, 0.0, 0.0]),
            (PrimitiveKind::OpenPolylineCurve, [0.0, 0.0, 0.0]),
            (PrimitiveKind::ArcCurve, [4.0, 0.0, 0.0]),
            (PrimitiveKind::CircleCurve, [4.0, 0.0, 0.0]),
            (PrimitiveKind::PlanarCircleSurface, [4.0, 0.0, 0.0]),
            (PrimitiveKind::ExtrudedCircle, [4.0, 3.0, 0.0]),
            (PrimitiveKind::EllipseCurve, [4.0, 2.0, 0.0]),
            (PrimitiveKind::PlanarEllipseSurface, [4.0, 2.0, 0.0]),
            (PrimitiveKind::ExtrudedEllipse, [4.0, 2.0, 3.0]),
        ] {
            let model = PrimitiveModel::new(kind, values).unwrap();
            assert_eq!(PrimitiveModel::parse(&model.serialize()).unwrap(), model);
        }
    }

    #[test]
    fn rejects_invalid_dimensions_and_project_version() {
        assert!(PrimitiveModel::new(PrimitiveKind::Box, [0.0, 1.0, 1.0]).is_err());
        assert!(PrimitiveModel::new(PrimitiveKind::Torus, [2.0, 2.0, 0.0]).is_err());
        assert!(PrimitiveModel::new(PrimitiveKind::TruncatedCone, [2.0, 2.0, 5.0]).is_err());
        assert!(PrimitiveModel::new(PrimitiveKind::Pipe, [2.0, 2.0, 5.0]).is_err());
        assert!(PrimitiveModel::new(PrimitiveKind::RoundBrilliant, [6.0, 3.6, 6.0]).is_err());
        assert!(PrimitiveModel::parse("OPENJEWELCAD_PROJECT=20\n").is_err());
        assert!(PrimitiveModel::parse(
            "OPENJEWELCAD_PROJECT=1\nfeature_type=sphere\nradius_mm=2\nextra=1\n"
        )
        .is_err());
    }

    #[test]
    fn version_one_projects_load_with_identity_transform() {
        let model = PrimitiveModel::parse(
            "OPENJEWELCAD_PROJECT=1\nfeature_type=box\nwidth_mm=2\ndepth_mm=3\nheight_mm=4\n",
        )
        .unwrap();
        assert_eq!(model.transform, TransformParams::IDENTITY);
    }

    #[test]
    fn version_two_projects_still_load_density_and_transform() {
        let model = PrimitiveModel::parse(
            "OPENJEWELCAD_PROJECT=2\nfeature_type=box\nwidth_mm=2\ndepth_mm=3\nheight_mm=4\ntranslation_x_mm=1\ntranslation_y_mm=2\ntranslation_z_mm=3\nrotation_x_deg=0\nrotation_y_deg=0\nrotation_z_deg=45\nuniform_scale=1.5\nmaterial_density_g_cm3=19.3\n",
        )
        .unwrap();
        assert_eq!(model.transform.translation_mm, [1.0, 2.0, 3.0]);
        assert_eq!(model.transform.rotation_deg, [0.0, 0.0, 45.0]);
        assert_eq!(model.transform.uniform_scale, 1.5);
        assert_eq!(model.material_density_g_cm3, Some(19.3));
        assert_eq!(model.boolean, None);
    }

    #[test]
    fn boolean_features_round_trip_in_current_project_version_twenty() {
        let tool_transform = TransformParams {
            translation_mm: [5.0, 5.0, 1.0],
            rotation_deg: [0.0, 0.0, 15.0],
            uniform_scale: 1.25,
            mirror_axes: [true, false, false],
        };
        let boolean = BooleanFeature::new(
            BooleanOperation::Difference,
            PrimitiveKind::Sphere,
            [2.5, 0.0, 0.0],
            tool_transform,
        )
        .unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::Box, [10.0, 10.0, 2.0])
            .unwrap()
            .with_boolean(Some(boolean));
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let legacy = serialized
            .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=3", 1)
            .replace("tool_mirror_x=true\n", "tool_mirror_x=false\n");
        let legacy_model = PrimitiveModel::parse(&legacy).unwrap();
        assert_eq!(
            legacy_model.boolean.unwrap().tool_transform.mirror_axes,
            [false; 3]
        );
        assert!(PrimitiveModel::parse(
            "OPENJEWELCAD_PROJECT=3\nfeature_type=box\nwidth_mm=10\ndepth_mm=10\nheight_mm=2\nboolean_operation=unknown\n"
        )
        .is_err());
    }

    #[test]
    fn version_three_projects_remain_readable_after_array_upgrade() {
        let model = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 3.0, 4.0]).unwrap();
        let legacy =
            model
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=3", 1);
        let loaded = PrimitiveModel::parse(&legacy).unwrap();
        assert_eq!(loaded.kind, model.kind);
        assert_eq!(loaded.values(), model.values());
        assert_eq!(loaded.pattern, None);
    }

    #[test]
    fn version_eighteen_projects_remain_readable_after_bezier_upgrade() {
        let model = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 3.0, 4.0]).unwrap();
        let legacy =
            model
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=18", 1);
        assert_eq!(PrimitiveModel::parse(&legacy).unwrap(), model);
    }

    #[test]
    fn linear_and_polar_patterns_round_trip_in_current_project_version_twenty() {
        let base = PrimitiveModel::new(PrimitiveKind::RoundBrilliant, [6.0, 3.6, 3.3]).unwrap();
        for pattern in [
            PatternFeature::linear(5, [0.0, 8.0, 0.0]).unwrap(),
            PatternFeature::polar_z(8, 45.0).unwrap(),
        ] {
            let model = base.with_pattern(Some(pattern));
            let saved = model.serialize();
            assert!(saved.starts_with("OPENJEWELCAD_PROJECT=20\n"));
            assert_eq!(PrimitiveModel::parse(&saved).unwrap(), model);
            let version_five =
                saved.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=5", 1);
            assert_eq!(PrimitiveModel::parse(&version_five).unwrap(), model);
        }
        assert!(PatternFeature::linear(1, [1.0, 0.0, 0.0]).is_err());
        assert!(PatternFeature::linear(4, [0.0; 3]).is_err());
        assert!(PatternFeature::polar_z(4, 360.0).is_err());
    }

    #[test]
    fn extruded_polygon_profile_round_trips_and_rejects_invalid_outlines() {
        let points = [
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 1.0],
            [1.0, 1.0],
            [1.0, 3.0],
            [0.0, 3.0],
        ];
        let profile = PolygonProfile::from_points(&points).unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::ExtrudedPolygon, [2.0, 0.0, 0.0])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        let serialized = model.serialize();
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_five =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=5", 1);
        assert_eq!(PrimitiveModel::parse(&version_five).unwrap(), model);
        assert!(
            PolygonProfile::from_points(&[[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0],])
                .is_err()
        );
        assert!(PolygonProfile::from_points(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]).is_err());
    }

    #[test]
    fn extruded_polygon_generates_valid_concave_solid() {
        let kernel = Kernel::from_environment().unwrap();
        let profile = PolygonProfile::from_points(&[
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 1.0],
            [1.0, 1.0],
            [1.0, 3.0],
            [0.0, 3.0],
        ])
        .unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::ExtrudedPolygon, [2.0, 0.0, 0.0])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.volume_mm3().unwrap() - 10.0).abs() < 1e-8);
    }

    #[test]
    fn profile_moves_through_curve_surface_and_solid_stages() {
        let kernel = Kernel::from_environment().unwrap();
        let profile =
            PolygonProfile::from_points(&[[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]])
                .unwrap();
        let curve = PrimitiveModel::new(PrimitiveKind::PolylineCurve, [0.0; 3])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        assert_eq!(PrimitiveModel::parse(&curve.serialize()).unwrap(), curve);
        let version_five =
            curve
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=5", 1);
        assert!(PrimitiveModel::parse(&version_five).is_err());
        let version_six =
            curve
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=6", 1);
        assert_eq!(PrimitiveModel::parse(&version_six).unwrap(), curve);
        let curve_shape = curve.create_shape(&kernel).unwrap();
        assert!(curve_shape.is_valid().unwrap());
        assert!((curve_shape.length_mm().unwrap() - 40.0).abs() < 1e-8);

        let surface = PrimitiveModel::new(PrimitiveKind::PlanarSurface, [0.0; 3])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        assert_eq!(
            PrimitiveModel::parse(&surface.serialize()).unwrap(),
            surface
        );
        let surface_shape = surface.create_shape(&kernel).unwrap();
        assert!(surface_shape.is_valid().unwrap());
        assert!((surface_shape.mass_properties().unwrap()[1] - 100.0).abs() < 1e-8);

        let solid = PrimitiveModel::new(PrimitiveKind::ExtrudedPolygon, [2.0, 0.0, 0.0])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        assert_eq!(PrimitiveModel::parse(&solid.serialize()).unwrap(), solid);
        let solid_shape = solid.create_shape(&kernel).unwrap();
        assert!(solid_shape.is_valid().unwrap());
        assert!((solid_shape.volume_mm3().unwrap() - 200.0).abs() < 1e-8);
    }

    #[test]
    fn circle_profile_moves_through_curve_surface_and_solid_stages() {
        let kernel = Kernel::from_environment().unwrap();
        let path = env::temp_dir().join(format!(
            "openjewelcad-circle-stage-{}.ojcad",
            std::process::id()
        ));
        let file = path.to_string_lossy().into_owned();
        let curve = PrimitiveModel::new(PrimitiveKind::CircleCurve, [4.0, 0.0, 0.0]).unwrap();
        assert_eq!(PrimitiveModel::parse(&curve.serialize()).unwrap(), curve);
        save_model(&path, curve).unwrap();
        convert_circle_stage(&file, PrimitiveKind::PlanarCircleSurface, None).unwrap();

        let surface = load_model(&path).unwrap();
        assert_eq!(surface.kind, PrimitiveKind::PlanarCircleSurface);
        assert_eq!(surface.parameters_mm[0].as_mm(), 4.0);
        let surface_shape = surface.create_shape(&kernel).unwrap();
        assert!(
            (surface_shape.mass_properties().unwrap()[1] - std::f64::consts::PI * 16.0).abs()
                < 1e-7
        );
        convert_circle_stage(&file, PrimitiveKind::ExtrudedCircle, Some(3.0)).unwrap();

        let solid = load_model(&path).unwrap();
        assert_eq!(solid.kind, PrimitiveKind::ExtrudedCircle);
        assert_eq!(solid.parameters_mm[0].as_mm(), 4.0);
        assert_eq!(solid.parameters_mm[1].as_mm(), 3.0);
        assert_eq!(PrimitiveModel::parse(&solid.serialize()).unwrap(), solid);
        let solid_shape = solid.create_shape(&kernel).unwrap();
        assert!((solid_shape.volume_mm3().unwrap() - std::f64::consts::PI * 48.0).abs() < 1e-7);
        let _ = fs::remove_file(path);

        let version_seven =
            curve
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=7", 1);
        assert_eq!(PrimitiveModel::parse(&version_seven).unwrap(), curve);
        let version_six =
            curve
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=6", 1);
        assert!(PrimitiveModel::parse(&version_six).is_err());
    }

    #[test]
    fn ellipse_profile_moves_through_curve_surface_and_solid_stages() {
        let kernel = Kernel::from_environment().unwrap();
        let path = env::temp_dir().join(format!(
            "openjewelcad-ellipse-stage-{}.ojcad",
            std::process::id()
        ));
        let file = path.to_string_lossy().into_owned();
        let curve = PrimitiveModel::new(PrimitiveKind::EllipseCurve, [4.0, 2.0, 0.0]).unwrap();
        assert_eq!(PrimitiveModel::parse(&curve.serialize()).unwrap(), curve);
        save_model(&path, curve).unwrap();
        convert_ellipse_stage(&file, PrimitiveKind::PlanarEllipseSurface, None).unwrap();

        let surface = load_model(&path).unwrap();
        assert_eq!(surface.kind, PrimitiveKind::PlanarEllipseSurface);
        assert_eq!(surface.parameters_mm[0].as_mm(), 4.0);
        assert_eq!(surface.parameters_mm[1].as_mm(), 2.0);
        let surface_shape = surface.create_shape(&kernel).unwrap();
        assert!(
            (surface_shape.mass_properties().unwrap()[1] - std::f64::consts::PI * 8.0).abs() < 1e-7
        );
        convert_ellipse_stage(&file, PrimitiveKind::ExtrudedEllipse, Some(3.0)).unwrap();

        let solid = load_model(&path).unwrap();
        assert_eq!(solid.kind, PrimitiveKind::ExtrudedEllipse);
        assert_eq!(solid.parameters_mm[0].as_mm(), 4.0);
        assert_eq!(solid.parameters_mm[1].as_mm(), 2.0);
        assert_eq!(solid.parameters_mm[2].as_mm(), 3.0);
        assert_eq!(PrimitiveModel::parse(&solid.serialize()).unwrap(), solid);
        let solid_shape = solid.create_shape(&kernel).unwrap();
        let expected_volume = std::f64::consts::PI * 24.0;
        assert!(
            (solid_shape.volume_mm3().unwrap() - expected_volume).abs() < expected_volume * 1e-6
        );
        let _ = fs::remove_file(path);

        let version_seven =
            curve
                .serialize()
                .replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=7", 1);
        assert!(PrimitiveModel::parse(&version_seven).is_err());
    }

    #[test]
    fn arc_curve_keeps_degree_parameters_and_reports_open_length() {
        let kernel = Kernel::from_environment().unwrap();
        let arc_feature = ArcFeature::new(270.0, 90.0).unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::ArcCurve, [2.0, 0.0, 0.0])
            .unwrap()
            .with_arc(Some(arc_feature))
            .unwrap();
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_nine =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=9", 1);
        assert_eq!(PrimitiveModel::parse(&version_nine).unwrap(), model);
        let version_eight =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=8", 1);
        assert!(PrimitiveModel::parse(&version_eight).is_err());
        assert!(ArcFeature::new(0.0, 0.0).is_err());
        assert!(ArcFeature::new(0.0, 360.0).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.length_mm().unwrap() - std::f64::consts::PI).abs() < 1e-9);
        let bounds = shape.bounds_mm().unwrap();
        for (actual, expected) in bounds.into_iter().zip([0.0, -2.0, 0.0, 2.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn line_curve_keeps_xy_endpoints_and_reports_open_length() {
        let kernel = Kernel::from_environment().unwrap();
        let line_feature = LineFeature::new([-1.0, 2.0], [2.0, 6.0]).unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::LineCurve, [0.0; 3])
            .unwrap()
            .with_line(Some(line_feature))
            .unwrap();
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_nine =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=9", 1);
        assert!(PrimitiveModel::parse(&version_nine).is_err());
        let version_ten =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=10", 1);
        assert_eq!(PrimitiveModel::parse(&version_ten).unwrap(), model);
        assert!(LineFeature::new([1.0, 1.0], [1.0, 1.0]).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.length_mm().unwrap() - 5.0).abs() < 1e-9);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-1.0, 2.0, 0.0, 2.0, 6.0, 0.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn open_polyline_curve_edits_and_persists_without_closing() {
        let kernel = Kernel::from_environment().unwrap();
        let feature =
            OpenPolylineFeature::from_points(&[[-1.0, 2.0], [2.0, 6.0], [6.0, 3.0]]).unwrap();
        let mut changed = feature;
        changed.add_point().unwrap();
        changed.set_point_unchecked(3, [9.0, 1.0]);
        changed.remove_point(0).unwrap();
        assert_eq!(changed.points(), &[[2.0, 6.0], [6.0, 3.0], [9.0, 1.0]]);
        assert!(OpenPolylineFeature::from_points(&[[-1.0, 2.0], [-1.0, 2.0]]).is_err());

        let model = PrimitiveModel::new(PrimitiveKind::OpenPolylineCurve, [0.0; 3])
            .unwrap()
            .with_open_polyline(Some(changed))
            .unwrap();
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_eleven =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=11", 1);
        assert_eq!(PrimitiveModel::parse(&version_eleven).unwrap(), model);
        let version_ten =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=10", 1);
        assert!(PrimitiveModel::parse(&version_ten).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        let expected_length = 5.0 + 13.0_f64.sqrt();
        assert!((shape.length_mm().unwrap() - expected_length).abs() < 1e-9);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([2.0, 1.0, 0.0, 9.0, 6.0, 0.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn curve_reverse_reverses_line_open_and_closed_curve_data() {
        let suffix = format!("{}-{}", std::process::id(), "curve-reverse");
        let line_path = env::temp_dir().join(format!("openjewelcad-{suffix}-line.ojcad"));
        let open_path = env::temp_dir().join(format!("openjewelcad-{suffix}-open.ojcad"));
        let closed_path = env::temp_dir().join(format!("openjewelcad-{suffix}-closed.ojcad"));
        let line_model = PrimitiveModel::new(PrimitiveKind::LineCurve, [0.0; 3])
            .unwrap()
            .with_line(Some(LineFeature::new([-2.0, 1.0], [4.0, 3.0]).unwrap()))
            .unwrap();
        let open_model = PrimitiveModel::new(PrimitiveKind::OpenPolylineCurve, [0.0; 3])
            .unwrap()
            .with_open_polyline(Some(
                OpenPolylineFeature::from_points(&[[-2.0, 1.0], [0.0, 4.0], [4.0, 3.0]]).unwrap(),
            ))
            .unwrap();
        let closed_model = PrimitiveModel::new(PrimitiveKind::PolylineCurve, [0.0; 3])
            .unwrap()
            .with_profile(rectangle_profile(8.0, 4.0).unwrap())
            .unwrap();
        for (path, model) in [
            (&line_path, line_model),
            (&open_path, open_model),
            (&closed_path, closed_model),
        ] {
            save_model(path, model).unwrap();
            reverse_curve_project(&path.to_string_lossy()).unwrap();
        }
        let reversed_line = load_model(&line_path).unwrap();
        assert_eq!(reversed_line.line.unwrap().start_mm, [4.0, 3.0]);
        assert_eq!(reversed_line.line.unwrap().end_mm, [-2.0, 1.0]);
        let reversed_open = load_model(&open_path).unwrap();
        assert_eq!(
            reversed_open.open_polyline.unwrap().points(),
            &[[4.0, 3.0], [0.0, 4.0], [-2.0, 1.0]]
        );
        let reversed_closed = load_model(&closed_path).unwrap();
        assert_eq!(
            reversed_closed.profile.points(),
            &[[-4.0, 2.0], [4.0, 2.0], [4.0, -2.0], [-4.0, -2.0]]
        );
        for path in [line_path, open_path, closed_path] {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn curve_close_creates_a_valid_closed_polyline_and_persists_it() {
        let path = env::temp_dir().join(format!(
            "openjewelcad-{}-curve-close.ojcad",
            std::process::id()
        ));
        let open_curve = PrimitiveModel::new(PrimitiveKind::OpenPolylineCurve, [0.0; 3])
            .unwrap()
            .with_open_polyline(Some(
                OpenPolylineFeature::from_points(&[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 0.0]])
                    .unwrap(),
            ))
            .unwrap();
        save_model(&path, open_curve).unwrap();
        close_curve_project(&path.to_string_lossy()).unwrap();
        let closed_curve = load_model(&path).unwrap();
        assert_eq!(closed_curve.kind, PrimitiveKind::PolylineCurve);
        assert_eq!(
            closed_curve.profile.points(),
            &[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0]]
        );
        assert!(closed_curve.open_polyline.is_none());
        let shape = closed_curve
            .create_shape(&Kernel::from_environment().unwrap())
            .unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.length_mm().unwrap() - 12.0).abs() < 1e-9);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn revolved_solid_round_trips_and_builds_a_ring_from_its_section() {
        let kernel = Kernel::from_environment().unwrap();
        let profile = revolved_profile(3.0, 5.0, 4.0).unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::RevolvedSolid, [0.0; 3])
            .unwrap()
            .with_profile(profile)
            .unwrap();
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_twelve =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=12", 1);
        assert_eq!(PrimitiveModel::parse(&version_twelve).unwrap(), model);
        let version_eleven =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=11", 1);
        assert!(PrimitiveModel::parse(&version_eleven).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.volume_mm3().unwrap() - 64.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, -2.0, -5.0, 5.0, 2.0, 5.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn open_curve_surfaces_round_trip_and_keep_their_source_geometry() {
        let kernel = Kernel::from_environment().unwrap();
        let line = LineFeature::new([-5.0, 0.0], [5.0, 0.0]).unwrap();
        let line_surface = PrimitiveModel::new(PrimitiveKind::LineSurface, [4.0, 0.0, 0.0])
            .unwrap()
            .with_line(Some(line))
            .unwrap();
        let open_path =
            OpenPolylineFeature::from_points(&[[-5.0, 0.0], [0.0, 5.0], [5.0, 0.0]]).unwrap();
        let polyline_surface =
            PrimitiveModel::new(PrimitiveKind::OpenPolylineSurface, [4.0, 0.0, 0.0])
                .unwrap()
                .with_open_polyline(Some(open_path))
                .unwrap();

        for (model, expected_area) in [
            (line_surface, 40.0),
            (polyline_surface, 40.0 * std::f64::consts::SQRT_2),
        ] {
            let serialized = model.serialize();
            assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
            assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
            let version_thirteen =
                serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=13", 1);
            assert_eq!(PrimitiveModel::parse(&version_thirteen).unwrap(), model);
            let version_twelve =
                serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=12", 1);
            assert!(PrimitiveModel::parse(&version_twelve).is_err());
            let shape = model.create_shape(&kernel).unwrap();
            assert!(shape.is_valid().unwrap());
            assert!((shape.mass_properties().unwrap()[1] - expected_area).abs() < 1e-7);
            let mesh = shape.tessellate(0.05, 0.35).unwrap();
            assert!(mesh.triangles.len() >= 2);
        }

        let directed = PrimitiveModel::new(PrimitiveKind::LineSurface, [5.0, 0.0, 0.0])
            .unwrap()
            .with_line(Some(line))
            .unwrap()
            .with_extrusion_direction([0.0, 3.0, 4.0])
            .unwrap();
        let serialized = directed.serialize();
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), directed);
        let legacy_default = PrimitiveModel::new(PrimitiveKind::LineSurface, [5.0, 0.0, 0.0])
            .unwrap()
            .with_line(Some(line))
            .unwrap()
            .serialize()
            .replace("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=16")
            .lines()
            .filter(|line| !line.starts_with("extrusion_direction_"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            PrimitiveModel::parse(&legacy_default)
                .unwrap()
                .extrusion_direction,
            [0.0, 0.0, 1.0]
        );
        let downgraded_custom =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=16", 1);
        assert!(PrimitiveModel::parse(&downgraded_custom).is_err());
        let shape = directed.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.mass_properties().unwrap()[1] - 50.0).abs() < 1e-7);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, 0.0, 0.0, 5.0, 3.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn closed_profiles_extrude_along_arbitrary_3d_directions_and_migrate_projects() {
        let kernel = Kernel::from_environment().unwrap();
        let direction = [0.0, 3.0, 4.0];
        let rectangle =
            PolygonProfile::from_points(&[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]]).unwrap();
        let polygon = PrimitiveModel::new(PrimitiveKind::ExtrudedPolygon, [5.0, 0.0, 0.0])
            .unwrap()
            .with_profile(rectangle)
            .unwrap()
            .with_extrusion_direction(direction)
            .unwrap();
        let circle = PrimitiveModel::new(PrimitiveKind::ExtrudedCircle, [2.0, 5.0, 0.0])
            .unwrap()
            .with_extrusion_direction(direction)
            .unwrap();
        let ellipse = PrimitiveModel::new(PrimitiveKind::ExtrudedEllipse, [4.0, 2.0, 5.0])
            .unwrap()
            .with_extrusion_direction(direction)
            .unwrap();

        for (model, volume, expected_bounds) in [
            (polygon, 48.0, [0.0, 0.0, 0.0, 4.0, 6.0, 4.0]),
            (
                circle,
                16.0 * std::f64::consts::PI,
                [-2.0, -2.0, 0.0, 2.0, 5.0, 4.0],
            ),
            (
                ellipse,
                32.0 * std::f64::consts::PI,
                [-4.0, -2.0, 0.0, 4.0, 5.0, 4.0],
            ),
        ] {
            let serialized = model.serialize();
            assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
            assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
            let shape = model.create_shape(&kernel).unwrap();
            assert!(shape.is_valid().unwrap());
            assert!((shape.volume_mm3().unwrap() - volume).abs() < volume.abs().max(1.0) * 1e-6);
            for (actual, expected) in shape.bounds_mm().unwrap().into_iter().zip(expected_bounds) {
                assert!((actual - expected).abs() < 1e-4);
            }
        }

        let legacy_circle = PrimitiveModel::new(PrimitiveKind::ExtrudedCircle, [2.0, 5.0, 0.0])
            .unwrap()
            .serialize()
            .replace("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=17")
            .lines()
            .filter(|line| !line.starts_with("extrusion_direction_"))
            .collect::<Vec<_>>()
            .join("\n");
        let loaded_legacy = PrimitiveModel::parse(&legacy_circle).unwrap();
        assert_eq!(loaded_legacy.kind, PrimitiveKind::ExtrudedCircle);
        assert_eq!(loaded_legacy.extrusion_direction, [0.0, 0.0, 1.0]);

        let custom_from_old_version =
            PrimitiveModel::new(PrimitiveKind::ExtrudedCircle, [2.0, 5.0, 0.0])
                .unwrap()
                .with_extrusion_direction(direction)
                .unwrap()
                .serialize()
                .replace("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=17");
        assert!(PrimitiveModel::parse(&custom_from_old_version).is_err());
        assert!(
            PrimitiveModel::new(PrimitiveKind::ExtrudedCircle, [2.0, 5.0, 0.0])
                .unwrap()
                .with_extrusion_direction([0.0; 3])
                .is_err()
        );
        assert!(
            PrimitiveModel::new(PrimitiveKind::ExtrudedCircle, [2.0, 5.0, 0.0])
                .unwrap()
                .with_extrusion_direction([1.0, 0.0, 0.0])
                .is_err()
        );
    }

    #[test]
    fn arc_surface_round_trips_and_measures_the_exact_extruded_arc() {
        let kernel = Kernel::from_environment().unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::ArcSurface, [2.0, 3.0, 0.0])
            .unwrap()
            .with_arc(Some(ArcFeature::new(0.0, 90.0).unwrap()))
            .unwrap();
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_fourteen =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=14", 1);
        assert_eq!(PrimitiveModel::parse(&version_fourteen).unwrap(), model);
        let version_thirteen =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=13", 1);
        assert!(PrimitiveModel::parse(&version_thirteen).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.mass_properties().unwrap()[1] - 3.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 2.0, 2.0, 3.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(shape.tessellate(0.05, 0.35).unwrap().triangles.len() >= 2);
    }

    #[test]
    fn revolved_surface_round_trips_and_preserves_its_open_profile() {
        let kernel = Kernel::from_environment().unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::RevolvedSurface, [0.0; 3]).unwrap();
        assert_eq!(
            model.open_polyline.unwrap().points(),
            &[[5.0, -2.0], [5.0, 2.0]]
        );
        let serialized = model.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), model);
        let version_fourteen =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=14", 1);
        assert!(PrimitiveModel::parse(&version_fourteen).is_err());

        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.mass_properties().unwrap()[1] - 40.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, -2.0, -5.0, 5.0, 2.0, 5.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(shape.tessellate(0.05, 0.35).unwrap().triangles.len() >= 32);
    }

    #[test]
    fn ring_band_has_expected_inner_diameter_and_volume() {
        let kernel = Kernel::from_environment().unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::RingBand, [18.0, 2.0, 4.0]).unwrap();
        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        let expected_volume = std::f64::consts::PI * ((11.0_f64).powi(2) - 9.0_f64.powi(2)) * 4.0;
        assert!((shape.volume_mm3().unwrap() - expected_volume).abs() < 1e-7);
        let bounds = shape.bounds_mm().unwrap();
        assert!((bounds[3] - bounds[0] - 22.0).abs() < 1e-7);
        assert!((bounds[4] - bounds[1] - 22.0).abs() < 1e-7);
        assert!((bounds[5] - bounds[2] - 4.0).abs() < 1e-7);
    }

    #[test]
    fn round_brilliant_builds_valid_measurable_geometry() {
        let kernel = Kernel::from_environment().unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::RoundBrilliant, [6.0, 3.6, 3.3]).unwrap();
        let shape = model.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!(shape.volume_mm3().unwrap() > 0.0);
        let bounds = shape.bounds_mm().unwrap();
        for (actual, expected) in bounds.into_iter().zip([-3.0, -3.0, 0.0, 3.0, 3.0, 3.6]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        let mesh = shape.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.triangles.len() > 300);
    }

    #[test]
    fn array_feature_preserves_source_solid_and_generates_valid_copies() {
        let kernel = Kernel::from_environment().unwrap();
        let model = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 2.0, 2.0])
            .unwrap()
            .with_pattern(Some(PatternFeature::linear(3, [4.0, 0.0, 0.0]).unwrap()));
        let result = model.create_shape(&kernel).unwrap();
        assert!(result.is_valid().unwrap());
        assert!((result.volume_mm3().unwrap() - 24.0).abs() < 1e-8);
        let bounds = result.bounds_mm().unwrap();
        for (actual, expected) in bounds.into_iter().zip([0.0, 0.0, 0.0, 10.0, 2.0, 2.0]) {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn bezier_curve_extrudes_to_surface_and_round_trips_in_version_twenty() {
        let controls =
            OpenPolylineFeature::from_points(&[[-5.0, 0.0], [-2.0, 6.0], [2.0, -6.0], [5.0, 0.0]])
                .unwrap();
        let curve = PrimitiveModel::new(PrimitiveKind::BezierCurve, [0.0; 3])
            .unwrap()
            .with_open_polyline(Some(controls))
            .unwrap();
        let serialized = curve.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), curve);
        let version_nineteen =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=19", 1);
        assert_eq!(PrimitiveModel::parse(&version_nineteen).unwrap(), curve);
        let version_eighteen =
            serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=18", 1);
        assert!(PrimitiveModel::parse(&version_eighteen).is_err());
        assert!(curve
            .with_open_polyline(Some(OpenPolylineFeature::default()))
            .is_err());

        let surface = PrimitiveModel::new(PrimitiveKind::BezierSurface, [5.0, 0.0, 0.0])
            .unwrap()
            .with_open_polyline(Some(controls))
            .unwrap()
            .with_extrusion_direction([0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(
            PrimitiveModel::parse(&surface.serialize()).unwrap(),
            surface
        );
        let kernel = Kernel::from_environment().unwrap();
        let curve_shape = curve.create_shape(&kernel).unwrap();
        let surface_shape = surface.create_shape(&kernel).unwrap();
        assert!(curve_shape.is_valid().unwrap());
        assert!(surface_shape.is_valid().unwrap());
        let expected_area = curve_shape.length_mm().unwrap() * 5.0;
        assert!(
            (surface_shape.mass_properties().unwrap()[1] - expected_area).abs()
                < expected_area * 1e-2
        );
    }

    #[test]
    fn revolved_bezier_surface_round_trips_and_builds_exact_occt_shell() {
        let controls = OpenPolylineFeature::from_points(&[
            [5.0, -2.0],
            [5.0, -2.0 / 3.0],
            [5.0, 2.0 / 3.0],
            [5.0, 2.0],
        ])
        .unwrap();
        let surface = PrimitiveModel::new(PrimitiveKind::RevolvedBezierSurface, [0.0; 3])
            .unwrap()
            .with_open_polyline(Some(controls))
            .unwrap();
        let serialized = surface.serialize();
        assert!(serialized.starts_with("OPENJEWELCAD_PROJECT=20\n"));
        assert_eq!(PrimitiveModel::parse(&serialized).unwrap(), surface);
        let legacy = serialized.replacen("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=19", 1);
        assert!(PrimitiveModel::parse(&legacy).is_err());
        assert!(
            PrimitiveModel::new(PrimitiveKind::RevolvedBezierSurface, [0.0; 3])
                .unwrap()
                .with_open_polyline(Some(
                    OpenPolylineFeature::from_points(&[
                        [0.0, -2.0],
                        [5.0, -2.0 / 3.0],
                        [5.0, 2.0 / 3.0],
                        [5.0, 2.0],
                    ])
                    .unwrap()
                ))
                .is_err()
        );

        let kernel = Kernel::from_environment().unwrap();
        let shape = surface.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        let properties = shape.mass_properties().unwrap();
        assert!((properties[1] - 40.0 * std::f64::consts::PI).abs() < 0.4);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, -2.0, -5.0, 5.0, 2.0, 5.0])
        {
            assert!((actual - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn transform_values_are_validated_and_round_trip() {
        let transform = TransformParams {
            translation_mm: [1.0, -2.0, 3.0],
            rotation_deg: [15.0, 30.0, 90.0],
            uniform_scale: 2.5,
            mirror_axes: [true, false, true],
        };
        let model = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 3.0, 4.0])
            .unwrap()
            .with_transform(transform)
            .unwrap()
            .with_material_density(Some(19.32))
            .unwrap();
        assert_eq!(PrimitiveModel::parse(&model.serialize()).unwrap(), model);
        assert!(model.with_material_density(Some(f64::NAN)).is_err());
        assert!(TransformParams {
            uniform_scale: 0.0,
            ..TransformParams::IDENTITY
        }
        .validate()
        .is_err());
        assert!(TransformParams {
            translation_mm: [f64::NAN, 0.0, 0.0],
            ..TransformParams::IDENTITY
        }
        .validate()
        .is_err());
    }

    #[test]
    fn mirror_command_persists_plane_and_regenerates_world_geometry() {
        let path = env::temp_dir().join(format!(
            "openjewelcad-{}-mirror-command.ojcad",
            std::process::id()
        ));
        let model = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 3.0, 4.0])
            .unwrap()
            .with_transform(TransformParams {
                translation_mm: [5.0, 0.0, 0.0],
                ..TransformParams::IDENTITY
            })
            .unwrap();
        save_model(&path, model).unwrap();
        mirror_project(&path.to_string_lossy(), "yz", "on").unwrap();
        let mirrored = load_model(&path).unwrap();
        assert_eq!(mirrored.transform.mirror_axes, [true, false, false]);
        let kernel = Kernel::from_environment().unwrap();
        let shape = mirrored.create_shape(&kernel).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.volume_mm3().unwrap() - 24.0).abs() < 1e-8);
        for (actual, expected) in shape
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-7.0, 0.0, 0.0, -5.0, 3.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-8);
        }
        mirror_project(&path.to_string_lossy(), "yz", "off").unwrap();
        assert_eq!(load_model(&path).unwrap().transform.mirror_axes, [false; 3]);

        let legacy = PrimitiveModel::new(PrimitiveKind::Box, [2.0, 3.0, 4.0])
            .unwrap()
            .serialize()
            .replace("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=15")
            .replace("mirror_x=false\n", "")
            .replace("mirror_y=false\n", "")
            .replace("mirror_z=false\n", "");
        assert_eq!(
            PrimitiveModel::parse(&legacy)
                .unwrap()
                .transform
                .mirror_axes,
            [false; 3]
        );
        let legacy_with_mirror = mirrored
            .serialize()
            .replace("OPENJEWELCAD_PROJECT=20", "OPENJEWELCAD_PROJECT=15");
        assert!(PrimitiveModel::parse(&legacy_with_mirror).is_err());
        assert!(mirror_project(&path.to_string_lossy(), "yz", "sideways").is_err());
        let _ = fs::remove_file(path);
    }
}
