//! # Smart Matter Bootloader
//!
//! On power-up: checks the OTA metadata sector (`META_START`) for a pending
//! update, SHA-256-validates the staged image, applies it to the application
//! region, and jumps to the application vector table. Fail-closed: a
//! corrupt or oversized image is never booted and the old application is
//! preserved.
//!
//! ## Building
//!
//! ```bash
//! # Host simulation (runs the OTA scenarios on the desktop)
//! cargo run -p smart-matter-bootloader
//!
//! # Embedded firmware (STM32G031J6, Cortex-M0+)
//! cargo build -p smart-matter-bootloader --target thumbv6m-none-eabi \
//!     --no-default-features --features embedded
//! ```

#![cfg_attr(not(feature = "host"), no_std)]
#![cfg_attr(not(feature = "host"), no_main)]

pub mod boot;

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "embedded")]
mod platform;

#[cfg(feature = "host")]
fn main() {
    host::run();
}

#[cfg(feature = "embedded")]
#[cortex_m_rt::entry]
fn main() -> ! {
    platform::run()
}

#[cfg(feature = "embedded")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
