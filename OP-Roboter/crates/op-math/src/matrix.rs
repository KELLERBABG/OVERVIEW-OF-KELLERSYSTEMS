//! 3×3 matrix type used for rotations.

use crate::{Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Row-major 3×3 matrix.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Matrix3 {
    /// Row 0.
    pub r0: Vector3,
    /// Row 1.
    pub r1: Vector3,
    /// Row 2.
    pub r2: Vector3,
}

impl Matrix3 {
    /// Identity matrix.
    pub const IDENTITY: Self = Self::from_rows(Vector3::X, Vector3::Y, Vector3::Z);

    /// Construct from three rows.
    #[inline]
    pub const fn from_rows(r0: Vector3, r1: Vector3, r2: Vector3) -> Self {
        Self { r0, r1, r2 }
    }

    /// Construct from three columns.
    #[inline]
    pub fn from_columns(c0: Vector3, c1: Vector3, c2: Vector3) -> Self {
        Self {
            r0: Vector3::new(c0.x, c1.x, c2.x),
            r1: Vector3::new(c0.y, c1.y, c2.y),
            r2: Vector3::new(c0.z, c1.z, c2.z),
        }
    }

    /// Matrix-matrix product.
    #[inline]
    pub fn mul_matrix(self, rhs: Self) -> Self {
        // Result column j = M · (column j of S).
        let c0 = Vector3::new(rhs.r0.x, rhs.r1.x, rhs.r2.x);
        let c1 = Vector3::new(rhs.r0.y, rhs.r1.y, rhs.r2.y);
        let c2 = Vector3::new(rhs.r0.z, rhs.r1.z, rhs.r2.z);
        Self::from_columns(self * c0, self * c1, self * c2)
    }

    /// Transpose.
    #[inline]
    pub fn transpose(self) -> Self {
        Self::from_columns(self.r0, self.r1, self.r2)
    }

    /// Determinant.
    #[inline]
    pub fn determinant(self) -> Scalar {
        let m = self;
        m.r0.x * (m.r1.y * m.r2.z - m.r1.z * m.r2.y)
            - m.r0.y * (m.r1.x * m.r2.z - m.r1.z * m.r2.x)
            + m.r0.z * (m.r1.x * m.r2.y - m.r1.y * m.r2.x)
    }

    /// True when the matrix is a rotation (orthonormal, det = +1) within `tol`.
    #[inline]
    pub fn is_rotation(self, tol: Scalar) -> bool {
        let t = self.transpose();
        let prod = self.mul_matrix(t);
        let id_err = (prod.r0 - Vector3::X).norm_squared()
            + (prod.r1 - Vector3::Y).norm_squared()
            + (prod.r2 - Vector3::Z).norm_squared();
        id_err < tol * tol && (self.determinant() - 1.0).abs() < tol
    }

    /// Access an element by row `i` and column `j`.
    #[inline]
    pub fn at(self, i: usize, j: usize) -> Scalar {
        debug_assert!(i < 3 && j < 3);
        let row = [self.r0, self.r1, self.r2][i];
        [row.x, row.y, row.z][j]
    }
}

impl std::ops::Mul<Vector3> for Matrix3 {
    type Output = Vector3;
    #[inline]
    fn mul(self, rhs: Vector3) -> Self::Output {
        Vector3::new(self.r0.dot(rhs), self.r1.dot(rhs), self.r2.dot(rhs))
    }
}

impl std::ops::Mul<Scalar> for Matrix3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Scalar) -> Self::Output {
        Self::from_rows(self.r0 * rhs, self.r1 * rhs, self.r2 * rhs)
    }
}

#[cfg(test)]
mod tests {
    use crate::EPSILON;
    use super::*;

    #[test]
    fn identity_multiplies_vector() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(Matrix3::IDENTITY * v, v);
    }

    #[test]
    fn identity_is_rotation() {
        assert!(Matrix3::IDENTITY.is_rotation(EPSILON));
    }

    #[test]
    fn from_columns_round_trip() {
        let m = Matrix3::from_columns(Vector3::X, Vector3::Y, Vector3::Z);
        assert_eq!(m, Matrix3::IDENTITY);
    }

    #[test]
    fn mul_matrix_is_correct() {
        // Rotation about Z by 90° times itself should be a 180° rotation.
        let rz90 = Matrix3::from_rows(
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let rz180 = rz90.mul_matrix(rz90);
        assert!(rz180.is_rotation(EPSILON));
        // 90° twice → (X → -X).
        let v = rz180 * Vector3::X;
        assert!(v.approx_eq(-Vector3::X, 1e-12));
    }
}