//! # Flash Memory Map (STM32G031J6, 64 KB)
//!
//! Single source of truth for the flash layout — used by the bootloader,
//! the application firmware, and the flashing tool. Matches whitepaper §2.4:
//!
//! ```text
//! 0x0800_0000 – 0x0800_3FFF  Bootloader        (16 KB)
//! 0x0800_4000 – 0x0800_BFFF  Application       (32 KB)
//! 0x0800_C000 – 0x0800_EEFF  Wear-level log    (12 KB)
//! 0x0800_F000 – 0x0800_F7FF  Configuration     (2 KB)
//! 0x0800_F800 – 0x0800_FFFF  OTA metadata      (2 KB)
//! ```

/// Bootloader region.
pub const BOOTLOADER_START: usize = 0x0800_0000;
pub const BOOTLOADER_SIZE: usize = 16 * 1024;

/// Application firmware region (linked with `firmware/node/memory.x`).
pub const APP_START: usize = 0x0800_4000;
pub const APP_SIZE: usize = 32 * 1024;

/// Wear-leveling log store (3 × 4 KB sectors).
pub const LOG_STORE_START: usize = 0x0800_C000;
pub const LOG_STORE_SIZE: usize = 12 * 1024;

/// Configuration / calibration region.
pub const CONFIG_START: usize = 0x0800_F000;
pub const CONFIG_SIZE: usize = 2 * 1024;

/// OTA update metadata region (last free sector).
pub const META_START: usize = 0x0800_F800;
pub const META_SIZE: usize = 2 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_map_fits_64kb() {
        assert_eq!(BOOTLOADER_START, 0x0800_0000);
        assert_eq!(APP_START, BOOTLOADER_START + BOOTLOADER_SIZE);
        assert_eq!(LOG_STORE_START, APP_START + APP_SIZE);
        assert_eq!(CONFIG_START, LOG_STORE_START + LOG_STORE_SIZE);
        assert_eq!(META_START, CONFIG_START + CONFIG_SIZE);
        assert_eq!(META_START + META_SIZE, 0x0801_0000); // 64 KB total
    }
}
