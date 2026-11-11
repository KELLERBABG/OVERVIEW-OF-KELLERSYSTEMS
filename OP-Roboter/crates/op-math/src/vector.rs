//! 3D vector type.

use crate::{Scalar, EPSILON};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 3D vector used for positions, offsets, velocities and forces.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vector3 {
    /// X component.
    pub x: Scalar,
    /// Y component.
    pub y: Scalar,
    /// Z component.
    pub z: Scalar,
}

impl Vector3 {
    /// Zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// Unit vector along the X axis.
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    /// Unit vector along the Y axis.
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    /// Unit vector along the Z axis.
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Construct a new vector.
    #[inline]
    pub const fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }

    /// Squared Euclidean norm.
    #[inline]
    pub fn norm_squared(self) -> Scalar {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Euclidean norm (length).
    #[inline]
    pub fn norm(self) -> Scalar {
        self.norm_squared().sqrt()
    }

    /// Unit vector in the same direction.
    ///
    /// Returns `ZERO` when the input has negligible length.
    #[inline]
    pub fn normalized(self) -> Self {
        let n = self.norm();
        if n < EPSILON {
            Self::ZERO
        } else {
            self / n
        }
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, rhs: Self) -> Scalar {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Cross product.
    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    /// Component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// Elementwise minimum with another vector.
    #[inline]
    pub fn min(self, rhs: Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y), self.z.min(rhs.z))
    }

    /// Elementwise maximum with another vector.
    #[inline]
    pub fn max(self, rhs: Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y), self.z.max(rhs.z))
    }

    /// Clamp each component into `[min, max]`.
    #[inline]
    pub fn clamp(self, min: Scalar, max: Scalar) -> Self {
        Self::new(self.x.clamp(min, max), self.y.clamp(min, max), self.z.clamp(min, max))
    }

    /// Distance to another point.
    #[inline]
    pub fn distance(self, rhs: Self) -> Scalar {
        (self - rhs).norm()
    }

    /// Squared distance to another point.
    #[inline]
    pub fn distance_squared(self, rhs: Self) -> Scalar {
        (self - rhs).norm_squared()
    }

    /// True when the vector is within `tol` of another vector.
    #[inline]
    pub fn approx_eq(self, rhs: Self, tol: Scalar) -> bool {
        self.distance(rhs) <= tol
    }
}

impl Add for Vector3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vector3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vector3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vector3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<Scalar> for Vector3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Scalar) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vector3> for Scalar {
    type Output = Vector3;
    #[inline]
    fn mul(self, rhs: Vector3) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<Scalar> for Vector3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Scalar) {
        *self = *self * rhs;
    }
}

impl Div<Scalar> for Vector3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Scalar) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl DivAssign<Scalar> for Vector3 {
    #[inline]
    fn div_assign(&mut self, rhs: Scalar) {
        *self = *self / rhs;
    }
}

impl Neg for Vector3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_works() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vector3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vector3::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vector3::new(2.0, 4.0, 6.0));
        assert_eq!(2.0 * a, Vector3::new(2.0, 4.0, 6.0));
        assert_eq!(a / 2.0, Vector3::new(0.5, 1.0, 1.5));
    }

    #[test]
    fn norm_and_dot_cross() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(0.0, 1.0, 0.0);
        assert!((a.norm() - 1.0).abs() < EPSILON);
        assert!((a.dot(b)).abs() < EPSILON);
        assert_eq!(a.cross(b), Vector3::Z);
    }

    #[test]
    fn normalized_zero_is_safe() {
        let z = Vector3::ZERO.normalized();
        assert_eq!(z, Vector3::ZERO);
    }
}