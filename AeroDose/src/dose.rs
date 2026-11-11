//! Occupational PM2.5 shift dose and TWA (Time-Weighted Average) models.

use serde::{Deserialize, Serialize};

/// Regulatory alert level according to OSHA 2026 and EU Directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertTier {
    /// Safe / Baseline ambient conditions.
    Normal,
    /// Advisory: PM2.5 > 20.4 ug/m3 (AQI > 68) — N95 recommended.
    Caution,
    /// Mandatory N95 / Respirator required: PM2.5 > 35.5 ug/m3 (AQI > 101).
    RespiratorRequired,
    /// Hazard: PM2.5 > 250.4 ug/m3 (AQI > 301) — mandatory filtered shelter / stop-work.
    StopWork,
}

/// Regulatory exposure standard thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureLimits {
    pub caution_pm25_ug_m3: f64,
    pub respirator_pm25_ug_m3: f64,
    pub stop_work_pm25_ug_m3: f64,
    pub shift_max_cumulative_dose_ug_h_m3: f64,
}

impl Default for ExposureLimits {
    fn default() -> Self {
        Self {
            caution_pm25_ug_m3: 20.4,
            respirator_pm25_ug_m3: 35.5,
            stop_work_pm25_ug_m3: 250.4,
            shift_max_cumulative_dose_ug_h_m3: 284.0, // 8h * 35.5 ug/m3
        }
    }
}

/// Shift dose accumulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftDoseTracker {
    pub limits: ExposureLimits,
    /// Total duration tracked in seconds.
    pub total_seconds: f64,
    /// Cumulative exposure dose in (ug * hour / m3).
    pub cumulative_dose_ug_h_m3: f64,
    /// Peak instantaneous PM2.5 reading in ug/m3.
    pub peak_pm25_ug_m3: f64,
    /// Current alert level.
    pub current_tier: AlertTier,
    /// Reading count.
    pub sample_count: usize,
}

impl ShiftDoseTracker {
    pub fn new(limits: ExposureLimits) -> Self {
        Self {
            limits,
            total_seconds: 0.0,
            cumulative_dose_ug_h_m3: 0.0,
            peak_pm25_ug_m3: 0.0,
            current_tier: AlertTier::Normal,
            sample_count: 0,
        }
    }

    /// Add a calibrated PM2.5 reading (ug/m3) over interval dt_seconds.
    pub fn record_sample(&mut self, pm25_ug_m3: f64, dt_seconds: f64) -> AlertTier {
        self.sample_count = self.sample_count.saturating_add(1);
        self.total_seconds += dt_seconds;
        if pm25_ug_m3 > self.peak_pm25_ug_m3 {
            self.peak_pm25_ug_m3 = pm25_ug_m3;
        }

        // Integrate dose: dose = PM2.5 (ug/m3) * dt (hours)
        let dt_hours = dt_seconds / 3600.0;
        self.cumulative_dose_ug_h_m3 += pm25_ug_m3 * dt_hours;

        // Evaluate alert tier
        self.current_tier = if pm25_ug_m3 >= self.limits.stop_work_pm25_ug_m3 
            || self.cumulative_dose_ug_h_m3 >= self.limits.shift_max_cumulative_dose_ug_h_m3 * 1.5 
        {
            AlertTier::StopWork
        } else if pm25_ug_m3 >= self.limits.respirator_pm25_ug_m3 
            || self.cumulative_dose_ug_h_m3 >= self.limits.shift_max_cumulative_dose_ug_h_m3 
        {
            AlertTier::RespiratorRequired
        } else if pm25_ug_m3 >= self.limits.caution_pm25_ug_m3 {
            AlertTier::Caution
        } else {
            AlertTier::Normal
        };

        self.current_tier
    }

    /// Calculate Time-Weighted Average (TWA) concentration in ug/m3.
    pub fn twa_pm25_ug_m3(&self) -> f64 {
        if self.total_seconds <= 0.0 {
            0.0
        } else {
            let total_hours = self.total_seconds / 3600.0;
            self.cumulative_dose_ug_h_m3 / total_hours
        }
    }
}
