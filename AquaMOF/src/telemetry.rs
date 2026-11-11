//! LoRa/BLE Sensor telemetry payload generator.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvesterTelemetry {
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub ambient_temp_c: f64,
    pub ambient_rh_pct: f64,
    pub bed_temp_c: f64,
    pub daily_water_yield_liters: f64,
    pub battery_voltage_v: f64,
    pub potability_status: String,
}

impl HarvesterTelemetry {
    pub fn sample(bed_temp_c: f64, ambient_temp_c: f64, rh_pct: f64, yield_liters: f64) -> Self {
        Self {
            device_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            ambient_temp_c,
            ambient_rh_pct: rh_pct,
            bed_temp_c,
            daily_water_yield_liters: yield_liters,
            battery_voltage_v: 3.65,
            potability_status: "EU_2020_2184_Compliant".to_string(),
        }
    }
}
