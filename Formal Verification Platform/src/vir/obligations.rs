//! VIR obligations: spatial and temporal memory safety properties.

use std::fmt;

/// A spatial obligation (buffer overflow/underflow).
#[derive(Debug, Clone)]
pub enum SpatialObligation {
    BufferOverflow {
        func: String,
        variable: String,
        index: usize,
        bound: usize,
    },
    BufferUnderflow {
        func: String,
        variable: String,
        offset: i64,
    },
}

impl fmt::Display for SpatialObligation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpatialObligation::BufferOverflow { func, variable, index, bound } => {
                write!(f, "BUFFER_OVERFLOW: {}[{}] in {} (bound={})", variable, index, func, bound)
            }
            SpatialObligation::BufferUnderflow { func, variable, offset } => {
                write!(f, "BUFFER_UNDERFLOW: {}{} in {}", variable, offset, func)
            }
        }
    }
}

impl SpatialObligation {
    pub fn func(&self) -> &str {
        match self {
            SpatialObligation::BufferOverflow { func, .. } => func,
            SpatialObligation::BufferUnderflow { func, .. } => func,
        }
    }

    pub fn variable(&self) -> &str {
        match self {
            SpatialObligation::BufferOverflow { variable, .. } => variable,
            SpatialObligation::BufferUnderflow { variable, .. } => variable,
        }
    }

    pub fn index(&self) -> Option<usize> {
        match self {
            SpatialObligation::BufferOverflow { index, .. } => Some(*index),
            _ => None,
        }
    }
}

/// A temporal obligation (use-after-free, double-free, leak).
#[derive(Debug, Clone)]
pub enum TemporalObligation {
    UseAfterFree {
        func: String,
        variable: String,
    },
    DoubleFree {
        func: String,
        variable: String,
    },
    MemoryLeak {
        func: String,
        variable: String,
    },
}

impl fmt::Display for TemporalObligation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemporalObligation::UseAfterFree { func, variable } => {
                write!(f, "USE_AFTER_FREE: {} in {}", variable, func)
            }
            TemporalObligation::DoubleFree { func, variable } => {
                write!(f, "DOUBLE_FREE: {} in {}", variable, func)
            }
            TemporalObligation::MemoryLeak { func, variable } => {
                write!(f, "MEMORY_LEAK: {} in {}", variable, func)
            }
        }
    }
}

impl TemporalObligation {
    pub fn func(&self) -> &str {
        match self {
            TemporalObligation::UseAfterFree { func, .. } => func,
            TemporalObligation::DoubleFree { func, .. } => func,
            TemporalObligation::MemoryLeak { func, .. } => func,
        }
    }

    pub fn variable(&self) -> &str {
        match self {
            TemporalObligation::UseAfterFree { variable, .. } => variable,
            TemporalObligation::DoubleFree { variable, .. } => variable,
            TemporalObligation::MemoryLeak { variable, .. } => variable,
        }
    }
}

/// A set of all obligations for a verification run.
#[derive(Debug, Clone)]
pub struct ObligationSet {
    pub spatial: Vec<SpatialObligation>,
    pub temporal: Vec<TemporalObligation>,
}

impl ObligationSet {
    pub fn new() -> Self {
        Self { spatial: Vec::new(), temporal: Vec::new() }
    }

    pub fn total_count(&self) -> usize {
        self.spatial.len() + self.temporal.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
}

impl Default for ObligationSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_obligation_display() {
        let obl = SpatialObligation::BufferOverflow {
            func: "main".into(),
            variable: "buf".into(),
            index: 10,
            bound: 5,
        };
        let s = format!("{}", obl);
        assert!(s.contains("BUFFER_OVERFLOW"));
        assert!(s.contains("buf"));
    }

    #[test]
    fn test_temporal_obligation_display() {
        let obl = TemporalObligation::DoubleFree {
            func: "cleanup".into(),
            variable: "ptr".into(),
        };
        let s = format!("{}", obl);
        assert!(s.contains("DOUBLE_FREE"));
    }

    #[test]
    fn test_obligation_set() {
        let set = ObligationSet {
            spatial: vec![SpatialObligation::BufferUnderflow {
                func: "f".into(),
                variable: "p".into(),
                offset: -1,
            }],
            temporal: vec![],
        };
        assert_eq!(set.total_count(), 1);
    }
}
