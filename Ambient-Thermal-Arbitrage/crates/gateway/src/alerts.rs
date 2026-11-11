//! Alert system: emits structured alerts to a webhook with exponential retry.
//!
//! Alert severity levels:
//!   - `critical`: Cold chain integrity at risk — emergency / thermal guard triggered
//!   - `warning`: Abnormal condition — TTF trending low, price anomalies
//!   - `info`: Operational events — startup, mode changes, OTA

use crate::config::AlertsConfig;
use serde::Serialize;
use tokio::sync::mpsc;

/// An alert emitted by the system.
#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub unit_id: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub timestamp: String,
    pub event_id: String,
}

/// Errors returned by the alert sender.
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("channel closed")]
    ChannelClosed,
}

/// Handle for sending alerts to the background sender task.
#[derive(Clone)]
pub struct AlertSender {
    tx: mpsc::Sender<Alert>,
}

impl AlertSender {
    /// Send an alert to the background dispatcher. Never blocks: uses a bounded
    /// channel (capacity 256) and drops the alert with a warning if full.
    pub fn send(&self, alert: Alert) -> Result<(), AlertError> {
        self.tx.try_send(alert).map_err(|_| AlertError::ChannelClosed)
    }
}

/// Start the alert dispatch background task.
///
/// Returns a sender handle and a task JoinHandle. The task drains the channel
/// and delivers each alert to the configured webhook with exponential backoff.
pub fn spawn_alert_dispatcher(
    _unit_id: String,
    cfg: AlertsConfig,
) -> (AlertSender, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<Alert>(256);

    let handle = tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        while let Some(alert) = rx.recv().await {
            if !cfg.enabled {
                // Log-only mode
                match alert.severity.as_str() {
                    "critical" => log::error!("🚨 ALERT [{}] {} (unit: {})", alert.severity, alert.message, alert.unit_id),
                    "warning" => log::warn!("⚠️  ALERT [{}] {} (unit: {})", alert.severity, alert.message, alert.unit_id),
                    _ => log::info!("ℹ️  ALERT [{}] {} (unit: {})", alert.severity, alert.message, alert.unit_id),
                }
                crate::metrics::inc_alert(&alert.severity);
                continue;
            }

            let Some(webhook_url) = &cfg.webhook_url else {
                // No webhook configured: log only
                log::warn!("⚠️  Alert enabled but no webhook URL configured. Logging only: {}", alert.message);
                crate::metrics::inc_alert(&alert.severity);
                continue;
            };

            let mut delivered = false;
            for attempt in 0..cfg.max_retries {
                match client.post(webhook_url).json(&alert).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        log::info!("✅ Alert delivered to webhook (attempt {})", attempt + 1);
                        crate::metrics::inc_alert(&alert.severity);
                        delivered = true;
                        break;
                    }
                    Ok(resp) => {
                        log::warn!("⚠️  Webhook returned HTTP {} (attempt {}/{})", resp.status(), attempt + 1, cfg.max_retries);
                    }
                    Err(e) => {
                        // without_url(): the failed request URL carries the webhook
                        // secret token in its path — never log it in full.
                        log::warn!("⚠️  Webhook delivery failed: {} (attempt {}/{})", e.without_url(), attempt + 1, cfg.max_retries);
                    }
                }
                // Exponential backoff: base * 2^attempt
                let backoff = cfg.retry_backoff_sec << attempt;
                tokio::time::sleep(std::time::Duration::from_secs(backoff.min(60))).await;
            }

            if !delivered {
                log::error!("❌ Alert dropped after {} attempts: {}", cfg.max_retries, alert.message);
                crate::metrics::inc_alert(&format!("{}_failed", alert.severity));
            }
        }
    });

    let sender = AlertSender { tx };
    (sender, handle)
}

/// Convenience constructors for common alerts.
pub mod factory {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make(unit_id: &str, severity: &str, message: &str, value: Option<f64>) -> Alert {
        Alert {
            unit_id: unit_id.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
            value,
            timestamp: Utc::now().to_rfc3339(),
            event_id: Uuid::new_v4().to_string(),
        }
    }

    /// Emergency cold-chain alert (Mode 1).
    pub fn emergency(unit_id: &str, temp_c: f64, ttf_sec: f64) -> Alert {
        make(unit_id, "critical", &format!("EMERGENCY: Cold chain at risk! T={:.1}°C TTF={:.0}s", temp_c, ttf_sec), Some(temp_c))
    }

    /// Thermal Guard alert (Mode 2).
    pub fn thermal_guard(unit_id: &str, temp_c: f64, integral: f64) -> Alert {
        make(unit_id, "warning", &format!("Thermal Guard engaged: T={:.1}°C I={:.0}", temp_c, integral), Some(temp_c))
    }

    /// Grid balancing event.
    pub fn grid_balancing(unit_id: &str, freq_hz: f64) -> Alert {
        make(unit_id, "info", &format!("Grid balancing response: f={:.3} Hz", freq_hz), Some(freq_hz))
    }

    /// Sensor loss / communication failure.
    pub fn sensor_loss(unit_id: &str, source: &str) -> Alert {
        make(unit_id, "warning", &format!("Sensor/comm loss: {}", source), None)
    }

    /// Market price feed stale (arbitrage/MPC suppressed until it recovers).
    pub fn price_stale(unit_id: &str) -> Alert {
        make(unit_id, "warning", "Live price feed stale — arbitrage/MPC suppressed", None)
    }

    /// OTA upgrade notification.
    pub fn ota(unit_id: &str, version: &str) -> Alert {
        make(unit_id, "info", &format!("Firmware upgrade to {}", version), None)
    }

    /// Startup alert.
    pub fn startup(unit_id: &str) -> Alert {
        make(unit_id, "info", "System started", None)
    }

    /// Shutdown alert.
    pub fn startup_off(unit_id: &str) -> Alert {
        make(unit_id, "info", "System shutting down", None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_serializes_to_json() {
        let alert = factory::emergency("unit-1", -18.5, 600.0);
        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("\"severity\":\"critical\""));
        assert!(json.contains("\"unit_id\":\"unit-1\""));
        assert!(json.contains("event_id"));
    }

    #[test]
    fn test_factory_alerts_have_correct_severity() {
        assert_eq!(factory::emergency("u", -18.0, 100.0).severity, "critical");
        assert_eq!(factory::thermal_guard("u", -19.0, 4000.0).severity, "warning");
        assert_eq!(factory::grid_balancing("u", 50.03).severity, "info");
        assert_eq!(factory::startup("u").severity, "info");
    }

    #[test]
    fn test_alert_has_unique_event_ids() {
        let a1 = factory::startup("u");
        let a2 = factory::startup("u");
        assert_ne!(a1.event_id, a2.event_id);
    }
}