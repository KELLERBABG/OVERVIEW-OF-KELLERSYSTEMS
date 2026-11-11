//! Prometheus metrics for the ATA gateway.
//!
//! Exposes `/metrics` endpoint in Prometheus text exposition format.
//! Counter/Histogram types from crate `prometheus` are registered in a global registry.

use prometheus::{Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry, TextEncoder};
use std::sync::OnceLock;

/// Global metrics registry (lazily initialized).
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Temperature gauge: internal temp in °C, labeled by unit.
static INTERNAL_TEMP: OnceLock<IntGauge> = OnceLock::new();

/// Compressor state gauge: 1 = ON, 0 = OFF.
static COMPRESSOR_STATE: OnceLock<IntGauge> = OnceLock::new();

/// Compressor VFD speed gauge: 0..100 (= 0..100 % speed).
static COMPRESSOR_SPEED: OnceLock<IntGauge> = OnceLock::new();

/// Price gauge in €/MWh.
static PRICE_EUR: OnceLock<IntGauge> = OnceLock::new();

/// TTF gauge in seconds.
static TTF_SECONDS: OnceLock<IntGauge> = OnceLock::new();

/// Control mode counter, labeled by mode.
static CONTROL_MODE_COUNTER: OnceLock<IntCounterVec> = OnceLock::new();

/// Alert counter, labeled by severity.
static ALERT_COUNTER: OnceLock<IntCounterVec> = OnceLock::new();

/// OTA attempts counter, labeled by status (success/failure).
static OTA_COUNTER: OnceLock<IntCounterVec> = OnceLock::new();

/// HTTP request counter, labeled by path and status.
static HTTP_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();

/// Control loop duration histogram (seconds).
static LOOP_DURATION: OnceLock<HistogramVec> = OnceLock::new();

/// VFD oil-return pulses triggered (predictive-maintenance analytics).
static OIL_RETURN_PULSES: OnceLock<IntCounter> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let r = Registry::new();

        let internal_temp = IntGauge::new("ata_internal_temp_c", "Internal temperature in °C").unwrap();
        r.register(Box::new(internal_temp.clone())).ok();
        let _ = INTERNAL_TEMP.set(internal_temp);

        let comp = IntGauge::new("ata_compressor_on", "Compressor state (1=ON, 0=OFF)").unwrap();
        r.register(Box::new(comp.clone())).ok();
        let _ = COMPRESSOR_STATE.set(comp);

        let comp_speed = IntGauge::new("ata_compressor_speed", "Compressor VFD speed (0-100 = 0-100%)").unwrap();
        r.register(Box::new(comp_speed.clone())).ok();
        let _ = COMPRESSOR_SPEED.set(comp_speed);

        let price = IntGauge::new("ata_price_eur_per_mwh", "Electricity price in €/MWh").unwrap();
        r.register(Box::new(price.clone())).ok();
        let _ = PRICE_EUR.set(price);

        let ttf = IntGauge::new("ata_ttf_seconds", "Time-to-failure in seconds").unwrap();
        r.register(Box::new(ttf.clone())).ok();
        let _ = TTF_SECONDS.set(ttf);

        let modes = IntCounterVec::new(Opts::new("ata_control_mode_total", "Control mode activations"), &["mode"]).unwrap();
        r.register(Box::new(modes.clone())).ok();
        let _ = CONTROL_MODE_COUNTER.set(modes);

        let alerts = IntCounterVec::new(Opts::new("ata_alerts_total", "Alerts raised"), &["severity"]).unwrap();
        r.register(Box::new(alerts.clone())).ok();
        let _ = ALERT_COUNTER.set(alerts);

        let ota = IntCounterVec::new(Opts::new("ata_ota_total", "OTA update attempts"), &["status"]).unwrap();
        r.register(Box::new(ota.clone())).ok();
        let _ = OTA_COUNTER.set(ota);

        let http = IntCounterVec::new(Opts::new("ata_http_requests_total", "HTTP requests"), &["path", "status"]).unwrap();
        r.register(Box::new(http.clone())).ok();
        let _ = HTTP_REQUESTS.set(http);

        let loop_dur = HistogramVec::new(HistogramOpts::new("ata_loop_duration_seconds", "Control loop duration"), &["phase"]).unwrap();
        r.register(Box::new(loop_dur.clone())).ok();
        let _ = LOOP_DURATION.set(loop_dur);

        let oil = IntCounter::new("ata_oil_return_pulses_total", "VFD oil-return pulses triggered").unwrap();
        r.register(Box::new(oil.clone())).ok();
        let _ = OIL_RETURN_PULSES.set(oil);

        r
    })
}

// ─── Metric update helpers ─────────────────────────────────────────────
//
// Every setter/incr explicitly initializes the registry first (idempotent
// OnceLock). Without this, values recorded before the first `/metrics`
// scrape are silently dropped because the static OnceLocks are still empty.

pub fn set_internal_temp(v: f64) {
    registry();
    if let Some(m) = INTERNAL_TEMP.get() { m.set((v * 100.0) as i64); }
}

pub fn set_compressor_state(on: bool) {
    registry();
    if let Some(m) = COMPRESSOR_STATE.get() { m.set(on as i64); }
}

pub fn set_compressor_speed(v: f64) {
    registry();
    if v.is_finite() {
        if let Some(m) = COMPRESSOR_SPEED.get() { m.set((v.clamp(0.0, 1.0) * 100.0) as i64); }
    }
}

pub fn set_price(v: f64) {
    registry();
    if let Some(m) = PRICE_EUR.get() { m.set((v * 100.0) as i64); }
}

pub fn set_ttf(v: f64) {
    registry();
    if v.is_finite() {
        if let Some(m) = TTF_SECONDS.get() { m.set(v as i64); }
    }
}

pub fn inc_control_mode(mode: &str) {
    registry();
    if let Some(m) = CONTROL_MODE_COUNTER.get() { m.with_label_values(&[mode]).inc(); }
}

pub fn inc_alert(severity: &str) {
    registry();
    if let Some(m) = ALERT_COUNTER.get() { m.with_label_values(&[severity]).inc(); }
}

pub fn inc_ota(status: &str) {
    registry();
    if let Some(m) = OTA_COUNTER.get() { m.with_label_values(&[status]).inc(); }
}

pub fn inc_http(path: &str, status: u16) {
    registry();
    if let Some(m) = HTTP_REQUESTS.get() { m.with_label_values(&[path, &status.to_string()]).inc(); }
}

pub fn inc_oil_return_pulse() {
    registry();
    if let Some(m) = OIL_RETURN_PULSES.get() { m.inc(); }
}

pub fn observe_loop(phase: &str, seconds: f64) {
    registry();
    if let Some(m) = LOOP_DURATION.get() { m.with_label_values(&[phase]).observe(seconds); }
}

/// Render the complete metric set in Prometheus text format.
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    if let Err(e) = encoder.encode(&registry().gather(), &mut buf) {
        log::error!("Failed to encode metrics: {}", e);
        return String::new();
    }
    String::from_utf8_lossy(&buf).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_contains_headers() {
        let out = render();
        assert!(!out.is_empty());
        // Prometheus text format always contains these markers for our metric families.
        assert!(out.contains("ata_internal_temp_c"));
    }

    #[test]
    fn test_update_and_render() {
        set_internal_temp(-20.5);
        set_compressor_state(true);
        set_price(45.0);
        set_ttf(1200.0);
        inc_control_mode("Arbitrage");
        inc_alert("warning");
        let out = render();
        assert!(out.contains("ata_compressor_on 1"));
        assert!(out.contains("ata_control_mode_total{mode=\"Arbitrage\"} 1"));
    }
}