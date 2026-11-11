//! # Smart Matter Node Firmware
//!
//! Main entry point for the embedded compute node. Integrates all core
//! subsystems: energy harvesting, mesh networking, dead-drop storage,
//! localized broadcast, environmental sampling, thermal management,
//! and structural integrity monitoring.
//!
//! ## Architecture
//!
//! The main loop operates in energy-aware phases (see `node::Node::tick`):
//! 1. **Wake**: Node wakes from vibration (external interrupt or RTC).
//! 2. **Sample**: Read sensors, check energy budget.
//! 3. **Process**: Handle incoming radio messages, update routing, check integrity.
//! 4. **Transmit**: Send queued messages if energy budget allows.
//! 5. **Sleep**: Return to low-power state until next wake event.
//!
//! ## Building
//!
//! ```bash
//! # Host simulation (runs the full loop with simulated sensors)
//! cargo run -p smart-matter-node -- --ticks 120
//!
//! # Embedded firmware (STM32G031J6, Cortex-M0+)
//! cargo build -p smart-matter-node --target thumbv6m-none-eabi \
//!     --no-default-features --features embedded
//! ```

#![cfg_attr(not(feature = "host"), no_std)]
#![cfg_attr(not(feature = "host"), no_main)]

mod node;

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "embedded")]
mod platform;

#[cfg(feature = "host")]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    host::run(args);
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
