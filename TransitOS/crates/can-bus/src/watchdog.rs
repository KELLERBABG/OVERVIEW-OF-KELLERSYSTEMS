//! CAN message timeout watchdog.
//!
//! Monitors per-message-ID freshness and triggers an alert when no frame
//! is received within the configured deadline. Essential for ASIL-D
//! actuator supervision.

use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct WatchdogEntry {
    pub last_received: Instant,
    pub timeout: std::time::Duration,
    pub alive_counter: u64,
}

#[derive(Debug)]
pub struct CanWatchdog {
    entries: HashMap<u32, WatchdogEntry>,
    stale_ids: Vec<u32>,
}

impl CanWatchdog {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            stale_ids: Vec::new(),
        }
    }

    /// Register a CAN ID with its maximum allowed interval.
    pub fn register(&mut self, id: u32, timeout: std::time::Duration) {
        self.entries.insert(
            id,
            WatchdogEntry {
                last_received: Instant::now(),
                timeout,
                alive_counter: 0,
            },
        );
    }

    /// Feed a frame — resets the timer for its arbitration ID.
    pub fn feed(&mut self, id: u32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.last_received = Instant::now();
            entry.alive_counter += 1;
        }
    }

    /// Check all registered IDs and return any that have timed out.
    pub fn check(&mut self) -> Vec<u32> {
        let now = Instant::now();
        self.stale_ids.clear();

        for (&id, entry) in &self.entries {
            if now.duration_since(entry.last_received) > entry.timeout {
                self.stale_ids.push(id);
            }
        }

        self.stale_ids.clone()
    }

    /// True if any registered message has timed out.
    pub fn any_stale(&mut self) -> bool {
        !self.check().is_empty()
    }

    /// Get alive counter for a specific CAN ID.
    pub fn alive_count(&self, id: u32) -> u64 {
        self.entries.get(&id).map(|e| e.alive_counter).unwrap_or(0)
    }

    /// Number of currently stale CAN IDs.
    pub fn stale_count(&mut self) -> usize {
        self.check().len()
    }
}

impl Default for CanWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

/// Safety-critical CAN bus monitor combining CRC, sequence, and watchdog.
#[derive(Debug)]
pub struct SafetyCanMonitor {
    pub watchdog: CanWatchdog,
    pub crc_failures: u64,
    pub sequence_gaps: u64,
    pub stale_events: u64,
}

impl SafetyCanMonitor {
    pub fn new() -> Self {
        Self {
            watchdog: CanWatchdog::new(),
            crc_failures: 0,
            sequence_gaps: 0,
            stale_events: 0,
        }
    }

    /// Report a CRC failure.
    pub fn on_crc_failure(&mut self) {
        self.crc_failures += 1;
        tracing::warn!(crc_failures = self.crc_failures, "CAN CRC failure detected");
    }

    /// Report a sequence gap.
    pub fn on_sequence_gap(&mut self) {
        self.sequence_gaps += 1;
        tracing::warn!(
            sequence_gaps = self.sequence_gaps,
            "CAN sequence gap detected"
        );
    }

    /// Report a stale message event.
    pub fn on_stale(&mut self) {
        self.stale_events += 1;
        tracing::error!(
            stale_events = self.stale_events,
            "CAN message timeout — data may be stale"
        );
    }

    /// Check watchdog and update stale counter.
    pub fn check_watchdog(&mut self) -> Vec<u32> {
        let stale = self.watchdog.check();
        if !stale.is_empty() {
            for _ in &stale {
                self.on_stale();
            }
        }
        stale
    }

    /// True if the bus is in a safe state (no failures).
    pub fn is_healthy(&mut self) -> bool {
        self.crc_failures == 0 && self.sequence_gaps == 0 && !self.watchdog.any_stale()
    }
}

impl Default for SafetyCanMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_detects_timeout() {
        let mut wd = CanWatchdog::new();
        wd.register(0x100, std::time::Duration::from_millis(10));
        wd.feed(0x100);

        // Not stale immediately
        assert!(!wd.any_stale());

        // After waiting, should be stale
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(wd.any_stale());
    }

    #[test]
    fn watchdog_resets_on_feed() {
        let mut wd = CanWatchdog::new();
        wd.register(0x100, std::time::Duration::from_millis(50));
        wd.feed(0x100);

        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!wd.any_stale());

        wd.feed(0x100); // reset
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!wd.any_stale());
    }

    #[test]
    fn safety_monitor_healthy_state() {
        let mut monitor = SafetyCanMonitor::new();
        monitor
            .watchdog
            .register(0x100, std::time::Duration::from_secs(1));
        monitor.watchdog.feed(0x100);
        assert!(monitor.is_healthy());
    }

    #[test]
    fn safety_monitor_detects_crc_failure() {
        let mut monitor = SafetyCanMonitor::new();
        monitor.on_crc_failure();
        assert!(!monitor.is_healthy());
        assert_eq!(monitor.crc_failures, 1);
    }
}
