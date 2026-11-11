//! # Host demo: simulate a full OTA cycle
//!
//! Runs the bootloader logic against simulated flash/staging and prints the
//! decisions — useful as a desktop demonstration and for debugging the boot
//! sequence without hardware.
//!
//! ```bash
//! cargo run -p smart-matter-bootloader
//! ```

use sha2::{Digest, Sha256};
use smart_matter_core::memmap::{APP_SIZE, APP_START};

use crate::boot::{
    write_metadata, BootDecision, Bootloader, Flash, FlashError, StagedImage, UpdateMetadata,
    FLAG_PENDING,
};

const FLASH_SIZE: usize = 64 * 1024;
const FLASH_BASE: usize = 0x0800_0000;
const FLASH_EMPTY: u8 = 0xFF;

struct SimFlash {
    data: Vec<u8>,
}

impl SimFlash {
    fn new() -> Self {
        Self {
            data: vec![FLASH_EMPTY; FLASH_SIZE],
        }
    }
}

impl Flash for SimFlash {
    fn read(&self, addr: usize, buf: &mut [u8]) -> Result<(), FlashError> {
        let rel = addr.checked_sub(FLASH_BASE).ok_or(FlashError::ReadFailed)?;
        if rel + buf.len() > FLASH_SIZE {
            return Err(FlashError::ReadFailed);
        }
        buf.copy_from_slice(&self.data[rel..rel + buf.len()]);
        Ok(())
    }
    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), FlashError> {
        let rel = addr
            .checked_sub(FLASH_BASE)
            .ok_or(FlashError::WriteFailed)?;
        if rel + data.len() > FLASH_SIZE {
            return Err(FlashError::WriteFailed);
        }
        self.data[rel..rel + data.len()].copy_from_slice(data);
        Ok(())
    }
    fn erase_pages(&mut self, start: usize, size: usize) -> Result<(), FlashError> {
        let rel = start
            .checked_sub(FLASH_BASE)
            .ok_or(FlashError::EraseFailed)?;
        if rel + size > FLASH_SIZE {
            return Err(FlashError::EraseFailed);
        }
        self.data[rel..rel + size].fill(FLASH_EMPTY);
        Ok(())
    }
}

struct SimStaged(Vec<u8>);

impl StagedImage for SimStaged {
    fn len(&self) -> Result<usize, FlashError> {
        Ok(self.0.len())
    }
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, FlashError> {
        let n = self.0.len().saturating_sub(offset).min(buf.len());
        buf[..n].copy_from_slice(&self.0[offset..offset + n]);
        Ok(n)
    }
}

pub fn run() {
    println!("Smart Matter bootloader (host simulation)");
    println!("==========================================");

    // 1. Build a fake app image (vector table + code).
    let mut image = vec![0u8; 4096];
    image[0..4].copy_from_slice(&0x2000_2000u32.to_le_bytes());
    image[4..8].copy_from_slice(&(APP_START as u32 + 0x09).to_le_bytes());
    for (i, b) in image.iter_mut().enumerate().skip(8) {
        *b = (i & 0xFF) as u8;
    }

    // 2. Boot with no update pending → jump to existing app.
    let mut flash = SimFlash::new();
    let mut bl = Bootloader::new(&mut flash, SimStaged(Vec::new()));
    println!(
        "no update staged        → {:?} (correct)",
        bl.boot().unwrap()
    );

    // 3. Stage an update (metadata + staged image) and boot → applied.
    let digest = Sha256::digest(&image);
    let mut sha = [0u8; 32];
    sha.copy_from_slice(&digest);
    write_metadata(
        &mut flash,
        &UpdateMetadata {
            image_len: image.len() as u32,
            sha256: sha,
            flags: FLAG_PENDING,
        },
    )
    .unwrap();

    let mut bl = Bootloader::new(&mut flash, SimStaged(image.clone()));
    let decision = bl.boot().unwrap();
    println!("valid staged update     → {decision:?}");
    assert_eq!(decision, BootDecision::AppliedUpdate);
    assert_eq!(
        &flash.data[APP_START - FLASH_BASE..APP_START - FLASH_BASE + image.len()],
        &image[..],
        "app region must contain the new image"
    );

    // 4. Boot again → pending cleared, jump.
    let mut bl = Bootloader::new(&mut flash, SimStaged(Vec::new()));
    println!(
        "second boot              → {:?} (correct)",
        bl.boot().unwrap()
    );

    // 5. Corrupt staged image → rejected, old app kept.
    let mut flash2 = SimFlash::new();
    let mut corrupt = image.clone();
    corrupt[100] ^= 0xFF;
    write_metadata(
        &mut flash2,
        &UpdateMetadata {
            image_len: corrupt.len() as u32,
            sha256: sha,
            flags: FLAG_PENDING,
        },
    )
    .unwrap();
    let mut bl = Bootloader::new(&mut flash2, SimStaged(corrupt));
    println!(
        "corrupt staged update    → {:?} (fail-closed, app untouched)",
        bl.boot().unwrap()
    );
    assert!(
        flash2.data[APP_START - FLASH_BASE..APP_START - FLASH_BASE + 4096]
            .iter()
            .all(|&b| b == FLASH_EMPTY)
    );

    println!();
    println!(
        "All OTA scenarios verified. (max app size: {} bytes)",
        APP_SIZE
    );
}
