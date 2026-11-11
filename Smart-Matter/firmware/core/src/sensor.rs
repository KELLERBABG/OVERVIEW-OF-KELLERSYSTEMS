//! # Environmental Signature Sampling
//!
//! Sensor interface for vibration monitoring, environmental sensing, and
//! entropy generation from physical noise sources.
//!
//! Uses onboard ADC noise + accelerometer data as local entropy source for
//! cryptographic nonce generation.

use crate::types::{EntropySample, VibrationSample};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default accelerometer sampling rate (Hz).
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 100;

/// Number of samples for entropy extraction.
pub const ENTROPY_POOL_SIZE: usize = 64;

/// Accelerometer full-scale range (±g).
pub const ACCEL_RANGE_G: f32 = 2.0;

// ---------------------------------------------------------------------------
// Sensor Traits
// ---------------------------------------------------------------------------

/// Accelerometer interface.
pub trait Accelerometer {
    /// Read a vibration sample (x, y, z in milli-g).
    fn read_sample(&mut self) -> Result<VibrationSample, SensorError>;

    /// Configure sampling rate.
    fn set_rate(&mut self, hz: u32) -> Result<(), SensorError>;

    /// Get the noise floor estimate (μg/√Hz).
    fn noise_floor(&self) -> f32;
}

/// ADC interface for entropy sampling.
pub trait AdcNoiseSource {
    /// Read a raw ADC sample from a floating/differential input.
    fn read_raw(&mut self) -> Result<u32, SensorError>;
}

/// Temperature sensor interface.
pub trait TemperatureSensor {
    /// Read temperature in millidegrees Celsius.
    fn read_millicelsius(&mut self) -> Result<i32, SensorError>;
}

// ---------------------------------------------------------------------------
// Sensor Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorError {
    I2cError,
    SpiError,
    NotReady,
    Timeout,
    InvalidReading,
}

// ---------------------------------------------------------------------------
// Entropy Pool
// ---------------------------------------------------------------------------

/// Accumulates entropy from environmental noise sources.
#[derive(Debug, Clone)]
pub struct EntropyPool {
    /// Raw ADC noise samples.
    adc_samples: heapless::Vec<u32, ENTROPY_POOL_SIZE>,
    /// Vibration noise floor samples.
    vibration_samples: heapless::Vec<u16, ENTROPY_POOL_SIZE>,
    /// Temperature samples.
    temp_samples: heapless::Vec<i32, ENTROPY_POOL_SIZE>,
}

impl Default for EntropyPool {
    fn default() -> Self {
        Self::new()
    }
}

impl EntropyPool {
    pub fn new() -> Self {
        Self {
            adc_samples: heapless::Vec::new(),
            vibration_samples: heapless::Vec::new(),
            temp_samples: heapless::Vec::new(),
        }
    }

    /// Add an ADC noise sample.
    pub fn add_adc_sample(&mut self, sample: u32) {
        if self.adc_samples.len() >= ENTROPY_POOL_SIZE {
            self.adc_samples.remove(0);
        }
        self.adc_samples.push(sample).ok();
    }

    /// Add a vibration noise floor sample.
    pub fn add_vibration_sample(&mut self, sample: u16) {
        if self.vibration_samples.len() >= ENTROPY_POOL_SIZE {
            self.vibration_samples.remove(0);
        }
        self.vibration_samples.push(sample).ok();
    }

    /// Add a temperature sample.
    pub fn add_temp_sample(&mut self, sample: i32) {
        if self.temp_samples.len() >= ENTROPY_POOL_SIZE {
            self.temp_samples.remove(0);
        }
        self.temp_samples.push(sample).ok();
    }

    /// Extract entropy bytes for nonce generation.
    ///
    /// Uses a simple hash of the pooled samples to produce pseudo-entropy.
    pub fn extract_entropy(&self) -> [u8; 12] {
        let mut state: u64 = 0xDEADBEEF_CAFEBABE;

        for &s in self.adc_samples.iter() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(s as u64);
        }
        for &s in self.vibration_samples.iter() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(s as u64);
        }
        for &s in self.temp_samples.iter() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(s as u64);
        }

        let mut entropy = [0u8; 12];
        entropy[0..8].copy_from_slice(&state.to_le_bytes());
        entropy[8..12].copy_from_slice(&(state >> 32).to_le_bytes()[..4]);
        entropy
    }

    /// Build an entropy sample for reporting.
    pub fn to_sample(&self) -> EntropySample {
        let adc_avg = if !self.adc_samples.is_empty() {
            self.adc_samples.iter().copied().sum::<u32>() / self.adc_samples.len() as u32
        } else {
            0
        };
        let vib_avg = if !self.vibration_samples.is_empty() {
            self.vibration_samples.iter().copied().sum::<u16>()
                / self.vibration_samples.len() as u16
        } else {
            0
        };
        let temp_avg = if !self.temp_samples.is_empty() {
            self.temp_samples.iter().copied().sum::<i32>() / self.temp_samples.len() as i32
        } else {
            0
        };

        EntropySample {
            adc_noise: adc_avg,
            vibration_floor: vib_avg,
            temperature_mc: temp_avg,
        }
    }

    /// Number of ADC samples in the pool.
    pub fn adc_sample_count(&self) -> usize {
        self.adc_samples.len()
    }
}

// ---------------------------------------------------------------------------
// Vibration Analyzer
// ---------------------------------------------------------------------------

/// Analyzes vibration patterns for entropy and baseline monitoring.
#[derive(Debug, Clone)]
pub struct VibrationAnalyzer {
    baseline: Option<VibrationSample>,
    samples_taken: u32,
}

impl Default for VibrationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl VibrationAnalyzer {
    pub fn new() -> Self {
        Self {
            baseline: None,
            samples_taken: 0,
        }
    }

    /// Process a vibration sample and update baseline.
    pub fn process_sample(&mut self, sample: &VibrationSample) {
        self.samples_taken += 1;

        // Establish baseline after enough samples
        if self.samples_taken >= 100 {
            if let Some(ref baseline) = self.baseline {
                // Exponentially weighted moving average
                let alpha = 0.1f32;
                let new_x = baseline.x as f32 * (1.0 - alpha) + sample.x as f32 * alpha;
                let new_y = baseline.y as f32 * (1.0 - alpha) + sample.y as f32 * alpha;
                let new_z = baseline.z as f32 * (1.0 - alpha) + sample.z as f32 * alpha;

                self.baseline = Some(VibrationSample {
                    x: new_x as i16,
                    y: new_y as i16,
                    z: new_z as i16,
                    time: sample.time,
                });
            } else {
                self.baseline = Some(*sample);
            }
        }
    }

    /// Get the current baseline vibration.
    pub fn baseline(&self) -> Option<&VibrationSample> {
        self.baseline.as_ref()
    }

    /// Calculate deviation from baseline as percentage.
    pub fn deviation_pct(&self, current: &VibrationSample) -> u8 {
        match self.baseline {
            Some(ref base) => {
                let dx = (current.x as i32 - base.x as i32).unsigned_abs();
                let dy = (current.y as i32 - base.y as i32).unsigned_abs();
                let dz = (current.z as i32 - base.z as i32).unsigned_abs();
                let mag = libm::sqrtf((dx * dx + dy * dy + dz * dz) as f32);
                let base_mag_sq = (base.x as i32).unsigned_abs().pow(2)
                    + (base.y as i32).unsigned_abs().pow(2)
                    + (base.z as i32).unsigned_abs().pow(2);
                if base_mag_sq > 0 {
                    ((mag / libm::sqrtf(base_mag_sq as f32)) * 100.0) as u8
                } else {
                    // If baseline is zero, any non-zero sample is 100% deviation
                    if mag > 0.0 {
                        100
                    } else {
                        0
                    }
                }
            }
            None => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TimeMs;

    #[test]
    fn test_entropy_pool() {
        let mut pool = EntropyPool::new();
        assert_eq!(pool.adc_sample_count(), 0);

        pool.add_adc_sample(0xDEAD);
        pool.add_adc_sample(0xBEEF);
        assert_eq!(pool.adc_sample_count(), 2);

        let entropy = pool.extract_entropy();
        assert_eq!(entropy.len(), 12);
    }

    #[test]
    fn test_entropy_determinism() {
        let mut pool1 = EntropyPool::new();
        let mut pool2 = EntropyPool::new();

        pool1.add_adc_sample(42);
        pool1.add_vibration_sample(100);
        pool2.add_adc_sample(42);
        pool2.add_vibration_sample(100);

        assert_eq!(pool1.extract_entropy(), pool2.extract_entropy());
    }

    #[test]
    fn test_entropy_with_different_inputs() {
        let mut pool1 = EntropyPool::new();
        let mut pool2 = EntropyPool::new();

        pool1.add_adc_sample(42);
        pool2.add_adc_sample(43); // Different

        assert_ne!(pool1.extract_entropy(), pool2.extract_entropy());
    }

    #[test]
    fn test_vibration_baseline() {
        let mut analyzer = VibrationAnalyzer::new();
        assert!(analyzer.baseline().is_none());

        // Add 100 samples to establish baseline
        for i in 0..100 {
            let sample = VibrationSample {
                x: 10 + (i % 5) as i16,
                y: 20 + (i % 3) as i16,
                z: 30,
                time: TimeMs(i as u64),
            };
            analyzer.process_sample(&sample);
        }

        assert!(analyzer.baseline().is_some());
    }

    #[test]
    fn test_deviation() {
        let mut analyzer = VibrationAnalyzer::new();

        // Establish baseline at (0, 0, 0)
        for _ in 0..100 {
            analyzer.process_sample(&VibrationSample {
                x: 0,
                y: 0,
                z: 0,
                time: TimeMs(0),
            });
        }

        // Current sample with deviation
        let current = VibrationSample {
            x: 10,
            y: 10,
            z: 10,
            time: TimeMs(100),
        };

        let deviation = analyzer.deviation_pct(&current);
        assert!(deviation > 0);
    }
}
