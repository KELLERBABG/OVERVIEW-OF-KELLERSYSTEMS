use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sensor::SignedReading;

pub type EventId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_id: EventId,
    pub incident_window: IncidentWindow,
    pub readings: Vec<SignedReading>,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentWindow {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: String,
    pub vehicle_id: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Verdict {
    Approved,
    Rejected,
    PendingReview,
}
