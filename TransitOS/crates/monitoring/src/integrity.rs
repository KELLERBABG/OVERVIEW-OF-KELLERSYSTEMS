use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityStatus {
    Healthy,
    Degraded,
    Compromised,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorIntegrityReport {
    pub radar: IntegrityStatus,
    pub lidar: IntegrityStatus,
    pub camera: IntegrityStatus,
    pub gps: IntegrityStatus,
    pub imu: IntegrityStatus,
    pub overall: IntegrityStatus,
    pub tainted_commands_detected: u64,
}

impl SensorIntegrityReport {
    pub fn new() -> Self {
        Self {
            radar: IntegrityStatus::Healthy,
            lidar: IntegrityStatus::Healthy,
            camera: IntegrityStatus::Healthy,
            gps: IntegrityStatus::Healthy,
            imu: IntegrityStatus::Healthy,
            overall: IntegrityStatus::Healthy,
            tainted_commands_detected: 0,
        }
    }

    pub fn mark_taint_detected(&mut self) {
        self.tainted_commands_detected += 1;
        self.overall = IntegrityStatus::Degraded;
    }

    pub fn mark_sensor_compromised(&mut self, sensor: &str) {
        match sensor {
            "radar" => self.radar = IntegrityStatus::Compromised,
            "lidar" => self.lidar = IntegrityStatus::Compromised,
            "camera" => self.camera = IntegrityStatus::Compromised,
            "gps" => self.gps = IntegrityStatus::Compromised,
            "imu" => self.imu = IntegrityStatus::Compromised,
            _ => {}
        }
        self.recompute_overall();
    }

    fn recompute_overall(&mut self) {
        let all = [self.radar, self.lidar, self.camera, self.gps, self.imu];
        if all.contains(&IntegrityStatus::Compromised) {
            self.overall = IntegrityStatus::Compromised;
        } else if all.contains(&IntegrityStatus::Degraded) {
            self.overall = IntegrityStatus::Degraded;
        } else {
            self.overall = IntegrityStatus::Healthy;
        }
    }
}

impl Default for SensorIntegrityReport {
    fn default() -> Self {
        Self::new()
    }
}
