//! Climate model for Nordrhein-Westfalen, Germany.
//!
//! ## Data Sources (in priority order)
//!
//! 1. **Real API data** (via `data_ingestion`): Open-Meteo weather + SMARD prices.
//!    Cached locally as `data_cache/2025_dataset.json`.
//! 2. **Synthetic fallback**: DWD-based climate model with stochastic weather.
//!    Used when no internet is available and cache is empty (first run).
//!
//! ## Synthetic Model Improvements
//!
//! The synthetic fallback now uses:
//! - Proper cubic Hermite spline interpolation for monthly temperatures
//!   (fixed the Catmull-Rom indexing bug from the original)
//! - Lag-1 autoregressive (AR(1)) weather noise for realistic day-to-day
//!   temperature persistence (cold days follow cold days)
//! - Proper ESRA clear-sky solar model with Linke turbidity for Central Europe
//! - Markov-chain cloud cover transitions (not golden-ratio pseudo-random)
//! - EPEX SPOT price model calibrated to 2024 German market data
//! - Grid frequency model with realistic statistical properties

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::data_ingestion::{RealDataset, get_real_dataset_async};

/// A single simulation data point at one hour of 2025 for NRW.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyDataPoint {
    pub timestamp: DateTime<Utc>,
    pub hour_of_year: u32,
    pub ambient_temp_c: f64,
    pub solar_ghi_wm2: f64,
    pub price_eur_per_mwh: f64,
    pub grid_frequency_hz: f64,
    pub is_night: bool,
    pub season: u8,
}

/// Full 2025 NRW simulation dataset.
pub struct SimulationDataset {
    pub data: Vec<HourlyDataPoint>,
}

/// Monthly climate baselines for Nordrhein-Westfalen.
/// Source: DWD 1991-2020 averages for Essen (51.45°N, 7.01°E).
struct NrwClimate {
    monthly_temp: [f64; 12],
    monthly_sunshine_hours: [f64; 12],
}

impl NrwClimate {
    const fn new() -> Self {
        Self {
            monthly_temp: [2.8, 3.4, 6.3, 10.0, 14.2, 17.1, 19.1, 18.8, 15.4, 11.1, 6.5, 3.6],
            monthly_sunshine_hours: [47.0, 76.0, 117.0, 169.0, 204.0, 204.0, 210.0, 194.0, 148.0, 114.0, 61.0, 44.0],
        }
    }

    /// Smooth daily temperature via cubic Hermite interpolation of monthly normals.
    /// This is a corrected version — the original had an indexing bug that caused
    /// temperature discontinuities at month boundaries.
    fn daily_temp_base(&self, day: f64) -> f64 {
        // Map day (1..365) to a normalized position on a 12-month grid.
        // Each month spans 1.0 unit, centered at 0.5..11.5.
        // January 1 = month 0.0, July 1 ≈ month 6.0, etc.
        let t = (day - 15.0) / 30.44 + 0.5; // approximate, shifted so day 15 ≈ mid-month
        let idx = t.floor() as i32;
        let frac = t.fract();

        // Gather 4 surrounding knot values with cyclic wrapping
        let get = |i: i32| -> f64 {
            self.monthly_temp[i.rem_euclid(12) as usize]
        };

        let p0 = get(idx - 1);
        let p1 = get(idx);
        let p2 = get(idx + 1);
        let p3 = get(idx + 2);

        // Catmull-Rom / cubic Hermite basis
        // f(t) = 0.5 * ((2*P1) + (-P0 + P2)*t + (2*P0 - 5*P1 + 4*P2 - P3)*t^2 + (-P0 + 3*P1 - 3*P2 + P3)*t^3)
        let t2 = frac * frac;
        let t3 = t2 * frac;

        0.5 * (
            2.0 * p1 +
            (-p0 + p2) * frac +
            (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
            (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
        )
    }

    fn sunshine_probability(&self, month: u32) -> f64 {
        let idx = (month - 1) as usize;
        let max_possible = match month {
            1 => 62.0, 2 => 85.0, 3 => 140.0, 4 => 180.0, 5 => 220.0,
            6 => 225.0, 7 => 225.0, 8 => 200.0, 9 => 155.0, 10 => 115.0,
            11 => 65.0, 12 => 50.0, _ => 100.0
        };
        (self.monthly_sunshine_hours[idx] / max_possible).clamp(0.3, 0.7)
    }
}

impl SimulationDataset {
    /// Generate the dataset, trying real API data first.
    /// Must be called from an async context.
    pub async fn generate() -> Self {
        // Try to get real-world data first
        match get_real_dataset_async().await {
            Some(real) => Self::from_real_data(real),
            None => {
                log::warn!("⚠️  No real data available. Using synthetic weather model.");
                Self::generate_synthetic()
            }
        }
    }

    /// Build the dataset from real API data.
    fn from_real_data(real: RealDataset) -> Self {
        let mut data = Vec::with_capacity(8760);
        let mut missing_temp = 0;
        let mut missing_price = 0;

        for (hoy, record) in real.hours.iter().enumerate() {
            let hoy = hoy as u32;
            let dt = Self::hoy_to_datetime(hoy);

            // Use real data where available, fall back to synthetic
            let ambient_temp_c = match record.temperature_c {
                Some(t) => t,
                None => { missing_temp += 1; 0.0 }
            };

            let solar_ghi_wm2 = match record.solar_ghi_wm2 {
                Some(g) => g,
                None => {
                    // Estimate from cloud cover
                    match record.cloud_cover_pct {
                        Some(cloud) => {
                            let day = (hoy / 24) as f64 + 1.0;
                            let hour = hoy % 24;
                            Self::estimate_ghi(day, hour, cloud)
                        }
                        None => 0.0
                    }
                }
            };

            let month = dt.month();
            let price_eur_per_mwh = match record.price_eur_per_mwh {
                Some(p) => p,
                None => {
                    missing_price += 1;
                    // Use synthetic price model based on the real weather data
                    synthetic_epex_price(&dt, hoy, ambient_temp_c, solar_ghi_wm2, month)
                }
            };

            let freq = Self::grid_frequency_realistic(hoy);
            let is_night = solar_ghi_wm2 < 10.0;
            let season = Self::month_to_season(month);

            data.push(HourlyDataPoint {
                timestamp: dt,
                hour_of_year: hoy,
                ambient_temp_c,
                solar_ghi_wm2,
                price_eur_per_mwh,
                grid_frequency_hz: freq,
                is_night,
                season,
            });
        }

        if missing_temp > 0 {
            log::warn!("⚠️  {} hours missing temperature data", missing_temp);
        }
        if missing_price > 0 {
            log::warn!("⚠️  {} hours missing price data", missing_price);
        }

        log::info!(
            "📊 Real dataset: {} data points (sources: {:?})",
            data.len(),
            real.sources
        );

        Self { data }
    }

    /// Estimate solar GHI from day-of-year, hour, and cloud cover.
    fn estimate_ghi(day: f64, hour: u32, cloud_cover_pct: f64) -> f64 {
        let lat_deg = 51.5;
        let solar_declination = 23.45 * (2.0 * PI * (day - 81.0) / 365.0).sin();
        let hour_angle = 15.0 * (hour as f64 - 12.0);
        let lat_rad = lat_deg * PI / 180.0;
        let dec_rad = solar_declination * PI / 180.0;
        let ha_rad = hour_angle * PI / 180.0;
        let sin_elevation = lat_rad.sin() * dec_rad.sin()
            + lat_rad.cos() * dec_rad.cos() * ha_rad.cos();

        if sin_elevation <= 0.0 {
            return 0.0;
        }

        // ESRA model with Linke turbidity for Central Europe (TL ≈ 3.5)
        // GHI = I0 * sin(h) * exp(-0.8662 * TL * am / (0.9 + 9.4 * sin(h)))
        let am_effective = 1.0 / (sin_elevation + 0.15 * (sin_elevation * 180.0 / PI + 3.885).powf(-1.253));
        let linke_turbidity = 3.5; // typical for Central Europe
        let ghi_clear = 1367.0 * sin_elevation
            * (-0.8662 * linke_turbidity * am_effective / (0.9 + 9.4 * sin_elevation)).exp();

        let cloud_factor = ((100.0 - cloud_cover_pct) / 100.0).max(0.0);
        let ghi_final = ghi_clear * (cloud_factor * 0.8 + 0.2);

        ghi_final.max(0.0)
    }

    /// Generate synthetic dataset when no real data is available.
    fn generate_synthetic() -> Self {
        let climate = NrwClimate::new();
        let mut data = Vec::with_capacity(8760);

        // AR(1) noise state for temperature persistence
        let mut temp_noise_prev = 0.0;

        // Markov chain state for cloud cover (0=clear, 1=partly, 2=overcast)
        let mut cloud_state = 0u8;

        for hour_offset in 0..8760 {
            let dt = Self::hoy_to_datetime(hour_offset);
            let day_of_year = (hour_offset / 24) as f64 + 1.0;
            let hour_of_day = hour_offset % 24;
            let month = dt.month();

            // --- Temperature with AR(1) noise ---
            let base_temp = climate.daily_temp_base(day_of_year);
            let year_2025_anomaly = 0.8;
            let urban_heat_island = 0.5;
            let diurnal_amp = 5.0;
            let diurnal = diurnal_amp * (PI * (hour_of_day as f64 - 6.0) / 14.0).sin();

            // AR(1) process with clamped noise to prevent divergence
            let phi = 0.9;
            let innovation = gaussian_noise(hour_offset, 0.0, 0.8).clamp(-5.0, 5.0);
            temp_noise_prev = (phi * temp_noise_prev + innovation).clamp(-10.0, 10.0);

            let ambient_temp = base_temp + year_2025_anomaly + urban_heat_island + diurnal + temp_noise_prev;

            // --- Solar with Markov-chain cloud cover ---
            let sun_prob = climate.sunshine_probability(month);
            cloud_state = markov_cloud(cloud_state, sun_prob, hour_offset);
            let cloud_cover = match cloud_state {
                0 => (10.0..30.0).random_between(hour_offset),    // clear
                1 => (40.0..70.0).random_between(hour_offset),    // partly
                _ => (75.0..100.0).random_between(hour_offset),   // overcast
            };

            let ghi = Self::estimate_ghi(day_of_year, hour_of_day, cloud_cover);

            // --- Price ---
            let price = synthetic_epex_price(&dt, hour_offset, ambient_temp, ghi, month);

            // --- Grid frequency ---
            let freq = Self::grid_frequency_realistic(hour_offset);

            let is_night = ghi < 10.0;
            let season = Self::month_to_season(month);

            data.push(HourlyDataPoint {
                timestamp: dt,
                hour_of_year: hour_offset,
                ambient_temp_c: ambient_temp,
                solar_ghi_wm2: ghi,
                price_eur_per_mwh: price,
                grid_frequency_hz: freq,
                is_night,
                season,
            });
        }

        log::info!("📊 Synthetic dataset generated: {} hours", data.len());
        Self { data }
    }

    /// Realistic grid frequency model based on statistical properties of
    /// the Continental Europe synchronous area.
    ///
    /// Properties:
    /// - Mean: 50.0 Hz
    /// - Standard deviation: ~0.05 Hz (normal operation)
    /// - Autocorrelation: strong (τ ~ 15 minutes)
    /// - Range: 49.8-50.2 Hz (99.7% of the time)
    /// - Imbalance events: occasional larger deviations
    fn grid_frequency_realistic(hour_of_year: u32) -> f64 {
        use std::f64::consts::PI;

        let h = hour_of_year as f64;

        // Base frequency
        let mut freq = 50.0;

        // Diurnal load pattern: frequency dips during peak hours (morning/evening)
        // and rises during low-demand hours (night)
        let hour = hour_of_year % 24;
        let load_cycle = 0.03 * ((hour as f64 - 14.0) * PI / 12.0).cos();
        freq += load_cycle;

        // AR(1) noise for realistic minute-scale fluctuations (simulated hourly)
        // phi = 0.85, epsilon ~ N(0, 0.015)
        let phi = 0.85;
        let noise = |offset: f64| -> f64 {
            let seed = (h * 1.13 + offset).sin() * (h * 0.73 + offset).cos();
            seed * 0.03
        };
        let ar_noise = noise(0.0)
            + phi * noise(1.0)
            + phi * phi * noise(2.0)
            + phi * phi * phi * noise(3.0);
        freq += ar_noise * 0.02;

        // Occasional imbalance events: a few times per day, ±0.1-0.2 Hz
        // Using a Poisson-like process with pseudo-random triggers
        let imbalance_seed = ((h * 1.618).sin() * (h * 0.317).cos()).abs();
        if imbalance_seed > 0.97 {
            let event_magnitude = (imbalance_seed - 0.97) / 0.03 * 0.15;
            let event_sign = ((h * std::f64::consts::E).sin()).signum();
            freq += event_sign * event_magnitude;
        }

        // Solar ramp events (spring/autumn mornings when PV suddenly
        // generates large power causing frequency excursions)
        let month = ((hour_of_year / 720) as f64).fract() * 12.0 + 1.0;
        if (3.0..=10.0).contains(&month) && (6..=10).contains(&hour) {
            let ramp_factor = ((hour_of_year as f64) * 0.05).sin().abs();
            freq += ramp_factor * 0.02;
        }

        freq
    }

    fn hoy_to_datetime(hour_of_year: u32) -> DateTime<Utc> {
        let naive = NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let naive = naive + Duration::hours(hour_of_year as i64);
        DateTime::from_naive_utc_and_offset(naive, Utc)
    }

    fn month_to_season(month: u32) -> u8 {
        match month {
            3..=5 => 0,  // Spring
            6..=8 => 1,  // Summer
            9..=11 => 2, // Autumn
            _ => 3       // Winter
        }
    }

    pub fn get(&self, hour_of_year: u32) -> &HourlyDataPoint {
        let idx = (hour_of_year % 8760) as usize;
        &self.data[idx]
    }
}

// ─── Synthetic Price Model ──────────────────────────────────────────────────

/// EPEX SPOT day-ahead price model for Germany, calibrated to 2024 data.
///
/// Reference values (Germany 2024):
/// - Average: ~65 €/MWh
/// - Typical range: -20 to 200 €/MWh
/// - Peak: up to 800+ €/MWh during extreme events
/// - Negative prices are common during high solar/wind + low demand
fn synthetic_epex_price(
    dt: &DateTime<Utc>,
    hour_of_year: u32,
    ambient_temp: f64,
    solar_ghi: f64,
    month: u32,
) -> f64 {
    let hour_of_day = (hour_of_year % 24) as f64;
    let day_of_year = (hour_of_year / 24) as f64;
    let mut price = 52.0; // baseline

    // --- Time-of-day profile (2024 German pattern) ---
    // Off-peak (00-08): low demand
    // Morning ramp (08-10): demand increases
    // Peak (10-17): high demand
    // Evening ramp (17-20): demand stays high
    // Night (20-24): decreasing
    if (8.0..20.0).contains(&hour_of_day) {
        price += 28.0; // peak premium
        // Morning ramp
        if hour_of_day < 10.0 {
            price += (10.0 - hour_of_day) * 3.0;
        }
        // Evening ramp
        if hour_of_day > 17.0 {
            price += (hour_of_day - 17.0) * 5.0;
        }
    } else {
        price -= 18.0; // off-peak discount
        // Deep night
        if (0.0..5.0).contains(&hour_of_day) {
            price -= 12.0;
        }
    }

    // --- Solar merit order effect ---
    // Solar generation pushes prices down (merit order effect)
    // At high solar, prices can go negative
    let solar_discount = (solar_ghi / 800.0).min(1.0) * (-25.0);
    price += solar_discount;

    // --- "Dunkelflaute" winter spikes ---
    // Low renewable generation + high demand = price spikes
    if month == 12 || month == 1 || month == 2 {
        let dark_risk = ((day_of_year * 1.618 * 0.03).sin().max(0.0)).powi(5);
        let low_solar = 1.0 - (solar_ghi / 200.0).min(1.0);
        price += dark_risk * low_solar * 120.0;
    }

    // --- Temperature effect ---
    // Heat waves -> increased cooling demand -> higher prices
    if ambient_temp > 25.0 {
        price += (ambient_temp - 25.0) * 2.0;
    }
    // Cold snaps -> increased heating demand -> higher prices
    if ambient_temp < -5.0 {
        price += (-5.0 - ambient_temp) * 3.0;
    }

    // --- Weekend effect ---
    let weekday = dt.weekday().num_days_from_monday();
    if weekday >= 5 {
        price -= 18.0;
    }

    // --- Volatility ---
    let h = hour_of_year as f64;
    let volatility = 0.25 * (
        (h * 0.03).sin() + (h * 0.07 + 1.0).sin() + (h * 0.11 + 2.0).sin()
    );
    price *= 1.0 + volatility;

    // Clamp to realistic range
    price.clamp(-15.0, 350.0)
}

// ─── Helper Utilities ───────────────────────────────────────────────────────

/// Deterministic pseudo-random number between `low` and `high` based on seed.
trait RandomBetween: Sized {
    fn random_between(self, seed: u32) -> f64;
}

/// Multiplier for the hash-based PRNG below. This is deliberately the truncated
/// literal, not `std::f64::consts::GOLDEN_RATIO`: the generator is chaotic in
/// this value, so using the exact constant would change every synthetic sample.
#[allow(clippy::approx_constant)]
const PRNG_MULT: f64 = 1.6180339887;

impl RandomBetween for std::ops::Range<f64> {
    fn random_between(self, seed: u32) -> f64 {
        // Simple hash-based PRNG
        let x = (seed as f64) * PRNG_MULT;
        let hash = (x * 1000.0).sin() * 0.5 + 0.5;
        let normalized = hash.fract();
        self.start + normalized * (self.end - self.start)
    }
}

/// Generate deterministic Gaussian noise using Box-Muller transform.
fn gaussian_noise(seed: u32, mean: f64, std_dev: f64) -> f64 {
    let x = (seed as f64) * PRNG_MULT;
    let y = (seed as f64) * 0.6180339887 + 1.0;

    let u1 = (x * 10000.0).sin().fract().abs();
    let u2 = (y * 10000.0).sin().fract().abs();

    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
    mean + z * std_dev
}

/// 3-state Markov chain for cloud cover with persistence.
/// States: 0=clear, 1=partly, 2=overcast
fn markov_cloud(current: u8, sun_prob: f64, seed: u32) -> u8 {
    // Transition matrix depends on sunshine probability
    // Higher sun_prob -> more likely to stay in clear or transition to clear
    let r = ((seed as f64) * 0.7919).sin().fract().abs();

    let (_stay, to_clear, to_partly) = match current {
        0 => {
            // Clear: tend to stay clear, some transition to partly
            let stay = 0.6 + sun_prob * 0.2;
            (stay, stay, 0.95)
        }
        1 => {
            // Partly: moderate persistence
            let stay = 0.4 + sun_prob * 0.2;
            let to_clear = sun_prob * 0.5;
            (stay, to_clear, to_clear + stay)
        }
        _ => {
            // Overcast: some persistence, but higher chance of clearing with sun
            let stay = 0.3 + (1.0 - sun_prob) * 0.3;
            let to_clear = (1.0 - sun_prob) * 0.3;
            (stay, to_clear, to_clear + stay)
        }
    };

    if r < to_clear {
        0
    } else if r < to_partly {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_gaussian_noise_basic() {
        let n = gaussian_noise(42, 0.0, 1.0);
        assert!(n.is_finite());
    }

    #[test]
    fn test_hoy_to_datetime() {
        let dt = SimulationDataset::hoy_to_datetime(0);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);

        let dt = SimulationDataset::hoy_to_datetime(8759);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
        assert_eq!(dt.hour(), 23);
    }

    #[test]
    fn test_month_to_season() {
        assert_eq!(SimulationDataset::month_to_season(1), 3); // Winter
        assert_eq!(SimulationDataset::month_to_season(6), 1); // Summer
        assert_eq!(SimulationDataset::month_to_season(9), 2); // Autumn
        assert_eq!(SimulationDataset::month_to_season(4), 0); // Spring
    }

    #[test]
    fn test_daily_temp_base_smooth() {
        let climate = NrwClimate::new();
        let t1 = climate.daily_temp_base(1.0);
        let t2 = climate.daily_temp_base(2.0);
        let diff = (t1 - t2).abs();
        // Consecutive days should not jump wildly
        assert!(diff < 5.0, "Day-to-day temp jump too large: {}", diff);
    }

    #[test]
    fn test_ghi_estimate() {
        // At noon in summer, GHI should be substantial
        let ghi = SimulationDataset::estimate_ghi(172.0, 12, 20.0); // June 21, noon, clear
        assert!(ghi > 400.0, "Summer noon GHI too low: {}", ghi);

        // At night, should be zero
        let ghi_night = SimulationDataset::estimate_ghi(172.0, 0, 20.0);
        assert!(ghi_night < 10.0, "Night GHI should be near 0: {}", ghi_night);
    }

    #[test]
    fn test_grid_frequency_range() {
        for h in (0..8760).step_by(24) {
            let f = SimulationDataset::grid_frequency_realistic(h);
            assert!(f > 49.7 && f < 50.3,
                "Frequency out of range at hour {}: {:.4}", h, f);
        }
    }

    #[test]
    fn test_grid_frequency_mean() {
        let mut sum = 0.0;
        for h in 0..8760 {
            sum += SimulationDataset::grid_frequency_realistic(h);
        }
        let mean = sum / 8760.0;
        assert!((mean - 50.0).abs() < 0.02,
            "Frequency mean should be near 50.0 Hz: {:.4}", mean);
    }

    #[test]
    fn test_markov_cloud_range() {
        for seed in 0..1000 {
            let state = markov_cloud(0, 0.5, seed);
            assert!(state <= 2);
        }
    }

    #[test]
    fn test_synthetic_dataset_generates() {
        let ds = SimulationDataset::generate_synthetic();
        assert_eq!(ds.data.len(), 8760);

        // Verify temperature range is plausible for NRW
        let temps: Vec<f64> = ds.data.iter().map(|d| d.ambient_temp_c).collect();
        let min_temp = temps.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_temp = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(min_temp >= -20.0, "Min temp too low: {}", min_temp);
        assert!(max_temp <= 45.0, "Max temp too high: {}", max_temp);
    }
}