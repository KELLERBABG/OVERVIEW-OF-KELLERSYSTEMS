use serde::{Deserialize, Serialize};

use crate::financial::BuildingConfig;

/// Top-level configuration for the thermal arbitrage system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub thermal: ThermalConfig,
    #[serde(default)]
    pub control: ControlConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub sensor: SensorConfig,
    #[serde(default)]
    pub building: BuildingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub modbus: ModbusConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,
    #[serde(default)]
    pub ota: OtaConfig,
    #[serde(default)]
    pub live_data: LiveDataConfig,
    #[serde(default)]
    pub watchdog: WatchdogConfig,
}

/// System-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemConfig {
    pub unit_id: String,
    pub loop_interval_sec: f64,
    pub bundesland: String,
}

/// Thermal model parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThermalConfig {
    pub r_th: f64,
    pub c_th: f64,
    pub t_min: f64,
    pub t_max: f64,
    pub t_setpoint_normal: f64,
    pub t_setpoint_min: f64,
    pub t_setpoint_max: f64,
    /// RLS ambient-variance guard (°C): max allowed ambient swing over a
    /// compressor-OFF window before the C_th update is skipped (door openings,
    /// loading events distort the decay observation). 0.0 disables the guard.
    pub rls_ambient_variance_max_k: f64,
    /// Enable cargo core thermal lag modeling in control and safety checks.
    pub cargo_lag_enabled: bool,
    /// Thermal coupling resistance between air envelope and cargo (K/W).
    pub cargo_r_th: f64,
    /// Cargo thermal mass (J/K). Defaults to building product mass × specific heat.
    pub cargo_c_th: f64,
    /// Maximum critical cargo core temperature threshold (°C).
    pub cargo_t_max: f64,
}

/// Control engine parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlConfig {
    pub thermal_integral_max: f64,
    pub ttf_critical_sec: f64,
    pub reaction_time_sec: f64,
    pub droop_gain_k: f64,
    pub freq_nominal: f64,
    /// Minimum compressor runtime (seconds) — short-cycling protection.
    pub min_runtime_sec: f64,
    /// Minimum compressor off-time (seconds) — short-cycling protection.
    pub min_off_time_sec: f64,
    /// Grid balancing deadband (±Hz) — frequency must deviate beyond this to trigger response.
    pub grid_balancing_deadband_hz: f64,
    /// MPC cheap price threshold (€/MWh) — MPC will pre-cool if price is below this.
    pub mpc_cheap_threshold_eur_per_mwh: f64,
    /// Continuous VFD modulation: when true (AND a Modbus `compressor_speed_addr`
    /// register is configured), the Idle/thermostat mode runs the compressor at a
    /// heat-balance speed instead of hard on/off cycling. False (default) keeps
    /// the legacy on/off coil behavior. Forced-cooling modes (Emergency /
    /// ThermalGuard / fail-safe) always command 100% speed regardless.
    pub vfd_enabled: bool,
    /// VFD oil-return pulse cadence (seconds). When > 0 (and `vfd_enabled`), the
    /// gateway periodically forces a short high-speed run so oil returns to the
    /// compressor sump on legacy racks operating at low speed. The pulse respects
    /// min-run/min-offtime governor locks and never overrides an active grid
    /// response. 0 disables (default).
    pub vfd_oil_return_pulse_every_sec: f64,
    /// Oil-return pulse duration (seconds).
    pub vfd_oil_return_pulse_duration_sec: f64,
    /// Oil-return pulse speed (0..1, fraction of rated speed).
    pub vfd_oil_return_pulse_speed: f64,
    /// Intraday fast-path trigger (€/MWh): when the LIVE price deviates from the
    /// day-ahead forecast for the current hour by more than this, the pre-cool /
    /// shed decision is re-triggered immediately. 0 disables (default).
    pub mpc_intraday_deviation_threshold_eur_per_mwh: f64,
    /// Enable dynamic price thresholds based on lookahead percentiles instead
    /// of static 30.0 / 80.0 EUR/MWh thresholds.
    pub dynamic_price_thresholds_enabled: bool,
    /// Lower percentile (0.0..1.0) for dynamic buy/pre-cool threshold (default 0.25).
    pub dynamic_price_charge_percentile: f64,
    /// Upper percentile (0.0..1.0) for dynamic shed threshold (default 0.75).
    pub dynamic_price_shed_percentile: f64,
    /// Proportional gain for dynamic VFD temperature tracking (default 0.10).
    pub vfd_kp: f64,
    /// Integral gain for dynamic VFD temperature tracking (default 0.001).
    pub vfd_ki: f64,
    /// Slew rate limit for setpoint transitions (°C/hour). Prevents thermal shock / liquid hammer.
    /// Default 6.0 °C/h (1.0 °C per 10 minutes). 0.0 disables limiting.
    pub max_setpoint_slew_c_per_hour: f64,
    /// High ambient temperature threshold (°C) above which shedding and aggressive precooling
    /// are derated/suppressed to protect compressors from high-pressure / thermal over-stress.
    /// Default 28.0 °C.
    pub derate_ambient_c_threshold: f64,
}

/// Data logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub csv_path: String,
    pub interval_sec: f64,
}

/// HTTP API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub bind_address: String,
    /// Optional TLS certificate chain path (PEM). If None, serves plain HTTP.
    pub tls_cert_path: Option<String>,
    /// Optional TLS private key path (PEM). If None, serves plain HTTP.
    pub tls_key_path: Option<String>,
}

/// Security (authentication) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// A secret key used to derive API bearer tokens (HMAC-SHA256).
    /// In production, set via environment variable `ATA_SECRET`.
    pub api_secret: String,
    /// Required for writes (POST/PUT). If empty, all routes are unauthenticated.
    pub auth_enabled: bool,
}

/// Modbus PLC interface configuration (supports Modbus TCP and Modbus RTU / RS-485).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModbusConfig {
    pub enabled: bool,
    /// Protocol variant: "tcp" (default) or "rtu" (serial RS-485).
    pub protocol: String,
    /// PLC host address for Modbus TCP (e.g. "192.168.0.10")
    pub host: String,
    /// Modbus TCP port (default 502)
    pub port: u16,
    /// Serial device path for Modbus RTU (e.g. "/dev/ttyUSB0" or "COM3")
    pub serial_port: Option<String>,
    /// Baud rate for Modbus RTU (default 9600)
    pub baud_rate: u32,
    /// Modbus slave Unit ID / Station address (default 1)
    pub unit_id: u8,
    /// Poll interval in seconds.
    pub poll_interval_sec: f64,
    /// Register map addresses.
    pub registers: ModbusRegisters,
}

/// Hardware watchdog & supervisor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WatchdogConfig {
    pub enabled: bool,
    /// Path to hardware watchdog character device on Linux (default: "/dev/watchdog").
    pub device_path: String,
    /// Heartbeat ping interval in seconds (default: 5.0).
    pub ping_interval_sec: f64,
    /// Whether to notify systemd (WATCHDOG=1) if running under systemd supervisor (default: true).
    pub systemd_notify: bool,
}

/// Modbus register address map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModbusRegisters {
    /// Internal temperature (signed int16, °C × 100) — coil/register address.
    pub internal_temp_addr: u16,
    /// Compressor command (bool) address.
    pub compressor_cmd_addr: u16,
    /// Ambient temperature input (signed int16, °C × 100).
    pub ambient_temp_addr: u16,
    /// Alarm output (bool).
    pub alarm_addr: u16,
    /// Optional grid frequency input (signed int16, Hz × 100, e.g. 5000 = 50.00 Hz).
    /// If set, the gateway uses the meter's REAL frequency for grid balancing;
    /// otherwise frequency falls back to the live-data source, then the dataset.
    pub grid_frequency_addr: Option<u16>,
    /// Optional VFD compressor speed register (uint16, 0..1000 = 0.0..100.0 %).
    /// When set, the speed register is the AUTHORITATIVE compressor command
    /// (on/off decisions map to 1000/0) and the coil is not written — the
    /// gateway then modulates continuously when `control.vfd_enabled = true`.
    pub compressor_speed_addr: Option<u16>,
    /// Optional heartbeat / watchdog register on the PLC (uint16 incrementing counter).
    /// When set, the gateway increments and writes this register on every poll cycle.
    /// The PLC watchdog monitors this to detect gateway hangs and fall back to local control.
    pub heartbeat_addr: Option<u16>,
}

/// Alerting configuration (webhook, retry policy).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertsConfig {
    pub enabled: bool,
    /// Webhook URL to receive JSON alert POSTs (e.g. Slack/Discord/Teams).
    pub webhook_url: Option<String>,
    /// Max retries before dropping an alert.
    pub max_retries: u32,
    /// Retry backoff base in seconds (exponential: base * 2^attempt).
    pub retry_backoff_sec: u64,
}

/// OTA firmware update configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OtaConfig {
    pub enabled: bool,
    /// URL template for the firmware manifest. `{version}` is substituted with the target version.
    pub manifest_url: Option<String>,
    /// Ed25519 public key (hex) used to verify firmware signatures.
    pub verify_key_hex: String,
    /// Check interval in seconds.
    pub check_interval_sec: f64,
    /// Directory where downloaded firmware bundles are staged.
    pub staging_dir: String,
}

/// Live market/weather data configuration (production real-time mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveDataConfig {
    pub enabled: bool,
    /// How often to refresh day-ahead prices (seconds).
    pub price_refresh_sec: f64,
    /// How often to refresh weather (seconds).
    pub weather_refresh_sec: f64,
    /// Open-Meteo forecast API base URL (default: https://api.open-meteo.com/v1/forecast).
    pub open_meteo_url: String,
    /// Latitude for weather fetch.
    pub latitude: f64,
    /// Longitude for weather fetch.
    pub longitude: f64,
    /// Optional ENTSO-E Transparency Platform API key.
    ///
    /// ENTSO-E is the official European transmission-system-operator data hub
    /// and the most reliable free day-ahead price source. A key is obtained
    /// free at https://transparency.entsoe.eu (request "Data via API" access).
    /// Prefer setting it via the `ENTSOE_API_KEY` environment variable so the
    /// secret never lands in config files. When set, it is tried as the
    /// primary price source (energy-charts.info and SMARD remain fallbacks).
    #[serde(default)]
    pub entsoe_api_key: Option<String>,
    /// A day-ahead price is considered STALE after this many hours without a
    /// successful refresh. While stale, arbitrage/pre-cooling are suppressed
    /// (thermostat + safety modes keep running) and an alert is raised.
    pub price_max_age_hours: f64,
    /// Optional JSON endpoint returning real grid frequency, e.g.
    /// `{"frequency_hz": 50.01}` (contract documented in DEPLOYMENT.md).
    /// Recommended production path is the Modbus `grid_frequency_addr`
    /// register (real meter); this URL source is for operators who have a
    /// frequency publisher but no meter register. Falls back to dataset.
    #[serde(default)]
    pub frequency_url: Option<String>,
}

/// Sensor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorConfig {
    pub simulate: bool,
    pub ambient_temp_c: f64,
    pub initial_temp_c: f64,
    pub cooling_power_w: f64,
    pub heat_leak_w: f64,
}

impl Default for SystemConfig {
    fn default() -> Self { Self { unit_id: "nrw-store-001".into(), loop_interval_sec: 1.0, bundesland: "NW".into() } }
}
impl Default for ThermalConfig {
    fn default() -> Self {
        Self {
            r_th: 0.05,
            c_th: 1_000_000.0,
            t_min: -24.0,
            t_max: -18.0,
            t_setpoint_normal: -20.0,
            t_setpoint_min: -24.0,
            t_setpoint_max: -18.0,
            rls_ambient_variance_max_k: 3.0,
            cargo_lag_enabled: false,
            cargo_r_th: 0.005,
            cargo_c_th: 400_000_000.0,
            cargo_t_max: -18.0,
        }
    }
}
impl Default for ControlConfig {
    fn default() -> Self {
        Self { thermal_integral_max: 3600.0, ttf_critical_sec: 900.0, reaction_time_sec: 60.0,
            droop_gain_k: 1000.0, freq_nominal: 50.0, min_runtime_sec: 900.0, min_off_time_sec: 600.0,
            grid_balancing_deadband_hz: 0.1, mpc_cheap_threshold_eur_per_mwh: 25.0,
            vfd_enabled: false,
            vfd_oil_return_pulse_every_sec: 0.0,
            vfd_oil_return_pulse_duration_sec: 30.0,
            vfd_oil_return_pulse_speed: 1.0,
            mpc_intraday_deviation_threshold_eur_per_mwh: 0.0,
            dynamic_price_thresholds_enabled: false,
            dynamic_price_charge_percentile: 0.25,
            dynamic_price_shed_percentile: 0.75,
            vfd_kp: 0.10,
            vfd_ki: 0.001,
            max_setpoint_slew_c_per_hour: 6.0,
            derate_ambient_c_threshold: 28.0,
        }
    }
}
impl Default for LoggingConfig {
    fn default() -> Self { Self { csv_path: "thermal_arbitrage_log.csv".into(), interval_sec: 5.0 } }
}
impl Default for ApiConfig {
    fn default() -> Self { Self { bind_address: "0.0.0.0:8085".into(), tls_cert_path: None, tls_key_path: None } }
}
impl Default for SecurityConfig {
    // Auth ON by default: a config file that omits [security] must NOT leave
    // every mutating endpoint unauthenticated (fail-open by omission).
    fn default() -> Self { Self { api_secret: "change-me-in-production".into(), auth_enabled: true } }
}
impl Default for ModbusRegisters {
    fn default() -> Self { Self { internal_temp_addr: 0, compressor_cmd_addr: 1, ambient_temp_addr: 2, alarm_addr: 3, grid_frequency_addr: None, compressor_speed_addr: None, heartbeat_addr: None } }
}
impl Default for ModbusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: "tcp".into(),
            host: "192.168.0.10".into(),
            port: 502,
            serial_port: None,
            baud_rate: 9600,
            unit_id: 1,
            poll_interval_sec: 2.0,
            registers: ModbusRegisters::default(),
        }
    }
}
impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            device_path: "/dev/watchdog".into(),
            ping_interval_sec: 5.0,
            systemd_notify: true,
        }
    }
}
impl Default for AlertsConfig {
    fn default() -> Self { Self { enabled: false, webhook_url: None, max_retries: 5, retry_backoff_sec: 2 } }
}
impl Default for OtaConfig {
    fn default() -> Self { Self { enabled: false, manifest_url: None, verify_key_hex: String::new(), check_interval_sec: 3600.0, staging_dir: "ota_staging".into() } }
}
impl Default for LiveDataConfig {
    fn default() -> Self { Self { enabled: false, price_refresh_sec: 900.0, weather_refresh_sec: 3600.0, open_meteo_url: "https://api.open-meteo.com/v1/forecast".into(), latitude: 51.45, longitude: 7.01, entsoe_api_key: None, price_max_age_hours: 6.0, frequency_url: None } }
}
impl Default for SensorConfig {
    fn default() -> Self { Self { simulate: true, ambient_temp_c: 25.0, initial_temp_c: -20.0, cooling_power_w: 5000.0, heat_leak_w: 500.0 } }
}

impl Config {
    /// Load config.toml. Partial files now MERGE with defaults per section
    /// (`#[serde(default)]`), so a minimal site config no longer silently
    /// reverts the whole gateway to simulation-mode defaults. Parse errors are
    /// logged loudly instead of being swallowed.
    pub fn load(path: &str) -> Self {
        let candidate_paths = [
            std::env::var("ATA_CONFIG").ok(),
            Some(path.to_string()),
            Some(format!("crates/gateway/{}", path)),
        ];

        let content = candidate_paths
            .into_iter()
            .flatten()
            .find_map(|p| std::fs::read_to_string(&p).ok());

        match content {
            Some(content) => match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    // Environment override for the API secret — never bake secrets into config files.
                    if let Ok(secret) = std::env::var("ATA_SECRET") {
                        config.security.api_secret = secret;
                    }
                    if let Ok(key) = std::env::var("ENTSOE_API_KEY") {
                        config.live_data.entsoe_api_key = Some(key);
                    }
                    config
                }
                Err(e) => {
                    log::error!(
                        "⚠️  {path} is invalid ({}). Falling back to DEFAULT config — the gateway will run in simulation mode until the file is fixed.",
                        e
                    );
                    Config::default()
                }
            },
            None => {
                log::info!("ℹ️  No {path} found — using default configuration.");
                let mut config = Config::default();
                if let Ok(secret) = std::env::var("ATA_SECRET") {
                    config.security.api_secret = secret;
                }
                if let Ok(key) = std::env::var("ENTSOE_API_KEY") {
                    config.live_data.entsoe_api_key = Some(key);
                }
                config
            }
        }
    }

    pub fn time_constant(&self) -> f64 {
        self.thermal.r_th * self.thermal.c_th
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_config_merges_with_defaults() {
        // A minimal site config (like the DEPLOYMENT.md example) must MERGE with
        // defaults — not silently replace the whole config with simulation defaults.
        let path = std::env::temp_dir().join("ata_test_partial_config.toml");
        std::fs::write(
            &path,
            "[system]\nunit_id = \"test-1\"\nloop_interval_sec = 2.0\n",
        )
        .unwrap();
        let cfg = Config::load(path.to_str().unwrap());
        std::fs::remove_file(&path).ok();
        assert_eq!(cfg.system.unit_id, "test-1");
        assert_eq!(cfg.system.loop_interval_sec, 2.0);
        // Unspecified sections keep their defaults:
        assert_eq!(cfg.thermal.t_max, -18.0);
        assert!(cfg.sensor.simulate);
        assert!(!cfg.modbus.enabled);
    }

    #[test]
    fn vfd_defaults_preserve_legacy_on_off() {
        // VFD modulation must be opt-in: default config keeps on/off semantics
        // (no speed register, modulation disabled) so existing sites are unchanged.
        let cfg = Config::default();
        assert!(!cfg.control.vfd_enabled);
        assert!(cfg.modbus.registers.compressor_speed_addr.is_none());
    }

    #[test]
    fn shipped_config_parses_and_is_simulation_profile() {
        // Config::load() falls back to defaults on a parse error, which hides a
        // broken file. Parse the shipped config strictly so CI catches invalid
        // TOML (e.g. duplicate tables) or the E2E test config being committed
        // over the real one.
        let cfg: Config = toml::from_str(include_str!("../config.toml"))
            .expect("thermal-arbitrage-mvp/config.toml must be valid TOML");
        assert_eq!(cfg.system.unit_id, "nrw-store-001");
        assert!(cfg.sensor.simulate, "shipped config must default to simulation mode");
        assert!(!cfg.modbus.enabled, "shipped config must not point at a PLC");
        assert_eq!(cfg.sensor.cooling_power_w, 5000.0);
        assert_eq!(cfg.thermal.rls_ambient_variance_max_k, 3.0);
        assert!(cfg.control.vfd_enabled);
        assert!(cfg.security.auth_enabled);
        assert_eq!(cfg.security.api_secret, "change-me-in-production");
    }
}