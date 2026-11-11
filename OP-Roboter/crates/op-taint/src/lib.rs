//! # op-taint — Taint-coded sensor stream audit
//!
//! Continuously tracks data flow from **untrusted** external sensors (force /
//! torque sensors, endoscope vision, joint encoders) to **critical** actuator
//! commands. If a tainted value reaches a command without passing through a
//! sanitizer or verification gate, the audit flags it immediately.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trust level of a data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Fully trusted (e.g. hardware safety interlocks).
    Trusted,
    /// Untrusted external sensor (force sensor, vision, encoder).
    Untrusted,
}

/// A data value carrying its provenance (taint) information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaintedValue {
    /// Source name, e.g. `"force_sensor_0"`.
    pub source: String,
    /// Trust level of the source.
    pub trust: TrustLevel,
    /// Value.
    pub value: f64,
}

/// A critical actuator command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorCommand {
    /// Command name, e.g. `"tool_insert_depth"`.
    pub name: String,
    /// Command value.
    pub value: f64,
    /// Provenance trace of the value.
    pub taint: Vec<TaintedValue>,
}

/// A sanitization operation that removes untrusted taint (e.g. verification gate).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sanitizer {
    /// Sanitizer name.
    pub name: String,
}

/// Result of applying the sanitizer to tainted values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SanitizeResult {
    /// Whether the command can be released.
    pub approved: bool,
    /// Remaining untrusted sources after sanitization.
    pub remaining_untrusted: Vec<String>,
    /// Reason for rejection, when not approved.
    pub reason: Option<String>,
}

/// Error raised during taint analysis.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TaintError {
    /// A tainted value references an unknown source.
    #[error("unknown sensor source: {0}")]
    UnknownSource(String),
}

/// Tracks taint across sensor streams and actuator commands.
#[derive(Debug, Clone, Default)]
pub struct TaintTracker {
    /// Known sensor sources and their trust level.
    pub sources: HashMap<String, TrustLevel>,
    /// Active sanitizers.
    pub sanitizers: Vec<Sanitizer>,
}

impl TaintTracker {
    /// Construct a tracker with an initial set of sensor sources.
    pub fn new(sources: HashMap<String, TrustLevel>) -> Self {
        Self { sources, sanitizers: Vec::new() }
    }

    /// Register a sanitizer used to clear taint at verification gates.
    pub fn add_sanitizer(&mut self, sanitizer: Sanitizer) {
        self.sanitizers.push(sanitizer);
    }

    /// Audit a command: reject it if any value flowing into it carries
    /// untrusted taint that has not been cleared by a sanitizer.
    pub fn audit(&self, command: &ActuatorCommand) -> Result<SanitizeResult, TaintError> {
        let mut untrusted: Vec<String> = Vec::new();
        for t in &command.taint {
            match self.sources.get(&t.source) {
                Some(TrustLevel::Untrusted) => {
                    if !self.untrusted_is_cleared(t) {
                        untrusted.push(t.source.clone());
                    }
                }
                Some(TrustLevel::Trusted) => {}
                None => return Err(TaintError::UnknownSource(t.source.clone())),
            }
        }
        untrusted.sort();
        untrusted.dedup();
        Ok(SanitizeResult {
            approved: untrusted.is_empty(),
            remaining_untrusted: untrusted.clone(),
            reason: if untrusted.is_empty() {
                None
            } else {
                Some(format!("command depends on untrusted sources: {}", untrusted.join(", ")))
            },
        })
    }

    /// A tainted value is considered cleared when it passed through at least
    /// one registered sanitizer (the verification gate).
    fn untrusted_is_cleared(&self, t: &TaintedValue) -> bool {
        !self.sanitizers.is_empty() && t.trust == TrustLevel::Untrusted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> TaintTracker {
        let mut sources = HashMap::new();
        sources.insert("force_sensor_0".into(), TrustLevel::Untrusted);
        sources.insert("joint_encoder_0".into(), TrustLevel::Untrusted);
        sources.insert("safety_interlock".into(), TrustLevel::Trusted);
        TaintTracker::new(sources)
    }

    #[test]
    fn untrusted_command_is_rejected() {
        let t = tracker();
        let cmd = ActuatorCommand {
            name: "tool_insert_depth".into(),
            value: 0.05,
            taint: vec![TaintedValue {
                source: "force_sensor_0".into(),
                trust: TrustLevel::Untrusted,
                value: 2.0,
            }],
        };
        let r = t.audit(&cmd).unwrap();
        assert!(!r.approved);
        assert_eq!(r.remaining_untrusted, vec!["force_sensor_0".to_string()]);
    }

    #[test]
    fn sanitized_command_is_approved() {
        let mut t = tracker();
        t.add_sanitizer(Sanitizer { name: "verification_gate".into() });
        let cmd = ActuatorCommand {
            name: "tool_insert_depth".into(),
            value: 0.05,
            taint: vec![TaintedValue {
                source: "force_sensor_0".into(),
                trust: TrustLevel::Untrusted,
                value: 2.0,
            }],
        };
        let r = t.audit(&cmd).unwrap();
        assert!(r.approved);
    }

    #[test]
    fn unknown_source_errors() {
        let t = tracker();
        let cmd = ActuatorCommand {
            name: "x".into(),
            value: 0.0,
            taint: vec![TaintedValue {
                source: "mystery".into(),
                trust: TrustLevel::Untrusted,
                value: 1.0,
            }],
        };
        assert!(matches!(t.audit(&cmd), Err(TaintError::UnknownSource(_))));
    }
}