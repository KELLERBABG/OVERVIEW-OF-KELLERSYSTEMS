//! Quaternion rotations.

use crate::{Matrix3, Scalar, Vector3, EPSILON};
use serde::{Deserialize, Serialize};

/// Unit quaternion `w + xi + yj + zk` representing a rotation.
///
/// Callers must keep the quaternion normalized; `normalized()` is provided
/// for that purpose.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Quaternion {
    /// Scalar (cos half-angle) component.
    pub w: Scalar,
    /// I (x) component.
    pub x: Scalar,
    /// J (y) component.
    pub y: Scalar,
    /// K (z) component.
    pub z: Scalar,
}

impl Quaternion {
    /// Identity quaternion (no rotation).
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 0.0);

    /// Construct a quaternion.
    #[inline]
    pub const fn new(w: Scalar, x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { w, x, y, z }
    }

    /// Construct a rotation of `angle` radians around the given (non-zero) axis.
    pub fn from_axis_angle(axis: Vector3, angle: Scalar) -> Self {
        let axis = axis.normalized();
        let half = angle * 0.5;
        let s = half.sin();
        Self::new(half.cos(), axis.x * s, axis.y * s, axis.z * s)
    }

    /// Construct from a rotation matrix. The matrix must be a rotation.
    pub fn from_matrix(m: Matrix3) -> Self {
        let trace = m.r0.x + m.r1.y + m.r2.z;
        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            let w = 0.25 * s;
            let x = (m.at(2, 1) - m.at(1, 2)) / s;
            let y = (m.at(0, 2) - m.at(2, 0)) / s;
            let z = (m.at(1, 0) - m.at(0, 1)) / s;
            Self::new(w, x, y, z)
        } else if m.at(0, 0) > m.at(1, 1) && m.at(0, 0) > m.at(2, 2) {
            let s = (1.0 + m.at(0, 0) - m.at(1, 1) - m.at(2, 2)).sqrt() * 2.0;
            let w = (m.at(2, 1) - m.at(1, 2)) / s;
            let x = 0.25 * s;
            let y = (m.at(0, 1) + m.at(1, 0)) / s;
            let z = (m.at(0, 2) + m.at(2, 0)) / s;
            Self::new(w, x, y, z)
        } else if m.at(1, 1) > m.at(2, 2) {
            let s = (1.0 + m.at(1, 1) - m.at(0, 0) - m.at(2, 2)).sqrt() * 2.0;
            let w = (m.at(0, 2) - m.at(2, 0)) / s;
            let x = (m.at(0, 1) + m.at(1, 0)) / s;
            let y = 0.25 * s;
            let z = (m.at(1, 2) + m.at(2, 1)) / s;
            Self::new(w, x, y, z)
        } else {
            let s = (1.0 + m.at(2, 2) - m.at(0, 0) - m.at(1, 1)).sqrt() * 2.0;
            let w = (m.at(1, 0) - m.at(0, 1)) / s;
            let x = (m.at(0, 2) + m.at(2, 0)) / s;
            let y = (m.at(1, 2) + m.at(2, 1)) / s;
            let z = 0.25 * s;
            Self::new(w, x, y, z)
        }
    }

    /// Norm.
    #[inline]
    pub fn norm(self) -> Scalar {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalized copy. Returns the identity when norm is negligible.
    #[inline]
    pub fn normalized(self) -> Self {
        let n = self.norm();
        if n < EPSILON {
            Self::IDENTITY
        } else {
            Self::new(self.w / n, self.x / n, self.y / n, self.z / n)
        }
    }

    /// Conjugate (inverse for unit quaternions).
    #[inline]
    pub fn conjugate(self) -> Self {
        Self::new(self.w, -self.x, -self.y, -self.z)
    }

    /// Hamilton product.
    #[inline]
    pub fn mul_quaternion(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        )
    }

    /// Rotate a vector.
    #[inline]
    pub fn rotate(self, v: Vector3) -> Vector3 {
        let q = self.normalized();
        let qv = Vector3::new(q.x, q.y, q.z);
        let t = qv.cross(v) * 2.0;
        v + t * q.w + qv.cross(t)
    }

    /// Rotation matrix equivalent.
    pub fn to_matrix(self) -> Matrix3 {
        let q = self.normalized();
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        Matrix3::from_rows(
            Vector3::new(
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
            ),
            Vector3::new(
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
            ),
            Vector3::new(
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
            ),
        )
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.mul_quaternion(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rotation_is_identity() {
        let q = Quaternion::IDENTITY;
        assert_eq!(q.rotate(Vector3::X), Vector3::X);
        assert_eq!(q.to_matrix(), Matrix3::IDENTITY);
    }

    #[test]
    fn rotate_90_degrees_about_z() {
        let q = Quaternion::from_axis_angle(Vector3::Z, std::f64::consts::FRAC_PI_2);
        let v = q.rotate(Vector3::X);
        assert!(v.approx_eq(Vector3::Y, 1e-12));
    }

    #[test]
    fn from_matrix_round_trip() {
        let q = Quaternion::from_axis_angle(Vector3::new(1.0, 2.0, 3.0), 0.7);
        let m = q.to_matrix();
        assert!(m.is_rotation(1e-12));
        let q2 = Quaternion::from_matrix(m);
        assert!((q2.mul_quaternion(q.conjugate()).norm() - 1.0).abs() < 1e-9);
    }
}