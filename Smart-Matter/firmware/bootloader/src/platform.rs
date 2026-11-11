//! # Embedded Platform (STM32G031J6 bootloader)
//!
//! - **Flash:** the internal flash is accessed through the HAL
//!   (`FLASH.unlock()` → `Read`/`WriteErase`). Note the G031 erases in
//!   **2 KB pages** (whitepaper says 4 KB sectors; region layout is
//!   unaffected — see `smart_matter_core::memmap`).
//! - **Staged image:** the OTA image is staged in the optional external SPI
//!   flash (AT25SF041, BOM U7) because the 64 KB internal flash has no room
//!   for a second bank. The SPI driver is a documented bring-up point
//!   (mirrors `firmware/node/src/platform.rs`).
//! - **Jump:** after validation the app vector table (SP + reset) is read
//!   from `APP_START` and branched to with interrupts disabled.

use stm32g0xx_hal::flash::{FlashExt, FlashPage, Read as _, UnlockedFlash, WriteErase as _};

use smart_matter_core::memmap::APP_START;

use crate::boot::{BootDecision, Bootloader, Flash, FlashError, StagedImage};

// ---------------------------------------------------------------------------
// Internal flash
// ---------------------------------------------------------------------------

/// HAL-backed internal flash.
pub struct InternalFlash {
    flash: UnlockedFlash,
}

impl InternalFlash {
    pub fn new(flash_pac: stm32g0xx_hal::pac::FLASH) -> Self {
        // Safety: we are the only flash user (bootloader stage).
        let unlocked = flash_pac.unlock().expect("flash unlock");
        Self { flash: unlocked }
    }
}

impl Flash for InternalFlash {
    fn read(&self, addr: usize, buf: &mut [u8]) -> Result<(), FlashError> {
        self.flash.read(addr, buf);
        Ok(())
    }

    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), FlashError> {
        self.flash
            .write(addr, data)
            .map_err(|_| FlashError::WriteFailed)
    }

    fn erase_pages(&mut self, start: usize, size: usize) -> Result<(), FlashError> {
        let page_size = 2048usize; // G031 page size
        let first_page = start / page_size;
        let last_page = (start + size - 1) / page_size;
        for page in first_page..=last_page {
            self.flash
                .erase_page(FlashPage(page * page_size))
                .map_err(|_| FlashError::EraseFailed)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Staged image (external SPI flash — bring-up)
// ---------------------------------------------------------------------------

/// AT25SF041 staging source placeholder — plug the SPI driver in here.
pub struct SpiStagedImage;

impl StagedImage for SpiStagedImage {
    fn len(&self) -> Result<usize, FlashError> {
        // TODO(bring-up): read the length header from the SPI flash.
        Err(FlashError::StagingEmpty)
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize, FlashError> {
        // TODO(bring-up): SPI flash read.
        Err(FlashError::StagingEmpty)
    }
}

// ---------------------------------------------------------------------------
// Application jump
// ---------------------------------------------------------------------------

/// Read a 32-bit word from flash (app vector table).
fn read_word(flash: &impl Flash, addr: usize) -> u32 {
    let mut b = [0u8; 4];
    if flash.read(addr, &mut b).is_ok() {
        u32::from_le_bytes(b)
    } else {
        0
    }
}

/// Jump to the application at `APP_START` (never returns).
pub fn jump_to_application(flash: &impl Flash) -> ! {
    let initial_sp = read_word(flash, APP_START);
    let reset_vector = read_word(flash, APP_START + 4);

    if reset_vector == 0 || reset_vector & 1 == 0 {
        // No valid app — hang (should never happen with the boot decision
        // logic, but stay fail-closed).
        loop {
            cortex_m::asm::nop();
        }
    }

    // Disable interrupts and switch to the app's stack, then branch.
    unsafe {
        cortex_m::interrupt::disable();
        core::arch::asm!(
            "msr msp, {0}",
            in(reg) initial_sp,
            options(nomem, nostack, preserves_flags),
        );
        let app: extern "C" fn() -> ! = core::mem::transmute(reset_vector);
        app()
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() -> ! {
    let dp = stm32g0xx_hal::pac::Peripherals::take().expect("peripherals");
    let mut flash = InternalFlash::new(dp.FLASH);

    let mut bootloader = Bootloader::new(&mut flash, SpiStagedImage);
    match bootloader.boot() {
        Ok(BootDecision::AppliedUpdate) | Ok(BootDecision::JumpToApp) => {}
        Err(_) => {
            // Fail-closed: boot whatever is in the app region.
        }
    }

    jump_to_application(&flash)
}
