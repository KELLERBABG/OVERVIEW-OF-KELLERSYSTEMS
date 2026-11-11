use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorType {
    Radar,
    Lidar,
    Camera,
    Ultrasonic,
    Gps,
    Imu,
    WheelSpeed,
    BrakePressureSensor,
    SteeringAngleSensor,
    V2X,
}

impl SensorType {
    pub fn trust_level(&self) -> f64 {
        match self {
            SensorType::Imu => 0.95,
            SensorType::WheelSpeed => 0.98,
            SensorType::BrakePressureSensor => 0.99,
            SensorType::SteeringAngleSensor => 0.99,
            SensorType::Radar => 0.85,
            SensorType::Lidar => 0.90,
            SensorType::Camera => 0.75,
            SensorType::Ultrasonic => 0.80,
            SensorType::Gps => 0.70,
            SensorType::V2X => 0.50,
        }
    }

    pub fn spoofable(&self) -> bool {
        matches!(self, SensorType::Gps | SensorType::V2X | SensorType::Camera)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorSource {
    pub sensor: SensorType,
    pub id: String,
}
