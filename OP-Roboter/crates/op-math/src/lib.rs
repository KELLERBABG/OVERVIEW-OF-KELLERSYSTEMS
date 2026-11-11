//! # op-math — Foundation math for OP-Roboter
//!
//! Vectors, matrices, quaternions and homogeneous transforms used by every
//! higher layer of the formally verified surgical robot control stack.
//!
//! All types are plain `f64`-backed, `Copy`, and `serde`-serializable so they
//! can flow through constraint generation, SMT verification and the execution
//! gate without lossy conversions.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod matrix;
mod quaternion;
mod transform;
mod vector;

pub use matrix::Matrix3;
pub use quaternion::Quaternion;
pub use transform::{Isometry3, Transform};
pub use vector::Vector3;

/// Shared floating point type across the workspace.
pub type Scalar = f64;

/// Smallest meaningful length used by workspace/tissue checks (metres).
pub const EPSILON: Scalar = 1e-9;

/// Convert degrees to radians.
#[inline]
pub const fn deg_to_rad(deg: Scalar) -> Scalar {
    deg * std::f64::consts::PI / 180.0
}

/// Convert radians to degrees.
#[inline]
pub fn rad_to_deg(rad: Scalar) -> Scalar {
    rad * 180.0 / std::f64::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degree_conversion_round_trips() {
        assert!((rad_to_deg(deg_to_rad(30.0)) - 30.0).abs() < EPSILON);
        assert!((rad_to_deg(deg_to_rad(90.0)) - 90.0).abs() < EPSILON);
    }
}