//! # Piezoelectric Kinetic Extraction
//!
//! Energy harvesting management for Smart Matter nodes.
//!
//! Models the piezoelectric element → bridge rectifier → supercapacitor → LDO
//! regulator → MCU power chain. Tracks energy budgets, predicts available
//! energy for compute and transmission, and manages power states.

use crate::types::{EnergyLevel, VoltageReading};
use core::cmp;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Typical piezo open-circuit voltage (mV) under nominal vibration (0.5g @ 60 Hz).
pub const PIEZO_NOMINAL_MV: u32 = 5_000;

/// Supercapacitor target charge voltage (mV).
pub const SUPERCAP_TARGET_MV: u32 = 3_300;

/// LDO output voltage (mV).
pub const LDO_OUTPUT_MV: u32 = 1_800;

/// Minimum supercapacitor voltage (mV) to power the MCU.
pub const MCU_UVLO_MV: u32 = 2_200;

/// Energy required for a single sub-ms transmission slice (μJ).
pub const ENERGY_PER_SLICE_UJ: u32 = 50;

/// Energy required for a sensor read + ADC sample (μJ).
pub const ENERGY_PER_SAMPLE_UJ: u32 = 10;

// ---------------------------------------------------------------------------
// Energy Budget
// ---------------------------------------------------------------------------

/// Tracks the energy budget for the node.
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Current supercapacitor voltage (mV).
    storage_mv: u32,
    /// Last measured harvester voltage (mV).
    harvester_mv: u32,
    /// Estimated energy stored (μJ). E = ½CV².
    stored_uj: u64,
    /// Capacitance of storage capacitor (μF).
    capacitance_uf: u32,
}

impl EnergyBudget {
    /// Create a new energy budget tracker.
    ///
    /// `capacitance_uf`: Storage supercapacitor value in microfarads.
    pub fn new(capacitance_uf: u32) -> Self {
        Self {
            storage_mv: 0,
            harvester_mv: 0,
            stored_uj: 0,
            capacitance_uf,
        }
    }

    /// Update from a voltage reading.
    pub fn update(&mut self, reading: &VoltageReading) {
        self.harvester_mv = reading.harvester_mv;
        self.storage_mv = reading.storage_mv;
        self.stored_uj = Self::energy_from_voltage(self.storage_mv, self.capacitance_uf);
    }

    /// Calculate stored energy (μJ) from voltage across a capacitor.
    fn energy_from_voltage(mv: u32, capacitance_uf: u32) -> u64 {
        // E = ½ * C * V²
        // C in μF, V in mV => need factor of 10⁻¹² for J, multiply by 10⁶ for μJ
        // E_μJ = ½ * C_μF * (V_V)² * 10⁶
        // V_V = mv / 1000
        // E_μJ = ½ * C_μF * (mv² / 10⁶) * 10⁶ = ½ * C_μF * mv² / 10⁶
        let v = mv as u64;
        let c = capacitance_uf as u64;
        (c * v * v) / 2_000_000
    }

    /// Remaining energy in μJ.
    pub fn stored_uj(&self) -> u64 {
        self.stored_uj
    }

    /// Determine energy level from current storage.
    pub fn energy_level(&self) -> EnergyLevel {
        if self.storage_mv >= SUPERCAP_TARGET_MV {
            EnergyLevel::Surplus
        } else if self.storage_mv >= MCU_UVLO_MV + 500 {
            EnergyLevel::Nominal
        } else if self.storage_mv >= MCU_UVLO_MV {
            EnergyLevel::Low
        } else {
            EnergyLevel::Critical
        }
    }

    /// How many transmission slices can we afford?
    pub fn available_slices(&self) -> u32 {
        let per_slice = ENERGY_PER_SLICE_UJ as u64;
        if per_slice == 0 {
            return 0;
        }
        cmp::min(
            (self.stored_uj / per_slice) as u32,
            255, // Cap at 255 for protocol field
        )
    }

    /// Reserve energy for `count` slices. Returns true if successful.
    pub fn reserve_slices(&mut self, count: u32) -> bool {
        let needed = (count as u64) * ENERGY_PER_SLICE_UJ as u64;
        if needed <= self.stored_uj {
            self.stored_uj -= needed;
            true
        } else {
            false
        }
    }

    /// Voltage reading for status reporting.
    pub fn voltage_reading(&self) -> VoltageReading {
        VoltageReading {
            harvester_mv: self.harvester_mv,
            storage_mv: self.storage_mv,
            vdd_mv: if self.storage_mv >= MCU_UVLO_MV {
                LDO_OUTPUT_MV
            } else {
                0
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Piezoelectric Model
// ---------------------------------------------------------------------------

/// Simple model of piezoelectric energy extraction.
#[derive(Debug, Clone)]
pub struct PiezoModel {
    /// Resonance frequency (Hz).
    resonance_hz: u32,
    /// Quality factor.
    q_factor: f32,
    /// Open-circuit voltage amplitude at resonance (mV).
    oc_voltage_mv: u32,
}

impl PiezoModel {
    pub fn new(resonance_hz: u32, q_factor: f32, oc_voltage_mv: u32) -> Self {
        Self {
            resonance_hz,
            q_factor,
            oc_voltage_mv,
        }
    }

    /// Estimate power output (μW) for a given vibration frequency and amplitude.
    ///
    /// Uses a simple resonant peak model:
    ///   P(f) = P_peak * (Q² / (1 + Q² * (f/f₀ - f₀/f)²))
    pub fn estimated_power_uw(&self, vibration_hz: u32, amplitude_g: f32) -> u32 {
        if amplitude_g < 0.05 {
            return 0; // Below noise floor
        }

        let ratio = vibration_hz as f32 / self.resonance_hz as f32;
        let detuning = ratio - 1.0 / ratio;
        let response = (self.q_factor * self.q_factor)
            / (1.0 + self.q_factor * self.q_factor * detuning * detuning);

        // Peak power scales with amplitude² and voltage² / R_load
        let peak_power_uw = (self.oc_voltage_mv as f32 / 1000.0)
            * (self.oc_voltage_mv as f32 / 1000.0)
            / 10_000.0 // Assume 10kΩ optimal load
            * 1_000_000.0;

        (peak_power_uw * response * amplitude_g) as u32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_calculation() {
        // 1 F supercapacitor at 3.3 V (matches whitepaper §2.1.5 / BOM):
        // E = ½ * 1 * 3.3² = 5.445 J = 5,445,000 μJ
        let e = EnergyBudget::energy_from_voltage(3300, 1_000_000);
        assert!(e > 5_000_000 && e < 6_000_000);
    }

    #[test]
    fn test_energy_levels() {
        let cap_uf = 1_000_000; // 1 F
        let mut budget = EnergyBudget::new(cap_uf);

        // At 0V, should be critical
        assert_eq!(budget.energy_level(), EnergyLevel::Critical);

        // At supercap target, should be surplus
        budget.update(&VoltageReading {
            harvester_mv: 5000,
            storage_mv: 3300,
            vdd_mv: 1800,
        });
        assert_eq!(budget.energy_level(), EnergyLevel::Surplus);

        // Near UVLO
        budget.update(&VoltageReading {
            harvester_mv: 5000,
            storage_mv: 2300,
            vdd_mv: 1800,
        });
        assert_eq!(budget.energy_level(), EnergyLevel::Low);
    }

    #[test]
    fn test_reserve_slices() {
        let cap_uf = 1_000_000; // 1 F
        let mut budget = EnergyBudget::new(cap_uf);
        budget.update(&VoltageReading {
            harvester_mv: 5000,
            storage_mv: 3300,
            vdd_mv: 1800,
        });

        // We have ~5,445,000 μJ / 50 = ~108,900 slices worth of energy
        // (capped at 255 by available_slices()).
        let total_available = budget.available_slices();
        assert_eq!(total_available, 255); // Capped by the protocol field

        // Reserving the cap consumes 255 × 50 μJ from the stored budget.
        assert!(budget.reserve_slices(255));
        assert_eq!(budget.stored_uj(), 5_445_000 - 255 * 50);
        // The session cap is enforced by available_slices(); the store still
        // holds ~108,000 slices of headroom for later sessions.
        assert!(budget.available_slices() >= 255);
    }

    #[test]
    fn test_piezo_model() {
        let model = PiezoModel::new(60, 30.0, 5000);
        // At resonance with 1g
        let power = model.estimated_power_uw(60, 1.0);
        assert!(power > 0);
        // Off-resonance should be less
        let off_power = model.estimated_power_uw(120, 1.0);
        assert!(off_power < power);
        // Below noise floor
        let noise = model.estimated_power_uw(60, 0.01);
        assert_eq!(noise, 0);
    }
}
