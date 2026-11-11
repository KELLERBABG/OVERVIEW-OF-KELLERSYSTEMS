//! Thermodynamic drift compensation for Sensirion SPS30 optical particle counts.
//!
//! Corrects raw mass concentrations for hygroscopic particle growth (RH-dependent)
//! and laser diode power drift (temperature-dependent).

use thiserror::Error;

/// Ambient conditions measured alongside the SPS30 reading.
#[derive(Debug, Clone, Copy)]
pub struct AmbientConditions {
    /// Temperature in degrees Celsius. Valid range: -20..60.
    pub temperature_c: f64,
    /// Relative humidity as a percentage (0.0–100.0).
    pub relative_humidity_pct: f64,
}

/// A corrected particle concentration reading.
#[derive(Debug, Clone, Copy)]
pub struct CorrectedReading {
    pub pm1_0_ug_m3: f64,
    pub pm2_5_ug_m3: f64,
    pub pm4_0_ug_m3: f64,
    pub pm10_ug_m3: f64,
    pub particle_count_p_cc: f64,
}

/// Errors that can occur during ambient condition validation.
#[derive(Debug, Error)]
pub enum ThermoError {
    #[error("temperature {0}°C out of sensor range [-20, 60]")]
    TemperatureOutOfRange(f64),
    #[error("relative humidity {0}% out of range [0, 100]")]
    HumidityOutOfRange(f64),
}

/// Stateless thermodynamic drift compensator for SPS30 readings.
#[derive(Debug, Clone)]
pub struct ThermoCompensator;

impl ThermoCompensator {
    /// Creates a new `ThermoCompensator`. No internal state is held.
    pub fn new() -> Self {
        Self
    }

    /// Applies hygroscopic growth and temperature drift correction to a raw reading.
    ///
    /// - Hygroscopic growth factor: `g = 1.0 + 0.0045 * max(rh - 50.0, 0.0)`
    ///   Models ~0.45 %/RH-point mass inflation above 50 % RH for NaCl-like aerosols.
    /// - Temperature factor: `t_factor = 1.0 + 0.002 * (25.0 - temperature_c)`
    ///   Compensates for laser diode power drift (~0.2 %/°C around 25 °C reference).
    /// - Corrected value = raw_value / (g * t_factor), clamped to >= 0.0.
    pub fn correct(&self, raw: &CorrectedReading, ambient: &AmbientConditions) -> CorrectedReading {
        let rh = ambient.relative_humidity_pct;
        let temp = ambient.temperature_c;

        // Hygroscopic growth factor (only above 50 % RH)
        let g = 1.0 + 0.0045 * (rh - 50.0).max(0.0);

        // Temperature drift factor (laser diode power compensation)
        let t_factor = 1.0 + 0.002 * (25.0 - temp);

        let combined = g * t_factor;

        CorrectedReading {
            pm1_0_ug_m3: (raw.pm1_0_ug_m3 / combined).max(0.0),
            pm2_5_ug_m3: (raw.pm2_5_ug_m3 / combined).max(0.0),
            pm4_0_ug_m3: (raw.pm4_0_ug_m3 / combined).max(0.0),
            pm10_ug_m3: (raw.pm10_ug_m3 / combined).max(0.0),
            particle_count_p_cc: (raw.particle_count_p_cc / combined).max(0.0),
        }
    }

    /// Validates that ambient conditions are within the sensor's operational range.
    ///
    /// Returns `Err(ThermoError::TemperatureOutOfRange)` if temperature is outside [-20, 60] °C,
    /// or `Err(ThermoError::HumidityOutOfRange)` if RH is outside [0, 100] %.
    pub fn validate_ambient(&self, ambient: &AmbientConditions) -> Result<(), ThermoError> {
        if !(-20.0..=60.0).contains(&ambient.temperature_c) {
            return Err(ThermoError::TemperatureOutOfRange(ambient.temperature_c));
        }
        if !(0.0..=100.0).contains(&ambient.relative_humidity_pct) {
            return Err(ThermoError::HumidityOutOfRange(ambient.relative_humidity_pct));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_correction_at_reference_conditions() {
        let comp = ThermoCompensator::new();
        let raw = CorrectedReading {
            pm1_0_ug_m3: 10.0,
            pm2_5_ug_m3: 20.0,
            pm4_0_ug_m3: 30.0,
            pm10_ug_m3: 40.0,
            particle_count_p_cc: 500.0,
        };
        let ambient = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: 50.0,
        };

        let corrected = comp.correct(&raw, &ambient);

        // At 25 °C and 50 % RH, g = 1.0 and t_factor = 1.0, so output == input.
        assert!((corrected.pm1_0_ug_m3 - raw.pm1_0_ug_m3).abs() < 1e-9);
        assert!((corrected.pm2_5_ug_m3 - raw.pm2_5_ug_m3).abs() < 1e-9);
        assert!((corrected.pm4_0_ug_m3 - raw.pm4_0_ug_m3).abs() < 1e-9);
        assert!((corrected.pm10_ug_m3 - raw.pm10_ug_m3).abs() < 1e-9);
        assert!((corrected.particle_count_p_cc - raw.particle_count_p_cc).abs() < 1e-9);
    }

    #[test]
    fn test_hygroscopic_correction_at_high_rh() {
        let comp = ThermoCompensator::new();
        let raw = CorrectedReading {
            pm1_0_ug_m3: 10.0,
            pm2_5_ug_m3: 100.0,
            pm4_0_ug_m3: 30.0,
            pm10_ug_m3: 40.0,
            particle_count_p_cc: 500.0,
        };
        // 25 °C (no temp correction), 80 % RH
        let ambient = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: 80.0,
        };

        let corrected = comp.correct(&raw, &ambient);

        // g = 1.0 + 0.0045 * (80 - 50) = 1.0 + 0.135 = 1.135
        // t_factor = 1.0 (at 25 °C)
        // Expected PM2.5 = 100.0 / 1.135 ≈ 88.09...
        let expected_pm25 = 100.0 / 1.135;
        assert!(
            (corrected.pm2_5_ug_m3 - expected_pm25).abs() < 1e-6,
            "Expected PM2.5 ≈ {}, got {}",
            expected_pm25,
            corrected.pm2_5_ug_m3
        );

        // Verify the approximate factor: 1/(1+0.0045*30) ≈ 0.8809...
        let factor = 1.0 / (1.0 + 0.0045 * 30.0);
        assert!(
            (corrected.pm2_5_ug_m3 - raw.pm2_5_ug_m3 * factor).abs() < 1e-6,
            "Factor check failed"
        );
    }

    #[test]
    fn test_out_of_range_temperature_returns_err() {
        let comp = ThermoCompensator::new();

        // Too cold
        let cold = AmbientConditions {
            temperature_c: -25.0,
            relative_humidity_pct: 50.0,
        };
        assert!(matches!(
            comp.validate_ambient(&cold),
            Err(ThermoError::TemperatureOutOfRange(_))
        ));

        // Too hot
        let hot = AmbientConditions {
            temperature_c: 65.0,
            relative_humidity_pct: 50.0,
        };
        assert!(matches!(
            comp.validate_ambient(&hot),
            Err(ThermoError::TemperatureOutOfRange(_))
        ));
    }

    #[test]
    fn test_out_of_range_humidity_returns_err() {
        let comp = ThermoCompensator::new();

        // RH too low (negative)
        let dry = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: -1.0,
        };
        assert!(matches!(
            comp.validate_ambient(&dry),
            Err(ThermoError::HumidityOutOfRange(_))
        ));

        // RH too high
        let wet = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: 101.0,
        };
        assert!(matches!(
            comp.validate_ambient(&wet),
            Err(ThermoError::HumidityOutOfRange(_))
        ));
    }

    #[test]
    fn test_negative_raw_values_clamped_to_zero() {
        let comp = ThermoCompensator::new();
        let raw = CorrectedReading {
            pm1_0_ug_m3: -5.0,
            pm2_5_ug_m3: -10.0,
            pm4_0_ug_m3: -1.0,
            pm10_ug_m3: -20.0,
            particle_count_p_cc: -100.0,
        };
        let ambient = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: 50.0,
        };

        let corrected = comp.correct(&raw, &ambient);

        assert_eq!(corrected.pm1_0_ug_m3, 0.0);
        assert_eq!(corrected.pm2_5_ug_m3, 0.0);
        assert_eq!(corrected.pm4_0_ug_m3, 0.0);
        assert_eq!(corrected.pm10_ug_m3, 0.0);
        assert_eq!(corrected.particle_count_p_cc, 0.0);
    }

    #[test]
    fn test_valid_ambient_returns_ok() {
        let comp = ThermoCompensator::new();

        let valid = AmbientConditions {
            temperature_c: 25.0,
            relative_humidity_pct: 50.0,
        };
        assert!(comp.validate_ambient(&valid).is_ok());

        // Boundary values should be valid
        let cold_boundary = AmbientConditions {
            temperature_c: -20.0,
            relative_humidity_pct: 0.0,
        };
        assert!(comp.validate_ambient(&cold_boundary).is_ok());

        let hot_boundary = AmbientConditions {
            temperature_c: 60.0,
            relative_humidity_pct: 100.0,
        };
        assert!(comp.validate_ambient(&hot_boundary).is_ok());
    }
}
