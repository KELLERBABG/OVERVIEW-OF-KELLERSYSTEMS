use crate::control::ControlDecision;
use crate::sensor::SensorReading;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

/// A single log record combining sensor data + control decision.
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub internal_temp_c: f64,
    pub ambient_temp_c: f64,
    pub compressor_on: bool,
    pub power_consumption_w: f64,
    pub grid_frequency_hz: f64,
    pub decision_compressor_on: bool,
    pub decision_setpoint_c: f64,
    pub active_controller: String,
    pub ttf_sec: f64,
    pub thermal_integral: f64,
    pub virtual_price_eur_per_mwh: f64,
}

/// CSV logger that writes records to a file, rotating daily so a 24/7 device
/// never grows a single unbounded file (SD-card wear / disk exhaustion).
pub struct CsvLogger {
    file: Mutex<File>,
    interval_sec: f64,
    last_log: Mutex<f64>,
    path: String,
    current_day: Mutex<chrono::NaiveDate>,
}

/// Write the CSV header row to a file.
fn write_header(mut file: &File) -> std::io::Result<()> {
    use std::io::Write;
    writeln!(
        file,
        "timestamp,internal_temp_c,ambient_temp_c,compressor_on,power_w,grid_hz,\
         decision_compressor,decision_setpoint,active_controller,ttf_sec,\
         thermal_integral,price_eur_per_mwh"
    )
}

impl CsvLogger {
    /// Open (or create) a CSV log file and write headers.
    pub fn new(path: &str, interval_sec: f64) -> Result<Self, std::io::Error> {
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        // Header only when the file is brand-new (append mode survives restarts).
        if file.metadata().map(|m| m.len() == 0).unwrap_or(false) {
            write_header(&file)?;
        }

        Ok(Self {
            file: Mutex::new(file),
            interval_sec,
            last_log: Mutex::new(0.0),
            path: path.to_string(),
            current_day: Mutex::new(chrono::Utc::now().date_naive()),
        })
    }

    /// Roll over to a dated file (e.g. `thermal_arbitrage_log_20260806.csv`)
    /// when the UTC day changes. Best-effort: on failure keep writing to the
    /// current file rather than dropping data.
    fn maybe_rotate(&self, day: chrono::NaiveDate) {
        let mut cur = self.current_day.lock().unwrap_or_else(|e| e.into_inner());
        if *cur == day {
            return;
        }
        let stem = self.path.trim_end_matches(".csv");
        let rotated = format!("{}_{}.csv", stem, day.format("%Y%m%d"));
        match std::fs::OpenOptions::new().create(true).append(true).open(&rotated) {
            Ok(f) => {
                if write_header(&f).is_ok() {
                    *self.file.lock().unwrap_or_else(|e| e.into_inner()) = f;
                    *cur = day;
                    log::info!("📝 CSV rotated → {}", rotated);
                }
            }
            Err(e) => log::warn!("⚠️  CSV rotation to {} failed: {} — continuing on current file", rotated, e),
        }
    }

    /// Check if enough time has passed and log if so.
    pub fn try_log(&self, reading: &SensorReading, decision: &ControlDecision) {
        let now = now_secs();
        let mut last = self.last_log.lock().unwrap();
        if now - *last < self.interval_sec {
            return;
        }
        *last = now;

        // Daily rotation (before writing, so each day starts a fresh file).
        self.maybe_rotate(reading.timestamp.date_naive());

        let record = LogRecord {
            timestamp: reading.timestamp,
            internal_temp_c: reading.internal_temp_c,
            ambient_temp_c: reading.ambient_temp_c,
            compressor_on: reading.compressor_on,
            power_consumption_w: reading.power_consumption_w,
            grid_frequency_hz: reading.grid_frequency_hz,
            decision_compressor_on: decision.compressor_on,
            decision_setpoint_c: decision.setpoint_c,
            active_controller: format!("{:?}", decision.active_controller),
            ttf_sec: decision.ttf_sec,
            thermal_integral: decision.thermal_integral,
            virtual_price_eur_per_mwh: decision.virtual_price_eur_per_mwh,
        };

        if let Ok(mut file) = self.file.lock() {
            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                record.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                record.internal_temp_c,
                record.ambient_temp_c,
                record.compressor_on as u8,
                record.power_consumption_w,
                record.grid_frequency_hz,
                record.decision_compressor_on as u8,
                record.decision_setpoint_c,
                record.active_controller,
                record.ttf_sec,
                record.thermal_integral,
                record.virtual_price_eur_per_mwh,
            )
            .ok();
        }
    }
}

fn now_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControllerMode;
    use chrono::TimeZone;

    fn reading_at(ts: DateTime<Utc>) -> SensorReading {
        SensorReading {
            timestamp: ts,
            internal_temp_c: -20.0,
            ambient_temp_c: 25.0,
            compressor_on: true,
            power_consumption_w: 50_000.0,
            grid_frequency_hz: 50.0,
            estimated_c_th: 1.0,
        }
    }

    fn decision() -> ControlDecision {
        ControlDecision {
            compressor_on: true,
            setpoint_c: -22.0,
            active_controller: ControllerMode::Idle,
            ttf_sec: 3600.0,
            thermal_integral: 0.0,
            grid_deviation_hz: 0.0,
            virtual_price_eur_per_mwh: 50.0,
            mpc_optimal_window_h: 0.0,
            mpc_optimal_duration_h: 0.0,
            mpc_predicted_cost_eur: 0.0,
            mpc_confidence: 0.0,
            compressor_speed: None,
        }
    }

    #[test]
    fn csv_rotates_to_dated_file_on_utc_day_change() {
        let base = std::env::temp_dir().join(format!("ata_rotate_test_{}.csv", std::process::id()));
        let stem = base.to_str().unwrap().trim_end_matches(".csv");
        let day2 = chrono::NaiveDate::from_ymd_opt(2026, 8, 7).unwrap();
        let rotated = format!("{}_{}.csv", stem, day2.format("%Y%m%d"));
        let _ = std::fs::remove_file(&base);
        let _ = std::fs::remove_file(&rotated);

        let day1 = chrono::NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        let logger = CsvLogger::new(base.to_str().unwrap(), 0.0).unwrap();
        // Pin the logger to a fixed day so the test is deterministic.
        *logger.current_day.lock().unwrap() = day1;

        // Same UTC day → record stays in the base file, no dated file.
        let ts1 = Utc.with_ymd_and_hms(2026, 8, 6, 12, 0, 0).unwrap();
        logger.try_log(&reading_at(ts1), &decision());
        assert!(base.exists());
        assert!(!std::path::Path::new(&rotated).exists(), "no rotation within the same day");
        let base_len_before = std::fs::metadata(&base).unwrap().len();

        // New UTC day → dated file is created and becomes the active handle,
        // so the next record lands there and the base file stops growing.
        let ts2 = Utc.with_ymd_and_hms(2026, 8, 7, 0, 0, 1).unwrap();
        logger.try_log(&reading_at(ts2), &decision());
        assert!(std::path::Path::new(&rotated).exists(), "dated file must be created on day change");
        assert_eq!(std::fs::metadata(&base).unwrap().len(), base_len_before, "base file must not grow after rotation");
        let content = std::fs::read_to_string(&rotated).unwrap();
        assert!(content.starts_with("timestamp,"), "rotated file must carry the CSV header");
        assert!(content.contains("2026-08-07"), "rotated file must contain the new day's record");

        let _ = std::fs::remove_file(&base);
        let _ = std::fs::remove_file(&rotated);
    }
}