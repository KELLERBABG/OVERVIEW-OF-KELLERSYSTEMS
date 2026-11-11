//! # Boot Logic (platform-independent)
//!
//! On power-up the bootloader:
//!
//! 1. Reads the OTA metadata sector at [`META_START`] (see
//!    [`smart_matter_core::memmap`]).
//! 2. If no update is pending → jump straight to the application.
//! 3. If an update is pending → **SHA-256-validate** the staged image (staged
//!    in external SPI flash by the mesh flasher — the 64 KB internal flash
//!    has no room for a second bank), erase the application region, copy the
//!    validated image, clear the pending flag, and boot it.
//! 4. Any validation failure leaves the old application untouched and clears
//!    the pending flag (fail-closed: never boot a corrupt image).
//!
//! The logic is generic over [`Flash`] (internal flash) and [`StagedImage`]
//! (the external staging medium), so it runs on the host against simulated
//! storage for testing and on the MCU against real hardware.

use sha2::{Digest, Sha256};
use smart_matter_core::memmap::{APP_SIZE, APP_START, META_START};

// ---------------------------------------------------------------------------
// Metadata layout (META_START, 44 bytes)
// ---------------------------------------------------------------------------

/// Magic marking a valid metadata sector.
pub const PENDING_MAGIC: u32 = 0xDEAD_BEEF;

pub const META_MAGIC_OFFSET: usize = 0;
pub const META_LEN_OFFSET: usize = 4;
pub const META_SHA_OFFSET: usize = 8;
pub const META_FLAGS_OFFSET: usize = 40;
pub const META_RECORD_LEN: usize = 44;

/// Flag: an update is staged and ready to apply.
pub const FLAG_PENDING: u32 = 0x0000_0001;

/// Decoded OTA metadata record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateMetadata {
    pub image_len: u32,
    pub sha256: [u8; 32],
    pub flags: u32,
}

// ---------------------------------------------------------------------------
// Abstraction traits
// ---------------------------------------------------------------------------

/// Error type for flash / staging operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashError {
    ReadFailed,
    WriteFailed,
    EraseFailed,
    StagingEmpty,
}

/// Internal flash (or a simulation of it).
pub trait Flash {
    fn read(&self, addr: usize, buf: &mut [u8]) -> Result<(), FlashError>;
    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), FlashError>;
    /// Erase the pages covering `[start_addr, start_addr + size)`.
    fn erase_pages(&mut self, start_addr: usize, size: usize) -> Result<(), FlashError>;
}

/// Lets `Bootloader` borrow the flash instead of owning it.
impl<F: Flash> Flash for &mut F {
    fn read(&self, addr: usize, buf: &mut [u8]) -> Result<(), FlashError> {
        (**self).read(addr, buf)
    }
    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), FlashError> {
        (**self).write(addr, data)
    }
    fn erase_pages(&mut self, start_addr: usize, size: usize) -> Result<(), FlashError> {
        (**self).erase_pages(start_addr, size)
    }
}

/// `len()` is fallible (staging medium may be absent), so no `is_empty()`
/// helper is provided — callers compare against 0 explicitly.
#[allow(clippy::len_without_is_empty)]
pub trait StagedImage {
    fn len(&self) -> Result<usize, FlashError>;
    /// Read up to `buf.len()` bytes at `offset`; returns bytes read (0 = EOF).
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, FlashError>;
}

// ---------------------------------------------------------------------------
// Bootloader
// ---------------------------------------------------------------------------

/// Outcome of a boot pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootDecision {
    /// No valid pending update — boot the existing application.
    JumpToApp,
    /// A staged image was validated, applied, and will now boot.
    AppliedUpdate,
}

pub struct Bootloader<F: Flash, S: StagedImage> {
    flash: F,
    staged: S,
}

impl<F: Flash, S: StagedImage> Bootloader<F, S> {
    pub fn new(flash: F, staged: S) -> Self {
        Self { flash, staged }
    }

    /// Run one boot pass and return the decision.
    pub fn boot(&mut self) -> Result<BootDecision, FlashError> {
        match self.read_metadata() {
            Some(meta) if meta.flags & FLAG_PENDING != 0 => self.apply_update(&meta),
            _ => Ok(BootDecision::JumpToApp),
        }
    }

    /// Validate and apply a pending staged update (fail-closed).
    fn apply_update(&mut self, meta: &UpdateMetadata) -> Result<BootDecision, FlashError> {
        let len = self.staged.len()?;
        if len == 0 {
            self.clear_pending()?;
            return Ok(BootDecision::JumpToApp);
        }
        if len as u32 != meta.image_len || len > APP_SIZE {
            // Length mismatch or oversized image — reject, keep old app.
            self.clear_pending()?;
            return Ok(BootDecision::JumpToApp);
        }

        let digest = self.hash_staged(len)?;
        if digest != meta.sha256 {
            // Checksum mismatch — the staged image is corrupt or tampered.
            self.clear_pending()?;
            return Ok(BootDecision::JumpToApp);
        }

        // Valid: erase the application region and copy the image in.
        self.flash.erase_pages(APP_START, APP_SIZE)?;
        let mut buf = [0u8; 64];
        let mut offset = 0usize;
        while offset < len {
            let n = self.staged.read_at(offset, &mut buf)?;
            if n == 0 {
                return Err(FlashError::StagingEmpty); // truncated image
            }
            self.flash.write(APP_START + offset, &buf[..n])?;
            offset += n;
        }

        self.clear_pending()?;
        Ok(BootDecision::AppliedUpdate)
    }

    /// SHA-256 over the staged image, read incrementally (64 B chunks).
    fn hash_staged(&self, len: usize) -> Result<[u8; 32], FlashError> {
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64];
        let mut offset = 0usize;
        while offset < len {
            let n = self.staged.read_at(offset, &mut buf)?;
            if n == 0 {
                return Err(FlashError::StagingEmpty);
            }
            hasher.update(&buf[..n]);
            offset += n;
        }
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Ok(out)
    }

    /// Decode the metadata record at [`META_START`], if present.
    fn read_metadata(&self) -> Option<UpdateMetadata> {
        let mut record = [0u8; META_RECORD_LEN];
        self.flash.read(META_START, &mut record).ok()?;
        let magic = u32::from_le_bytes(record[0..4].try_into().ok()?);
        if magic != PENDING_MAGIC {
            return None;
        }
        let image_len = u32::from_le_bytes(record[4..8].try_into().ok()?);
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&record[8..40]);
        let flags = u32::from_le_bytes(record[40..44].try_into().ok()?);
        Some(UpdateMetadata {
            image_len,
            sha256,
            flags,
        })
    }

    /// Clear the pending flag (keeps the rest of the record intact).
    fn clear_pending(&mut self) -> Result<(), FlashError> {
        let mut record = [0u8; META_RECORD_LEN];
        self.flash.read(META_START, &mut record)?;
        let flags_off = META_FLAGS_OFFSET;
        record[flags_off..flags_off + 4].copy_from_slice(&0u32.to_le_bytes());
        self.flash.write(META_START, &record)
    }
}

/// Write a metadata record to flash (used by the flasher and tests).
pub fn write_metadata(flash: &mut impl Flash, meta: &UpdateMetadata) -> Result<(), FlashError> {
    let mut record = [0u8; META_RECORD_LEN];
    record[0..4].copy_from_slice(&PENDING_MAGIC.to_le_bytes());
    record[4..8].copy_from_slice(&meta.image_len.to_le_bytes());
    record[8..40].copy_from_slice(&meta.sha256);
    record[40..44].copy_from_slice(&meta.flags.to_le_bytes());
    flash.write(META_START, &record)
}

// ---------------------------------------------------------------------------
// Tests (host — simulated flash + staged image)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const FLASH_SIZE: usize = 64 * 1024;
    /// Simulated flash is indexed relative to the 0x0800_0000 base (the boot
    /// code uses absolute addresses, like the real hardware).
    const FLASH_BASE: usize = 0x0800_0000;
    const FLASH_EMPTY: u8 = 0xFF;

    /// Simple RAM-backed flash simulation.
    #[derive(Clone)]
    struct SimFlash {
        data: Vec<u8>,
    }

    impl SimFlash {
        fn new() -> Self {
            Self {
                data: vec![FLASH_EMPTY; FLASH_SIZE],
            }
        }
        fn app_region(&self) -> &[u8] {
            &self.data[APP_START - FLASH_BASE..APP_START - FLASH_BASE + APP_SIZE]
        }
    }

    impl Flash for SimFlash {
        fn read(&self, addr: usize, buf: &mut [u8]) -> Result<(), FlashError> {
            let rel = addr.checked_sub(FLASH_BASE).ok_or(FlashError::ReadFailed)?;
            let end = rel.checked_add(buf.len()).ok_or(FlashError::ReadFailed)?;
            if end > FLASH_SIZE {
                return Err(FlashError::ReadFailed);
            }
            buf.copy_from_slice(&self.data[rel..end]);
            Ok(())
        }
        fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), FlashError> {
            let rel = addr
                .checked_sub(FLASH_BASE)
                .ok_or(FlashError::WriteFailed)?;
            let end = rel.checked_add(data.len()).ok_or(FlashError::WriteFailed)?;
            if end > FLASH_SIZE {
                return Err(FlashError::WriteFailed);
            }
            self.data[rel..end].copy_from_slice(data);
            Ok(())
        }
        fn erase_pages(&mut self, start: usize, size: usize) -> Result<(), FlashError> {
            let rel = start
                .checked_sub(FLASH_BASE)
                .ok_or(FlashError::EraseFailed)?;
            let end = rel.checked_add(size).ok_or(FlashError::EraseFailed)?;
            if end > FLASH_SIZE {
                return Err(FlashError::EraseFailed);
            }
            self.data[rel..end].fill(FLASH_EMPTY);
            Ok(())
        }
    }

    /// RAM-backed staged-image simulation.
    struct SimStaged(Vec<u8>);

    impl StagedImage for SimStaged {
        fn len(&self) -> Result<usize, FlashError> {
            Ok(self.0.len())
        }
        fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, FlashError> {
            let remaining = self.0.len().saturating_sub(offset);
            let n = remaining.min(buf.len());
            buf[..n].copy_from_slice(&self.0[offset..offset + n]);
            Ok(n)
        }
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let digest = Sha256::digest(data);
        out.copy_from_slice(&digest);
        out
    }

    fn stage(flash: &mut SimFlash, image: &[u8]) {
        let meta = UpdateMetadata {
            image_len: image.len() as u32,
            sha256: sha256(image),
            flags: FLAG_PENDING,
        };
        write_metadata(flash, &meta).unwrap();
    }

    /// A plausible app image: vector table (SP + reset) + code bytes.
    fn app_image(len: usize) -> Vec<u8> {
        let mut img = vec![0u8; len];
        // Fake vector table: initial SP and reset vector.
        img[0..4].copy_from_slice(&0x2000_2000u32.to_le_bytes());
        img[4..8].copy_from_slice(&(APP_START as u32 + 0x09).to_le_bytes());
        for (i, b) in img.iter_mut().enumerate().skip(8) {
            *b = (i & 0xFF) as u8;
        }
        img
    }

    #[test]
    fn no_pending_update_jumps_to_app() {
        let flash = SimFlash::new(); // all 0xFF — no metadata
        let mut bl = Bootloader::new(flash, SimStaged(vec![]));
        assert_eq!(bl.boot().unwrap(), BootDecision::JumpToApp);
    }

    #[test]
    fn valid_update_is_applied() {
        let image = app_image(4096);
        let mut flash = SimFlash::new();
        stage(&mut flash, &image);

        let mut bl = Bootloader::new(flash.clone(), SimStaged(image.clone()));
        assert_eq!(bl.boot().unwrap(), BootDecision::AppliedUpdate);

        // App region now holds the image, pending flag cleared.
        let flash = bl.flash;
        assert_eq!(&flash.app_region()[..image.len()], &image[..]);
        let meta = Bootloader::<SimFlash, SimStaged>::new(flash, SimStaged(vec![])).read_metadata();
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().flags & FLAG_PENDING, 0);
    }

    #[test]
    fn corrupt_image_is_rejected_and_old_app_kept() {
        let image = app_image(2048);
        let mut flash = SimFlash::new();
        stage(&mut flash, &image);
        // Bootloader's own metadata-read check needs the read to work; the
        // staged image is tampered AFTER staging (hash mismatch).
        let mut staged = image.clone();
        staged[100] ^= 0xFF; // corrupt the "code"

        let mut bl = Bootloader::new(flash.clone(), SimStaged(staged));
        assert_eq!(bl.boot().unwrap(), BootDecision::JumpToApp);

        // Old app (0xFF — never had an app) untouched, pending cleared.
        let flash = bl.flash;
        assert!(flash.app_region().iter().all(|&b| b == FLASH_EMPTY));
        let meta = Bootloader::<SimFlash, SimStaged>::new(flash, SimStaged(vec![]))
            .read_metadata()
            .unwrap();
        assert_eq!(meta.flags & FLAG_PENDING, 0);
    }

    #[test]
    fn length_mismatch_is_rejected() {
        let image = app_image(2048);
        let mut flash = SimFlash::new();
        // Stage metadata claiming a different length than the image.
        let meta = UpdateMetadata {
            image_len: 3000, // staged is actually 2048
            sha256: sha256(&image),
            flags: FLAG_PENDING,
        };
        write_metadata(&mut flash, &meta).unwrap();

        let mut bl = Bootloader::new(flash, SimStaged(image));
        assert_eq!(bl.boot().unwrap(), BootDecision::JumpToApp);
    }

    #[test]
    fn oversized_image_is_rejected() {
        let mut flash = SimFlash::new();
        let meta = UpdateMetadata {
            image_len: (APP_SIZE + 1) as u32,
            sha256: [0u8; 32],
            flags: FLAG_PENDING,
        };
        write_metadata(&mut flash, &meta).unwrap();

        // Staged image larger than the app region.
        let mut bl = Bootloader::new(flash, SimStaged(vec![0xAB; APP_SIZE + 1]));
        assert_eq!(bl.boot().unwrap(), BootDecision::JumpToApp);
    }

    #[test]
    fn second_boot_sees_no_pending_update() {
        let image = app_image(1024);
        let mut flash = SimFlash::new();
        stage(&mut flash, &image);

        let mut bl = Bootloader::new(flash, SimStaged(image));
        assert_eq!(bl.boot().unwrap(), BootDecision::AppliedUpdate);

        // After apply, pending is cleared → a fresh boot jumps.
        let flash = bl.flash;
        let mut bl2 = Bootloader::new(flash, SimStaged(vec![]));
        assert_eq!(bl2.boot().unwrap(), BootDecision::JumpToApp);
    }
}
