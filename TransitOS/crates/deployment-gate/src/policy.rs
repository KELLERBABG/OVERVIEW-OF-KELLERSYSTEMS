use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatePolicy {
    Strict,
    #[default]
    Normal,
    Permissive,
}

impl GatePolicy {
    pub fn require_invariant_check(&self) -> bool {
        matches!(self, GatePolicy::Strict | GatePolicy::Normal)
    }

    pub fn require_taint_check(&self) -> bool {
        matches!(self, GatePolicy::Strict | GatePolicy::Normal)
    }

    pub fn require_collision_check(&self) -> bool {
        matches!(self, GatePolicy::Strict)
    }

    pub fn require_edge_case_check(&self) -> bool {
        matches!(self, GatePolicy::Strict | GatePolicy::Normal)
    }

    pub fn allow_on_timeout(&self) -> bool {
        matches!(self, GatePolicy::Permissive)
    }
}
