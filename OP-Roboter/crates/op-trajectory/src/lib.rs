//! # op-trajectory — 3D workspace, paths, RCM constraints
//!
//! Models the physical workspace of the surgical arm as a 3D constraint
//! system, represents candidate motion paths, and enforces
//! remote-center-of-motion (RCM) constraints so that tool motion pivots
//! around the trocar entry point. This layer is Z3-friendly: every geometric
//! check is expressed as scalar arithmetic that `op-constraints` later turns
//! into boolean/SMT constraints.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod path;
mod rcm;
mod workspace;

pub use path::{MotionPath, PathError, PathSample, interpolation};
pub use rcm::{RcmConstraint, RcmError};
pub use workspace::{TissueBoundary, Volume, Workspace, WorkspaceError};

use op_math::Isometry3;

/// A single waypoint along a planned motion path.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Waypoint {
    /// Pose of the tool frame.
    pub pose: Isometry3,
    /// Optional intended tool-tip force (N) at this waypoint.
    pub force: Option<op_math::Scalar>,
}

/// Minimum distance threshold used for collision/proximity checks (m).
pub const SAFETY_MARGIN: op_math::Scalar = 0.005;