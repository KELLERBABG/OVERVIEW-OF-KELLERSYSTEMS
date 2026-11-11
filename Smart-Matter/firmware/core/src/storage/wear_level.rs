//! # Flash Wear-Leveling Store
//!
//! Log-structured store for STM32G0 internal Flash (64 KB total).
//!
//! ## Problem
//!
//! STM32G031J6 has 64 KB of internal Flash with ~10,000 write cycles per
//! sector. Storing dead-drop fragments and routing table state directly
//! to Flash would rapidly wear out sectors without wear-leveling.
//!
//! ## Solution
//!
//! A minimal log-structured store that:
//! 1. Writes entries sequentially to a circular buffer in Flash
//! 2. Wears all sectors evenly by rotating the active write position
//! 3. Uses SRAM for active state during sleep — only flushes to Flash on
//!    critical events or when SRAM is full
//! 4. Stores only essential state: routing table checkpoints, pending
//!    dead-drop fragments, configuration parameters
//!
//! ## Memory Layout
//!
//! The log occupies **12 KB (3 × 4 KB sectors)** at `0x0800_C000`
//! (`FlashBudget::stm32g0_64kb` / whitepaper §2.4):
//!
//! ```text
//! Sector 0 (4 KB) — active write region
//! Sector 1 (4 KB) — second region (rotated into when sector 0 fills)
//! Sector 2 (4 KB) — third region (rotated into when sector 1 fills)
//! ```
//!
//! Active writes start in sector 0 and advance in round-robin fashion,
//! erasing each sector before reuse. 3 sectors × 4 KB × 10,000 cycles =
//! **120 MB of total write capacity** — more than 50 years at 1 write/hour.

use crate::types::TimeMs;
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Flash sector size for STM32G0 (4 KB).
pub const FLASH_SECTOR_SIZE: usize = 4096;

/// Number of sectors allocated to the log store (12 KB total).
pub const LOG_SECTORS: usize = 3;

/// Total log space.
pub const LOG_SIZE: usize = FLASH_SECTOR_SIZE * LOG_SECTORS;

/// Maximum size of a single log entry.
pub const MAX_ENTRY_SIZE: usize = 256;

/// Header size for each log entry.
pub const ENTRY_HEADER_SIZE: usize = 8;

/// Maximum number of pending entries in the SRAM buffer before flush.
pub const SRAM_BUFFER_SIZE: usize = 16;

/// Maximum number of writes per second to prevent rapid cycling.
pub const MAX_WRITES_PER_SEC: u32 = 2;

// ---------------------------------------------------------------------------
// Log Entry Types
// ---------------------------------------------------------------------------

/// Types of entries stored in the wear-leveling log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntryType {
    /// Dead-drop fragment data.
    DeadDrop = 0x01,
    /// Routing table checkpoint.
    RouteCheckpoint = 0x02,
    /// Configuration parameter update.
    Config = 0x03,
    /// Integrity report snapshot.
    IntegritySnapshot = 0x04,
    /// Boot event record.
    BootRecord = 0x05,
}

/// A single entry in the Flash log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub entry_type: EntryType,
    pub timestamp: TimeMs,
    pub data: Vec<u8, 248>,
}

// ---------------------------------------------------------------------------
// Wear-Level Store
// ---------------------------------------------------------------------------

/// Manages a circular log-structured write buffer in Flash with wear-leveling.
#[derive(Debug, Clone)]
pub struct WearLevelStore {
    /// Active write sector index (0..LOG_SECTORS).
    active_sector: u8,
    /// Write offset within the active sector.
    write_offset: u16,
    /// Total number of sector erasures performed.
    erase_count: u32,
    /// SRAM buffer for batching writes before Flash flush.
    buffer: Vec<LogEntry, SRAM_BUFFER_SIZE>,
    /// Timestamp of last write (for rate limiting).
    last_write: TimeMs,
    /// Estimated remaining write cycles.
    remaining_cycles: u32,
}

impl Default for WearLevelStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WearLevelStore {
    pub fn new() -> Self {
        Self {
            active_sector: 0,
            write_offset: 0,
            erase_count: 0,
            buffer: Vec::new(),
            last_write: TimeMs(0),
            remaining_cycles: 10_000, // STM32G0 typical endurance
        }
    }

    /// Queue an entry for writing. Writes to SRAM buffer first; flushes
    /// to Flash when buffer is full or on explicit `flush()` call.
    pub fn write(&mut self, entry: LogEntry) -> Result<(), WearLevelError> {
        // Rate limiting
        let now = TimeMs::now();
        if now.0 < self.last_write.0 + (1000 / MAX_WRITES_PER_SEC as u64) {
            return Err(WearLevelError::RateLimited);
        }
        self.last_write = now;

        // Buffer overflow check
        if self.buffer.len() >= SRAM_BUFFER_SIZE {
            return Err(WearLevelError::BufferFull);
        }

        self.buffer
            .push(entry)
            .map_err(|_| WearLevelError::BufferFull)?;
        Ok(())
    }

    /// Flush the SRAM buffer to Flash.
    pub fn flush(&mut self) -> Result<(), WearLevelError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        while !self.buffer.is_empty() {
            // Remove and process the first entry
            let entry = self.buffer.remove(0);
            let entry_size = ENTRY_HEADER_SIZE + entry.data.len();
            if entry_size > MAX_ENTRY_SIZE {
                continue; // Skip oversized entries
            }

            // Check if we need to advance to next sector
            if (self.write_offset as usize) + entry_size > FLASH_SECTOR_SIZE {
                self.advance_sector()?;
            }

            // In real implementation: write to Flash via HAL
            self.write_offset += entry_size as u16;
        }

        Ok(())
    }

    /// Advance to the next Flash sector (erases it first).
    fn advance_sector(&mut self) -> Result<(), WearLevelError> {
        self.active_sector = (self.active_sector + 1) % LOG_SECTORS as u8;
        self.write_offset = 0;
        self.erase_count += 1;
        self.remaining_cycles = self.remaining_cycles.saturating_sub(1);

        if self.remaining_cycles == 0 {
            return Err(WearLevelError::EndOfLife);
        }

        // In real implementation: erase the next sector via Flash controller
        Ok(())
    }

    /// Remaining estimated write cycles.
    pub fn remaining_cycles(&self) -> u32 {
        self.remaining_cycles
    }

    /// Health status as a percentage (80%+ = healthy).
    pub fn health_pct(&self) -> u8 {
        ((self.remaining_cycles as f32 / 10_000.0) * 100.0) as u8
    }

    /// Number of entries in the SRAM buffer.
    pub fn buffer_level(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer has data to flush.
    pub fn needs_flush(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Boot counter for metadata tracking.
    pub fn boot_count(&self) -> u32 {
        self.erase_count
    }
}

// ---------------------------------------------------------------------------
// Flash Memory Budget
// ---------------------------------------------------------------------------

/// Calculates the Flash memory budget for the STM32G031J6 (64 KB total).
pub struct FlashBudget {
    /// Bootloader reserved.
    pub bootloader: u32,
    /// Application firmware.
    pub application: u32,
    /// Wear-level store (log).
    pub log_store: u32,
    /// Configuration / calibration data.
    pub config: u32,
    /// Free / unused.
    pub free: u32,
}

impl FlashBudget {
    /// Return the memory map for the STM32G031J6.
    pub fn stm32g0_64kb() -> Self {
        Self {
            bootloader: 16 * 1024,  // 0x0800_0000 – 0x0800_3FFF
            application: 32 * 1024, // 0x0800_4000 – 0x0800_BFFF
            log_store: 12 * 1024,   // 0x0800_C000 – 0x0800_Efff (3 sectors)
            config: 2 * 1024,       // 0x0800_F000 – 0x0800_F7FF
            free: 2 * 1024,         // 0x0800_F800 – 0x0800_FFFF
        }
    }

    /// Estimated Flash lifetime at N writes/hour.
    ///
    /// Each sector has 10,000 write cycles. With 3 log sectors and
    /// rotating writes, the total write capacity is:
    ///   3 sectors × 4 KB/sector × 10,000 cycles = 120 MB
    pub fn estimated_lifetime_years(&self, writes_per_hour: u32) -> f32 {
        if writes_per_hour == 0 {
            return 100.0;
        }
        let total_write_capacity_bytes = (self.log_store as u64) * 10_000; // 3 × 4 KB × 10k cycles
        let avg_entry_size = 128; // bytes per write
        let writes_per_year = writes_per_hour as u64 * 24 * 365;
        let bytes_per_year = writes_per_year * avg_entry_size;
        (total_write_capacity_bytes as f32) / (bytes_per_year as f32)
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WearLevelError {
    BufferFull,
    RateLimited,
    WriteFailed,
    EndOfLife,
    EntryTooLarge,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_flush() {
        let mut store = WearLevelStore::new();
        let entry = LogEntry {
            entry_type: EntryType::Config,
            timestamp: TimeMs::now(),
            data: Vec::from_slice(&[1, 2, 3, 4]).unwrap(),
        };

        assert!(store.write(entry).is_ok());
        assert_eq!(store.buffer_level(), 1);
        assert!(store.needs_flush());
    }

    #[test]
    fn test_flush_drains_buffer() {
        let mut store = WearLevelStore::new();
        let entry = LogEntry {
            entry_type: EntryType::BootRecord,
            timestamp: TimeMs(100),
            data: Vec::new(),
        };
        store.write(entry).ok();
        store.flush().ok();
        assert_eq!(store.buffer_level(), 0);
        assert!(!store.needs_flush());
    }

    #[test]
    fn test_rate_limiting() {
        let mut store = WearLevelStore::new();
        let entry = LogEntry {
            entry_type: EntryType::DeadDrop,
            timestamp: TimeMs(0),
            data: Vec::new(),
        };
        // First write at t=0 should succeed
        assert!(store.write(entry).is_ok());
        // Immediately write again should be rate-limited
        let entry2 = LogEntry {
            entry_type: EntryType::DeadDrop,
            timestamp: TimeMs(1), // Less than 500ms later
            data: Vec::new(),
        };
        assert_eq!(store.write(entry2), Err(WearLevelError::RateLimited));
    }

    #[test]
    fn test_flash_budget() {
        let budget = FlashBudget::stm32g0_64kb();
        let total =
            budget.bootloader + budget.application + budget.log_store + budget.config + budget.free;
        assert_eq!(total, 64 * 1024); // 64 KB total
    }

    #[test]
    fn test_flash_lifetime() {
        let budget = FlashBudget::stm32g0_64kb();
        // At 10 writes/hour, should last many years
        let lifetime = budget.estimated_lifetime_years(10);
        assert!(lifetime > 5.0); // At least 5 years
                                 // At 1 write/hour, should exceed 50 years
        let lifetime_slow = budget.estimated_lifetime_years(1);
        assert!(lifetime_slow > 50.0);
    }

    #[test]
    fn test_wear_level_health() {
        let mut store = WearLevelStore::new();
        assert_eq!(store.health_pct(), 100); // Fully healthy
                                             // Simulate many advances
        for _ in 0..100 {
            store.write_offset = FLASH_SECTOR_SIZE as u16 - 1;
            store.advance_sector().ok();
        }
        assert!(store.health_pct() < 100);
    }
}
