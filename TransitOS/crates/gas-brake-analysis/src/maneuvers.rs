pub enum ManeuverType {
    EmergencyBrake,
    EvasiveSwerve,
    LaneChange,
    IntersectionStop,
    CurveNegotiation,
    AccelerationLaneMerge,
}

pub struct ManeuverEnvelope {
    pub max_long_accel: f64,
    pub max_long_decel: f64,
    pub max_lat_accel: f64,
    pub max_steering_rate: f64,
    pub max_jerk: f64,
}

impl ManeuverEnvelope {
    pub fn from_friction(mu: f64) -> Self {
        let g = 9.81;
        Self {
            max_long_accel: mu * g * 0.4,
            max_long_decel: mu * g,
            max_lat_accel: mu * g * 0.85,
            max_steering_rate: 0.5,
            max_jerk: 5.0,
        }
    }
}
