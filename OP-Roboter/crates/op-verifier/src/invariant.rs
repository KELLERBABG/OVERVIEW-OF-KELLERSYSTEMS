//! Lethal invariants enforced by the verifier.

use op_math::{Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Kind of lethal invariant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InvariantKind {
    /// Tool tip must stay inside the reachable workspace envelope.
    WorkspaceEnvelope {
        /// Minimum corner of the envelope (m).
        min: Vector3,
        /// Maximum corner of the envelope (m).
        max: Vector3,
    },
    /// Tool tip must stay at least `clearance` (m) on the protected side.
    TissueBoundary {
        /// Required clearance (m).
        clearance: Scalar,
    },
    /// Tool-tip force must not exceed `max_force` (N).
    ForceLimit {
        /// Maximum force (N).
        max_force: Scalar,
    },
    /// Incision depth must not exceed `max_depth` (m).
    DepthLimit {
        /// Maximum depth (m).
        max_depth: Scalar,
    },
    /// RCM (trocar) point must remain within `tolerance` (m) of the shaft axis.
    RcmStay {
        /// Allowed deviation (m).
        tolerance: Scalar,
    },
    /// Total energy delivered during the action must not exceed `max_joules` (J).
    EnergyBudgetLimit {
        /// Maximum allowed energy (J).
        max_joules: Scalar,
    },
    /// Peak power during the action must not exceed `max_watts` (W).
    PowerLimit {
        /// Maximum allowed peak power (W).
        max_watts: Scalar,
    },
}

/// A named lethal invariant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LethalInvariant {
    /// Invariant name (for reporting).
    pub name: String,
    /// Invariant kind and bound.
    pub kind: InvariantKind,
}

/// A set of lethal invariants.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LethalInvariantSet {
    /// All invariants in the set.
    pub invariants: Vec<LethalInvariant>,
}

impl LethalInvariantSet {
    /// Construct a set from invariants.
    pub fn new(invariants: Vec<LethalInvariant>) -> Self {
        Self { invariants }
    }
}

/// A reported violation of a lethal invariant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    /// Name of the violated invariant.
    pub invariant: String,
    /// Kind of the violated invariant.
    pub kind: InvariantKind,
    /// Human-readable detail.
    pub detail: String,
}