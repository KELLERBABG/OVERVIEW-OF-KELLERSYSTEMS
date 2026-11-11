use serde::{Deserialize, Serialize};
use transit_state_machine::params::VehicleParams;
use transit_taint_analysis::sensors::SensorSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorCommandMsg {
    pub steering: f64,
    pub throttle: f64,
    pub brake: f64,
    pub source_id: String,
    /// Sensor sources that produced this command, used for taint/trust gating.
    /// Empty means the command carries no sensor provenance (treated as trusted).
    pub sources: Vec<SensorSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyVerdictMsg {
    pub allowed: bool,
    pub reason: Option<String>,
    pub verification_id: u64,
    pub latency_us: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleStateMsg {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub velocity: f64,
    pub yaw_rate: f64,
    pub steering_angle: f64,
    pub params: VehicleParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleMsg {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub half_length: f64,
    pub half_width: f64,
}
