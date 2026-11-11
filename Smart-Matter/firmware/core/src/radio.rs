//! # Radio Abstraction Layer
//!
//! Frequency-agnostic radio driver trait supporting both 2.4 GHz (nRF24L01+)
//! and Sub-1 GHz (CC1101 / CC1310) transceivers.
//!
//! ## Rationale
//!
//! 2.4 GHz signals are severely attenuated by concrete and potting compound
//! due to moisture content and high dielectric constant of structural materials.
//! Testing has shown that 2.4 GHz range can collapse to under 1 meter when
//! nodes are embedded in concrete.
//!
//! Sub-1 GHz (868/915 MHz) offers significantly better penetration through
//! concrete and other structural materials at the cost of lower data rates.
//! The CC1101 is a drop-in compatible alternative for the nRF24L01+.
//!
//! This trait abstracts the radio interface so the same mesh protocol can
//! operate over either frequency band without modification.

use crate::types::ProtocolSlice;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Center frequencies for supported bands.
pub const FREQ_2_4_GHZ: u32 = 2_400_000_000; // nRF24L01+
pub const FREQ_868_MHZ: u32 = 868_000_000; // CC1101 (EU)
pub const FREQ_915_MHZ: u32 = 915_000_000; // CC1101 (US)

/// Maximum packet size in bytes for each radio.
pub const MAX_PACKET_2_4_GHZ: usize = 32; // nRF24L01+ max payload
pub const MAX_PACKET_SUB_1_GHZ: usize = 64; // CC1101 max with FIFO

// ---------------------------------------------------------------------------
// Frequency Band
// ---------------------------------------------------------------------------

/// Supported frequency bands for Smart Matter nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyBand {
    /// 2.4 GHz ISM band. Higher data rate, worse material penetration.
    GHz24,
    /// Sub-1 GHz ISM band (868 MHz EU, 915 MHz US). Lower data rate,
    /// significantly better penetration through concrete and structural materials.
    SubGhz,
}

impl FrequencyBand {
    /// Recommended band for concrete/structural embedding.
    pub fn recommended_for_concrete_embedding() -> Self {
        Self::SubGhz
    }

    /// Center frequency in Hz.
    pub fn center_hz(&self) -> u32 {
        match self {
            Self::GHz24 => FREQ_2_4_GHZ,
            Self::SubGhz => FREQ_915_MHZ, // 868 for EU
        }
    }

    /// Maximum reliable range in free space (meters).
    pub fn free_space_range_m(&self) -> u32 {
        match self {
            Self::GHz24 => 100,
            Self::SubGhz => 500,
        }
    }

    /// Expected range when embedded in concrete (meters, approximate).
    pub fn concrete_penetration_m(&self) -> f32 {
        match self {
            Self::GHz24 => 0.5,  // Can collapse below 1 m
            Self::SubGhz => 5.0, // Significantly better penetration
        }
    }

    /// Maximum data rate in kbps.
    pub fn max_data_rate_kbps(&self) -> u32 {
        match self {
            Self::GHz24 => 2000,
            Self::SubGhz => 250,
        }
    }
}

// ---------------------------------------------------------------------------
// Radio Error
// ---------------------------------------------------------------------------

/// Radio driver errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioError {
    /// CRC check failed on received packet.
    CrcMismatch,
    /// Radio channel collision detected.
    Collision,
    /// Transmit/receive buffer underrun.
    Underrun,
    /// Operation timed out.
    Timeout,
    /// Insufficient energy for radio operation.
    EnergyLow,
    /// Radio hardware busy.
    Busy,
    /// Invalid frequency/channel configuration.
    InvalidConfig,
}

// ---------------------------------------------------------------------------
// Radio Trait
// ---------------------------------------------------------------------------

/// Generic radio transceiver trait.
///
/// Implementations exist for:
/// - `nRF24L01+` on 2.4 GHz (default, higher data rate)
/// - `CC1101` on Sub-1 GHz (better concrete penetration)
///
/// The mesh protocol layer uses this trait exclusively, making it
/// frequency-agnostic. The same firmware binary can operate on either
/// band by swapping the radio driver at compile time or via runtime config.
pub trait Radio {
    /// Initialize the radio with the given frequency band.
    fn init(&mut self, band: FrequencyBand) -> Result<(), RadioError>;

    /// Set the transmit power (dBm).
    fn set_tx_power(&mut self, power_dbm: i8) -> Result<(), RadioError>;

    /// Get the current RSSI of the last received packet (dBm).
    fn last_rssi(&self) -> i16;

    /// Get the current frequency band.
    fn frequency_band(&self) -> FrequencyBand;

    /// Send a raw protocol slice.
    fn send(&mut self, slice: &ProtocolSlice) -> Result<(), RadioError>;

    /// Receive a protocol slice (blocking with timeout in milliseconds).
    fn receive(&mut self, timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError>;

    /// Enter low-power sleep mode.
    fn sleep(&mut self) -> Result<(), RadioError>;

    /// Wake from low-power sleep.
    fn wake(&mut self) -> Result<(), RadioError>;

    /// Check if the radio has a pending packet (non-blocking).
    fn has_pending(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Concrete Penetration Model
// ---------------------------------------------------------------------------

/// Models RF signal attenuation through concrete and structural materials.
///
/// Used by the integrity auditor and network simulator to predict
/// communication range based on the embedding medium.
#[derive(Debug, Clone)]
pub struct ConcretePenetrationModel {
    /// Frequency band in use.
    band: FrequencyBand,
    /// Concrete moisture content percentage (0–100).
    /// Wet concrete has significantly higher attenuation.
    moisture_pct: f32,
    /// Thickness of concrete between nodes (meters).
    thickness_m: f32,
}

impl ConcretePenetrationModel {
    pub fn new(band: FrequencyBand, moisture_pct: f32, thickness_m: f32) -> Self {
        Self {
            band,
            moisture_pct,
            thickness_m,
        }
    }

    /// Estimate path loss through concrete (dB).
    ///
    /// Based on ITU-R P.2040 model for building materials:
    ///   L_concrete(dB) = a × f^b × d
    ///
    /// Where:
    ///   a, b = material-dependent coefficients
    ///   f = frequency (GHz)
    ///   d = thickness (m)
    pub fn concrete_path_loss_db(&self) -> f32 {
        let freq_ghz: f32 = match self.band {
            FrequencyBand::GHz24 => 2.4,
            FrequencyBand::SubGhz => 0.915,
        };

        // ITU-R P.2040 coefficients for concrete (dry)
        let (a_dry, b_dry) = (5.31, 0.67);
        // Wet concrete multiplier (moisture increases attenuation ~3x)
        let moisture_factor = 1.0 + (self.moisture_pct / 100.0) * 2.0;

        a_dry * libm::powf(freq_ghz, b_dry) * self.thickness_m * moisture_factor
    }

    /// Estimate whether two nodes can communicate given concrete thickness.
    pub fn can_communicate(&self, tx_power_dbm: i8, rx_sensitivity_dbm: i16) -> bool {
        let link_budget = (tx_power_dbm as i16) - rx_sensitivity_dbm;
        let loss = self.concrete_path_loss_db() as i16;
        // Free space path loss at 1 m is ~40 dB at 2.4 GHz, ~20 dB at 915 MHz
        let free_space_loss = match self.band {
            FrequencyBand::GHz24 => 40.0,
            FrequencyBand::SubGhz => 20.0,
        } as i16;
        link_budget > (loss + free_space_loss)
    }

    /// Recommended maximum node spacing for reliable communication (meters).
    pub fn recommended_spacing_m(&self, _tx_power_dbm: i8) -> f32 {
        // Conservative estimate: space nodes so concrete loss < 20 dB
        let max_loss_db = 20.0;
        let freq_ghz: f32 = match self.band {
            FrequencyBand::GHz24 => 2.4,
            FrequencyBand::SubGhz => 0.915,
        };
        let moisture_factor = 1.0 + (self.moisture_pct / 100.0) * 2.0;
        let loss_per_m = 5.31 * libm::powf(freq_ghz, 0.67) * moisture_factor;
        (max_loss_db / loss_per_m).max(0.5)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_band_recommendation() {
        assert_eq!(
            FrequencyBand::recommended_for_concrete_embedding(),
            FrequencyBand::SubGhz
        );
    }

    #[test]
    fn test_sub_ghz_penetration_better_than_2_4() {
        let dry_concrete = ConcretePenetrationModel::new(
            FrequencyBand::GHz24,
            5.0, // Dry concrete
            0.1, // 10 cm wall
        );
        let loss_24 = dry_concrete.concrete_path_loss_db();

        let sub_ghz = ConcretePenetrationModel::new(FrequencyBand::SubGhz, 5.0, 0.1);
        let loss_sub = sub_ghz.concrete_path_loss_db();

        // Sub-1 GHz should have significantly less loss through concrete
        assert!(loss_sub < loss_24 * 0.6);
    }

    #[test]
    fn test_moisture_increases_loss() {
        let dry = ConcretePenetrationModel::new(
            FrequencyBand::GHz24,
            5.0, // Dry
            0.2, // 20 cm
        );
        let wet = ConcretePenetrationModel::new(
            FrequencyBand::GHz24,
            50.0, // Wet
            0.2,
        );
        assert!(wet.concrete_path_loss_db() > dry.concrete_path_loss_db());
    }

    #[test]
    fn test_can_communicate_sub_ghz_through_concrete() {
        let model = ConcretePenetrationModel::new(
            FrequencyBand::SubGhz,
            10.0, // Slightly damp
            0.3,  // 30 cm wall
        );
        // At 10 dBm TX with -110 dBm sensitivity
        assert!(model.can_communicate(10, -110));
    }

    #[test]
    fn test_recommended_spacing() {
        let model = ConcretePenetrationModel::new(
            FrequencyBand::SubGhz,
            5.0, // Dry
            0.0, // Not used for spacing calc
        );
        let spacing = model.recommended_spacing_m(10);
        // Should recommend positive spacing
        assert!(spacing > 0.1);
        // Sub-GHz should allow wider spacing than 2.4 GHz
        let model_24 = ConcretePenetrationModel::new(FrequencyBand::GHz24, 5.0, 0.0);
        let spacing_24 = model_24.recommended_spacing_m(10);
        assert!(spacing > spacing_24);
    }
}
