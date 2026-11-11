pub struct AlertConfig {
    pub pagerduty_key: Option<String>,
    pub slack_webhook: Option<String>,
}

pub struct AlertManager {
    pub config: AlertConfig,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        Self { config }
    }

    pub fn send_alert(&self, severity: &str, message: &str) {
        tracing::warn!("[{}] {}", severity, message);
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new(AlertConfig {
            pagerduty_key: None,
            slack_webhook: None,
        })
    }
}
