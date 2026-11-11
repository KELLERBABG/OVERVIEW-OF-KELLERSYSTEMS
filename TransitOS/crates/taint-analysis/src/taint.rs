use crate::sensors::{SensorSource, SensorType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaintStatus {
    Clean,
    Tainted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintedValue<T> {
    pub value: T,
    pub taint: TaintStatus,
    pub sources: Vec<SensorSource>,
}

impl<T> TaintedValue<T> {
    pub fn clean(value: T) -> Self {
        Self {
            value,
            taint: TaintStatus::Clean,
            sources: Vec::new(),
        }
    }

    pub fn tainted(value: T, sensor: SensorType, id: &str) -> Self {
        Self {
            value,
            taint: TaintStatus::Tainted,
            sources: vec![SensorSource {
                sensor,
                id: id.to_string(),
            }],
        }
    }

    pub fn from_source(value: T, source: SensorSource) -> Self {
        let taint = if source.sensor.spoofable() {
            TaintStatus::Tainted
        } else {
            TaintStatus::Clean
        };
        Self {
            value,
            taint,
            sources: vec![source],
        }
    }

    /// A value is trusted only when it is not tainted *and* every contributing
    /// sensor source meets the high-trust bar. A `Clean` value with a low-trust
    /// source (e.g. radar at 0.85) is therefore not trusted.
    pub fn is_trusted(&self) -> bool {
        self.taint == TaintStatus::Clean
            && self.sources.iter().all(|s| s.sensor.trust_level() > 0.9)
    }
}
