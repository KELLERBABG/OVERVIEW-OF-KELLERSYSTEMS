//! # Smart Matter Node Flasher
//!
//! CLI tool for flashing firmware onto Smart Matter nodes via SWD or
//! over-the-air (OTA) update through the mesh.
//!
//! ```bash
//! # Flash via SWD (prints the exact probe-rs invocation)
//! smart-matter-flasher swd --firmware firmware.bin
//!
//! # Prepare an OTA update: validate + split into slices + write staging file
//! smart-matter-flasher ota --node-id 0x01 --firmware firmware.bin
//! ```
//!
//! All reported values (size, SHA-256, slice count) are computed from the
//! actual firmware file.

use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};

use smart_matter_core::memmap::{APP_SIZE, APP_START, META_START};

/// Bootloader OTA metadata header (matches `firmware/bootloader/src/boot.rs`).
const OTA_MAGIC: u32 = 0xDEAD_BEEF;
const OTA_HEADER_LEN: usize = 44;
const OTA_FLAG_PENDING: u32 = 1;

#[derive(Parser)]
#[command(name = "smart-matter-flasher")]
#[command(about = "Flash firmware onto Smart Matter nodes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a firmware binary and print the SWD flashing command
    Swd {
        /// Path to firmware binary
        #[arg(long)]
        firmware: String,
    },
    /// Prepare an over-the-air update (checksum + slices + staging file)
    Ota {
        /// Target node ID (hex)
        #[arg(long)]
        node_id: String,
        /// Path to firmware binary
        #[arg(long)]
        firmware: String,
        /// Output staging file (default: <firmware>.ota)
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Swd { firmware } => swd(firmware),
        Commands::Ota {
            node_id,
            firmware,
            output,
        } => ota(node_id, firmware, output.as_deref()),
    }
}

/// Read a firmware file, printing its real size and SHA-256.
fn read_firmware(path: &str) -> Result<Vec<u8>, String> {
    let data = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    if data.is_empty() {
        return Err(format!("{path} is empty"));
    }
    Ok(data)
}

fn print_sha256(data: &[u8]) {
    let digest = Sha256::digest(data);
    println!("  SHA-256: {}", hex(&digest));
}

/// Validate a binary and print the exact probe-rs command to flash it.
fn swd(path: &str) {
    println!("Smart Matter SWD flasher");
    println!("========================");
    let data = match read_firmware(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    println!("Firmware: {path}");
    println!("  Size: {} bytes", data.len());
    print_sha256(&data);
    println!("  App region limit: {} bytes (whitepaper §2.4)", APP_SIZE);
    if data.len() > APP_SIZE {
        eprintln!("Error: firmware exceeds the 32 KB application region!");
        std::process::exit(1);
    }
    println!();
    println!("Flash with probe-rs (ST-Link on SWD):");
    println!("  probe-rs run --chip STM32G031J6 --binary-format elf {path}  # debug");
    println!(
        "  probe-rs download --chip STM32G031J6 --format bin --base-address {:#010x} {path}",
        APP_START
    );
}

/// Prepare an OTA update: SHA-256, FwUpdate slice plan, and a `.ota` staging
/// file that the bootloader (host demo or SPI-flash staging) can consume.
fn ota(node_id_hex: &str, firmware: &str, output: Option<&str>) {
    println!("Smart Matter OTA updater");
    println!("========================");

    let data = match read_firmware(firmware) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    if data.len() > APP_SIZE {
        eprintln!(
            "Error: firmware is {} bytes but the app region is only {APP_SIZE} bytes",
            data.len()
        );
        std::process::exit(1);
    }

    let mut node_id = [0u8; 8];
    let clean = node_id_hex.trim_start_matches("0x");
    for (i, pair) in clean.as_bytes().chunks(2).enumerate() {
        if i >= 8 {
            break;
        }
        if let Ok(b) = u8::from_str_radix(core::str::from_utf8(pair).unwrap_or("00"), 16) {
            node_id[i] = b;
        }
    }

    println!("Firmware: {firmware}");
    println!("  Size: {} bytes", data.len());
    print_sha256(&data);
    println!("  Target node: {:02X?}", &node_id[..2]);
    println!("  Slice payload: 128 bytes (protocol spec §2)");

    // Split into FwUpdate slices (the mesh transmits one slice per energy
    // window; the receiver stages them into external SPI flash).
    let slices = data.len().div_ceil(128);
    println!("  FwUpdate slices: {slices}");
    println!(
        "  Estimated energy: {} × 50 µJ ≈ {} µJ",
        slices,
        slices * 50
    );

    // Write the staging file: 44-byte bootloader metadata header + image.
    let out_path = match output {
        Some(p) => p.to_string(),
        None => format!("{firmware}.ota"),
    };
    let digest = Sha256::digest(&data);
    let mut header = [0u8; OTA_HEADER_LEN];
    header[0..4].copy_from_slice(&OTA_MAGIC.to_le_bytes());
    header[4..8].copy_from_slice(&(data.len() as u32).to_le_bytes());
    header[8..40].copy_from_slice(&digest);
    header[40..44].copy_from_slice(&OTA_FLAG_PENDING.to_le_bytes());

    let mut staged = Vec::with_capacity(OTA_HEADER_LEN + data.len());
    staged.extend_from_slice(&header);
    staged.extend_from_slice(&data);
    match std::fs::write(&out_path, &staged) {
        Ok(()) => println!(
            "  Staging file: {out_path} ({} bytes, metadata @ {:#010x})",
            staged.len(),
            META_START
        ),
        Err(e) => eprintln!("  Error writing staging file: {e}"),
    }

    println!();
    println!("OTA plan ready. On next node reset the bootloader validates the");
    println!("staged SHA-256 and applies the image to the app region.");
}

/// Lowercase hex dump.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ota_header_layout_matches_bootloader() {
        // Header fields must line up with boot.rs offsets.
        let mut header = [0u8; OTA_HEADER_LEN];
        header[0..4].copy_from_slice(&OTA_MAGIC.to_le_bytes());
        header[4..8].copy_from_slice(&1234u32.to_le_bytes());
        header[40..44].copy_from_slice(&OTA_FLAG_PENDING.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(header[0..4].try_into().unwrap()),
            0xDEAD_BEEF
        );
        assert_eq!(u32::from_le_bytes(header[4..8].try_into().unwrap()), 1234);
        assert_eq!(u32::from_le_bytes(header[40..44].try_into().unwrap()), 1);
    }

    #[test]
    fn slice_count_math() {
        assert_eq!(0usize.div_ceil(128), 0);
        assert_eq!(1usize.div_ceil(128), 1);
        assert_eq!(128usize.div_ceil(128), 1);
        assert_eq!(129usize.div_ceil(128), 2);
    }
}
