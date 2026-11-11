//! Live market & weather data ingestion (production real-time mode).
//!
//! Continuously refreshes:
//!   - Day-ahead / intraday electricity prices (ENTSO-E → energy-charts.info → SMARD)
//!   - Weather forecast (Open-Meteo), including the hourly series for MPC
//!   - Grid frequency (optional URL source; the recommended path is a Modbus
//!     `grid_frequency_addr` register read by `modbus_client`)
//!
//! With multi-source failover. The latest values (including the full day-ahead
//! price series and hourly temperature series) are exposed via
//! `Arc<tokio::Mutex<LiveState>>` for the control loop to consume.

use crate::config::LiveDataConfig;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Latest live data snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveState {
    /// Current/nearest electricity price in €/MWh.
    pub price_eur_per_mwh: f64,
    /// Full hourly day-ahead price series (€/MWh), newest last. Used to build
    /// a REAL MPC lookahead in live mode (was a flat repeat → confidence 0).
    pub price_series: Vec<f64>,
    /// Hourly ambient temperature forecast (°C), aligned to `price_series` by
    /// index when both are hourly series from the same refresh cycle.
    pub temp_series: Vec<f64>,
    /// When the price was last refreshed (None = never fetched).
    pub price_updated_at: Option<DateTime<Utc>>,
    /// Current ambient temperature in °C.
    pub ambient_temp_c: f64,
    /// Solar GHI in W/m².
    pub solar_ghi_wm2: f64,
    /// Grid frequency in Hz (defaults to 50.0 if unavailable).
    pub grid_frequency_hz: f64,
    /// When the frequency was last refreshed (None = never fetched).
    pub frequency_updated_at: Option<DateTime<Utc>>,
    /// Timestamp of the last successful update.
    pub last_update: Option<String>,
    /// Which data source served the last successful update.
    pub source: String,
    /// Consecutive failures count.
    pub failures: u32,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            price_eur_per_mwh: 50.0,
            price_series: Vec::new(),
            temp_series: Vec::new(),
            price_updated_at: None,
            ambient_temp_c: 20.0,
            solar_ghi_wm2: 0.0,
            grid_frequency_hz: 50.0,
            frequency_updated_at: None,
            last_update: None,
            source: "none".into(),
            failures: 0,
        }
    }
}

impl LiveState {
    /// A day-ahead price is considered STALE when it has not been refreshed for
    /// `max_age_hours`. While stale the control loop suppresses arbitrage and
    /// MPC (thermostat + safety modes keep running).
    pub fn is_price_fresh(&self, max_age_hours: f64) -> bool {
        match self.price_updated_at {
            Some(t) => (Utc::now() - t).num_minutes() as f64 / 60.0 <= max_age_hours,
            None => false,
        }
    }
}

/// Shared state for the live data pipeline.
#[derive(Clone, Default)]
pub struct LiveStateHandle {
    inner: Arc<Mutex<LiveState>>,
}

impl LiveStateHandle {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(LiveState::default())) }
    }

    pub async fn read(&self) -> LiveState {
        self.inner.lock().await.clone()
    }

    /// Single-value price update (e.g. POST /price, cache restore). Clears the
    /// lookahead series — no day-ahead curve is known for this value.
    pub async fn update_price(&self, price: f64) {
        let mut s = self.inner.lock().await;
        s.price_eur_per_mwh = price;
        s.price_series.clear();
        s.price_updated_at = Some(Utc::now());
        s.last_update = Some(Utc::now().to_rfc3339());
        s.source = "live".into();
        s.failures = 0;
    }

    /// Full refresh: current price + the whole day-ahead hourly series.
    pub async fn update_price_full(&self, price: f64, series: Vec<f64>) {
        let mut s = self.inner.lock().await;
        s.price_eur_per_mwh = price;
        s.price_series = series;
        s.price_updated_at = Some(Utc::now());
        s.last_update = Some(Utc::now().to_rfc3339());
        s.source = "live".into();
        s.failures = 0;
    }

    /// Record which price source served the last refresh ("entsoe",
    /// "energy-charts", "smard"). Used by the failover chain and the
    /// `--validate-live` report so the operator can see the real source.
    pub async fn set_price_source(&self, source: &str) {
        let mut s = self.inner.lock().await;
        s.source = source.to_string();
    }

    pub async fn update_weather(&self, ambient_c: f64, solar_ghi_wm2: f64, temp_series: Vec<f64>) {
        let mut s = self.inner.lock().await;
        s.ambient_temp_c = ambient_c;
        s.solar_ghi_wm2 = solar_ghi_wm2;
        s.temp_series = temp_series;
        s.last_update = Some(Utc::now().to_rfc3339());
        if !s.source.contains("weather") {
            s.source = format!("{} + weather(open-meteo)", s.source);
        }
    }

    pub async fn update_frequency(&self, hz: f64) {
        let mut s = self.inner.lock().await;
        s.grid_frequency_hz = hz;
        s.frequency_updated_at = Some(Utc::now());
        if !s.source.contains("frequency") {
            s.source = format!("{} + frequency(live)", s.source);
        }
    }

    pub async fn record_failure(&self) {
        let mut s = self.inner.lock().await;
        s.failures = s.failures.saturating_add(1);
    }
}

#[derive(Debug, Deserialize)]
struct EnergyChartsResponse {
    #[serde(default)]
    price: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoForecast {
    current: Option<OpenMeteoCurrent>,
    #[serde(default)]
    hourly: Option<OpenMeteoHourly>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    #[serde(default)]
    temperature_2m: Option<f64>,
    #[serde(default)]
    shortwave_radiation: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    #[serde(default)]
    temperature_2m: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct FrequencyResponse {
    #[serde(default)]
    frequency_hz: Option<f64>,
}

/// Run the live data loop: refreshes price, weather, and frequency on their own
/// schedules.
pub async fn run_live_data_loop(
    cfg: LiveDataConfig,
    state: LiveStateHandle,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !cfg.enabled {
            return;
        }
        log::info!("Live data loop started");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("AmbientThermalArbitrage/0.2.0")
            .build()
            .unwrap_or_default();

        let price_interval = Duration::from_secs_f64(cfg.price_refresh_sec.max(10.0));
        let weather_interval = Duration::from_secs_f64(cfg.weather_refresh_sec.max(10.0));
        // Frequency moves fast for grid services, but the URL source is a
        // best-effort publisher (the Modbus meter register is the real path).
        let frequency_interval = Duration::from_secs_f64(cfg.price_refresh_sec.max(10.0));
        let mut price_timer = tokio::time::interval(price_interval);
        let mut weather_timer = tokio::time::interval(weather_interval);
        let mut frequency_timer = tokio::time::interval(frequency_interval);

        let entsoe_key = std::env::var("ENTSOE_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| cfg.entsoe_api_key.clone().filter(|k| !k.is_empty()));
        if entsoe_key.is_some() {
            log::info!("📈 Price source #1: ENTSO-E Transparency Platform (configured)");
        }

        // Restore the last-known-good price so a restart doesn't start blind
        // if every price source is briefly unavailable.
        if let Some(last_price) = load_last_price() {
            state.update_price(last_price).await;
            log::info!("💶 Restored last-known price from cache: {:.2} €/MWh", last_price);
        }

        fetch_price_with_failover(&client, &state, entsoe_key.as_deref()).await;
        if let Some((ambient, ghi, series)) = fetch_weather(&client, &cfg).await {
            log::debug!("Weather updated: {:.1}C GHI {:.0} W/m2 ({} hourly pts)", ambient, ghi, series.len());
            state.update_weather(ambient, ghi, series).await;
        }
        if let Some(url) = &cfg.frequency_url {
            if let Some(hz) = fetch_frequency(&client, url).await {
                state.update_frequency(hz).await;
                log::debug!("Frequency updated: {:.3} Hz", hz);
            }
        }

        loop {
            tokio::select! {
                _ = price_timer.tick() => {
                    fetch_price_with_failover(&client, &state, entsoe_key.as_deref()).await;
                }
                _ = weather_timer.tick() => {
                    if let Some((ambient, ghi, series)) = fetch_weather(&client, &cfg).await {
                        state.update_weather(ambient, ghi, series).await;
                    }
                }
                _ = frequency_timer.tick() => {
                    if let Some(url) = &cfg.frequency_url {
                        if let Some(hz) = fetch_frequency(&client, url).await {
                            state.update_frequency(hz).await;
                        }
                    }
                }
            }
        }
    })
}

/// Fetch price with a multi-source failover chain:
///   1. ENTSO-E Transparency Platform (official, most reliable) — if a key is configured
///   2. energy-charts.info (free, no key)
///   3. SMARD (free, no key)
///   4. Last-known-good price cache (data_cache/last_price.json)
///
/// ENTSO-E and energy-charts return the FULL day-ahead hourly series, which
/// feeds the MPC lookahead. SMARD and the cache provide a single value only
/// (the lookahead then falls back to the current price).
async fn fetch_price_with_failover(client: &reqwest::Client, state: &LiveStateHandle, entsoe_key: Option<&str>) {
    if let Some(key) = entsoe_key {
        if let Some((price, series)) = fetch_price_entsoe(client, key).await {
            state.update_price_full(price, series).await;
            state.set_price_source("entsoe").await;
            save_last_price(price);
            return;
        }
        log::warn!("ENTSO-E failed; trying energy-charts fallback...");
    }

    if let Some((price, series)) = fetch_price_energy_charts(client).await {
        state.update_price_full(price, series).await;
        state.set_price_source("energy-charts").await;
        save_last_price(price);
        return;
    }

    log::warn!("energy-charts failed; trying SMARD fallback...");
    if let Some(price) = fetch_price_smard(client).await {
        state.update_price_full(price, Vec::new()).await;
        state.set_price_source("smard").await;
        save_last_price(price);
        return;
    }

    state.record_failure().await;
    let s = state.read().await;
    log::warn!("All price sources failed (failure #{})", s.failures);
    if let Some(cached) = load_last_price() {
        // The LiveState already holds a value, but re-applying the cache here
        // keeps the price fresh even after an extended outage.
        state.update_price(cached).await;
        log::warn!("💶 Falling back to cached price: {:.2} €/MWh", cached);
    }
}

/// Fetch the current German day-ahead price AND its full hourly series from
/// energy-charts.info (price field is €/MWh, hourly).
async fn fetch_price_energy_charts(client: &reqwest::Client) -> Option<(f64, Vec<f64>)> {
    let url = "https://api.energy-charts.info/price?country=de&resolution=hour";
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<EnergyChartsResponse>().await {
                Ok(data) => {
                    let series = data.price;
                    let last = *series.last()?;
                    Some((last, series))
                }
                Err(e) => { log::warn!("energy-charts JSON parse failed: {}", e); None }
            }
        }
        Ok(resp) => { log::warn!("energy-charts HTTP {}", resp.status()); None }
        Err(e) => { log::warn!("energy-charts request failed: {}", e); None }
    }
}

/// Fallback: SMARD day-ahead price (single latest value).
async fn fetch_price_smard(client: &reqwest::Client) -> Option<f64> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let url = format!("https://www.smard.de/app/chart_data/8000269/8000269_hours_{}.json", now_ms);
    #[derive(Deserialize)]
    struct SmardResp {
        #[serde(default)]
        series: Vec<SmardSeries>,
    }
    #[derive(Deserialize)]
    struct SmardSeries {
        #[serde(default)]
        values: Vec<Option<f64>>,
    }
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<SmardResp>().await {
                Ok(data) => {
                    data.series.iter()
                        .flat_map(|s| s.values.iter().rev().flatten().copied().next())
                        .next()
                }
                Err(e) => { log::warn!("SMARD JSON parse failed: {}", e); None }
            }
        }
        Ok(resp) => { log::warn!("SMARD HTTP {}", resp.status()); None }
        Err(e) => { log::warn!("SMARD request failed: {}", e); None }
    }
}

/// Build the ENTSO-E day-ahead price query URL.
///
/// `documentType=A44` (Price Document) covers BOTH day-ahead and intraday
/// prices, so `processType=A01` is mandatory to get only the day-ahead series
/// — without it the response may mix intraday TimeSeries and the "latest
/// price" could be an intraday value (verified against the official API docs:
/// transparencyplatform.zendesk.com, Request Parameters article).
fn entsoe_prices_url(api_key: &str, period_start: &str, period_end: &str) -> String {
    format!(
        "https://web-api.tp.entsoe.eu/api?securityToken={}\
         &documentType=A44&processType=A01&in_Domain=10Y1001A1001A82H&out_Domain=10Y1001A1001A82H\
         &periodStart={}&periodEnd={}",
        api_key, period_start, period_end
    )
}

/// Fetch the latest German day-ahead price AND the full hourly series from the
/// official ENTSO-E Transparency Platform REST API (document type A44,
/// bidding zone DE/LU).
///
/// Free API key: https://transparency.entsoe.eu → "Data via API" registration.
async fn fetch_price_entsoe(client: &reqwest::Client, api_key: &str) -> Option<(f64, Vec<f64>)> {
    // Request today 00:00 → tomorrow 00:00 (day-ahead). The last point of the
    // returned series is the most recent known day-ahead price.
    let now = chrono::Utc::now();
    let start = now.date_naive();
    let end = start + chrono::Days::new(1);
    let period_start = format!("{}0000", start.format("%Y%m%d"));
    let period_end = format!("{}0000", end.format("%Y%m%d"));

    let url = entsoe_prices_url(api_key, &period_start, &period_end);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => { log::warn!("ENTSO-E response read failed: {}", e); return None; }
            };
            let series = parse_entsoe_xml_prices(&text);
            match series.last() {
                Some(_) => Some((*series.last().unwrap(), series)),
                None => {
                    log::warn!("ENTSO-E response contained no usable prices ({} bytes)", text.len());
                    None
                }
            }
        }
        Ok(resp) => {
            // 4xx/5xx — usually a missing/expired key or daily quota exceeded.
            log::warn!("ENTSO-E HTTP {}", resp.status());
            None
        }
        Err(e) => { log::warn!("ENTSO-E request failed: {}", e.without_url()); None }
    }
}

/// Extract the full day-ahead price series (€/MWh) from an ENTSO-E A44 XML
/// document. Each `<Point>` has a `<position>` and a `<price.amount>`; all
/// amounts are returned in document order. Parser is deliberately lightweight
/// (string scanning) because the ENTSO-E XML schema is stable.
fn parse_entsoe_xml_prices(xml: &str) -> Vec<f64> {
    let mut prices: Vec<f64> = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<price.amount>") {
        rest = &rest[start + "<price.amount>".len()..];
        if let Some(end) = rest.find("</price.amount>") {
            let raw = rest[..end].trim();
            if let Ok(v) = raw.parse::<f64>() {
                prices.push(v);
            }
            rest = &rest[end..];
        }
    }
    prices
}

/// Persist the last-known-good price so the gateway can recover from
/// price-source outages across restarts.
fn save_last_price(price: f64) {
    if std::fs::create_dir_all("data_cache").is_ok() {
        let payload = serde_json::json!({
            "price_eur_per_mwh": price,
            "saved_at": chrono::Utc::now().to_rfc3339(),
        });
        let _ = std::fs::write("data_cache/last_price.json", serde_json::to_string(&payload).unwrap_or_default());
    }
}

/// Load the persisted last-known-good price, if any.
fn load_last_price() -> Option<f64> {
    let content = std::fs::read_to_string("data_cache/last_price.json").ok()?;
    let parsed: serde_json::Value = serde_json::from_str(strip_bom(&content)).ok()?;
    parsed.get("price_eur_per_mwh")?.as_f64()
}

/// Strip a leading UTF-8 BOM (`\u{feff}`) — some editors/scripts write one,
/// and `serde_json` rejects it.
fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{feff}').unwrap_or(content)
}

/// Fetch weather forecast from Open-Meteo. Returns
/// `(ambient °C, solar GHI W/m², hourly temp series)` on success, including
/// the hourly series (aligned to the day-ahead price series for MPC).
async fn fetch_weather(client: &reqwest::Client, cfg: &LiveDataConfig) -> Option<(f64, f64, Vec<f64>)> {
    let url = format!(
        "{}?latitude={}&longitude={}&current=temperature_2m,shortwave_radiation&hourly=temperature_2m&forecast_days=1",
        cfg.open_meteo_url, cfg.latitude, cfg.longitude
    );
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<OpenMeteoForecast>().await {
                Ok(data) => {
                    let temp_series = data.hourly.as_ref()
                        .map(|h| h.temperature_2m.clone())
                        .unwrap_or_default();
                    let (ambient, ghi) = match data.current {
                        Some(cur) => (
                            cur.temperature_2m.unwrap_or(20.0),
                            cur.shortwave_radiation.unwrap_or(0.0),
                        ),
                        None => (20.0, 0.0),
                    };
                    Some((ambient, ghi, temp_series))
                }
                Err(e) => { log::warn!("Open-Meteo JSON parse failed: {}", e); None }
            }
        }
        Ok(resp) => { log::warn!("Open-Meteo HTTP {}", resp.status()); None }
        Err(e) => { log::warn!("Open-Meteo request failed: {}", e); None }
    }
}

/// Fetch grid frequency from an operator-provided JSON endpoint.
/// Contract: `{"frequency_hz": 50.01}` (see DEPLOYMENT.md). The recommended
/// production path is a Modbus `grid_frequency_addr` register; this is the
/// URL fallback for operators without a meter register.
async fn fetch_frequency(client: &reqwest::Client, url: &str) -> Option<f64> {
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<FrequencyResponse>().await {
                Ok(data) => match data.frequency_hz {
                    Some(hz) if hz.is_finite() && (47.0..=53.0).contains(&hz) => Some(hz),
                    _ => { log::warn!("Frequency response contained an implausible value"); None }
                },
                Err(e) => { log::warn!("Frequency JSON parse failed: {}", e); None }
            }
        }
        Ok(resp) => { log::warn!("Frequency HTTP {}", resp.status()); None }
        Err(e) => { log::warn!("Frequency request failed: {}", e); None }
    }
}

/// Build the MPC lookahead (price, ambient) for the next `hours` hours.
///
/// In live mode this uses the REAL day-ahead price series fetched from
/// ENTSO-E / energy-charts, paired with the hourly temperature forecast —
/// previously it repeated the current price, which forced `mpc_confidence = 0`
/// and meant MpcPreCool never activated in production. Falls back to a flat
/// lookahead only when no series is available.
pub fn mpc_lookahead_from_live(state: &LiveState, hours: usize) -> Vec<(f64, f64)> {
    if state.price_series.is_empty() {
        return std::iter::repeat_n((state.price_eur_per_mwh, state.ambient_temp_c), hours)
            .collect();
    }
    let last_price = *state.price_series.last().unwrap_or(&state.price_eur_per_mwh);
    (0..hours)
        .map(|i| {
            let price = state.price_series.get(i).copied().unwrap_or(last_price);
            let temp = state.temp_series.get(i).copied().unwrap_or(state.ambient_temp_c);
            (price, temp)
        })
        .collect()
}

// ─── Live-data validation (--validate-live) ─────────────────────────────
//
// One-shot real-world self-test: probes every price/weather/frequency source
// through the SAME fetch + parse code the gateway uses, and reports per-source
// results. Run it with an ENTSO-E key configured to validate the official
// price path end-to-end:
//
//   ENTSOE_API_KEY=... cargo run --release -- --validate-live

/// Result of probing one live-data source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceCheck {
    /// Which kind of data the source provides: "price", "weather", "frequency", "cache".
    pub role: &'static str,
    pub source: &'static str,
    pub ok: bool,
    pub detail: String,
}

/// Report produced by `validate_live_sources`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveValidationReport {
    pub checked_at: String,
    pub entsoe_key_configured: bool,
    pub checks: Vec<SourceCheck>,
    /// Source that won the end-to-end failover chain ("entsoe" / "energy-charts" / "smard").
    pub end_to_end_source: String,
    pub end_to_end_price_eur_per_mwh: f64,
    pub price_fresh: bool,
    pub mpc_series_points: usize,
}

/// Probe every live-data source once, using the real fetch/parse code.
/// The caller (main `--validate-live` mode) decides the exit code from
/// `price_ok` + `weather_ok` (available in `checks` via `role`).
pub async fn validate_live_sources(cfg: &LiveDataConfig) -> LiveValidationReport {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("AmbientThermalArbitrage/0.2.0")
        .build()
        .unwrap_or_default();

    let entsoe_key = std::env::var("ENTSOE_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| cfg.entsoe_api_key.clone().filter(|k| !k.is_empty()));

    let mut checks = Vec::new();

    // 1. ENTSO-E (official day-ahead price, needs a free key)
    match &entsoe_key {
        Some(key) => match fetch_price_entsoe(&client, key).await {
            Some((p, series)) => checks.push(SourceCheck {
                role: "price", source: "ENTSO-E (A44/A01)",
                ok: true, detail: format!("{p:.2} €/MWh, {} day-ahead series points", series.len()),
            }),
            None => checks.push(SourceCheck {
                role: "price", source: "ENTSO-E (A44/A01)",
                ok: false, detail: "request failed — check the key, daily quota, and network".into(),
            }),
        },
        None => checks.push(SourceCheck {
            role: "price", source: "ENTSO-E (A44/A01)",
            ok: false, detail: "no key configured — set ENTSOE_API_KEY to test this source".into(),
        }),
    }

    // 2. energy-charts.info (free, no key — flagged deprecated upstream)
    match fetch_price_energy_charts(&client).await {
        Some((p, series)) => checks.push(SourceCheck {
            role: "price", source: "energy-charts.info",
            ok: true, detail: format!("{p:.2} €/MWh, {} series points (API flagged deprecated upstream)", series.len()),
        }),
        None => checks.push(SourceCheck {
            role: "price", source: "energy-charts.info",
            ok: false, detail: "request failed — API is deprecated and may have been shut down".into(),
        }),
    }

    // 3. SMARD (free, no key)
    match fetch_price_smard(&client).await {
        Some(p) => checks.push(SourceCheck {
            role: "price", source: "SMARD",
            ok: true, detail: format!("{p:.2} €/MWh"),
        }),
        None => checks.push(SourceCheck {
            role: "price", source: "SMARD",
            ok: false, detail: "request failed — the chart_data endpoint has been 404 since Aug 2026".into(),
        }),
    }

    // 4. Last-known-good cache
    match load_last_price() {
        Some(p) => checks.push(SourceCheck {
            role: "cache", source: "data_cache/last_price.json",
            ok: true, detail: format!("cached {p:.2} €/MWh"),
        }),
        None => checks.push(SourceCheck {
            role: "cache", source: "data_cache/last_price.json",
            ok: false, detail: "no cached price yet — appears after the first successful fetch".into(),
        }),
    }

    // 5. Weather (Open-Meteo, free)
    match fetch_weather(&client, cfg).await {
        Some((ambient, ghi, series)) => checks.push(SourceCheck {
            role: "weather", source: "Open-Meteo",
            ok: true, detail: format!("{ambient:.1} °C, GHI {ghi:.0} W/m², {} hourly points", series.len()),
        }),
        None => checks.push(SourceCheck {
            role: "weather", source: "Open-Meteo",
            ok: false, detail: "request failed".into(),
        }),
    }

    // 6. Frequency (operator URL; the Modbus register is the real path)
    match &cfg.frequency_url {
        Some(url) => match fetch_frequency(&client, url).await {
            Some(hz) => checks.push(SourceCheck {
                role: "frequency", source: "frequency_url",
                ok: true, detail: format!("{hz:.3} Hz"),
            }),
            None => checks.push(SourceCheck {
                role: "frequency", source: "frequency_url",
                ok: false, detail: "request failed or value out of the plausible 47–53 Hz band".into(),
            }),
        },
        None => checks.push(SourceCheck {
            role: "frequency", source: "frequency_url",
            ok: false, detail: "not configured — for real grid services use a Modbus grid_frequency_addr register".into(),
        }),
    }

    // End-to-end: run the actual failover chain once (same code as the loop).
    let state = LiveStateHandle::new();
    fetch_price_with_failover(&client, &state, entsoe_key.as_deref()).await;
    let s = state.read().await;

    LiveValidationReport {
        checked_at: Utc::now().to_rfc3339(),
        entsoe_key_configured: entsoe_key.is_some(),
        checks,
        end_to_end_source: s.source.clone(),
        end_to_end_price_eur_per_mwh: s.price_eur_per_mwh,
        price_fresh: s.is_price_fresh(cfg.price_max_age_hours),
        mpc_series_points: s.price_series.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entsoe_url_requests_only_day_ahead_prices() {
        let url = entsoe_prices_url("TESTKEY", "202608110000", "202608120000");
        assert!(url.contains("securityToken=TESTKEY"));
        assert!(url.contains("documentType=A44"));
        // A44 covers day-ahead AND intraday — processType=A01 pins day-ahead so
        // the "latest price" can never be an intraday value.
        assert!(url.contains("processType=A01"), "missing processType=A01: {url}");
        assert!(url.contains("in_Domain=10Y1001A1001A82H"));
        assert!(url.contains("out_Domain=10Y1001A1001A82H"));
        assert!(url.contains("periodStart=202608110000&periodEnd=202608120000"));
    }

    #[tokio::test]
    async fn test_live_state_default() {
        let h = LiveStateHandle::new();
        let s = h.read().await;
        assert_eq!(s.price_eur_per_mwh, 50.0);
    }

    #[tokio::test]
    async fn test_update_price() {
        let h = LiveStateHandle::new();
        h.update_price(92.5).await;
        let s = h.read().await;
        assert_eq!(s.price_eur_per_mwh, 92.5);
        assert!(s.last_update.is_some());
    }

    #[tokio::test]
    async fn test_freshness() {
        let h = LiveStateHandle::new();
        // Never fetched → stale.
        assert!(!h.read().await.is_price_fresh(6.0));
        h.update_price(50.0).await;
        // Just updated → fresh.
        assert!(h.read().await.is_price_fresh(6.0));
    }

    #[test]
    fn test_mpc_lookahead_flat_fallback() {
        let state = LiveState { price_eur_per_mwh: 30.0, ambient_temp_c: 25.0, ..Default::default() };
        let lookahead = mpc_lookahead_from_live(&state, 12);
        assert_eq!(lookahead.len(), 12);
        assert!(lookahead.iter().all(|(p, t)| *p == 30.0 && *t == 25.0));
    }

    #[test]
    fn test_mpc_lookahead_uses_real_series() {
        // The fetched day-ahead curve must drive the lookahead (this is what
        // makes MpcPreCool actually fire in live mode).
        let state = LiveState {
            price_eur_per_mwh: 30.0,
            price_series: vec![10.0, 12.0, 90.0, 95.0],
            temp_series: vec![15.0, 16.0, 17.0],
            ambient_temp_c: 25.0,
            ..Default::default()
        };
        let lookahead = mpc_lookahead_from_live(&state, 5);
        assert_eq!(lookahead[0], (10.0, 15.0));
        assert_eq!(lookahead[2], (90.0, 17.0));
        // Beyond the series: last price, ambient fallback for temp.
        assert_eq!(lookahead[3], (95.0, 25.0));
        assert_eq!(lookahead[4], (95.0, 25.0));
    }

    #[test]
    fn test_parse_entsoe_xml_prices_extracts_series() {
        // Realistic subset of an ENTSO-E A44 day-ahead price document.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Publication_MarketDocument xmlns="urn:iec62325.351:tc57wg16:451-3:publicationdocument:7:0">
  <TimeSeries>
    <Period>
      <timeInterval><start>2026-08-03T22:00Z</start><end>2026-08-04T23:00Z</end></timeInterval>
      <resolution>PT60M</resolution>
      <Point><position>1</position><price.amount>45.3</price.amount></Point>
      <Point><position>2</position><price.amount>38.1</price.amount></Point>
      <Point><position>3</position><price.amount>72.6</price.amount></Point>
    </Period>
  </TimeSeries>
</Publication_MarketDocument>"#;
        assert_eq!(parse_entsoe_xml_prices(xml), vec![45.3, 38.1, 72.6]);
    }

    #[test]
    fn test_parse_entsoe_xml_prices_empty() {
        assert_eq!(parse_entsoe_xml_prices("no price data here"), Vec::<f64>::new());
        assert_eq!(parse_entsoe_xml_prices(""), Vec::<f64>::new());
    }

    #[test]
    fn test_parse_entsoe_xml_prices_negative() {
        // Negative day-ahead prices happen during renewable oversupply.
        let xml = "<Point><position>1</position><price.amount>-8.5</price.amount></Point>";
        assert_eq!(parse_entsoe_xml_prices(xml), vec![-8.5]);
        assert_eq!(parse_entsoe_xml_prices(xml).last(), Some(&-8.5));
    }

    #[test]
    fn test_strip_bom() {
        assert_eq!(strip_bom("\u{feff}{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_bom("{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_bom(""), "");
    }
}
