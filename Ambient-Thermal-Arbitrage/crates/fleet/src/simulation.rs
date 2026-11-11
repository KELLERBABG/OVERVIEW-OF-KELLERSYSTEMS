// ============================================================================
// German 16-State 2025 weather/price/grid frequency data pipeline
// ============================================================================

use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::f64::consts::PI;
use std::time::Duration;

use crate::bundesland::Bundesland;

/// One hour of simulation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourData {
    pub amb: f64,
    pub price: f64,
    pub freq: f64,
    pub ghi: f64,
    pub is_night: bool,
}

/// Full 8760-hour dataset.
pub struct SimData {
    pub hours: Vec<HourData>,
    #[allow(dead_code)]
    pub bundesland: Bundesland,
}

/// Load or generate the dataset for the selected Bundesland. Tries real API → cache → synthetic.
pub fn get_sim_data(state: Bundesland) -> SimData {
    let state_path = format!("data_cache/fleet_2025_{}.json", state.code());
    let legacy_path = "data_cache/fleet_2025.json";

    // Try state-specific cache first
    let path = if Path::new(&state_path).exists() {
        &state_path
    } else if state == Bundesland::NordrheinWestfalen && Path::new(legacy_path).exists() {
        legacy_path
    } else {
        &state_path
    };

    if Path::new(path).exists() {
        if let Ok(j) = fs::read_to_string(path) {
            if let Ok(d) = serde_json::from_str::<CachedData>(&j) {
                if d.hours.len() >= 8760 {
                    return SimData { hours: d.hours, bundesland: state };
                }
            }
        }
    }

    // Try real API
    log::info!("🌐 Fetching {} ({}) 2025 data...", state.name(), state.code());
    let temps = fetch_temps(state);
    let prices = fetch_prices();

    let mut hours = Vec::with_capacity(8760);
    for h in 0..8760 {
        let amb = temps.get(&(h as u32)).copied().unwrap_or_else(|| synth_temp(h, state));
        let price = prices.get(&(h as u32)).copied().unwrap_or_else(|| synth_price(h, amb));
        let freq = grid_frequency(h);
        let ghi = if amb > 0.0 { estimate_ghi(h, amb, state.lat()) } else { 0.0 };
        hours.push(HourData { amb, price, freq, ghi, is_night: ghi < 10.0 });
    }

    // Save cache
    if let Some(p) = Path::new(&state_path).parent() { let _ = fs::create_dir_all(p); }
    if let Ok(j) = serde_json::to_string(&CachedData { hours: hours.clone() }) {
        let _ = fs::write(&state_path, &j);
    }

    SimData { hours, bundesland: state }
}

#[derive(Serialize, Deserialize)]
struct CachedData { hours: Vec<HourData> }

// ============================================================================
// WEATHER: Open-Meteo API + synthetic fallback
// ============================================================================

fn fetch_temps(state: Bundesland) -> HashMap<u32, f64> {
    let url = format!(
        "https://archive-api.open-meteo.com/v1/archive?latitude={:.2}&longitude={:.2}&start_date=2025-01-01&end_date=2025-12-31&hourly=temperature_2m&timezone=Europe/Berlin",
        state.lat(),
        state.lon()
    );
    match fetch_json::<serde_json::Value>(&url) {
        Ok(v) => {
            let mut m = HashMap::new();
            if let Some(times) = v["hourly"]["time"].as_array() {
                for i in 0..times.len() {
                    if let Ok(ts) = NaiveDateTime::parse_from_str(times[i].as_str().unwrap_or(""), "%Y-%m-%dT%H:%M") {
                        let hoy = (ts.ordinal() - 1) * 24 + ts.hour();
                        if let Some(temp) = v["hourly"]["temperature_2m"][i].as_f64() {
                            m.insert(hoy, temp);
                        }
                    }
                }
            }
            log::info!("🌤️  Open-Meteo weather for {} ({}): {} hours", state.name(), state.code(), m.len());
            m
        }
        Err(e) => { log::warn!("⚠️ Weather API failed for {}: {}", state.code(), e); HashMap::new() }
    }
}

// ============================================================================
// PRICES: energy-charts.info API + synthetic fallback
// ============================================================================

fn fetch_prices() -> HashMap<u32, f64> {
    let url = "https://api.energy-charts.info/price?country=de&start=1735686000&end=1767225599";
    match fetch_json::<serde_json::Value>(url) {
        Ok(v) => {
            let mut m = HashMap::new();
            if let Some(price_arr) = v.get("price").and_then(|x| x.as_array()) {
                for series in price_arr {
                    if let Some(data) = series.get("data").and_then(|x| x.as_array()) {
                        for pt in data {
                            if let (Some(pr), Some(ts)) = (
                                pt.get("price").and_then(|x| x.as_f64()),
                                pt.get("unix_seconds").and_then(|x| x.as_i64()),
                            ) {
                                if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                                    let ndt = dt.naive_utc();
                                    let hoy = (ndt.ordinal() - 1) * 24 + ndt.hour();
                                    m.insert(hoy, pr);
                                }
                            }
                        }
                    }
                }
            }
            log::info!("💶 energy-charts prices: {} hours", m.len());
            m
        }
        Err(e) => { log::warn!("⚠️ Price API failed: {}", e); HashMap::new() }
    }
}

fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }
    serde_json::from_str(&resp.text().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

// ============================================================================
// SYNTHETIC FALLBACK
// ============================================================================

fn synth_temp(h: usize, state: Bundesland) -> f64 {
    let day = (h / 24) as f64 + 1.0;
    let hour = (h % 24) as f64;
    let monthly = state.monthly_temps();
    let t = (day - 15.0) / 30.44 + 0.5;
    let idx = t.floor() as i32;
    let f = t.fract();
    let g = |i: i32| monthly[i.rem_euclid(12) as usize];
    let (p0, p1, p2, p3) = (g(idx - 1), g(idx), g(idx + 1), g(idx + 2));
    let base = 0.5 * (2.0 * p1 + (-p0 + p2) * f + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * f * f * f);
    let diurnal = 5.0 * (PI * (hour - 6.0) / 14.0).sin();
    let ar_noise = 0.8 * ((h as f64 * 0.03).sin() * (h as f64 * 0.07).cos());
    base + 0.8 + diurnal + ar_noise
}

fn synth_price(h: usize, t: f64) -> f64 {
    let hod = (h % 24) as f64;
    let mut p = 52.0;
    if (8.0..20.0).contains(&hod) {
        p += 28.0;
        if hod < 10.0 { p += (10.0 - hod) * 3.0; }
        if hod > 17.0 { p += (hod - 17.0) * 5.0; }
    } else {
        p -= 18.0;
        if hod < 5.0 { p -= 12.0; }
    }
    let month = (h / 720) % 12 + 1;
    if month == 12 || month == 1 || month == 2 {
        let dark = ((h as f64 / 24.0 * 1.618 * 0.03).sin().max(0.0)).powi(5);
        p += dark * 120.0;
    }
    if t > 25.0 { p += (t - 25.0) * 2.0; }
    if t < -5.0 { p += (-5.0 - t) * 3.0; }
    p *= 1.0 + 0.25 * ((h as f64 * 0.03).sin() + (h as f64 * 0.07 + 1.0).sin());
    // Weekend effect
    let day = h / 24;
    let weekday = (day as i32 + 3) % 7; // 2025-01-01 = Wednesday
    if weekday >= 5 { p -= 18.0; }
    p.clamp(-15.0, 350.0)
}

fn estimate_ghi(h: usize, _amb: f64, lat_deg: f64) -> f64 {
    let hour = (h % 24) as f64;
    let lat_rad = lat_deg * PI / 180.0;
    let day = (h / 24) as f64 + 1.0;
    let declination = 23.45 * PI / 180.0 * (2.0 * PI * (day - 81.0) / 365.0).sin();
    let ha = 15.0 * (hour - 12.0) * PI / 180.0;
    let sin_elev = lat_rad.sin() * declination.sin() + lat_rad.cos() * declination.cos() * ha.cos();
    if sin_elev <= 0.0 { return 0.0; }
    let ghi_clear = 1367.0 * sin_elev * (-0.8662 * 3.5 / (sin_elev + 0.15 * (sin_elev * 180.0 / PI + 3.885).powf(-1.253))).exp();
    let cloud = 0.3 + 0.7 * ((h as f64 * 0.05).sin() * 0.5 + 0.5);
    (ghi_clear * (1.0 - cloud * 0.6)).max(0.0)
}

// ============================================================================
// GRID FREQUENCY (ATA.canvas Node 106)
// ============================================================================

fn grid_frequency(h: usize) -> f64 {
    let hour = (h % 24) as f64;
    let mut freq = 50.0;
    // Diurnal load pattern
    freq += 0.03 * ((hour - 14.0) * PI / 12.0).cos();
    // AR(1) noise
    let hf = h as f64;
    let noise = (hf * 1.13).sin() * (hf * 0.73 + 1.0).cos() * 0.03
        + 0.85 * (hf * 1.13 + 1.0).sin() * (hf * 0.73 + 2.0).cos() * 0.025
        + 0.7225 * (hf * 1.13 + 2.0).sin() * (hf * 0.73 + 3.0).cos() * 0.02;
    freq += noise * 0.02;
    // Occasional imbalance events
    let seed = ((hf * 1.618).sin() * (hf * 0.317).cos()).abs();
    if seed > 0.97 {
        let mag = (seed - 0.97) / 0.03 * 0.15;
        let sign = (hf * std::f64::consts::E).sin().signum();
        freq += sign * mag;
    }
    freq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_data_is_plausible() {
        for state in Bundesland::ALL {
            for h in [0usize, 100, 2000, 5000, 8000, 8759] {
                let temp = synth_temp(h, state);
                assert!(temp.is_finite() && temp > -30.0 && temp < 45.0, "temp {temp} in {} at h={h}", state.code());
                let price = synth_price(h, temp);
                assert!(price.is_finite() && price > -500.0 && price < 2000.0, "price {price} at h={h}");
                let f = grid_frequency(h);
                assert!(f.is_finite() && (49.8..=50.2).contains(&f), "freq {f} at h={h}");
                let ghi = estimate_ghi(h, temp, state.lat());
                assert!(ghi.is_finite() && (0.0..=1100.0).contains(&ghi), "ghi {ghi} in {} at h={h}", state.code());
            }
        }
    }

    #[test]
    fn synthetic_year_has_diurnal_solar_variation() {
        let night = estimate_ghi(0, 5.0, Bundesland::NordrheinWestfalen.lat()); // 00:00, Jan 1 → sun below horizon
        assert_eq!(night, 0.0, "night GHI must be zero");
        let summer_noon = estimate_ghi(24 * 30 * 6 + 12, 20.0, Bundesland::Bayern.lat()); // ~July noon
        assert!(summer_noon > 0.0, "summer noon GHI must be positive");
    }

    #[test]
    fn full_year_synthesizes_valid_hour_data() {
        let state = Bundesland::Bayern;
        let mut hours = Vec::with_capacity(8760);
        for h in 0..8760 {
            let amb = synth_temp(h, state);
            let price = synth_price(h, amb);
            let freq = grid_frequency(h);
            let ghi = estimate_ghi(h, amb, state.lat());
            hours.push(HourData { amb, price, freq, ghi, is_night: ghi < 10.0 });
        }
        assert_eq!(hours.len(), 8760);
        assert!(hours.iter().any(|d| d.is_night), "year must contain nights");
        assert!(hours.iter().any(|d| !d.is_night), "year must contain days");
        let avg_price: f64 = hours.iter().map(|d| d.price).sum::<f64>() / 8760.0;
        assert!((20.0..=120.0).contains(&avg_price), "avg price {avg_price:.1} €/MWh implausible");
    }
}