//! # op-kinematics — Surgical arm kinematics
//!
//! Denavit–Hartenberg modelling, forward/inverse kinematics, Jacobian
//! analysis and singularity detection for serial-chain surgical robot arms
//! (6–7 DoF) with a tool wrist. This is the formal model layer that higher
//! layers (trajectory, constraints, verifier) rely on.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod dh;
mod forward;
mod inverse;
mod jacobian;

pub use dh::{DhLink, DhParamError, JointType};
pub use forward::ForwardKinematics;
pub use inverse::InverseKinematics;
pub use jacobian::{Jacobian, SingularityKind, SingularityReport};

use op_math::{Isometry3, Scalar, Vector3};

/// A single joint configuration (radians for revolute, metres for prismatic).
pub type JointState = Vec<Scalar>;

/// A rigid body composed of links chained by joints.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Chain {
    /// Denavit–Hartenberg links, ordered from base to tip.
    pub links: Vec<DhLink>,
}

impl Chain {
    /// Number of joints.
    pub fn dof(&self) -> usize {
        self.links.len()
    }
}

/// Pose of the tool frame in the base frame.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolPose {
    /// Position of the tool tip in the base frame.
    pub position: Vector3,
    /// Orientation of the tool frame relative to the base frame.
    pub orientation: Isometry3,
}