//! Production HTTP API server for the thermal arbitrage system.
//!
//! Endpoints:
//!   - GET  /status        — current system state (JSON)
//!   - GET  /health        — liveness/readiness probes
//!   - GET  /metrics       — Prometheus text exposition format
//!   - GET  /config        — current effective configuration (redacted)
//!   - PUT  /config        — update configuration subset (authenticated, persisted)
//!   - POST /price         — update virtual electricity price
//!   - POST /alerts/test   — send a test alert through the dispatch pipeline
//!   - POST /ota/check     — trigger an immediate OTA check
//!
//! Authentication: every mutating route (PUT/POST) is protected by a
//! Bearer token derived via HMAC-SHA256 (see `crate::security`).

use crate::alerts::{AlertSender, factory};
use crate::config::Config;
use crate::control::{ControlDecision, ControlEngine};
use crate::live_data::LiveStateHandle;
use crate::ota;
use crate::sensor::SensorReading;
use crate::security::{AuthState, derive_token};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};

/// Shared application state accessible from API handlers.
pub struct AppState {
    pub unit_id: String,
    pub control_engine: Mutex<ControlEngine>,
    pub last_reading: Mutex<Option<SensorReading>>,
    pub last_decision: Mutex<Option<ControlDecision>>,
    pub config: RwLock<Config>,
    pub live_data: LiveStateHandle,
    pub alert_sender: Option<AlertSender>,
    pub auth: AuthState,
}

/// Acquire a lock even if a previous holder panicked (poisoned): a stuck lock
/// would otherwise take every API endpoint down after a single panic.
fn lock_mutex<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn read_rwlock<T>(r: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    r.read().unwrap_or_else(|e| e.into_inner())
}

fn write_rwlock<T>(r: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    r.write().unwrap_or_else(|e| e.into_inner())
}

/// Status response for the /status endpoint.
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub unit_id: String,
    pub version: String,
    pub running: bool,
    pub uptime_secs: u64,
    pub current_temp_c: Option<f64>,
    pub ambient_temp_c: Option<f64>,
    pub compressor_on: Option<bool>,
    pub active_controller: Option<String>,
    pub ttf_sec: Option<f64>,
    pub thermal_integral: Option<f64>,
    pub grid_frequency_hz: Option<f64>,
    pub price_eur_per_mwh: Option<f64>,
    pub mpc_confidence: Option<f64>,
    pub estimated_c_th: Option<f64>,
    pub cargo_temp_c: Option<f64>,
    pub compressor_speed: Option<f64>,
    pub price_thresholds: Option<(f64, f64)>,
    pub mpc_optimal_duration_h: Option<f64>,
    pub vfd_integral: Option<f64>,
}

/// Request body for updating price.
#[derive(Debug, Deserialize)]
pub struct PriceUpdate {
    pub price_eur_per_mwh: f64,
}

/// A subset of configuration that can be updated remotely.
#[derive(Debug, Deserialize)]
pub struct ConfigPatch {
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(default)]
    pub t_setpoint_normal: Option<f64>,
    #[serde(default)]
    pub t_setpoint_min: Option<f64>,
    #[serde(default)]
    pub t_setpoint_max: Option<f64>,
    #[serde(default)]
    pub ttf_critical_sec: Option<f64>,
    #[serde(default)]
    pub thermal_integral_max: Option<f64>,
    #[serde(default)]
    pub mpc_cheap_threshold_eur_per_mwh: Option<f64>,
    #[serde(default)]
    pub grid_balancing_deadband_hz: Option<f64>,
    #[serde(default)]
    pub min_runtime_sec: Option<f64>,
    #[serde(default)]
    pub min_off_time_sec: Option<f64>,
    #[serde(default)]
    pub vfd_enabled: Option<bool>,
    #[serde(default)]
    pub vfd_kp: Option<f64>,
    #[serde(default)]
    pub vfd_ki: Option<f64>,
    #[serde(default)]
    pub cargo_lag_enabled: Option<bool>,
    #[serde(default)]
    pub cargo_r_th: Option<f64>,
    #[serde(default)]
    pub cargo_c_th: Option<f64>,
    #[serde(default)]
    pub cargo_t_max: Option<f64>,
    #[serde(default)]
    pub dynamic_price_thresholds_enabled: Option<bool>,
    #[serde(default)]
    pub dynamic_price_charge_percentile: Option<f64>,
    #[serde(default)]
    pub dynamic_price_shed_percentile: Option<f64>,
}

/// Build the API router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Embedded Web Dashboard
        .route("/", get(dashboard_handler))
        .route("/ui", get(dashboard_handler))
        // Public read-only endpoints
        .route("/status", get(status_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/config", get(config_handler))
        // Authenticated mutating endpoints
        .route("/config", put(config_update_handler))
        .route("/price", post(update_price_handler))
        .route("/alerts/test", post(test_alert_handler))
        .route("/ota/check", post(ota_check_handler))
        .with_state(state)
}

/// GET /status — return current system state.
async fn status_handler(State(state): State<Arc<AppState>>) -> Json<SystemStatus> {
    let reading = lock_mutex(&state.last_reading).clone();
    let decision = lock_mutex(&state.last_decision).clone();
    let live = state.live_data.read().await;

    let (cargo_temp_c, price_thresholds, vfd_integral) = {
        let engine = lock_mutex(&state.control_engine);
        (
            if engine.config().thermal.cargo_lag_enabled { Some(engine.cargo_temp_c()) } else { None },
            Some(engine.price_thresholds()),
            if engine.config().control.vfd_enabled { Some(engine.vfd_integral()) } else { None },
        )
    };

    Json(SystemStatus {
        unit_id: state.unit_id.clone(),
        version: ota::CURRENT_VERSION.to_string(),
        running: true,
        uptime_secs: uptime(),
        current_temp_c: reading.as_ref().map(|r| r.internal_temp_c),
        ambient_temp_c: Some(live.ambient_temp_c),
        compressor_on: decision.as_ref().map(|d| d.compressor_on),
        active_controller: decision.as_ref().map(|d| format!("{:?}", d.active_controller)),
        ttf_sec: decision.as_ref().map(|d| d.ttf_sec),
        thermal_integral: decision.as_ref().map(|d| d.thermal_integral),
        grid_frequency_hz: reading.as_ref().map(|r| r.grid_frequency_hz),
        price_eur_per_mwh: Some(live.price_eur_per_mwh),
        mpc_confidence: decision.as_ref().map(|d| d.mpc_confidence),
        estimated_c_th: reading.as_ref().map(|r| r.estimated_c_th),
        cargo_temp_c,
        compressor_speed: decision.as_ref().and_then(|d| d.compressor_speed),
        price_thresholds,
        mpc_optimal_duration_h: decision.as_ref().map(|d| d.mpc_optimal_duration_h),
        vfd_integral,
    })
}

/// GET / and GET /ui — serve the embedded edge dashboard.
async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Ambient Thermal Arbitrage — Edge Gateway Dashboard</title>
  <style>
    :root {
      --bg: #0f172a;
      --card-bg: #1e293b;
      --border: #334155;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --primary: #38bdf8;
      --success: #22c55e;
      --warning: #f59e0b;
      --danger: #ef4444;
      --font: system-ui, -apple-system, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: var(--bg); color: var(--text); font-family: var(--font); padding: 1.5rem; line-height: 1.5; }
    header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap; gap: 0.75rem; }
    h1 { font-size: 1.5rem; font-weight: 700; color: var(--primary); display: flex; align-items: center; gap: 0.5rem; }
    .badge { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.25rem 0.6rem; border-radius: 9999px; font-size: 0.8rem; font-weight: 600; background: #064e3b; color: #34d399; }
    .badge.offline { background: #7f1d1d; color: #fca5a5; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; margin-bottom: 1.5rem; }
    .card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 0.75rem; padding: 1.25rem; }
    .card h2 { font-size: 0.95rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); margin-bottom: 1rem; border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 0.5rem; }
    .metric-row { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.75rem; }
    .metric-label { color: var(--text-muted); font-size: 0.9rem; }
    .metric-val { font-size: 1.15rem; font-weight: 700; font-variant-numeric: tabular-nums; }
    .progress-bar-bg { width: 100%; height: 8px; background: var(--border); border-radius: 4px; overflow: hidden; margin-top: 0.35rem; }
    .progress-bar { height: 100%; background: var(--primary); width: 0%; transition: width 0.3s; }
    footer { border-top: 1px solid var(--border); padding-top: 1rem; color: var(--text-muted); font-size: 0.85rem; display: flex; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
    a { color: var(--primary); text-decoration: none; }
    a:hover { text-decoration: underline; }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>❄️ Ambient Thermal Arbitrage</h1>
      <div style="color: var(--text-muted); font-size: 0.85rem;">Edge Gateway Steuerungssystem &bull; <span id="unit-id">-</span> &bull; v<span id="version">-</span></div>
    </div>
    <div style="display: flex; gap: 0.75rem; align-items: center;">
      <span id="status-badge" class="badge">● VERBUNDEN</span>
      <span style="color: var(--text-muted); font-size: 0.85rem;">Betriebszeit: <span id="uptime">0s</span></span>
    </div>
  </header>

  <div class="grid">
    <div class="card">
      <h2>🌡️ Thermische Masse & Produktkern</h2>
      <div class="metric-row"><span class="metric-label">Lufttemperatur (Lager):</span><span id="air-temp" class="metric-val">-- °C</span></div>
      <div class="metric-row"><span class="metric-label">Ware / Kerntemperatur:</span><span id="cargo-temp" class="metric-val">-- °C</span></div>
      <div class="metric-row"><span class="metric-label">Aussentemperatur:</span><span id="amb-temp" class="metric-val">-- °C</span></div>
      <div class="metric-row"><span class="metric-label">TTF Sicherheitsmarge:</span><span id="ttf-sec" class="metric-val">-- s</span></div>
      <div class="metric-row"><span class="metric-label">Thermisches Integral:</span><span id="thermal-int" class="metric-val">-- °C·s</span></div>
    </div>

    <div class="card">
      <h2>⚙️ Verdichter & Inverter (VFD)</h2>
      <div class="metric-row"><span class="metric-label">Aktiver Modus:</span><span id="ctrl-mode" class="metric-val" style="color: var(--primary)">--</span></div>
      <div class="metric-row"><span class="metric-label">Verdichter-Status:</span><span id="comp-state" class="metric-val">--</span></div>
      <div class="metric-row"><span class="metric-label">FU-Drehzahl (VFD):</span><span id="vfd-speed" class="metric-val">-- %</span></div>
      <div class="progress-bar-bg"><div id="vfd-bar" class="progress-bar"></div></div>
      <div class="metric-row" style="margin-top: 0.75rem;"><span class="metric-label">VFD PI-Integral:</span><span id="vfd-int" class="metric-val">--</span></div>
    </div>

    <div class="card">
      <h2>💶 Spotmarkt & Dynamische MPC</h2>
      <div class="metric-row"><span class="metric-label">Spotpreis:</span><span id="price" class="metric-val" style="color: var(--warning)">-- €/MWh</span></div>
      <div class="metric-row"><span class="metric-label">Lade- / Shedding-Bänder:</span><span id="price-bands" class="metric-val">-- / -- €</span></div>
      <div class="metric-row"><span class="metric-label">MPC Konfidenz:</span><span id="mpc-conf" class="metric-val">-- %</span></div>
      <div class="metric-row"><span class="metric-label">MPC Optimale Dauer:</span><span id="mpc-dur" class="metric-val">-- h</span></div>
    </div>

    <div class="card">
      <h2>⚡ Netzdienste & Schnittstellen</h2>
      <div class="metric-row"><span class="metric-label">Netzfrequenz:</span><span id="grid-freq" class="metric-val">-- Hz</span></div>
      <div class="metric-row"><span class="metric-label">Geschätzte Kapazität C_th:</span><span id="est-cth" class="metric-val">-- kJ/K</span></div>
      <div class="metric-row"><span class="metric-label">Statusdaten (JSON):</span><span><a href="/status" target="_blank">/status JSON</a></span></div>
      <div class="metric-row"><span class="metric-label">Prometheus Metriken:</span><span><a href="/metrics" target="_blank">/metrics</a></span></div>
    </div>
  </div>

  <footer>
    <div>Ambient Thermal Arbitrage — Edge Gateway Regelung</div>
    <div>Live-Aktualisierung: 1000ms &bull; <a href="/health">Systemdiagnose (Health)</a></div>
  </footer>

  <script>
    const modeTranslations = {
      'Emergency': '🚨 Notfall (Sicherheit)',
      'ThermalGuard': '🛡️ Temperaturschutz',
      'GridBalancing': '⚡ Netzstabilisierung',
      'Arbitrage': '💰 Arbitrage (Spotmarkt)',
      'MpcPreCool': '🔮 MPC Vor-Kühlung',
      'OilReturnPulse': '🔄 Ölrückführung',
      'Idle': '⏸️ Standby'
    };

    function formatUptime(secs) {
      if (secs < 60) return secs + 's';
      const m = Math.floor(secs / 60);
      const s = secs % 60;
      if (m < 60) return m + 'm ' + s + 's';
      const h = Math.floor(m / 60);
      return h + 'h ' + (m % 60) + 'm';
    }

    async function update() {
      try {
        const res = await fetch('/status');
        if (!res.ok) throw new Error('HTTP ' + res.status);
        const data = await res.json();
        document.getElementById('status-badge').className = 'badge';
        document.getElementById('status-badge').textContent = '● VERBUNDEN';
        document.getElementById('unit-id').textContent = data.unit_id || 'ATA-GW';
        document.getElementById('version').textContent = data.version || '0.2.0';
        document.getElementById('uptime').textContent = formatUptime(data.uptime_secs || 0);

        document.getElementById('air-temp').textContent = data.current_temp_c != null ? data.current_temp_c.toFixed(2) + ' °C' : '--';
        document.getElementById('cargo-temp').textContent = data.cargo_temp_c != null ? data.cargo_temp_c.toFixed(2) + ' °C' : (data.current_temp_c != null ? data.current_temp_c.toFixed(2) + ' °C' : '--');
        document.getElementById('amb-temp').textContent = data.ambient_temp_c != null ? data.ambient_temp_c.toFixed(1) + ' °C' : '--';
        document.getElementById('ttf-sec').textContent = data.ttf_sec != null ? Math.round(data.ttf_sec) + ' s' : 'Sicher';
        document.getElementById('thermal-int').textContent = data.thermal_integral != null ? data.thermal_integral.toFixed(1) : '0.0';

        const rawMode = data.active_controller || 'Normal';
        document.getElementById('ctrl-mode').textContent = modeTranslations[rawMode] || rawMode;
        const compOn = data.compressor_on;
        document.getElementById('comp-state').textContent = compOn ? 'LÄUFT (EIN)' : 'BEREIT (AUS)';
        document.getElementById('comp-state').style.color = compOn ? 'var(--success)' : 'var(--text-muted)';

        const speedPct = data.compressor_speed != null ? Math.round(data.compressor_speed * 100) : (compOn ? 100 : 0);
        document.getElementById('vfd-speed').textContent = speedPct + ' %';
        document.getElementById('vfd-bar').style.width = speedPct + '%';
        document.getElementById('vfd-int').textContent = data.vfd_integral != null ? data.vfd_integral.toFixed(2) : '--';

        document.getElementById('price').textContent = data.price_eur_per_mwh != null ? data.price_eur_per_mwh.toFixed(2) + ' €/MWh' : '--';
        if (data.price_thresholds) {
          document.getElementById('price-bands').textContent = data.price_thresholds[0].toFixed(1) + ' / ' + data.price_thresholds[1].toFixed(1) + ' €';
        }
        document.getElementById('mpc-conf').textContent = data.mpc_confidence != null ? Math.round(data.mpc_confidence * 100) + ' %' : '0 %';
        document.getElementById('mpc-dur').textContent = data.mpc_optimal_duration_h != null ? data.mpc_optimal_duration_h + ' h' : '--';

        document.getElementById('grid-freq').textContent = data.grid_frequency_hz != null ? data.grid_frequency_hz.toFixed(3) + ' Hz' : '50.000 Hz';
        document.getElementById('est-cth').textContent = data.estimated_c_th != null ? (data.estimated_c_th / 1000).toFixed(1) + ' kJ/K' : '--';
      } catch (err) {
        document.getElementById('status-badge').className = 'badge offline';
        document.getElementById('status-badge').textContent = '● GETRENNT';
      }
    }
    setInterval(update, 1000);
    update();
  </script>
</body>
</html>"#;

/// GET /health — liveness probe (200 OK if the process is alive).
async fn health_handler() -> impl IntoResponse {
    let status = if uptime() < 10 { "starting" } else { "healthy" };
    axum::response::Json(serde_json::json!({
        "status": status,
        "version": ota::CURRENT_VERSION,
        "uptime_secs": uptime(),
    }))
}

/// GET /metrics — Prometheus text exposition.
async fn metrics_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        crate::metrics::render(),
    )
}

/// GET /config — current effective configuration (redacted for secrets).
async fn config_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = read_rwlock(&state.config).clone();
    let mut json = serde_json::to_value(&cfg).unwrap_or(serde_json::Value::Null);
    if let Some(sec) = json.get_mut("security").and_then(|s| s.as_object_mut()) {
        if let Some(v) = sec.get_mut("api_secret") {
            *v = serde_json::Value::String("***REDACTED***".into());
        }
    }
    if let Some(ota_cfg) = json.get_mut("ota").and_then(|s| s.as_object_mut()) {
        if let Some(v) = ota_cfg.get_mut("verify_key_hex") {
            *v = serde_json::Value::String("***REDACTED***".into());
        }
    }
    // Redact all remaining secret material: live-data API key and alert webhook.
    if let Some(ld) = json.get_mut("live_data").and_then(|s| s.as_object_mut()) {
        if let Some(v) = ld.get_mut("entsoe_api_key") {
            *v = serde_json::Value::String("***REDACTED***".into());
        }
    }
    if let Some(al) = json.get_mut("alerts").and_then(|s| s.as_object_mut()) {
        if let Some(v) = al.get_mut("webhook_url") {
            *v = serde_json::Value::String("***REDACTED***".into());
        }
    }
    Json(json)
}

/// PUT /config — apply a configuration subset and persist to config.toml.
async fn config_update_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(patch): Json<ConfigPatch>,
) -> Result<impl IntoResponse, StatusCode> {
    require_auth_header(&state, &headers, "/config")?;

    // Bounds-check every patchable value before touching live state. A remote
    // caller must not be able to drive the controller into a nonsense regime
    // (absurd setpoints, negative safety thresholds, huge strings, ...).
    fn valid_temp(v: f64) -> bool { v.is_finite() && (-80.0..=80.0).contains(&v) }
    fn valid_positive(v: f64, max: f64) -> bool { v.is_finite() && v > 0.0 && v <= max }
    fn valid_nonneg(v: f64, max: f64) -> bool { v.is_finite() && (0.0..=max).contains(&v) }
    if let Some(v) = patch.unit_id.as_deref() {
        if v.is_empty() || v.len() > 64 || !v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if [patch.t_setpoint_normal, patch.t_setpoint_min, patch.t_setpoint_max]
        .into_iter().flatten().any(|x| !valid_temp(x)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if [patch.ttf_critical_sec, patch.min_runtime_sec, patch.min_off_time_sec]
        .into_iter().flatten().any(|x| !valid_positive(x, 86_400.0)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if [patch.thermal_integral_max, patch.mpc_cheap_threshold_eur_per_mwh]
        .into_iter().flatten().any(|x| !valid_positive(x, 1e9)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(x) = patch.grid_balancing_deadband_hz {
        if !valid_nonneg(x, 5.0) { return Err(StatusCode::BAD_REQUEST); }
    }
    if [patch.vfd_kp, patch.vfd_ki].into_iter().flatten().any(|x| !valid_nonneg(x, 10.0)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(x) = patch.cargo_r_th {
        if !valid_positive(x, 10.0) { return Err(StatusCode::BAD_REQUEST); }
    }
    if let Some(x) = patch.cargo_c_th {
        if !valid_positive(x, 1e9) { return Err(StatusCode::BAD_REQUEST); }
    }
    if let Some(x) = patch.cargo_t_max {
        if !valid_temp(x) { return Err(StatusCode::BAD_REQUEST); }
    }
    if [patch.dynamic_price_charge_percentile, patch.dynamic_price_shed_percentile]
        .into_iter().flatten().any(|x| !x.is_finite() || !(0.01..=0.99).contains(&x)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    {
        let mut cfg = write_rwlock(&state.config);
        let prev = cfg.clone();
        if let Some(v) = patch.unit_id { cfg.system.unit_id = v; }
        if let Some(v) = patch.t_setpoint_normal { cfg.thermal.t_setpoint_normal = v; }
        if let Some(v) = patch.t_setpoint_min { cfg.thermal.t_setpoint_min = v; }
        if let Some(v) = patch.t_setpoint_max { cfg.thermal.t_setpoint_max = v; }
        if let Some(v) = patch.ttf_critical_sec { cfg.control.ttf_critical_sec = v; }
        if let Some(v) = patch.thermal_integral_max { cfg.control.thermal_integral_max = v; }
        if let Some(v) = patch.mpc_cheap_threshold_eur_per_mwh { cfg.control.mpc_cheap_threshold_eur_per_mwh = v; }
        if let Some(v) = patch.grid_balancing_deadband_hz { cfg.control.grid_balancing_deadband_hz = v; }
        if let Some(v) = patch.min_runtime_sec { cfg.control.min_runtime_sec = v; }
        if let Some(v) = patch.min_off_time_sec { cfg.control.min_off_time_sec = v; }
        if let Some(v) = patch.vfd_enabled { cfg.control.vfd_enabled = v; }
        if let Some(v) = patch.vfd_kp { cfg.control.vfd_kp = v; }
        if let Some(v) = patch.vfd_ki { cfg.control.vfd_ki = v; }
        if let Some(v) = patch.cargo_lag_enabled { cfg.thermal.cargo_lag_enabled = v; }
        if let Some(v) = patch.cargo_r_th { cfg.thermal.cargo_r_th = v; }
        if let Some(v) = patch.cargo_c_th { cfg.thermal.cargo_c_th = v; }
        if let Some(v) = patch.cargo_t_max { cfg.thermal.cargo_t_max = v; }
        if let Some(v) = patch.dynamic_price_thresholds_enabled { cfg.control.dynamic_price_thresholds_enabled = v; }
        if let Some(v) = patch.dynamic_price_charge_percentile { cfg.control.dynamic_price_charge_percentile = v; }
        if let Some(v) = patch.dynamic_price_shed_percentile { cfg.control.dynamic_price_shed_percentile = v; }

        if cfg.control.dynamic_price_charge_percentile >= cfg.control.dynamic_price_shed_percentile {
            *cfg = prev;
            return Err(StatusCode::BAD_REQUEST);
        }
        // Cross-field consistency: the thermal band must stay ordered
        // t_min <= t_setpoint_min <= t_setpoint_normal <= t_setpoint_max <= t_max.
        // Per-field validation alone allows contradictory setpoints via two PUTs.
        let t = &cfg.thermal;
        if !(t.t_min <= t.t_setpoint_min
            && t.t_setpoint_min <= t.t_setpoint_normal
            && t.t_setpoint_normal <= t.t_setpoint_max
            && t.t_setpoint_max <= t.t_max) {
            *cfg = prev; // revert — don't leave memory in a contradictory state
            return Err(StatusCode::BAD_REQUEST);
        }
        // Apply to the RUNNING control engine immediately (was restart-only).
        lock_mutex(&state.control_engine).apply_config(cfg.clone());
    }

    let out = read_rwlock(&state.config);
    if let Err(e) = persist_config(&out) {
        log::error!("⚠️  Failed to persist config: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    log::info!("🔧 Remote configuration updated via API");
    Ok(Json(serde_json::json!({"status": "ok", "message": "configuration updated"})))
}

/// POST /price — update the virtual electricity price.
async fn update_price_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(price_update): Json<PriceUpdate>,
) -> Result<impl IntoResponse, StatusCode> {
    require_auth_header(&state, &headers, "/price")?;

    // Bounds-check the price: absurd finite values would corrupt the financial
    // accumulator (inf/NaN) and the arbitrage logic.
    let price = price_update.price_eur_per_mwh;
    if !price.is_finite() || !(0.0..=10_000.0).contains(&price) {
        return Err(StatusCode::BAD_REQUEST);
    }

    {
        let mut engine = lock_mutex(&state.control_engine);
        engine.update_price(price);
    }
    state.live_data.update_price(price).await;
    crate::metrics::set_price(price);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "price_eur_per_mwh": price
    })))
}

/// POST /alerts/test — send a test alert through the dispatch pipeline.
async fn test_alert_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    require_auth_header(&state, &headers, "/alerts/test")?;

    if let Some(sender) = &state.alert_sender {
        let _ = sender.send(factory::startup(&state.unit_id));
        Ok(Json(serde_json::json!({"status": "ok", "message": "test alert sent"})))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// POST /ota/check — trigger an immediate OTA manifest check.
async fn ota_check_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    require_auth_header(&state, &headers, "/ota/check")?;

    let cfg = read_rwlock(&state.config).ota.clone();
    match ota::check_for_update(&cfg).await {
        Ok(Some(version)) => Ok(Json(serde_json::json!({"status": "update_available", "version": version}))),
        Ok(None) => Ok(Json(serde_json::json!({"status": "up_to_date", "version": ota::CURRENT_VERSION}))),
        Err(e) => {
            log::warn!("📦 OTA check via API failed: {}", e);
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// Verify bearer token against the configured auth state for a route scope.
fn require_auth_header(state: &AppState, headers: &HeaderMap, scope: &str) -> Result<(), StatusCode> {
    if !state.auth.auth_enabled {
        return Ok(());
    }
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let expected = derive_token(&state.auth.secret, scope);
    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Persist the current config to config.toml.
/// NOTE: secrets are kept as-is. Clearing them here would BRICK the next
/// restart (empty secret + auth_enabled → fail-closed exit) and silently drop
/// alerting/OTA — worse than writing the operator's own values to their own
/// site file. The config.toml is operator-controlled local disk (chmod 600);
/// the "secrets via env" guidance in DEPLOYMENT.md still applies to new sites.
fn persist_config(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(test)]
    {
        let _ = cfg;
        Ok(())
    }
    #[cfg(not(test))]
    {
        let toml_str = toml::to_string(cfg)?;
        let target = if std::path::Path::new("crates/gateway/config.toml").exists() {
            "crates/gateway/config.toml"
        } else {
            "config.toml"
        };
        std::fs::write(target, toml_str)?;
        Ok(())
    }
}

/// System uptime in seconds.
fn uptime() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs()
}

/// Serve the API server, optionally with TLS.
pub async fn serve(state: Arc<AppState>, config: &crate::config::ApiConfig) -> Result<(), String> {
    let router = build_router(state);
    let addr = config.bind_address.clone();

    match (&config.tls_cert_path, &config.tls_key_path) {
        (Some(cert), Some(key)) => serve_tls(router, &addr, cert, key).await,
        (None, None) => serve_http(router, &addr).await,
        // Asymmetric TLS config: never silently downgrade. Refuse to serve
        // CLEARTEXT on a non-loopback address; loopback gets a loud warning.
        _ => {
            let loopback = addr.starts_with("127.") || addr.starts_with("localhost")
                || addr.starts_with("[::1]") || addr.starts_with("::1");
            if loopback {
                log::warn!("🔒 TLS misconfiguration: both tls_cert_path and tls_key_path are required — serving plain HTTP on loopback {} only", addr);
                serve_http(router, &addr).await
            } else {
                log::error!(
                    "🔒 Refusing to serve CLEARTEXT HTTP on non-loopback {} with TLS half-configured. Provide both tls_cert_path and tls_key_path, or remove both.",
                    addr
                );
                Err("TLS misconfigured (one of cert/key missing)".to_string())
            }
        }
    }
}

async fn serve_http(router: Router, addr: &str) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind {}: {}", addr, e))?;
    log::info!("🌐 API server listening on http://{}", addr);
    axum::serve(listener, router)
        .await
        .map_err(|e| format!("HTTP server error: {}", e))?;
    Ok(())
}

/// Serve the API over TLS (rustls).
async fn serve_tls(
    router: Router,
    addr: &str,
    cert_path: &str,
    key_path: &str,
) -> Result<(), String> {
    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .map_err(|e| format!("TLS config error: {}", e))?;

    let listener = std::net::TcpListener::bind(addr)
        .map_err(|e| format!("Failed to bind {}: {}", addr, e))?;
    log::info!("🔒 API server listening on https://{}", addr);

    axum_server::from_tcp_rustls(listener, tls_config)
        .serve(router.into_make_service())
        .await
        .map_err(|e| format!("TLS server error: {}", e))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tower::util::ServiceExt;
    use crate::config::Config;
    use crate::control::ControlEngine;

    fn make_state() -> Arc<AppState> {
        let mut config = Config::default();
        // Seed secret values so redaction tests actually have something to redact.
        config.live_data.entsoe_api_key = Some("test-entsoe-key".into());
        config.alerts.webhook_url = Some("https://hooks.example.com/test".into());
        let control_engine = ControlEngine::new(config.clone());
        Arc::new(AppState {
            unit_id: config.system.unit_id.clone(),
            control_engine: Mutex::new(control_engine),
            last_reading: Mutex::new(None),
            last_decision: Mutex::new(None),
            config: RwLock::new(config),
            live_data: LiveStateHandle::new(),
            alert_sender: None,
            auth: AuthState { secret: "test-secret".into(), auth_enabled: true },
        })
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("healthy") || text.contains("starting"));
    }

    #[tokio::test]
    async fn test_status_returns_ok() {
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().uri("/status").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_mutating_route_rejects_missing_token() {
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().method("POST").uri("/price")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#"{"price_eur_per_mwh": 42.0}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_mutating_route_accepts_valid_token() {
        let token = derive_token("test-secret", "/price");
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().method("POST").uri("/price")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(axum::body::Body::from(r#"{"price_eur_per_mwh": 42.0}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_config_redacts_secrets() {
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().uri("/config").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("REDACTED"));
        assert!(!text.contains("test-secret"));
        assert!(!text.contains("test-entsoe-key"));
        assert!(!text.contains("hooks.example.com"));
    }

    #[tokio::test]
    async fn test_dashboard_serves_html() {
        let app = build_router(make_state());
        let resp = app.oneshot(
            axum::http::Request::builder().uri("/").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/html; charset=utf-8");
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("Ambient Thermal Arbitrage"));
        assert!(text.contains("Produktkern"));
        assert!(text.contains("FU-Drehzahl (VFD)"));

        // Verify /ui route serves the same dashboard
        let app_ui = build_router(make_state());
        let resp_ui = app_ui.oneshot(
            axum::http::Request::builder().uri("/ui").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp_ui.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status_includes_advanced_telemetry() {
        let state = make_state();
        let app = build_router(state);
        let resp = app.oneshot(
            axum::http::Request::builder().uri("/status").body(axum::body::Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("price_thresholds").is_some());
        assert!(json.get("vfd_integral").is_some() || json.get("compressor_speed").is_some());
    }

    #[tokio::test]
    async fn test_config_patch_tuning_parameters() {
        let token = derive_token("test-secret", "/config");
        let state = make_state();
        let app = build_router(state.clone());
        let patch_body = serde_json::json!({
            "vfd_enabled": true,
            "vfd_kp": 0.08,
            "vfd_ki": 0.005,
            "cargo_lag_enabled": true,
            "cargo_r_th": 0.05,
            "cargo_c_th": 400000.0,
            "cargo_t_max": -17.5,
            "dynamic_price_thresholds_enabled": true,
            "dynamic_price_charge_percentile": 0.25,
            "dynamic_price_shed_percentile": 0.75
        });

        let resp = app.oneshot(
            axum::http::Request::builder().method("PUT").uri("/config")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(axum::body::Body::from(patch_body.to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify the live running config updated
        let current_cfg = state.config.read().unwrap().clone();
        assert!(current_cfg.control.vfd_enabled);
        assert_eq!(current_cfg.control.vfd_kp, 0.08);
        assert_eq!(current_cfg.control.vfd_ki, 0.005);
        assert!(current_cfg.thermal.cargo_lag_enabled);
        assert_eq!(current_cfg.thermal.cargo_t_max, -17.5);
        assert_eq!(current_cfg.control.dynamic_price_charge_percentile, 0.25);

        // Verify invalid percentiles (charge >= shed) are rejected with 400
        let bad_patch = serde_json::json!({
            "dynamic_price_charge_percentile": 0.80,
            "dynamic_price_shed_percentile": 0.20
        });
        let app2 = build_router(state);
        let resp2 = app2.oneshot(
            axum::http::Request::builder().method("PUT").uri("/config")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(axum::body::Body::from(bad_patch.to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
    }
}