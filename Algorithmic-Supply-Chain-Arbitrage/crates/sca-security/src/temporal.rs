//! Temporal Isolation (Fixed-Slot Scheduling).
//!
//! Time-windowed execution isolation for operations that must not
//! overlap. Prevents race conditions in critical settlement and
//! price discovery operations. Each operation type is assigned
//! a fixed time slot; operations outside their slot are queued
//! or rejected.

use std::collections::HashMap;
use std::sync::Mutex;

/// A time slot for a specific operation type.
#[derive(Debug, Clone)]
pub struct TimeSlot {
    /// Name of the operation.
    pub operation: String,
    /// Slot start (seconds within the hour, 0-3599).
    pub start_sec: u64,
    /// Slot duration in seconds.
    pub duration_secs: u64,
    /// Whether this slot allows concurrent operations.
    pub concurrent: bool,
}

/// A scheduling window — the repeating schedule for all operations.
#[derive(Debug, Clone)]
pub struct Schedule {
    slots: Vec<TimeSlot>,
    /// Cycle length in seconds (default: 3600 = 1 hour).
    cycle_secs: u64,
}

impl Schedule {
    pub fn new(cycle_secs: u64) -> Self {
        Self {
            slots: Vec::new(),
            cycle_secs,
        }
    }

    /// Add a time slot to the schedule.
    pub fn add_slot(&mut self, slot: TimeSlot) {
        assert!(
            slot.start_sec + slot.duration_secs <= self.cycle_secs,
            "Slot exceeds cycle length"
        );
        self.slots.push(slot);
    }

    /// Get the slot for an operation at a given time offset.
    pub fn slot_at(&self, operation: &str, time_offset: u64) -> Option<&TimeSlot> {
        let t = time_offset % self.cycle_secs;
        self.slots.iter().find(|s| {
            s.operation == operation
                && t >= s.start_sec
                && t < s.start_sec + s.duration_secs
        })
    }

    /// Check if an operation is allowed at a given time.
    pub fn is_allowed(&self, operation: &str, time_offset: u64) -> bool {
        self.slot_at(operation, time_offset).is_some()
    }

    /// Check for overlapping slots.
    pub fn validate_no_overlaps(&self) -> Result<(), String> {
        for (i, a) in self.slots.iter().enumerate() {
            for b in self.slots.iter().skip(i + 1) {
                let a_start = a.start_sec;
                let a_end = a.start_sec + a.duration_secs;
                let b_start = b.start_sec;
                let b_end = b.start_sec + b.duration_secs;

                if a_start < b_end && b_start < a_end && !a.concurrent && !b.concurrent {
                    return Err(format!(
                        "Overlap detected: {} ({}-{}) and {} ({}-{})",
                        a.operation, a_start, a_end, b.operation, b_start, b_end
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn slots(&self) -> &[TimeSlot] {
        &self.slots
    }
}

/// The temporal isolation scheduler.
///
/// Ensures critical operations run in their allocated time windows.
pub struct TemporalScheduler {
    schedule: Schedule,
    /// Track running operations.
    active_operations: Mutex<HashMap<String, u64>>,
    /// Start time (Unix timestamp when scheduler was created).
    epoch_start: i64,
}

impl TemporalScheduler {
    /// Create a new scheduler with a default supply chain schedule.
    pub fn new() -> Self {
        let mut schedule = Schedule::new(3600); // 1-hour cycles

        // Slot 1: Settlement operations (first 5 min of each hour)
        schedule.add_slot(TimeSlot {
            operation: "settlement".into(),
            start_sec: 0,
            duration_secs: 300,
            concurrent: false,
        });

        // Slot 2: SMPC price discovery (min 5-15)
        schedule.add_slot(TimeSlot {
            operation: "price_discovery".into(),
            start_sec: 300,
            duration_secs: 600,
            concurrent: false,
        });

        // Slot 3: Matching engine (min 15-25)
        schedule.add_slot(TimeSlot {
            operation: "matching".into(),
            start_sec: 900,
            duration_secs: 600,
            concurrent: false,
        });

        // Slot 4: P2P capacity negotiation (min 25-35)
        schedule.add_slot(TimeSlot {
            operation: "capacity_negotiation".into(),
            start_sec: 1500,
            duration_secs: 600,
            concurrent: true,
        });

        Self {
            schedule,
            active_operations: Mutex::new(HashMap::new()),
            epoch_start: chrono::Utc::now().timestamp(),
        }
    }

    /// Current time offset in the schedule cycle.
    fn current_offset(&self) -> u64 {
        let elapsed = (chrono::Utc::now().timestamp() - self.epoch_start).max(0) as u64;
        elapsed % self.schedule.cycle_secs
    }

    /// Try to acquire a slot for an operation.
    /// Returns Ok(true) if the operation may proceed.
    pub fn acquire_slot(&self, operation: &str) -> Result<bool, String> {
        let offset = self.current_offset();
        let slot = self
            .schedule
            .slot_at(operation, offset)
            .ok_or_else(|| {
                format!(
                    "Operation '{operation}' not in its time slot (offset={offset}s)"
                )
            })?;

        // Check if slot already has a running operation (non-concurrent only)
        if !slot.concurrent {
            let mut active = self.active_operations.lock().unwrap();
            if active.contains_key(operation) {
                return Err(format!(
                    "Operation '{operation}' already running in non-concurrent slot"
                ));
            }
            active.insert(operation.to_string(), offset);
        }

        Ok(true)
    }

    /// Release a slot when an operation completes.
    pub fn release_slot(&self, operation: &str) {
        if let Ok(mut active) = self.active_operations.lock() {
            active.remove(operation);
        }
    }

    /// Get the current schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Generate a human-readable schedule overview.
    pub fn schedule_overview(&self) -> Vec<String> {
        self.schedule
            .slots()
            .iter()
            .map(|s| {
                let start_min = s.start_sec / 60;
                let end_min = (s.start_sec + s.duration_secs) / 60;
                format!(
                    "{:>20}: {:02}:{:02}-{:02}:{:02} {}",
                    s.operation,
                    start_min / 60,
                    start_min % 60,
                    end_min / 60,
                    end_min % 60,
                    if s.concurrent { "(concurrent)" } else { "(exclusive)" }
                )
            })
            .collect()
    }
}

impl Default for TemporalScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_no_overlaps() {
        let scheduler = TemporalScheduler::new();
        assert!(scheduler.schedule().validate_no_overlaps().is_ok());
    }

    #[test]
    fn test_acquire_settlement_slot() {
        let scheduler = TemporalScheduler::new();
        // Settlement is slot 0..300s of each hour
        // We can't control time, but we can verify the schedule exists
        let slots = scheduler.schedule().slots();
        assert!(slots.iter().any(|s| s.operation == "settlement"));
    }

    #[test]
    fn test_schedule_overview() {
        let scheduler = TemporalScheduler::new();
        let overview = scheduler.schedule_overview();
        assert_eq!(overview.len(), 4);
        assert!(overview[0].contains("settlement"));
        assert!(overview[1].contains("price_discovery"));
        assert!(overview[2].contains("matching"));
    }

    #[test]
    fn test_cycle_length() {
        let scheduler = TemporalScheduler::new();
        assert_eq!(scheduler.schedule().cycle_secs, 3600);
    }

    #[test]
    fn test_detect_overlap() {
        let mut schedule = Schedule::new(3600);
        schedule.add_slot(TimeSlot {
            operation: "a".into(),
            start_sec: 0,
            duration_secs: 600,
            concurrent: false,
        });
        schedule.add_slot(TimeSlot {
            operation: "b".into(),
            start_sec: 300,
            duration_secs: 600,
            concurrent: false,
        });

        assert!(schedule.validate_no_overlaps().is_err());
    }
}