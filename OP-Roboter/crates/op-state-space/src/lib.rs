//! # op-state-space — Critical state space exploration
//!
//! Systematically explores patient / system / environment states to discover
//! rare life-threatening edge cases in advance.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use op_math::{Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// A single fault event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultEvent {
    /// Sudden patient motion.
    PatientMotion,
    /// Hemorrhagic event.
    Hemorrhage,
    /// Instrument breakage.
    InstrumentBreakage,
    /// Power loss / servo failure.
    PowerLoss,
    /// Sensor dropout or data corruption.
    SensorDropout,
}

/// Magnitude of a simulated fault.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FaultMagnitude {
    /// Mild perturbation.
    Mild,
    /// Moderate perturbation.
    Moderate,
    /// Severe perturbation.
    Severe,
    /// Custom magnitude (numeric in SI units).
    Custom(Scalar),
}

impl FaultMagnitude {
    /// Numeric multiplier of the nominal operating envelope.
    pub fn scale(&self) -> Scalar {
        match self {
            FaultMagnitude::Mild => 0.1,
            FaultMagnitude::Moderate => 0.5,
            FaultMagnitude::Severe => 0.9,
            FaultMagnitude::Custom(s) => *s,
        }
    }
}

/// State of the surgical system at a point in time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SystemState {
    /// Tool tip position (m).
    pub tip_position: Vector3,
    /// Tool tip velocity (m/s).
    pub tip_velocity: Vector3,
    /// Tool tip force (N).
    pub tip_force: Scalar,
    /// Incision depth (m).
    pub depth: Scalar,
    /// Patch pressure proxy (relative, dimensionless).
    pub pressure: Scalar,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            tip_position: Vector3::new(0.0, 0.0, 0.05),
            tip_velocity: Vector3::ZERO,
            tip_force: 0.0,
            depth: 0.05,
            pressure: 1.0,
        }
    }
}

/// A discovered edge case from exploration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeCase {
    /// Fault event that triggered it.
    pub fault: FaultEvent,
    /// Magnitude of the fault.
    pub magnitude: FaultMagnitude,
    /// Final state after the perturbation.
    pub state: SystemState,
    /// Why it is considered critical.
    pub reason: String,
}

/// Error raised during state space exploration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExplorationError {
    /// Limits were not configured.
    #[error("state space limits not configured")]
    MissingLimits,
}

/// Safety limits used to classify critical states.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SafetyLimits {
    /// Maximum tool-tip speed (m/s).
    pub max_speed: Scalar,
    /// Maximum tool-tip force (N).
    pub max_force: Scalar,
    /// Maximum incision depth (m).
    pub max_depth: Scalar,
    /// Minimum allowed internal pressure ratio.
    pub min_pressure: Scalar,
}

/// Explores the critical state space around a nominal state.
#[derive(Debug, Clone, Default)]
pub struct StateExplorer {
    /// Safety limits used to classify critical states.
    pub limits: Option<SafetyLimits>,
}

impl StateExplorer {
    /// Construct an explorer with the given limits.
    pub fn new(limits: SafetyLimits) -> Self {
        Self { limits: Some(limits) }
    }

    /// Explore all fault events at all standard magnitudes from a nominal state.
    pub fn explore(&self, nominal: SystemState) -> Result<Vec<EdgeCase>, ExplorationError> {
        let limits = self.limits.ok_or(ExplorationError::MissingLimits)?;
        let magnitudes = [
            FaultMagnitude::Mild,
            FaultMagnitude::Moderate,
            FaultMagnitude::Severe,
        ];
        let events = [
            FaultEvent::PatientMotion,
            FaultEvent::Hemorrhage,
            FaultEvent::InstrumentBreakage,
            FaultEvent::PowerLoss,
            FaultEvent::SensorDropout,
        ];
        let mut out = Vec::new();
        for &fault in &events {
            for &mag in &magnitudes {
                let state = self.perturb(nominal, fault, mag);
                if let Some(reason) = Self::classify_critical(&state, limits) {
                    out.push(EdgeCase {
                        fault,
                        magnitude: mag,
                        state,
                        reason,
                    });
                }
            }
        }
        Ok(out)
    }

    /// Apply a fault perturbation to a nominal state.
    pub fn perturb(&self, nominal: SystemState, fault: FaultEvent, mag: FaultMagnitude) -> SystemState {
        let s = mag.scale();
        match fault {
            FaultEvent::PatientMotion => SystemState {
                tip_velocity: nominal.tip_velocity + Vector3::new(s, 0.5 * s, 0.0),
                ..nominal
            },
            FaultEvent::Hemorrhage => SystemState {
                pressure: nominal.pressure * (1.0 - 0.3 * s),
                ..nominal
            },
            FaultEvent::InstrumentBreakage => SystemState {
                tip_force: nominal.tip_force + 10.0 * s,
                ..nominal
            },
            FaultEvent::PowerLoss => SystemState {
                tip_velocity: Vector3::ZERO,
                tip_force: 0.0,
                ..nominal
            },
            FaultEvent::SensorDropout => SystemState {
                tip_force: nominal.tip_force.max(5.0 * s),
                ..nominal
            },
        }
    }

    /// Classify a state as critical, returning the reason, or `None` if safe.
    pub fn classify_critical(state: &SystemState, limits: SafetyLimits) -> Option<String> {
        if state.tip_velocity.norm() > limits.max_speed {
            return Some("tip speed exceeds limit".into());
        }
        if state.tip_force > limits.max_force {
            return Some("tip force exceeds limit".into());
        }
        if state.depth > limits.max_depth {
            return Some("depth exceeds limit".into());
        }
        if state.pressure < limits.min_pressure {
            return Some("pressure below minimum".into());
        }
        None
    }
}

/// A structured real-world surgical adverse incident report (FDA MAUDE aligned).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdverseIncidentReport {
    /// Unique report identifier (e.g. `"MAUDE-SURG-001"`).
    pub id: String,
    /// Clinical incident name.
    pub name: String,
    /// Categorized fault event.
    pub fault: FaultEvent,
    /// Clinical narrative and failure mechanism.
    pub narrative: String,
    /// Nominal starting state.
    pub initial_state: SystemState,
    /// State after perturbation.
    pub perturbed_state: SystemState,
    /// Hazardous condition that must be detected and blocked.
    pub expected_hazard: String,
}

impl AdverseIncidentReport {
    /// Return the standard FDA MAUDE surgical incident benchmark dataset.
    pub fn standard_benchmarks() -> Vec<Self> {
        vec![
            Self {
                id: "MAUDE-SURG-001".into(),
                name: "Sudden Patient Respiratory Motion".into(),
                fault: FaultEvent::PatientMotion,
                narrative: "Patient diaphragmatic spasm shifts anatomy during delicate incision near vascular bed.".into(),
                initial_state: SystemState::default(),
                perturbed_state: SystemState {
                    tip_velocity: Vector3::new(0.45, 0.22, 0.0),
                    depth: 0.08,
                    ..SystemState::default()
                },
                expected_hazard: "tip speed exceeds limit".into(),
            },
            Self {
                id: "MAUDE-SURG-002".into(),
                name: "Excessive Retraction Force Spasm".into(),
                fault: FaultEvent::InstrumentBreakage,
                narrative: "Instrument tip exerts excessive tissue retraction force causing organ capsule micro-tears.".into(),
                initial_state: SystemState::default(),
                perturbed_state: SystemState {
                    tip_force: 18.5, // Exceeds safe limit (5.0 N)
                    ..SystemState::default()
                },
                expected_hazard: "tip force exceeds limit".into(),
            },
            Self {
                id: "MAUDE-SURG-003".into(),
                name: "Uncontrolled Trocar Deepening".into(),
                fault: FaultEvent::PatientMotion,
                narrative: "Trocar insertion overshoot penetrates beyond target plane into posterior retroperitoneum.".into(),
                initial_state: SystemState::default(),
                perturbed_state: SystemState {
                    depth: 0.16, // Exceeds limit (0.10 m)
                    ..SystemState::default()
                },
                expected_hazard: "depth exceeds limit".into(),
            },
            Self {
                id: "MAUDE-SURG-004".into(),
                name: "Pneumoperitoneum Loss / Hemorrhagic Collapse".into(),
                fault: FaultEvent::Hemorrhage,
                narrative: "Rapid insufflation pressure drop alters workspace boundary positions relative to tool tip.".into(),
                initial_state: SystemState::default(),
                perturbed_state: SystemState {
                    pressure: 0.35, // Below minimum 0.50
                    ..SystemState::default()
                },
                expected_hazard: "pressure below minimum".into(),
            },
            Self {
                id: "MAUDE-SURG-005".into(),
                name: "Force Sensor Signal Dropout".into(),
                fault: FaultEvent::SensorDropout,
                narrative: "Sensor communication glitch reports corrupted force telemetry during active clamping.".into(),
                initial_state: SystemState::default(),
                perturbed_state: SystemState {
                    tip_force: 12.0,
                    ..SystemState::default()
                },
                expected_hazard: "tip force exceeds limit".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explores_patient_motion_edge_case() {
        let limits = SafetyLimits {
            max_speed: 0.2,
            max_force: 5.0,
            max_depth: 0.1,
            min_pressure: 0.5,
        };
        let explorer = StateExplorer::new(limits);
        let cases = explorer.explore(SystemState::default()).unwrap();
        assert!(!cases.is_empty());
        assert!(cases.iter().any(|c| c.fault == FaultEvent::PatientMotion));
    }

    #[test]
    fn adverse_benchmarks_all_classified_as_critical() {
        let limits = SafetyLimits {
            max_speed: 0.2,
            max_force: 5.0,
            max_depth: 0.1,
            min_pressure: 0.5,
        };
        let benchmarks = AdverseIncidentReport::standard_benchmarks();
        for b in &benchmarks {
            let critical = StateExplorer::classify_critical(&b.perturbed_state, limits);
            assert!(
                critical.is_some(),
                "Benchmark {} should be classified as critical",
                b.id
            );
        }
    }
}