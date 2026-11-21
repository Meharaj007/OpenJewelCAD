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
        if value.is_finite() { Ok(Self(value)) } else { Err(GeometryError::NonFiniteValue) }
    }

    pub const fn zero() -> Self { Self(0.0) }
    pub const fn as_mm(self) -> f64 { self.0 }
}

/// A finite position in the right-handed, millimetre-based world coordinate system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3Mm { x: LengthMm, y: LengthMm, z: LengthMm }

impl Point3Mm {
    pub fn new(x_mm: f64, y_mm: f64, z_mm: f64) -> Result<Self, GeometryError> {
        Ok(Self { x: LengthMm::new(x_mm)?, y: LengthMm::new(y_mm)?, z: LengthMm::new(z_mm)? })
    }

    pub fn translated(self, offset: Vector3) -> Result<Self, GeometryError> {
        Self::new(self.x.as_mm() + offset.x, self.y.as_mm() + offset.y, self.z.as_mm() + offset.z)
    }

    pub const fn coordinates_mm(self) -> (f64, f64, f64) {
        (self.x.as_mm(), self.y.as_mm(), self.z.as_mm())
    }
}

/// A finite direction or displacement vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 { x: f64, y: f64, z: f64 }

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, GeometryError> {
        if [x, y, z].iter().all(|v| v.is_finite()) {
            Ok(Self { x, y, z })
        } else {
            Err(GeometryError::NonFiniteValue)
        }
    }

    pub fn dot(self, rhs: Self) -> f64 { self.x * rhs.x + self.y * rhs.y + self.z * rhs.z }

    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn magnitude(self) -> f64 { self.x.hypot(self.y).hypot(self.z) }
    pub const fn components(self) -> (f64, f64, f64) { (self.x, self.y, self.z) }

    pub fn normalized(self) -> Result<Self, GeometryError> {
        let magnitude = self.magnitude();
        if !magnitude.is_finite() { return Err(GeometryError::NonFiniteValue); }
        if magnitude == 0.0 { return Err(GeometryError::ZeroLengthDirection); }
        Self::new(self.x / magnitude, self.y / magnitude, self.z / magnitude)
    }
}

/// Explicit linear and angular tolerances for geometry operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryTolerance { linear: LengthMm, angular_radians: f64 }

impl GeometryTolerance {
    pub fn new(linear_mm: f64, angular_radians: f64) -> Result<Self, GeometryError> {
        if !angular_radians.is_finite() { return Err(GeometryError::NonFiniteValue); }
        if linear_mm <= 0.0 || angular_radians <= 0.0 {
            return Err(GeometryError::ToleranceMustBePositive);
        }
        Ok(Self { linear: LengthMm::new(linear_mm)?, angular_radians })
    }

    pub const fn linear_mm(self) -> f64 { self.linear.as_mm() }
    pub const fn angular_radians(self) -> f64 { self.angular_radians }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    NonFiniteValue,
    ZeroLengthDirection,
    ToleranceMustBePositive,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => write!(f, "geometry values must be finite"),
            Self::ZeroLengthDirection => write!(f, "a zero-length vector has no direction"),
            Self::ToleranceMustBePositive => write!(f, "geometry tolerances must be positive"),
        }
    }
}

impl std::error::Error for GeometryError {}
