//! Foundational geometry values for Open Jewel CAD.
//!
//! Coordinates and linear tolerances are represented in millimetres. These
//! types do not perform CAD-kernel operations and do not imply shape validity.

use std::fmt;

/// A finite distance in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct LengthMm(f64);

impl LengthMm {
    pub fn new(value: f64) -> Result<Self, GeometryError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub const fn zero() -> Self {
        Self(0.0)
    }
    pub const fn as_mm(self) -> f64 {
        self.0
    }
}

/// A finite position in the right-handed, millimetre-based world coordinate system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3Mm {
    x: LengthMm,
    y: LengthMm,
    z: LengthMm,
}

impl Point3Mm {
    pub fn new(x_mm: f64, y_mm: f64, z_mm: f64) -> Result<Self, GeometryError> {
        Ok(Self {
            x: LengthMm::new(x_mm)?,
            y: LengthMm::new(y_mm)?,
            z: LengthMm::new(z_mm)?,
        })
    }

    pub fn translated(self, offset: Vector3) -> Result<Self, GeometryError> {
        Self::new(
            self.x.as_mm() + offset.x,
            self.y.as_mm() + offset.y,
            self.z.as_mm() + offset.z,
        )
    }

    /// Returns the displacement from `self` to `other`.
    pub fn vector_to(self, other: Self) -> Result<Vector3, GeometryError> {
        Vector3::new(
            other.x.as_mm() - self.x.as_mm(),
            other.y.as_mm() - self.y.as_mm(),
            other.z.as_mm() - self.z.as_mm(),
        )
    }

    pub fn distance_to(self, other: Self) -> Result<LengthMm, GeometryError> {
        LengthMm::new(self.vector_to(other)?.magnitude()?)
    }

    pub const fn coordinates_mm(self) -> (f64, f64, f64) {
        (self.x.as_mm(), self.y.as_mm(), self.z.as_mm())
    }

    pub fn is_within(self, bounds: AxisAlignedBoxMm, tolerance: LengthMm) -> bool {
        let (x, y, z) = self.coordinates_mm();
        let (min, max) = bounds.corners_mm();
        let t = tolerance.as_mm();
        x >= min.0 - t
            && x <= max.0 + t
            && y >= min.1 - t
            && y <= max.1 + t
            && z >= min.2 - t
            && z <= max.2 + t
    }
}

/// A finite axis-aligned bounding box in millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisAlignedBoxMm {
    min: Point3Mm,
    max: Point3Mm,
}

impl AxisAlignedBoxMm {
    pub fn new(min: Point3Mm, max: Point3Mm) -> Result<Self, GeometryError> {
        let (min_x, min_y, min_z) = min.coordinates_mm();
        let (max_x, max_y, max_z) = max.coordinates_mm();
        if min_x > max_x || min_y > max_y || min_z > max_z {
            return Err(GeometryError::InvalidBounds);
        }
        Ok(Self { min, max })
    }

    pub const fn corners_mm(self) -> ((f64, f64, f64), (f64, f64, f64)) {
        (self.min.coordinates_mm(), self.max.coordinates_mm())
    }

    pub fn dimensions_mm(self) -> Result<(f64, f64, f64), GeometryError> {
        let (min, max) = self.corners_mm();
        Ok((
            LengthMm::new(max.0 - min.0)?.as_mm(),
            LengthMm::new(max.1 - min.1)?.as_mm(),
            LengthMm::new(max.2 - min.2)?.as_mm(),
        ))
    }

    pub fn volume_mm3(self) -> Result<f64, GeometryError> {
        let (x, y, z) = self.dimensions_mm()?;
        let volume = x * y * z;
        if volume.is_finite() {
            Ok(volume)
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub fn union(self, other: Self) -> Result<Self, GeometryError> {
        let (a_min, a_max) = self.corners_mm();
        let (b_min, b_max) = other.corners_mm();
        Self::new(
            Point3Mm::new(
                a_min.0.min(b_min.0),
                a_min.1.min(b_min.1),
                a_min.2.min(b_min.2),
            )?,
            Point3Mm::new(
                a_max.0.max(b_max.0),
                a_max.1.max(b_max.1),
                a_max.2.max(b_max.2),
            )?,
        )
    }
}

/// A finite direction or displacement vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, GeometryError> {
        if [x, y, z].iter().all(|v| v.is_finite()) {
            Ok(Self { x, y, z })
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub fn dot(self, rhs: Self) -> Result<f64, GeometryError> {
        let value = self.x * rhs.x + self.y * rhs.y + self.z * rhs.z;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub fn cross(self, rhs: Self) -> Result<Self, GeometryError> {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn magnitude(self) -> Result<f64, GeometryError> {
        let value = self.x.hypot(self.y).hypot(self.z);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub fn scaled(self, factor: f64) -> Result<Self, GeometryError> {
        if !factor.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }
    pub const fn components(self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    pub fn normalized(self) -> Result<Self, GeometryError> {
        let magnitude = self.magnitude()?;
        if magnitude == 0.0 {
            return Err(GeometryError::ZeroLengthDirection);
        }
        Self::new(self.x / magnitude, self.y / magnitude, self.z / magnitude)
    }
}

/// Explicit linear and angular tolerances for geometry operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryTolerance {
    linear: LengthMm,
    angular_radians: f64,
}

impl GeometryTolerance {
    pub fn new(linear_mm: f64, angular_radians: f64) -> Result<Self, GeometryError> {
        if !angular_radians.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }
        if linear_mm <= 0.0 || angular_radians <= 0.0 {
            return Err(GeometryError::ToleranceMustBePositive);
        }
        Ok(Self {
            linear: LengthMm::new(linear_mm)?,
            angular_radians,
        })
    }

    pub const fn linear_mm(self) -> f64 {
        self.linear.as_mm()
    }
    pub const fn angular_radians(self) -> f64 {
        self.angular_radians
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    NonFiniteValue,
    ZeroLengthDirection,
    ToleranceMustBePositive,
    InvalidBounds,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => write!(f, "geometry values must be finite"),
            Self::ZeroLengthDirection => write!(f, "a zero-length vector has no direction"),
            Self::ToleranceMustBePositive => write!(f, "geometry tolerances must be positive"),
            Self::InvalidBounds => write!(f, "bounding box minimum must not exceed its maximum"),
        }
    }
}

impl std::error::Error for GeometryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance_and_displacement_use_millimetres() {
        let a = Point3Mm::new(1.0, 2.0, 3.0).unwrap();
        let b = Point3Mm::new(4.0, 6.0, 3.0).unwrap();

        assert_eq!(a.vector_to(b).unwrap().components(), (3.0, 4.0, 0.0));
        assert_eq!(a.distance_to(b).unwrap().as_mm(), 5.0);
    }

    #[test]
    fn vector_dot_cross_and_normalization_are_consistent() {
        let x = Vector3::new(1.0, 0.0, 0.0).unwrap();
        let y = Vector3::new(0.0, 1.0, 0.0).unwrap();

        assert_eq!(x.dot(y).unwrap(), 0.0);
        assert_eq!(x.cross(y).unwrap().components(), (0.0, 0.0, 1.0));
        assert_eq!(
            Vector3::new(0.0, 3.0, 4.0)
                .unwrap()
                .normalized()
                .unwrap()
                .components(),
            (0.0, 0.6, 0.8)
        );
    }

    #[test]
    fn rejects_non_finite_geometry_values() {
        assert_eq!(
            Point3Mm::new(f64::NAN, 0.0, 0.0),
            Err(GeometryError::NonFiniteValue)
        );
        assert_eq!(
            Vector3::new(0.0, f64::INFINITY, 0.0),
            Err(GeometryError::NonFiniteValue)
        );
        assert_eq!(
            Vector3::new(f64::MAX, f64::MAX, 0.0).unwrap().magnitude(),
            Err(GeometryError::NonFiniteValue)
        );
    }

    #[test]
    fn zero_vector_cannot_be_normalized() {
        assert_eq!(
            Vector3::new(0.0, 0.0, 0.0).unwrap().normalized(),
            Err(GeometryError::ZeroLengthDirection)
        );
    }

    #[test]
    fn tolerance_requires_positive_finite_values() {
        assert_eq!(
            GeometryTolerance::new(0.0, 1e-6),
            Err(GeometryError::ToleranceMustBePositive)
        );
        assert_eq!(
            GeometryTolerance::new(0.01, f64::INFINITY),
            Err(GeometryError::NonFiniteValue)
        );
        let tolerance = GeometryTolerance::new(0.01, 1e-6).unwrap();
        assert_eq!(tolerance.linear_mm(), 0.01);
        assert_eq!(tolerance.angular_radians(), 1e-6);
    }

    #[test]
    fn bounding_box_reports_dimensions_volume_and_union() {
        let a = AxisAlignedBoxMm::new(
            Point3Mm::new(0.0, 0.0, 0.0).unwrap(),
            Point3Mm::new(2.0, 3.0, 4.0).unwrap(),
        )
        .unwrap();
        let b = AxisAlignedBoxMm::new(
            Point3Mm::new(-1.0, 1.0, 2.0).unwrap(),
            Point3Mm::new(1.0, 5.0, 3.0).unwrap(),
        )
        .unwrap();

        assert_eq!(a.dimensions_mm().unwrap(), (2.0, 3.0, 4.0));
        assert_eq!(a.volume_mm3().unwrap(), 24.0);
        assert_eq!(
            a.union(b).unwrap().corners_mm(),
            ((-1.0, 0.0, 0.0), (2.0, 5.0, 4.0))
        );
        assert!(Point3Mm::new(2.005, 1.0, 1.0)
            .unwrap()
            .is_within(a, LengthMm::new(0.01).unwrap()));
    }

    #[test]
    fn rejects_inverted_bounding_box() {
        assert_eq!(
            AxisAlignedBoxMm::new(
                Point3Mm::new(1.0, 0.0, 0.0).unwrap(),
                Point3Mm::new(0.0, 1.0, 1.0).unwrap()
            ),
            Err(GeometryError::InvalidBounds)
        );
    }
}
