//! CAN bus interface for production vehicle integration.
//!
//! Provides CRC-validated CAN frame handling, sequence number tracking,
//! message timeout watchdog, and ISO-TP multi-frame reassembly for
//! payloads exceeding 8 bytes. Designed for ISO 26262 ASIL-D compliance.

pub mod crc;
pub mod frame;
pub mod shutdown;
pub mod watchdog;

pub use crc::*;
pub use frame::*;
pub use shutdown::*;
pub use watchdog::*;
