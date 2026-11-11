//! Hardware Watchdog & Systemd Supervisor Interface.
//!
//! Provides industrial fail-safe heartbeat protection for edge gateways:
//! - Pings `/dev/watchdog` (Linux hardware watchdog timer) on each healthy loop iteration.
//! - Pings systemd watchdog (`WATCHDOG=1`) when supervised by systemd.
//! - Cross-platform abstraction: provides simulated/mock behavior on non-Linux systems.
//! - Safe disarm: writes magic character `'V'` before closing `/dev/watchdog` on graceful shutdown.

use crate::config::WatchdogConfig;
use std::time::Instant;

/// Watchdog supervisor instance.
#[derive(Debug)]
pub struct Watchdog {
    config: WatchdogConfig,
    last_ping: Instant,
    ping_count: u64,
    disarmed: bool,
    #[cfg(target_os = "linux")]
    device_file: Option<std::fs::File>,
}

impl Watchdog {
    /// Initialize a watchdog supervisor with the given configuration.
    pub fn new(config: WatchdogConfig) -> Self {
        #[cfg(target_os = "linux")]
        let device_file = if config.enabled {
            match std::fs::OpenOptions::new().write(true).open(&config.device_path) {
                Ok(f) => {
                    log::info!("🛡️ Hardware watchdog opened at {}", config.device_path);
                    Some(f)
                }
                Err(e) => {
                    log::warn!("⚠️ Failed to open hardware watchdog {}: {}", config.device_path, e);
                    None
                }
            }
        } else {
            None
        };

        if config.enabled {
            log::info!(
                "🛡️ Watchdog supervisor initialized (interval: {:.1}s, systemd_notify: {})",
                config.ping_interval_sec,
                config.systemd_notify
            );
        }

        Self {
            config,
            last_ping: Instant::now(),
            ping_count: 0,
            disarmed: false,
            #[cfg(target_os = "linux")]
            device_file,
        }
    }

    /// Ping the watchdog supervisor to reset the hardware timeout counter.
    ///
    /// Respects `ping_interval_sec` to avoid flooding the device node with writes.
    pub fn ping(&mut self) {
        if !self.config.enabled || self.disarmed {
            return;
        }

        let now = Instant::now();
        if self.ping_count > 0 && now.duration_since(self.last_ping).as_secs_f64() < self.config.ping_interval_sec {
            return;
        }

        self.last_ping = now;
        self.ping_count = self.ping_count.saturating_add(1);

        // Hardware device ping on Linux
        #[cfg(target_os = "linux")]
        if let Some(ref mut file) = self.device_file {
            use std::io::Write;
            if let Err(e) = file.write_all(b"\0") {
                log::warn!("⚠️ Failed to ping hardware watchdog: {}", e);
            }
        }

        // Systemd notify ping on Linux
        #[cfg(target_os = "linux")]
        if self.config.systemd_notify {
            if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
                if !socket_path.is_empty() {
                    use std::os::unix::net::UnixDatagram;
                    if let Ok(sock) = UnixDatagram::unbound() {
                        let _ = sock.send_to(b"WATCHDOG=1", socket_path);
                    }
                }
            }
        }

        log::trace!("🛡️ Watchdog heartbeat #{}", self.ping_count);
    }

    /// Disarm the watchdog for clean, intentional process shutdown.
    ///
    /// On Linux `/dev/watchdog`, writing `'V'` before closing prevents the kernel
    /// driver from triggering an immediate hardware reboot upon service stop.
    pub fn disarm(&mut self) {
        if !self.config.enabled || self.disarmed {
            return;
        }

        #[cfg(target_os = "linux")]
        if let Some(ref mut file) = self.device_file {
            use std::io::Write;
            if let Err(e) = file.write_all(b"V") {
                log::warn!("⚠️ Failed to write magic disarm byte to watchdog: {}", e);
            } else {
                let _ = file.flush();
                log::info!("🛡️ Hardware watchdog cleanly disarmed ('V' magic byte written)");
            }
        }

        self.disarmed = true;
        log::info!("🛡️ Watchdog supervisor disarmed cleanly (total heartbeats: {})", self.ping_count);
    }

    /// Get the total number of heartbeats emitted.
    pub fn ping_count(&self) -> u64 {
        self.ping_count
    }

    /// Whether the watchdog is currently armed and enabled.
    pub fn is_armed(&self) -> bool {
        self.config.enabled && !self.disarmed
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        if self.config.enabled && !self.disarmed {
            self.disarm();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_disabled_is_noop() {
        let cfg = WatchdogConfig {
            enabled: false,
            ..WatchdogConfig::default()
        };
        let mut wd = Watchdog::new(cfg);
        assert!(!wd.is_armed());
        wd.ping();
        assert_eq!(wd.ping_count(), 0);
        wd.disarm();
        assert!(!wd.is_armed());
    }

    #[test]
    fn test_watchdog_enabled_ping_and_disarm() {
        let cfg = WatchdogConfig {
            enabled: true,
            ping_interval_sec: 0.0, // Allow immediate pings for test
            ..WatchdogConfig::default()
        };
        let mut wd = Watchdog::new(cfg);
        assert!(wd.is_armed());

        wd.ping();
        assert_eq!(wd.ping_count(), 1);

        wd.ping();
        assert_eq!(wd.ping_count(), 2);

        wd.disarm();
        assert!(!wd.is_armed());

        // Further pings after disarm should be ignored
        wd.ping();
        assert_eq!(wd.ping_count(), 2);
    }
}
