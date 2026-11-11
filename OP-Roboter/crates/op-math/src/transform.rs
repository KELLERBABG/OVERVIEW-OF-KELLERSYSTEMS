//! Homogeneous transforms (isometries) in 3D.

use crate::{Matrix3, Quaternion, Vector3};
use serde::{Deserialize, Serialize};

/// A rigid isometry: rotation plus translation.
///
/// This maps points from a child frame into a parent frame:
/// `p_parent = rotation * p_child + translation`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Isometry3 {
    /// Rotation (orthonormal, det +1).
    pub rotation: Matrix3,
    /// Translation.
    pub translation: Vector3,
}

/// Convenience alias — an isometry is the natural "transform" for rigid links.
pub type Transform = Isometry3;

impl Isometry3 {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        rotation: Matrix3::IDENTITY,
        translation: Vector3::ZERO,
    };

    /// Construct from rotation matrix and translation.
    #[inline]
    pub const fn new(rotation: Matrix3, translation: Vector3) -> Self {
        Self { rotation, translation }
    }

    /// Pure translation transform.
    #[inline]
    pub const fn from_translation(translation: Vector3) -> Self {
        Self::new(Matrix3::IDENTITY, translation)
    }

    /// Pure rotation transform.
    #[inline]
    pub const fn from_rotation(rotation: Matrix3) -> Self {
        Self::new(rotation, Vector3::ZERO)
    }

    /// Transform from rotation quaternion and translation.
    #[inline]
    pub fn from_rotation_quaternion(rotation: Quaternion, translation: Vector3) -> Self {
        Self::new(rotation.to_matrix(), translation)
    }

    /// Transform a point (or direction offset) from the child into the parent frame.
    #[inline]
    pub fn transform_point(self, p: Vector3) -> Vector3 {
        self.rotation * p + self.translation
    }

    /// Transform a direction only (no translation part).
    #[inline]
    pub fn transform_vector(self, v: Vector3) -> Vector3 {
        self.rotation * v
    }

    /// Compose `self` after `rhs`: applies `rhs` first, then `self`.
    #[inline]
    pub fn then(self, rhs: Self) -> Self {
        Self {
            rotation: self.rotation.mul_matrix(rhs.rotation),
            translation: self.rotation * rhs.translation + self.translation,
        }
    }

    /// Inverse transform.
    #[inline]
    pub fn inverse(self) -> Self {
        let r_inv = self.rotation.transpose();
        Self {
            rotation: r_inv,
            translation: -(r_inv * self.translation),
        }
    }

    /// Equivalent rotation quaternion.
    #[inline]
    pub fn quaternion(self) -> Quaternion {
        Quaternion::from_matrix(self.rotation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_transform_is_identity() {
        let t = Isometry3::IDENTITY;
        let p = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(t.transform_point(p), p);
        assert_eq!(t.inverse().transform_point(p), p);
    }

    #[test]
    fn transform_round_trip_with_inverse() {
        let t = Isometry3::from_rotation_quaternion(
            Quaternion::from_axis_angle(Vector3::Y, 0.8),
            Vector3::new(0.1, -0.2, 0.3),
        );
        let p = Vector3::new(0.4, 0.5, -0.6);
        let back = t.inverse().transform_point(t.transform_point(p));
        assert!(back.approx_eq(p, 1e-12));
    }

    #[test]
    fn composition_equals_sequential_application() {
        let a = Isometry3::from_translation(Vector3::new(1.0, 0.0, 0.0));
        let b = Isometry3::from_rotation_quaternion(
            Quaternion::from_axis_angle(Vector3::Z, 0.5),
            Vector3::ZERO,
        );
        let p = Vector3::new(0.0, 1.0, 0.0);
        let composed = a.then(b).transform_point(p);
        let sequential = a.transform_point(b.transform_point(p));
        assert!(composed.approx_eq(sequential, 1e-12));
    }
}