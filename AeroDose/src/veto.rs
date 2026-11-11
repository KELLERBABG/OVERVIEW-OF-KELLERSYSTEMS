//! Non-disableable haptic emergency veto state machine.
//!
//! This module implements an autonomous safety mechanism that enforces
//! haptic + piezo alerts at critical toxic thresholds. The key invariant is
//! that the wearer **cannot** mute or dismiss alerts once they reach
//! `Mandatory` or `EmergencyVeto` states.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetoState {
    /// Safe conditions — no alert required.
    Nominal,
    /// TWA approaching limit — advisory haptic pulse.
    Advisory,
    /// TWA exceeded — alert cannot be dismissed by wearer.
    Mandatory,
    /// Acute spike detected — maximum intensity, no software override.
    EmergencyVeto,
}

#[derive(Debug, Clone)]
pub struct HapticVeto {
    state: VetoState,
    /// Peak PM2.5 in the current acute window (last 60 s).
    acute_peak_ug_m3: f64,
    /// Running TWA in µg/m³.
    twa_ug_m3: f64,
    /// OSHA 8-hr PEL for PM2.5 (µg/m³) — configurable.
    pel_ug_m3: f64,
    /// Acute spike threshold (µg/m³) — triggers EmergencyVeto.
    acute_spike_threshold_ug_m3: f64,
    /// Whether the wearer attempted to mute (logged, never acted on).
    mute_attempts: u32,
    /// Counter tracking consecutive calls below 0.5× acute threshold.
    /// Must reach 3 before EmergencyVeto can transition out.
    recovery_counter: u32,
}

impl HapticVeto {
    /// Create a new `HapticVeto` in the `Nominal` state.
    ///
    /// # Arguments
    /// * `pel_ug_m3` — OSHA 8-hour PEL for PM2.5 in µg/m³.
    /// * `acute_spike_threshold_ug_m3` — Acute spike threshold in µg/m³.
    pub fn new(pel_ug_m3: f64, acute_spike_threshold_ug_m3: f64) -> Self {
        Self {
            state: VetoState::Nominal,
            acute_peak_ug_m3: 0.0,
            twa_ug_m3: 0.0,
            pel_ug_m3,
            acute_spike_threshold_ug_m3,
            mute_attempts: 0,
            recovery_counter: 0,
        }
    }

    /// Update the veto state based on current PM2.5 and TWA readings.
    ///
    /// Transition rules:
    /// - If `current_pm25 >= acute_spike_threshold_ug_m3` → `EmergencyVeto`.
    /// - Else if `twa >= pel_ug_m3` → `Mandatory`.
    /// - Else if `twa >= 0.8 * pel_ug_m3` → `Advisory`.
    /// - Else → `Nominal` (only from Advisory; never from Mandatory/EmergencyVeto
    ///   without the acute recovery condition).
    ///
    /// EmergencyVeto recovery: requires 3 consecutive calls where PM2.5 is
    /// below 0.5× the acute threshold. On recovery, transitions to whatever
    /// state the TWA would dictate (Mandatory, Advisory, or Nominal).
    pub fn update(&mut self, current_pm25: f64, twa: f64) {
        // Track acute peak
        if current_pm25 > self.acute_peak_ug_m3 {
            self.acute_peak_ug_m3 = current_pm25;
        }
        self.twa_ug_m3 = twa;

        let recovery_threshold = 0.5 * self.acute_spike_threshold_ug_m3;

        // Handle EmergencyVeto recovery logic first
        if self.state == VetoState::EmergencyVeto {
            if current_pm25 < recovery_threshold {
                self.recovery_counter = self.recovery_counter.saturating_add(1);
            } else {
                // Reset counter if PM rises back above recovery threshold
                self.recovery_counter = 0;
            }

            if self.recovery_counter >= 3 {
                // Recovery complete — transition based on TWA
                self.recovery_counter = 0;
                self.state = self.twa_to_state(twa);
                return;
            }
            // Still in EmergencyVeto, no state change
            return;
        }

        // Not in EmergencyVeto — reset recovery counter
        self.recovery_counter = 0;

        // Determine new state based on current readings
        let new_state = if current_pm25 >= self.acute_spike_threshold_ug_m3 {
            VetoState::EmergencyVeto
        } else if twa >= self.pel_ug_m3 {
            VetoState::Mandatory
        } else if twa >= 0.8 * self.pel_ug_m3 {
            VetoState::Advisory
        } else {
            VetoState::Nominal
        };

        // Apply transition rules:
        // - Can always escalate to a higher severity state.
        // - Can only downgrade from Advisory to Nominal.
        // - Cannot downgrade from Mandatory or EmergencyVeto (except via recovery above).
        self.state = match (self.state, new_state) {
            // Escalation is always allowed
            (_, VetoState::EmergencyVeto) => VetoState::EmergencyVeto,
            (VetoState::Nominal | VetoState::Advisory, VetoState::Mandatory) => VetoState::Mandatory,
            (VetoState::Nominal, VetoState::Advisory) => VetoState::Advisory,
            // Downgrade only from Advisory to Nominal
            (VetoState::Advisory, VetoState::Nominal) => VetoState::Nominal,
            // Stay in current state if new state is lower but not allowed downgrade
            _ => self.state,
        };
    }

    /// Attempt to mute the haptic alert.
    ///
    /// **Invariant:** This function ALWAYS returns `false`. The mute attempt
    /// is logged (incrementing `mute_attempts`) but never changes the state.
    /// This enforces the non-disableable safety guarantee.
    pub fn attempt_mute(&mut self) -> bool {
        self.mute_attempts = self.mute_attempts.saturating_add(1);
        false
    }

    /// Return the current veto state.
    pub fn current_state(&self) -> VetoState {
        self.state
    }

    /// Alias for current_state()
    pub fn state(&self) -> VetoState {
        self.state
    }

    /// Return the number of mute attempts made by the wearer.
    pub fn mute_attempt_count(&self) -> u32 {
        self.mute_attempts
    }

    /// Return the haptic intensity level (0.0 to 1.0).
    ///
    /// - `Nominal` → 0.0
    /// - `Advisory` → 0.3
    /// - `Mandatory` → 0.7
    /// - `EmergencyVeto` → 1.0
    pub fn haptic_intensity(&self) -> f64 {
        match self.state {
            VetoState::Nominal => 0.0,
            VetoState::Advisory => 0.3,
            VetoState::Mandatory => 0.7,
            VetoState::EmergencyVeto => 1.0,
        }
    }

    /// Map a TWA value to the appropriate non-emergency state.
    fn twa_to_state(&self, twa: f64) -> VetoState {
        if twa >= self.pel_ug_m3 {
            VetoState::Mandatory
        } else if twa >= 0.8 * self.pel_ug_m3 {
            VetoState::Advisory
        } else {
            VetoState::Nominal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEL: f64 = 50.0; // µg/m³
    const ACUTE_THRESHOLD: f64 = 250.0; // µg/m³

    #[test]
    fn test_start_nominal_and_advisory_transition() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);
        assert_eq!(veto.current_state(), VetoState::Nominal);
        assert_eq!(veto.haptic_intensity(), 0.0);

        // TWA at 90% PEL → Advisory
        let twa_90 = 0.9 * PEL; // 45.0
        veto.update(10.0, twa_90);
        assert_eq!(veto.current_state(), VetoState::Advisory);
        assert_eq!(veto.haptic_intensity(), 0.3);
    }

    #[test]
    fn test_mandatory_transition() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // TWA at 110% PEL → Mandatory
        let twa_110 = 1.1 * PEL; // 55.0
        veto.update(10.0, twa_110);
        assert_eq!(veto.current_state(), VetoState::Mandatory);
        assert_eq!(veto.haptic_intensity(), 0.7);
    }

    #[test]
    fn test_emergency_veto_on_spike() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // PM spike above acute threshold → EmergencyVeto
        veto.update(300.0, 10.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);
        assert_eq!(veto.haptic_intensity(), 1.0);
    }

    #[test]
    fn test_attempt_mute_in_emergency_veto() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Trigger EmergencyVeto
        veto.update(300.0, 10.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        // Attempt mute — should return false, state unchanged, count increments
        let result = veto.attempt_mute();
        assert!(!result);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);
        assert_eq!(veto.mute_attempt_count(), 1);

        // Second attempt
        let result2 = veto.attempt_mute();
        assert!(!result2);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);
        assert_eq!(veto.mute_attempt_count(), 2);
    }

    #[test]
    fn test_emergency_veto_recovery_to_mandatory() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Trigger EmergencyVeto with high TWA
        veto.update(300.0, 60.0); // TWA > PEL
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        let recovery_threshold = 0.5 * ACUTE_THRESHOLD; // 125.0

        // First call below recovery threshold
        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        // Second call below recovery threshold
        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        // Third call below recovery threshold → recovery complete
        // TWA is still > PEL, so should transition to Mandatory
        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::Mandatory);
        assert_eq!(veto.haptic_intensity(), 0.7);
    }

    #[test]
    fn test_emergency_veto_recovery_resets_on_spike() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Trigger EmergencyVeto
        veto.update(300.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        let recovery_threshold = 0.5 * ACUTE_THRESHOLD; // 125.0

        // One call below recovery threshold
        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        // Spike back above acute threshold — resets recovery counter
        veto.update(300.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        // Now need 3 more calls below recovery threshold
        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::EmergencyVeto);

        veto.update(100.0, 60.0);
        assert_eq!(veto.current_state(), VetoState::Mandatory);
    }

    #[test]
    fn test_cannot_downgrade_from_mandatory_to_nominal() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Go to Mandatory
        veto.update(10.0, 55.0);
        assert_eq!(veto.current_state(), VetoState::Mandatory);

        // Try to go to Nominal (TWA drops below 80% PEL)
        // Should stay in Mandatory — cannot downgrade from Mandatory
        veto.update(10.0, 30.0);
        assert_eq!(veto.current_state(), VetoState::Mandatory);
    }

    #[test]
    fn test_advisory_can_downgrade_to_nominal() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Go to Advisory (TWA at 90% PEL)
        veto.update(10.0, 45.0);
        assert_eq!(veto.current_state(), VetoState::Advisory);

        // TWA drops below 80% PEL → can downgrade to Nominal
        veto.update(10.0, 30.0);
        assert_eq!(veto.current_state(), VetoState::Nominal);
    }

    #[test]
    fn test_haptic_intensity_values() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Nominal
        assert_eq!(veto.haptic_intensity(), 0.0);

        // Advisory
        veto.update(10.0, 45.0);
        assert_eq!(veto.haptic_intensity(), 0.3);

        // Mandatory
        veto.update(10.0, 55.0);
        assert_eq!(veto.haptic_intensity(), 0.7);

        // EmergencyVeto
        veto.update(300.0, 55.0);
        assert_eq!(veto.haptic_intensity(), 1.0);
    }

    #[test]
    fn test_mute_attempts_always_increment() {
        let mut veto = HapticVeto::new(PEL, ACUTE_THRESHOLD);

        // Even in Nominal state, mute attempts are logged
        assert!(!veto.attempt_mute());
        assert_eq!(veto.mute_attempt_count(), 1);

        assert!(!veto.attempt_mute());
        assert_eq!(veto.mute_attempt_count(), 2);

        // State should still be Nominal
        assert_eq!(veto.current_state(), VetoState::Nominal);
    }
}
