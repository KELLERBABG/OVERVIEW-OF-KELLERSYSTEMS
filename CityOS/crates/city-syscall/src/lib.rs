//! **City OS kernel syscall ABI** — the exact opcode / slot constants the
//! KELLER kernel exposes for City ring-3 tasks.
//!
//! MUST mirror `sovereign-core/src/city.rs` / `caps.rs` exactly:
//!   - Service ids: `CITY_DAG=0` … `CITY_ENTROPY=9`
//!   - Capability slots: `CAP_CITY_DAG=7` … `CAP_CITY_ENTROPY=16`
//!   - Syscall opcodes: `SYS_CITY_BASE=64`, `SYS_CITY_DAG=64` …
//!     `SYS_CITY_SDR_RX=77`
//!
//! Host-side services and the `city-node` demo use these constants to call
//! the same ABI the kernel dispatches, keeping the application layer in
//! lock-step with the ring-3 integration (Phase 3.6).

#![no_std]

#[cfg(test)]
extern crate std;

use city_types::CapSlot;

/// Service ids (match `sovereign-core/src/city.rs`).
pub const CITY_DAG: u32 = 0;
pub const CITY_BRP: u32 = 1;
pub const CITY_DEADDROP: u32 = 2;
pub const CITY_SHARD: u32 = 3;
pub const CITY_ALLOC: u32 = 4;
pub const CITY_VOTE: u32 = 5;
pub const CITY_HEAL: u32 = 6;
pub const CITY_INTEROP: u32 = 7;
pub const CITY_INCENTIVE: u32 = 8;
pub const CITY_ENTROPY: u32 = 9;
pub const CITY_SERVICES: usize = 10;

/// Kernel capability slots (match `sovereign-core/src/caps.rs` 7–16).
pub const CAP_CITY_DAG: CapSlot = 7;
pub const CAP_CITY_BRP: CapSlot = 8;
pub const CAP_CITY_DEADDROP: CapSlot = 9;
pub const CAP_CITY_SHARD: CapSlot = 10;
pub const CAP_CITY_ALLOC: CapSlot = 11;
pub const CAP_CITY_VOTE: CapSlot = 12;
pub const CAP_CITY_HEAL: CapSlot = 13;
pub const CAP_CITY_INTEROP: CapSlot = 14;
pub const CAP_CITY_INCENTIVE: CapSlot = 15;
pub const CAP_CITY_ENTROPY: CapSlot = 16;

/// Syscall opcode base + range (match `sovereign-core/src/city.rs`).
pub const SYS_CITY_BASE: u32 = 64;

pub const SYS_CITY_DAG: u32 = SYS_CITY_BASE;
pub const SYS_CITY_BRP: u32 = SYS_CITY_BASE + 1;
pub const SYS_CITY_DEADDROP: u32 = SYS_CITY_BASE + 2;
pub const SYS_CITY_SHARD: u32 = SYS_CITY_BASE + 3;
pub const SYS_CITY_ALLOC: u32 = SYS_CITY_BASE + 4;
pub const SYS_CITY_VOTE: u32 = SYS_CITY_BASE + 5;
pub const SYS_CITY_HEAL: u32 = SYS_CITY_BASE + 6;
pub const SYS_CITY_INTEROP: u32 = SYS_CITY_BASE + 7;
pub const SYS_CITY_INCENTIVE: u32 = SYS_CITY_BASE + 8;
pub const SYS_CITY_ENTROPY: u32 = SYS_CITY_BASE + 9;
pub const SYS_CITY_NET_TX: u32 = SYS_CITY_BASE + 10;
pub const SYS_CITY_NET_RX: u32 = SYS_CITY_BASE + 11;
pub const SYS_CITY_SDR_TX: u32 = SYS_CITY_BASE + 12;
pub const SYS_CITY_SDR_RX: u32 = SYS_CITY_BASE + 13;

/// Map a City service id to its kernel capability slot (mirrors the kernel's
/// `city_cap_slot`). This is how a ring-3 task knows which cap it must hold.
pub fn service_cap_slot(service: u32) -> Option<CapSlot> {
    Some(match service {
        CITY_DAG => CAP_CITY_DAG,
        CITY_BRP => CAP_CITY_BRP,
        CITY_DEADDROP => CAP_CITY_DEADDROP,
        CITY_SHARD => CAP_CITY_SHARD,
        CITY_ALLOC => CAP_CITY_ALLOC,
        CITY_VOTE => CAP_CITY_VOTE,
        CITY_HEAL => CAP_CITY_HEAL,
        CITY_INTEROP => CAP_CITY_INTEROP,
        CITY_INCENTIVE => CAP_CITY_INCENTIVE,
        CITY_ENTROPY => CAP_CITY_ENTROPY,
        _ => return None,
    })
}

/// Map a service id to its service opcode.
pub fn service_opcode(service: u32) -> Option<u32> {
    Some(
        SYS_CITY_BASE
            + match service {
                CITY_DAG => 0,
                CITY_BRP => 1,
                CITY_DEADDROP => 2,
                CITY_SHARD => 3,
                CITY_ALLOC => 4,
                CITY_VOTE => 5,
                CITY_HEAL => 6,
                CITY_INTEROP => 7,
                CITY_INCENTIVE => 8,
                CITY_ENTROPY => 9,
                _ => return None,
            },
    )
}

/// Human-readable service name (dashboard / logs).
pub fn service_name(service: u32) -> &'static str {
    match service {
        CITY_DAG => "dag",
        CITY_BRP => "brp",
        CITY_DEADDROP => "deaddrop",
        CITY_SHARD => "shard",
        CITY_ALLOC => "alloc",
        CITY_VOTE => "vote",
        CITY_HEAL => "heal",
        CITY_INTEROP => "interop",
        CITY_INCENTIVE => "incentive",
        CITY_ENTROPY => "entropy",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_mirrors_kernel() {
        // Opcode range: 64..=77 (14 opcodes: 10 services + 4 transport).
        assert_eq!(SYS_CITY_BASE, 64);
        assert_eq!(SYS_CITY_DAG, 64);
        assert_eq!(SYS_CITY_SDR_RX, 77);

        // Capability slots 7..=16.
        assert_eq!(CAP_CITY_DAG, 7);
        assert_eq!(CAP_CITY_ENTROPY, 16);

        // Every service has a cap slot + opcode.
        for s in 0..CITY_SERVICES as u32 {
            assert!(service_cap_slot(s).is_some());
            assert!(service_opcode(s).is_some());
        }
        assert!(service_cap_slot(99).is_none());
    }
}
