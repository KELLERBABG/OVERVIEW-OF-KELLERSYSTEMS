//! Real-world data ingestion for the thermal arbitrage simulation.
//!
//! Fetches historical weather and electricity price data from free public APIs:
//!   - **Weather**: Open-Meteo Archive API (no API key required)
//!   - **Prices**: energy-charts.info (German electricity market data, no API key required)
//!
//! Data is cached locally as JSON files so the simulation works offline after
//! the first fetch. If no internet is available and no cache exists, falls back
//! to the synthetic model in `simulation_data`.
//!
//! NOTE: All fetch operations use `tokio::task::spawn_blocking` to safely run
//! blocking HTTP calls within the Tokio async runtime.

use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Cached dataset for a full year (8760 hours).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealDataset {
    pub hours: Vec<RealHourRecord>,
    pub fetched_at: String,
    pub sources: Vec<String>,
}

/// A single hour of real-world data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealHourRecord {
    pub timestamp: String,
    pub temperature_c: Option<f64>,
    pub solar_ghi_wm2: Option<f64>,
    pub cloud_cover_pct: Option<f64>,
    pub price_eur_per_mwh: Option<f64>,
}

/// Cache file path.
const CACHE_FILE: &str = "data_cache/2025_dataset.json";

/// Fetch or load cached real-world data.
pub async fn get_real_dataset_async() -> Option<RealDataset> {
    if let Some(cached) = load_cache() {
        log::info!("📦 Loaded cached real-world dataset ({} hours)", cached.hours.len());
        return Some(cached);
    }

    log::info!("🌐 No cache found. Fetching real-world data from APIs...");

    let (weather, prices) = tokio::task::spawn_blocking(|| {
        let weather = fetch_weather_openmeteo();
        let prices = fetch_prices();
        (weather, prices)
    })
    .await
    .unwrap_or_default();

    let dataset = merge_datasets(weather, prices);
    save_cache(&dataset);
    Some(dataset)
}

// ─── Open-Meteo Weather API ─────────────────────────────────────────────────
// Docs: https://open-meteo.com/en/docs/historical-weather-api
// Free, no API key, 10k req/day limit.
// Using `shortwave_radiation` which is the best GHI estimate in W/m².
// `diffuse_radiation` and `direct_normal_irradiance` also available if needed.

const OPEN_METEO_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

fn fetch_weather_openmeteo() -> HashMap<u32, (f64, f64, f64)> {
    let url = format!(
        "{}?latitude=51.45&longitude=7.01&start_date=2025-01-01&end_date=2025-12-31\
         &hourly=temperature_2m,shortwave_radiation,cloud_cover\
         &timezone=Europe/Berlin",
        OPEN_METEO_URL
    );

    match fetch_json::<OpenMeteoResponse>(&url) {
        Ok(resp) => {
            let n = resp.hourly.time.len();
            log::info!("🌤️  Open-Meteo returned {} weather records", n);

            let mut result = HashMap::new();
            for i in 0..n {
                if let Some(ts) = parse_openmeteo_time(&resp.hourly.time[i]) {
                    let h_o_y = hour_of_year(ts);
                    let temp = resp.hourly.temperature_2m[i];
                    let ghi = resp.hourly.shortwave_radiation[i]
                        .map(|v| v.max(0.0))
                        .unwrap_or(0.0);
                    let cloud = resp.hourly.cloud_cover[i]
                        .map(|v| v.clamp(0.0, 100.0))
                        .unwrap_or(50.0);
                    result.insert(h_o_y, (temp, ghi, cloud));
                }
            }
            log::info!("🌤️  Parsed {} weather hours from Open-Meteo", result.len());
            result
        }
        Err(e) => {
            log::warn!("⚠️  Open-Meteo fetch failed: {}. Using synthetic weather.", e);
            HashMap::new()
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    hourly: OpenMeteoHourly,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    #[serde(default)]
    shortwave_radiation: Vec<Option<f64>>,
    #[serde(default)]
    cloud_cover: Vec<Option<f64>>,
}

fn parse_openmeteo_time(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").ok()
}

// ─── Electricity Price API ─────────────────────────────────────────────────
// Primary: energy-charts.info API (https://api.energy-charts.info/)
//   Endpoint: /price?country=de&year=2025
//   Returns day-ahead prices for Germany in €/MWh
//   Free, no API key required, well-documented.
//
// Fallback: SMARD direct chart data from smard.de
//   Endpoint: /app/chart_data/8000269/8000269_hours_<epoch_ms>.json
//   Filter 8000269 = day-ahead price DE/AT/LU

fn fetch_prices() -> HashMap<u32, f64> {
    // Primary: energy-charts.info
    // Docs: https://api.energy-charts.info/
    // Uses start/end date range to get full year 2025
    let ec_url = "https://api.energy-charts.info/price?country=de&start=2025-01-01T00:00:00+01:00&end=2025-12-31T23:00:00+01:00";
    match fetch_json::<EnergyChartsResponse>(ec_url) {
        Ok(resp) => {
            let mut all_prices = HashMap::new();
            for series in &resp.price {
                for point in &series.data {
                    if let Some(price) = point.get("price").and_then(|v| v.as_f64()) {
                        if let Some(ts) = point.get("unix_seconds").and_then(|v| v.as_i64()) {
                            if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                                let h_o_y = hour_of_year(dt.naive_utc());
                                all_prices.insert(h_o_y, price);
                            }
                        }
                    }
                }
            }
            log::info!("💶 energy-charts.info returned {} price records", all_prices.len());
            all_prices
        }
        Err(e) => {
            log::warn!("⚠️  energy-charts.info failed: {}. Trying SMARD direct...", e);
            fetch_prices_smard_direct()
        }
    }
}

fn fetch_prices_smard_direct() -> HashMap<u32, f64> {
    let mut all_prices = HashMap::new();
    let jan_1_2025_ms = 1735686000000i64;
    let url = format!(
        "{}/8000269_hours_{}.json",
        "https://www.smard.de/app/chart_data/8000269",
        jan_1_2025_ms
    );

    match fetch_json::<SmardResponse>(&url) {
        Ok(resp) => {
            if let Some(series) = resp.series {
                for s in &series {
                    if s.timestamps.len() != s.values.len() {
                        continue;
                    }
                    for i in 0..s.timestamps.len() {
                        if let Some(price) = s.values[i] {
                            let ts_ms = s.timestamps[i];
                            if let Some(dt) = chrono::DateTime::from_timestamp(ts_ms / 1000, 0) {
                                let h_o_y = hour_of_year(dt.naive_utc());
                                all_prices.insert(h_o_y, price);
                            }
                        }
                    }
                }
            }
            log::info!("💶 SMARD direct returned {} price records", all_prices.len());
        }
        Err(e) => {
            log::warn!("⚠️  SMARD direct failed: {}", e);
        }
    }
    all_prices
}

#[derive(Debug, Deserialize)]
struct SmardResponse {
    #[serde(default)]
    series: Option<Vec<SmardSeries>>,
}

#[derive(Debug, Deserialize)]
struct SmardSeries {
    #[serde(default)]
    timestamps: Vec<i64>,
    #[serde(default)]
    values: Vec<Option<f64>>,
}

use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize)]
struct EnergyChartsResponse {
    #[serde(default)]
    price: Vec<EnergyChartsSeries>,
}

#[derive(Debug, Deserialize)]
struct EnergyChartsSeries {
    #[serde(default)]
    data: Vec<HashMap<String, JsonValue>>,
}

// ─── Merge Datasets ─────────────────────────────────────────────────────────

fn merge_datasets(
    weather: HashMap<u32, (f64, f64, f64)>,
    prices: HashMap<u32, f64>,
) -> RealDataset {
    let mut hours = Vec::with_capacity(8760);

    for hoy in 0..8760 {
        let (temp, ghi, cloud) = weather.get(&hoy).copied().unwrap_or((f64::NAN, f64::NAN, f64::NAN));
        let price = prices.get(&hoy).copied().unwrap_or(f64::NAN);

        let ts = hour_to_timestamp(hoy);
        let ts_str = ts.format("%Y-%m-%dT%H:%M:%S+00:00").to_string();

        hours.push(RealHourRecord {
            timestamp: ts_str,
            temperature_c: if temp.is_finite() { Some(temp) } else { None },
            solar_ghi_wm2: if ghi.is_finite() { Some(ghi) } else { None },
            cloud_cover_pct: if cloud.is_finite() { Some(cloud) } else { None },
            price_eur_per_mwh: if price.is_finite() { Some(price) } else { None },
        });
    }

    let filled_temp = hours.iter().filter(|h| h.temperature_c.is_some()).count();
    let filled_solar = hours.iter().filter(|h| h.solar_ghi_wm2.is_some()).count();
    let filled_price = hours.iter().filter(|h| h.price_eur_per_mwh.is_some()).count();

    log::info!(
        "📊 Dataset merge: {}/{} temp, {}/{} solar, {}/{} price",
        filled_temp, 8760, filled_solar, 8760, filled_price, 8760
    );

    let mut sources = Vec::new();
    if filled_temp > 0 { sources.push("Open-Meteo (weather)".into()); }
    if filled_price > 0 { sources.push("energy-charts/SMARD (prices)".into()); }

    RealDataset {
        hours,
        fetched_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        sources,
    }
}

// ─── Cache Layer ────────────────────────────────────────────────────────────

fn cache_path() -> String {
    CACHE_FILE.to_string()
}

fn load_cache() -> Option<RealDataset> {
    let path = cache_path();
    if !Path::new(&path).exists() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(json) => match serde_json::from_str::<RealDataset>(&json) {
            Ok(ds) => {
                let has_weather = ds.hours.iter().any(|h| h.temperature_c.is_some());
                let has_prices = ds.hours.iter().any(|h| h.price_eur_per_mwh.is_some());
                if ds.hours.len() >= 8760 && (has_weather || has_prices) {
                    Some(ds)
                } else {
                    log::warn!("⚠️  Cache file exists but has insufficient data, refetching...");
                    None
                }
            }
            Err(e) => {
                log::warn!("⚠️  Cache file corrupt: {}. Refetching...", e);
                None
            }
        },
        Err(e) => {
            log::warn!("⚠️  Cache read error: {}. Refetching...", e);
            None
        }
    }
}

fn save_cache(dataset: &RealDataset) {
    let path = cache_path();
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(dataset) {
        Ok(json) => match fs::write(&path, &json) {
            Ok(_) => log::info!("📦 Cached dataset saved to {}", path),
            Err(e) => log::warn!("⚠️  Could not write cache: {}", e),
        },
        Err(e) => log::warn!("⚠️  Could not serialize cache: {}", e),
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn hour_of_year(dt: NaiveDateTime) -> u32 {
    let day_of_year = dt.ordinal() - 1;
    day_of_year * 24 + dt.hour()
}

fn hour_to_timestamp(hour_of_year: u32) -> NaiveDateTime {
    let day = hour_of_year / 24;
    let hour = hour_of_year % 24;
    NaiveDate::from_ymd_opt(2025, 1, 1)
        .unwrap()
        .and_hms_opt(hour, 0, 0)
        .unwrap()
        + chrono::Duration::days(day as i64)
}

fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    log::debug!("🌐 Fetching: {}", url);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("AmbientThermalArbitrage/1.0")
        .build()
        .map_err(|e| format!("HTTP client build error: {}", e))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} from {}", resp.status(), url));
    }

    let body = resp
        .text()
        .map_err(|e| format!("Response body error: {}", e))?;

    serde_json::from_str::<T>(&body)
        .map_err(|e| format!("JSON parse error: {} (body preview: {})", e, &body[..200.min(body.len())]))
}