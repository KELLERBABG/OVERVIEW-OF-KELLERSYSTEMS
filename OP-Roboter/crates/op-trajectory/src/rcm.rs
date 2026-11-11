//! Remote-center-of-motion (RCM) constraint enforcement.

use op_math::{Scalar, Vector3, EPSILON};
use serde::{Deserialize, Serialize};

/// Error raised when an RCM check fails.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RcmError {
    /// The tool axis is degenerate (zero length) so the RCM cannot be evaluated.
    #[error("tool axis is degenerate")]
    DegenerateAxis,
    /// The tool axis does not pass through the RCM point within tolerance.
    #[error("tool axis does not pass through the RCM point (off-axis distance {0:.6} m)")]
    OffAxis(Scalar),
    /// The RCM point is outside the tool shaft segment.
    #[error("RCM point is outside the tool shaft segment")]
    OutsideSegment,
}

/// Remote-center-of-motion constraint: the tool shaft must always pass through
/// a fixed trocar entry point.
///
/// Given the tool tip position `p` and the tool orientation axis `z`
/// (the unit tool-shaft axis in the base frame), the signed distance from the
/// RCM point to the shaft line is `|| (rcm - p) × z ||`. A valid configuration
/// keeps this distance below a tolerance, and keeps the RCM point between the
/// tool tip and the tool mount (i.e. inside the shaft segment).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RcmConstraint {
    /// Fixed trocar (port) point in the base frame (m).
    pub point: Vector3,
    /// Allowed off-axis deviation of the shaft from the RCM point (m).
    pub tolerance: Scalar,
    /// Distance from the tool tip to the tool mount along the shaft (m).
    pub shaft_length: Scalar,
}

impl RcmConstraint {
    /// Construct a new RCM constraint.
    pub fn new(point: Vector3, tolerance: Scalar, shaft_length: Scalar) -> Self {
        Self {
            point,
            tolerance,
            shaft_length,
        }
    }

    /// Compute the off-axis distance of the RCM point from the shaft line.
    ///
    /// `tip` is the tool tip position; `axis` is a non-zero direction along the
    /// tool shaft (need not be normalized).
    pub fn off_axis_distance(&self, tip: Vector3, axis: Vector3) -> Result<Scalar, RcmError> {
        let n = axis.norm();
        if n < EPSILON {
            return Err(RcmError::DegenerateAxis);
        }
        Ok((self.point - tip).cross(axis).norm() / n)
    }

    /// Verify that a tool pose satisfies the RCM constraint.
    ///
    /// `tip` is the tool tip and `axis` is the tool-shaft direction in the
    /// base frame (e.g. the Z basis vector of the tool frame).
    pub fn check(&self, tip: Vector3, axis: Vector3) -> Result<(), RcmError> {
        let d = self.off_axis_distance(tip, axis)?;
        if d > self.tolerance + EPSILON {
            return Err(RcmError::OffAxis(d));
        }
        // The RCM point must lie within the shaft segment behind the tip.
        let axis = axis.normalized();
        let along = (self.point - tip).dot(axis);
        if along < -EPSILON || along > self.shaft_length + EPSILON {
            return Err(RcmError::OutsideSegment);
        }
        Ok(())
    }

    /// Signed parameter `t` along the shaft at which the RCM point projects
    /// (0 at the tip, `shaft_length` at the mount).
    pub fn projection_along_shaft(&self, tip: Vector3, axis: Vector3) -> Result<Scalar, RcmError> {
        let axis = axis.normalized();
        if axis.norm() < EPSILON {
            return Err(RcmError::DegenerateAxis);
        }
        Ok((self.point - tip).dot(axis))
    }
}