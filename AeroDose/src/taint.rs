//! Taint-coded sensor stream audit module.
//!
//! Provides continuous data-integrity validation of the raw PM2.5 sensor
//! stream to detect clogged intake, stuck optical window, or spoofed telemetry.

/// Classification of a potential data-integrity fault in the sensor stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintFlag {
    /// No anomaly detected.
    Clean,
    /// Intake path is likely clogged (long run of identical readings).
    CloggedIntake,
    /// Optical window is stuck / not responding (near-zero variance in a full window).
    StuckOpticalWindow,
    /// Telemetry appears spoofed (unphysically large single-step jump).
    SpoofedTelemetry,
}

/// Rolling-window detector for sensor-stream taint conditions.
#[derive(Debug, Clone)]
pub struct TaintDetector {
    window_size: usize,
    /// Rolling PM2.5 values (µg/m³), oldest first.
    recent_readings: Vec<f64>,
    /// Below this population stddev (with a full window) → StuckOpticalWindow.
    min_variance_threshold: f64,
    /// Above this single-step percentage change → SpoofedTelemetry.
    max_jump_threshold: f64,
}

/// Result of evaluating the current window for taint conditions.
#[derive(Debug, Clone)]
pub struct TaintReport {
    pub flag: TaintFlag,
    /// Population standard deviation of the current window.
    pub window_stddev: f64,
    /// Longest run of exactly-equal consecutive samples in the window.
    pub max_consecutive_equal: usize,
    /// Maximum absolute percentage change between consecutive samples.
    pub max_single_step_jump_pct: f64,
}

impl TaintDetector {
    /// Create a new detector with the given rolling window size (must be >= 4).
    ///
    /// Defaults:
    /// - `min_variance_threshold` = 0.5 µg/m³
    /// - `max_jump_threshold` = 200.0 %
    pub fn new(window_size: usize) -> Self {
        assert!(window_size >= 4, "TaintDetector window_size must be >= 4");
        Self {
            window_size,
            recent_readings: Vec::with_capacity(window_size),
            min_variance_threshold: 0.5,
            max_jump_threshold: 200.0,
        }
    }

    /// Append a new PM2.5 sample (µg/m³) to the rolling window.
    /// Evicts the oldest sample if the window exceeds `window_size`.
    pub fn push_sample(&mut self, pm25_ug_m3: f64) {
        self.recent_readings.push(pm25_ug_m3);
        if self.recent_readings.len() > self.window_size {
            self.recent_readings.remove(0);
        }
    }

    /// Evaluate the current window and return a [`TaintReport`].
    ///
    /// Priority order (highest first):
    /// 1. `SpoofedTelemetry` — max single-step jump > `max_jump_threshold` %
    /// 2. `CloggedIntake` — longest run of identical values >= `window_size / 2`
    /// 3. `StuckOpticalWindow` — window full AND population stddev < `min_variance_threshold`
    /// 4. `Clean`
    pub fn evaluate(&self) -> TaintReport {
        let readings = &self.recent_readings;

        // --- Population standard deviation ---
        let window_stddev = if readings.is_empty() {
            0.0
        } else {
            let n = readings.len() as f64;
            let mean = readings.iter().sum::<f64>() / n;
            let variance = readings.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            variance.sqrt()
        };

        // --- Longest run of exactly-equal consecutive values ---
        let max_consecutive_equal = if readings.is_empty() {
            0
        } else {
            let mut max_run = 1usize;
            let mut current_run = 1usize;
            for i in 1..readings.len() {
                if readings[i] == readings[i - 1] {
                    current_run = current_run.saturating_add(1);
                } else {
                    current_run = 1;
                }
                max_run = max_run.max(current_run);
            }
            max_run
        };

        // --- Maximum absolute single-step percentage change ---
        let max_single_step_jump_pct = if readings.len() < 2 {
            0.0
        } else {
            let mut max_jump = 0.0f64;
            for i in 1..readings.len() {
                let prev = readings[i - 1];
                let curr = readings[i];
                if prev.abs() > f64::EPSILON {
                    let pct = ((curr - prev) / prev).abs() * 100.0;
                    max_jump = max_jump.max(pct);
                } else if curr.abs() > f64::EPSILON {
                    // Previous is ~0 but current is non-zero: treat as infinite jump
                    // capped at a large sentinel so it always exceeds threshold.
                    max_jump = max_jump.max(f64::INFINITY);
                }
            }
            max_jump
        };

        // --- Determine flag by priority ---
        let window_full = readings.len() >= self.window_size;
        let half_window = self.window_size / 2;

        let flag = if max_single_step_jump_pct > self.max_jump_threshold {
            TaintFlag::SpoofedTelemetry
        } else if max_consecutive_equal >= half_window && max_consecutive_equal > 0 {
            TaintFlag::CloggedIntake
        } else if window_full && window_stddev < self.min_variance_threshold {
            TaintFlag::StuckOpticalWindow
        } else {
            TaintFlag::Clean
        };

        TaintReport {
            flag,
            window_stddev,
            max_consecutive_equal,
            max_single_step_jump_pct,
        }
    }

    /// Clear the rolling window (reset all state).
    pub fn reset(&mut self) {
        self.recent_readings.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_values_trigger_stuck_or_clogged() {
        let mut det = TaintDetector::new(10);
        for _ in 0..10 {
            det.push_sample(42.0);
        }
        let report = det.evaluate();

        // 10 identical values: stddev = 0 < 0.5, max_consecutive_equal = 10 >= 5
        // Both CloggedIntake and StuckOpticalWindow conditions are met.
        // Priority: CloggedIntake > StuckOpticalWindow, so expect CloggedIntake.
        assert!(
            report.flag == TaintFlag::CloggedIntake || report.flag == TaintFlag::StuckOpticalWindow,
            "Expected CloggedIntake or StuckOpticalWindow, got {:?}",
            report.flag
        );
        // With all identical values, stddev must be 0.
        assert!(report.window_stddev < f64::EPSILON);
        assert_eq!(report.max_consecutive_equal, 10);
    }

    #[test]
    fn test_large_jump_triggers_spoofed() {
        let mut det = TaintDetector::new(10);
        // Feed a mostly-stable sequence with one 500% jump.
        // 10.0 → 60.0 is a 500% increase.
        let samples = [10.0, 10.5, 10.2, 60.0, 10.1, 10.3, 10.0, 10.4, 10.1, 10.2];
        for &s in &samples {
            det.push_sample(s);
        }
        let report = det.evaluate();

        assert_eq!(
            report.flag,
            TaintFlag::SpoofedTelemetry,
            "Expected SpoofedTelemetry due to 500% jump, got {:?} (jump_pct={})",
            report.flag,
            report.max_single_step_jump_pct
        );
        assert!(report.max_single_step_jump_pct > 200.0);
    }

    #[test]
    fn test_noisy_realistic_sequence_is_clean() {
        let mut det = TaintDetector::new(10);
        // Realistic noisy PM2.5 readings with moderate variance and no extreme jumps.
        let samples = [
            12.3, 14.1, 13.8, 15.2, 14.7,
            13.5, 14.9, 15.8, 14.2, 13.6,
        ];
        for &s in &samples {
            det.push_sample(s);
        }
        let report = det.evaluate();

        assert_eq!(
            report.flag,
            TaintFlag::Clean,
            "Expected Clean for noisy realistic data, got {:?} (stddev={:.4}, max_run={}, jump_pct={:.2})",
            report.flag,
            report.window_stddev,
            report.max_consecutive_equal,
            report.max_single_step_jump_pct
        );
        // Stddev should be well above 0.5 for this spread.
        assert!(report.window_stddev > 0.5);
        // No long runs of identical values.
        assert!(report.max_consecutive_equal < 5);
        // Max jump should be well under 200%.
        assert!(report.max_single_step_jump_pct < 200.0);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut det = TaintDetector::new(10);
        for _ in 0..10 {
            det.push_sample(99.0);
        }
        // Before reset: should be flagged.
        let before = det.evaluate();
        assert_ne!(before.flag, TaintFlag::Clean);

        det.reset();

        let after = det.evaluate();
        assert_eq!(after.flag, TaintFlag::Clean);
        assert_eq!(after.window_stddev, 0.0);
        assert_eq!(after.max_consecutive_equal, 0);
        assert_eq!(after.max_single_step_jump_pct, 0.0);
    }

    #[test]
    fn test_window_eviction() {
        let mut det = TaintDetector::new(4);
        // Push more than window_size samples; oldest should be evicted.
        for i in 0..6 {
            det.push_sample(10.0 + i as f64);
        }
        // Window should contain [12.0, 13.0, 14.0, 15.0]
        let report = det.evaluate();
        assert_eq!(report.max_consecutive_equal, 1);
        // All distinct, no jumps > 200%, stddev > 0.5 → Clean
        assert_eq!(report.flag, TaintFlag::Clean);
    }

    #[test]
    fn test_partial_window_does_not_trigger_stuck() {
        let mut det = TaintDetector::new(10);
        // Only 3 identical samples — window not full.
        for _ in 0..3 {
            det.push_sample(50.0);
        }
        let report = det.evaluate();
        // Window not full → StuckOpticalWindow should NOT trigger.
        // max_consecutive_equal = 3 < 10/2 = 5 → CloggedIntake should NOT trigger.
        assert_eq!(report.flag, TaintFlag::Clean);
    }
}
