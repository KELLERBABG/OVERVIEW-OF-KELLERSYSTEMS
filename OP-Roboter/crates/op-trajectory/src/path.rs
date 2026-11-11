//! Motion path representation and interpolation.

use crate::Waypoint;
use op_math::{Quaternion, Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Error raised while building or sampling a motion path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// A path needs at least two waypoints.
    #[error("a motion path requires at least two waypoints")]
    TooFewWaypoints,
    /// A sample parameter is out of the valid range.
    #[error("sample parameter out of range [0, 1]")]
    ParamOutOfRange,
    /// Path length is zero.
    #[error("path length is zero")]
    ZeroLength,
    /// The end effector is already at the target position.
    #[error("start and target positions are identical")]
    IdenticalEndpoints,
}

/// A sampled pose along a path.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathSample {
    /// Interpolated tool position.
    pub position: Vector3,
    /// Interpolated orientation quaternion.
    pub orientation: Quaternion,
    /// Arc-length parameter in `[0, 1]`.
    pub t: Scalar,
}

/// Linear interpolation helpers for scalar, vector and quaternion.
pub mod interpolation {
    use super::*;

    /// Linear interpolation between two scalars.
    #[inline]
    pub fn lerp(a: Scalar, b: Scalar, t: Scalar) -> Scalar {
        a + (b - a) * t
    }

    /// Linear interpolation between two vectors.
    #[inline]
    pub fn lerp_vec(a: Vector3, b: Vector3, t: Scalar) -> Vector3 {
        Vector3::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.z, b.z, t))
    }

    /// Spherical linear interpolation between two quaternions.
    pub fn slerp(a: Quaternion, b: Quaternion, t: Scalar) -> Quaternion {
        let a = a.normalized();
        let mut b = b.normalized();
        let mut dot = a.w * b.w + a.x * b.x + a.y * b.y + a.z * b.z;
        if dot < 0.0 {
            b = Quaternion::new(-b.w, -b.x, -b.y, -b.z);
            dot = -dot;
        }
        let dot = dot.clamp(-1.0, 1.0);
        if dot > 0.9995 {
            return Quaternion::new(
                lerp(a.w, b.w, t),
                lerp(a.x, b.x, t),
                lerp(a.y, b.y, t),
                lerp(a.z, b.z, t),
            )
            .normalized();
        }
        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let s0 = (theta_0 - theta).cos() - dot * sin_theta / theta_0.sin();
        let s1 = sin_theta / theta_0.sin();
        Quaternion::new(
            s0 * a.w + s1 * b.w,
            s0 * a.x + s1 * b.x,
            s0 * a.y + s1 * b.y,
            s0 * a.z + s1 * b.z,
        )
        .normalized()
    }
}

/// A linear motion path between two tool waypoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionPath {
    /// Start waypoint.
    pub start: Waypoint,
    /// End waypoint.
    pub end: Waypoint,
}

impl MotionPath {
    /// Construct a path between two waypoints.
    pub fn new(start: Waypoint, end: Waypoint) -> Self {
        Self { start, end }
    }

    /// Total translation distance of the path (m).
    pub fn length(&self) -> Scalar {
        self.end.pose.translation.distance(self.start.pose.translation)
    }

    /// Sample the path at parameter `t` in `[0, 1]`.
    pub fn sample(&self, t: Scalar) -> Result<PathSample, PathError> {
        if !(0.0..=1.0).contains(&t) {
            return Err(PathError::ParamOutOfRange);
        }
        let pos = interpolation::lerp_vec(self.start.pose.translation, self.end.pose.translation, t);
        let ori = interpolation::slerp(
            self.start.pose.quaternion(),
            self.end.pose.quaternion(),
            t,
        );
        Ok(PathSample {
            position: pos,
            orientation: ori,
            t,
        })
    }

    /// Sample the path at `n` equal steps (including both endpoints).
    pub fn sample_n(&self, n: usize) -> Result<Vec<PathSample>, PathError> {
        if n < 2 {
            return Err(PathError::TooFewWaypoints);
        }
        (0..n)
            .map(|i| self.sample(i as Scalar / (n - 1) as Scalar))
            .collect()
    }

    /// True when the tool tip stays on the protected side of any workspace
    /// boundary along the entire path (sampled densely).
    pub fn is_safe_along(
        &self,
        workspace: &crate::Workspace,
        margin: Scalar,
        samples: usize,
    ) -> Result<bool, PathError> {
        for s in self.sample_n(samples)? {
            if !workspace.contains_with_margin(s.position, margin) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}