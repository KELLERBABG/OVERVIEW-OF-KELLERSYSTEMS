//! # Material Integrity Auditing
//!
//! Self-diagnostic structural integrity monitoring via vibration pattern
//! analysis. Detects structural anomalies (cracks, delamination, material
//! fatigue) by comparing current vibration signatures against baselines.
//!
//! The core insight: cracks and structural changes alter the vibration
//! transmission characteristics of the material, which changes the
//! piezoelectric signal pattern. By continuously monitoring these patterns,
//! we can detect structural issues early.

use crate::sensor::VibrationAnalyzer;
use crate::types::{
    EnergyLevel, IntegrityReport, NodeId, ProtocolSlice, SliceType, VibrationSample,
};
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of anomaly events before triggering a warning.
pub const ANOMALY_WARNING_THRESHOLD: u32 = 5;

/// Maximum number of anomaly events stored.
pub const MAX_ANOMALY_LOG: usize = 32;

/// Minimum deviation percentage to consider an anomaly.
pub const ANOMALY_DEVIATION_THRESHOLD_PCT: u8 = 20;

/// Default integrity check interval (seconds).
pub const DEFAULT_CHECK_INTERVAL_SEC: u32 = 60;

// ---------------------------------------------------------------------------
// Integrity Auditor
// ---------------------------------------------------------------------------

/// Monitors structural integrity through vibration analysis.
#[derive(Debug, Clone)]
pub struct IntegrityAuditor {
    node_id: NodeId,
    analyzer: VibrationAnalyzer,
    anomaly_count: u32,
    anomaly_log: Vec<AnomalyEvent, MAX_ANOMALY_LOG>,
    check_interval_sec: u32,
    last_check_time: crate::types::TimeMs,
}

/// A recorded anomaly event.
#[derive(Debug, Clone, Copy)]
pub struct AnomalyEvent {
    pub timestamp: crate::types::TimeMs,
    pub deviation_pct: u8,
    pub sample: VibrationSample,
}

impl IntegrityAuditor {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            analyzer: VibrationAnalyzer::new(),
            anomaly_count: 0,
            anomaly_log: Vec::new(),
            check_interval_sec: DEFAULT_CHECK_INTERVAL_SEC,
            last_check_time: crate::types::TimeMs(0),
        }
    }

    /// Process a new vibration sample.
    ///
    /// Returns an alert slice if an anomaly is detected, None otherwise.
    pub fn process_sample(&mut self, sample: &VibrationSample) -> Option<AnomalyAlert> {
        self.analyzer.process_sample(sample);

        // Check if we have enough baseline data
        self.analyzer.baseline()?;

        let deviation = self.analyzer.deviation_pct(sample);

        if deviation >= ANOMALY_DEVIATION_THRESHOLD_PCT {
            self.anomaly_count += 1;

            let event = AnomalyEvent {
                timestamp: sample.time,
                deviation_pct: deviation,
                sample: *sample,
            };

            // Try to log it
            if self.anomaly_log.len() < MAX_ANOMALY_LOG {
                self.anomaly_log.push(event).ok();
            }

            // Trigger alert if threshold exceeded
            if self.anomaly_count >= ANOMALY_WARNING_THRESHOLD {
                let alert = AnomalyAlert {
                    node_id: self.node_id,
                    deviation_pct: deviation,
                    anomaly_count: self.anomaly_count,
                    confidence: self.calculate_confidence(deviation),
                };
                return Some(alert);
            }
        }

        None
    }

    /// Build an integrity report.
    pub fn build_report(&self) -> IntegrityReport {
        let baseline = self.analyzer.baseline();

        let (baseline_amp, current_amp) = if let Some(base) = baseline {
            let base_amp = ((base.x as i32).unsigned_abs()
                + (base.y as i32).unsigned_abs()
                + (base.z as i32).unsigned_abs()) as u16;
            // Use the last anomaly sample if available, otherwise baseline
            if let Some(last_anomaly) = self.anomaly_log.last() {
                let anomaly_amp = (last_anomaly.sample.x.unsigned_abs() as u32
                    + last_anomaly.sample.y.unsigned_abs() as u32
                    + last_anomaly.sample.z.unsigned_abs() as u32)
                    as u16;
                (base_amp, anomaly_amp)
            } else {
                (base_amp, base_amp)
            }
        } else {
            (0, 0)
        };

        IntegrityReport {
            node_id: self.node_id,
            baseline_amplitude_um: baseline_amp,
            current_amplitude_um: current_amp,
            deviation_pct: baseline_amp
                .checked_sub(current_amp)
                .map(|d| (d as u32 * 100 / baseline_amp.max(1) as u32) as u8)
                .unwrap_or(0),
            anomaly_count: self.anomaly_count,
            anomaly_confidence: self
                .anomaly_log
                .last()
                .map(|e| self.calculate_confidence(e.deviation_pct))
                .unwrap_or(0),
        }
    }

    /// Number of anomaly events detected so far.
    pub fn anomaly_count(&self) -> u32 {
        self.anomaly_count
    }

    /// Check if it's time for a periodic integrity check.
    pub fn should_check(&self) -> bool {
        let now = crate::types::TimeMs::now();
        (now.0 - self.last_check_time.0) / 1000 >= self.check_interval_sec as u64
    }

    /// Mark that a check was performed.
    pub fn mark_checked(&mut self) {
        self.last_check_time = crate::types::TimeMs::now();
    }

    /// Build a protocol slice for an integrity report.
    pub fn build_report_slice(&self, energy: EnergyLevel) -> ProtocolSlice {
        let report = self.build_report();
        let mut payload: Vec<u8, 128> = Vec::new();

        // Serialize report into payload
        payload.extend_from_slice(report.node_id.as_bytes()).ok();
        payload
            .extend_from_slice(&report.baseline_amplitude_um.to_le_bytes())
            .ok();
        payload
            .extend_from_slice(&report.current_amplitude_um.to_le_bytes())
            .ok();
        payload.push(report.deviation_pct).ok();
        payload
            .extend_from_slice(&report.anomaly_count.to_le_bytes())
            .ok();
        payload.push(report.anomaly_confidence).ok();

        ProtocolSlice {
            slice_type: SliceType::IntegrityReport,
            src: self.node_id,
            dst: NodeId([0xff; 8]),
            hops: 0,
            seqno: 0,
            payload,
            energy,
        }
    }

    /// Calculate anomaly confidence (0-255) from deviation percentage.
    fn calculate_confidence(&self, deviation_pct: u8) -> u8 {
        if deviation_pct < ANOMALY_DEVIATION_THRESHOLD_PCT {
            0
        } else {
            ((deviation_pct as u16 - ANOMALY_DEVIATION_THRESHOLD_PCT as u16) * 255
                / (100 - ANOMALY_DEVIATION_THRESHOLD_PCT as u16)) as u8
        }
    }
}

// ---------------------------------------------------------------------------
// Anomaly Alert
// ---------------------------------------------------------------------------

/// An anomaly alert generated by the integrity auditor.
#[derive(Debug, Clone, Copy)]
pub struct AnomalyAlert {
    pub node_id: NodeId,
    pub deviation_pct: u8,
    pub anomaly_count: u32,
    pub confidence: u8,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(id: u8) -> NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        NodeId(bytes)
    }

    #[test]
    fn test_initial_state() {
        let n1 = test_node_id(1);
        let auditor = IntegrityAuditor::new(n1);

        assert_eq!(auditor.anomaly_count, 0);
        assert!(auditor.analyzer.baseline().is_none());
    }

    #[test]
    fn test_baseline_establishment() {
        let n1 = test_node_id(1);
        let mut auditor = IntegrityAuditor::new(n1);

        // Add 100 samples to establish baseline
        for i in 0..100 {
            let sample = VibrationSample {
                x: 10,
                y: 20,
                z: 30,
                time: crate::types::TimeMs(i as u64),
            };
            assert!(auditor.process_sample(&sample).is_none());
        }

        assert!(auditor.analyzer.baseline().is_some());
    }

    #[test]
    fn test_anomaly_detection() {
        let n1 = test_node_id(1);
        let mut auditor = IntegrityAuditor::new(n1);

        // Establish baseline at (0, 0, 0)
        for i in 0..100 {
            auditor.process_sample(&VibrationSample {
                x: 0,
                y: 0,
                z: 0,
                time: crate::types::TimeMs(i as u64),
            });
        }

        // Anomalous sample (large deviation)
        let alert = auditor.process_sample(&VibrationSample {
            x: 100,
            y: 100,
            z: 100,
            time: crate::types::TimeMs(200),
        });

        // Should not trigger yet — need 5 anomalies
        assert!(alert.is_none());
        assert_eq!(auditor.anomaly_count, 1);

        // Trigger 4 more anomalies
        for i in 0..4 {
            let a = auditor.process_sample(&VibrationSample {
                x: 100,
                y: 100,
                z: 100,
                time: crate::types::TimeMs(200 + i as u64),
            });
            if i < 3 {
                assert!(a.is_none());
            } else {
                // 5th anomaly should trigger
                assert!(a.is_some());
                let alert = a.unwrap();
                assert!(alert.confidence > 0);
            }
        }
    }

    #[test]
    fn test_integrity_report() {
        let n1 = test_node_id(1);
        let mut auditor = IntegrityAuditor::new(n1);

        // Add baseline + anomaly
        for i in 0..100 {
            auditor.process_sample(&VibrationSample {
                x: 10,
                y: 10,
                z: 10,
                time: crate::types::TimeMs(i as u64),
            });
        }

        let report = auditor.build_report();
        assert_eq!(report.node_id, n1);
    }

    #[test]
    fn test_check_interval() {
        let n1 = test_node_id(1);
        let mut auditor = IntegrityAuditor::new(n1);

        assert!(auditor.should_check()); // Should check immediately
        auditor.mark_checked();
        assert!(!auditor.should_check()); // Should not check immediately after
    }
}
