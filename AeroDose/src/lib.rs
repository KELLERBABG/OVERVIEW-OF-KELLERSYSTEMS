//! AeroDose / SmokeShift Tracker — occupational PM2.5 monitoring library.
//!
//! Provides audit hash-chaining, compliance reporting, dose tracking,
//! thermodynamic drift compensation, taint-coded stream validation,
//! and a non-disableable haptic emergency veto state machine.

pub mod audit;
pub mod compliance;
pub mod dose;
pub mod sensor;
pub mod thermo;
pub mod taint;
pub mod veto;

// ─── Crate-root re-exports for CLI convenience ────────────────────────────────
pub use audit::AuditLog;
pub use compliance::ComplianceEngine;
pub use dose::{AlertTier, ExposureLimits, ShiftDoseTracker};
pub use sensor::{Sps30Calibrator, Sps30Reading};

// ─── Integration tests ──────────────────────────────────────────────────────────
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::audit::AuditLog;
    use crate::thermo::{AmbientConditions, CorrectedReading, ThermoCompensator};
    use crate::taint::{TaintDetector, TaintFlag};
    use crate::veto::{HapticVeto, VetoState};

    /// Full pipeline: thermo correction → taint validation.
    #[test]
    fn full_shift_pipeline_with_thermo_and_taint() {
        // 1. ThermoCompensator: raw reading at 35 °C / 70 % RH
        let comp = ThermoCompensator::new();
        let raw = CorrectedReading {
            pm1_0_ug_m3: 12.0,
            pm2_5_ug_m3: 25.0,
            pm4_0_ug_m3: 35.0,
            pm10_ug_m3: 50.0,
            particle_count_p_cc: 800.0,
        };
        let ambient = AmbientConditions {
            temperature_c: 35.0,
            relative_humidity_pct: 70.0,
        };

        let corrected = comp.correct(&raw, &ambient);

        // g = 1 + 0.0045*(70-50) = 1.09
        // t_factor = 1 + 0.002*(25-35) = 0.98
        // combined = 1.0682 → corrected < raw
        assert!(
            corrected.pm2_5_ug_m3 < raw.pm2_5_ug_m3,
            "Corrected PM2.5 ({:.4}) must be < raw PM2.5 ({:.4})",
            corrected.pm2_5_ug_m3,
            raw.pm2_5_ug_m3
        );

        // 2. TaintDetector window_size=10, push 10 realistic readings (±2 µg/m³)
        let mut taint = TaintDetector::new(10);
        let base = corrected.pm2_5_ug_m3;
        let offsets: [f64; 10] = [0.0, 1.5, -1.0, 2.0, -0.5, 1.0, -2.0, 0.5, -1.5, 1.2];
        for &off in &offsets {
            taint.push_sample(base + off);
        }

        let report = taint.evaluate();
        assert_eq!(
            report.flag,
            TaintFlag::Clean,
            "Realistic varying readings should be Clean, got {:?} (stddev={:.4})",
            report.flag,
            report.window_stddev
        );

        // 3. Push 8 identical values → flag must no longer be Clean
        for _ in 0..8 {
            taint.push_sample(base);
        }
        let report2 = taint.evaluate();
        assert_ne!(
            report2.flag,
            TaintFlag::Clean,
            "After 8 identical values the flag must not be Clean, got {:?} (max_run={})",
            report2.flag,
            report2.max_consecutive_equal
        );
    }

    /// Veto escalation: Nominal → Advisory → Mandatory → EmergencyVeto,
    /// and the mute-invariant (state never changes from EmergencyVeto).
    #[test]
    fn veto_escalation_and_mute_invariant() {
        let mut veto = HapticVeto::new(50.0, 300.0);

        // twa=45 → Advisory (45 >= 0.8*50=40, < 50)
        veto.update(10.0, 45.0);
        assert_eq!(
            veto.state(),
            VetoState::Advisory,
            "TWA 45 should escalate to Advisory"
        );

        // twa=55 → Mandatory (55 >= 50)
        veto.update(10.0, 55.0);
        assert_eq!(
            veto.state(),
            VetoState::Mandatory,
            "TWA 55 should escalate to Mandatory"
        );

        // pm25=350 → EmergencyVeto (350 >= 300)
        veto.update(350.0, 55.0);
        assert_eq!(
            veto.state(),
            VetoState::EmergencyVeto,
            "PM2.5 350 should trigger EmergencyVeto"
        );

        // Attempt mute 5 times — state must remain EmergencyVeto
        for _ in 0..5 {
            veto.attempt_mute();
        }
        assert_eq!(
            veto.state(),
            VetoState::EmergencyVeto,
            "State must remain EmergencyVeto after mute attempts"
        );
        assert_eq!(
            veto.mute_attempt_count(),
            5,
            "mute_attempt_count must equal 5"
        );
    }

    /// Audit hash-chain integrity survives thermo-corrected values.
    #[test]
    fn audit_chain_survives_thermo_correction() {
        use ed25519_dalek::{SigningKey, VerifyingKey};

        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = VerifyingKey::from(&signing_key);

        let comp = ThermoCompensator::new();
        let ambient = AmbientConditions {
            temperature_c: 30.0,
            relative_humidity_pct: 65.0,
        };

        let mut log = AuditLog::new(signing_key);

        // Three raw readings to be thermo-corrected before logging
        let raws: Vec<CorrectedReading> = vec![
            CorrectedReading {
                pm1_0_ug_m3: 8.0,
                pm2_5_ug_m3: 15.0,
                pm4_0_ug_m3: 22.0,
                pm10_ug_m3: 35.0,
                particle_count_p_cc: 600.0,
            },
            CorrectedReading {
                pm1_0_ug_m3: 9.5,
                pm2_5_ug_m3: 18.0,
                pm4_0_ug_m3: 25.0,
                pm10_ug_m3: 38.0,
                particle_count_p_cc: 700.0,
            },
            CorrectedReading {
                pm1_0_ug_m3: 7.0,
                pm2_5_ug_m3: 12.0,
                pm4_0_ug_m3: 20.0,
                pm10_ug_m3: 30.0,
                particle_count_p_cc: 500.0,
            },
        ];

        let worker_id = uuid::Uuid::new_v4();
        for raw in &raws {
            let corrected = comp.correct(raw, &ambient);
            log.append_entry(
                worker_id,
                corrected.pm2_5_ug_m3,
                corrected.pm10_ug_m3,
                "thermo_corrected",
            );
        }

        // Verify the full hash-chain with the corresponding public key
        let result = log.verify_chain(&verifying_key);
        assert!(
            result.is_ok(),
            "Audit chain verification must succeed after thermo-corrected appends: {:?}",
            result.err()
        );
    }
}
