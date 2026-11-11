//! Denavit–Hartenberg link model.

use op_math::{deg_to_rad, Isometry3, Matrix3, Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Joint type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointType {
    /// Revolute joint (rotation, radians).
    Revolute,
    /// Prismatic joint (translation, metres).
    Prismatic,
}

/// Error raised when DH parameters are invalid for a chain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DhParamError {
    /// One or more links are missing.
    #[error("chain must contain at least one link")]
    EmptyChain,
}

/// A single Denavit–Hartenberg link.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DhLink {
    /// Joint angle offset (radians) — `theta`.
    pub theta: Scalar,
    /// Link offset (metres) — `d`.
    pub d: Scalar,
    /// Link length (metres) — `a`.
    pub a: Scalar,
    /// Link twist (radians) — `alpha`.
    pub alpha: Scalar,
    /// Joint type.
    pub joint: JointType,
}

impl DhLink {
    /// Construct a new DH link.
    pub const fn new(theta: Scalar, d: Scalar, a: Scalar, alpha: Scalar, joint: JointType) -> Self {
        Self { theta, d, a, alpha, joint }
    }

    /// Convenience constructor for a revolute link.
    pub const fn revolute(theta: Scalar, d: Scalar, a: Scalar, alpha: Scalar) -> Self {
        Self::new(theta, d, a, alpha, JointType::Revolute)
    }

    /// Convenience constructor for a prismatic link.
    pub const fn prismatic(theta: Scalar, d: Scalar, a: Scalar, alpha: Scalar) -> Self {
        Self::new(theta, d, a, alpha, JointType::Prismatic)
    }

    /// The DH transform from this link's parent frame to its own frame,
    /// given the current joint variable `q`.
    ///
    /// For a revolute joint, `q` replaces `theta`; for a prismatic joint,
    /// `q` replaces `d`.
    pub fn transform(&self, q: Scalar) -> Isometry3 {
        let (theta, d) = match self.joint {
            JointType::Revolute => (self.theta + q, self.d),
            JointType::Prismatic => (self.theta, self.d + q),
        };
        let (ct, st) = (theta.cos(), theta.sin());
        let (ca, sa) = (self.alpha.cos(), self.alpha.sin());

        // Standard DH: RotZ(theta) * TransZ(d) * TransX(a) * RotX(alpha)
        Isometry3::new(
            Matrix3::from_rows(
                Vector3::new(ct, -st * ca, st * sa),
                Vector3::new(st, ct * ca, -ct * sa),
                Vector3::new(0.0, sa, ca),
            ),
            Vector3::new(self.a * ct, self.a * st, d),
        )
    }

    /// Convenience for building an RCM-style arm: a parallelogram
    /// remote-center-of-motion link pivot given a mount offset along the tool axis.
    pub const fn rcm_pivot_link(mount_offset: Scalar) -> Self {
        Self::revolute(0.0, mount_offset, 0.0, deg_to_rad(0.0))
    }
}