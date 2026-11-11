pub mod explorer;
pub mod report;
pub mod spaces;
pub mod trace;

pub use explorer::IncidentExplorer;
pub use report::{generate_report, generate_report_from_data, DamageReport};
pub use spaces::{DamageState, ExplorationPath, StateSpaceExplorer, StateSpaceReport, StateTransition};
pub use trace::{build_trace, ConstraintEvent, ConstraintTrace};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IncidentClass {
    /// Sudden, short-duration impact (e.g., car crash)
    AcuteImpact,
    /// Prolonged event spanning many readings
    SustainedEvent,
    /// Slow drift over time (e.g., material fatigue)
    GradualDegradation,
    /// Single extreme reading, likely spoofed or sensor fault
    Anomaly,
    /// No threshold conditions triggered
    NoEvent,
    /// Cannot determine from available data
    Undetermined,
}

impl IncidentClass {
    pub fn label(&self) -> &'static str {
        match self {
            IncidentClass::AcuteImpact => "Acute Impact",
            IncidentClass::SustainedEvent => "Sustained Event",
            IncidentClass::GradualDegradation => "Gradual Degradation",
            IncidentClass::Anomaly => "Anomaly / Possible Fraud",
            IncidentClass::NoEvent => "No Event",
            IncidentClass::Undetermined => "Undetermined",
        }
    }
}
