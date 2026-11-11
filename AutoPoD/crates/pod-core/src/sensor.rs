use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SensorId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorType {
    Accelerometer,
    Gyroscope,
    Speedometer,
    Gps,
    Temperature,
    Strain,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading<T = f64> {
    pub sensor_id: SensorId,
    pub sensor_type: SensorType,
    pub timestamp: i64,
    pub value: T,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedReading {
    pub reading: SensorReading,
    pub signature: Vec<u8>,
    pub signer_public_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    pub sensor_id: SensorId,
    pub sensor_type: SensorType,
    pub sample_rate_hz: f64,
    pub location: Option<SensorLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorLocation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_sensor_reading_roundtrip() {
        let r = SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type: SensorType::Accelerometer,
            timestamp: 1000,
            value: 9.81,
            unit: "m/s²".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: SensorReading = serde_json::from_str(&json).unwrap();
        assert_eq!(r.sensor_id, back.sensor_id);
        assert!((back.value - 9.81).abs() < 1e-6);
    }
}
