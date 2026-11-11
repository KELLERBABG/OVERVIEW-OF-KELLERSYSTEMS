//! **City dead-drop infrastructure** — asynchronous encrypted message storage.
//!
//! A `DeadDrop` stores a short message as an encrypted shard in the Keller
//! Vault (AES-XTS via `city-crypto::vault`), with a Time-To-Live in scheduler
//! ticks. Expired drops are wiped automatically on a sweep. This maps directly
//! to the "async message storage with TTL, encrypted shard format" item of the
//! City OS plan — and because it uses the kernel vault, a panic zeroes all
//! drop contents with the same Panic-Zeroing guarantee as the kernel.

#![no_std]

#[cfg(test)]
extern crate std;

use city_caps::CAP_CITY_DEADDROP;
use city_crypto::vault;
use city_types::Bytes32;

pub const DEADDROP_MAX_DROPS: usize = 16;
pub const DEADDROP_TTL_TICKS: u32 = 100;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DropSlot {
    pub active: bool,
    pub owner: u32,
    pub ttl_ticks: u32,
    pub remaining_ticks: u32,
    pub sector: usize,
    pub tag: Bytes32,
}

impl DropSlot {
    pub const fn empty() -> Self {
        Self {
            active: false,
            owner: 0,
            ttl_ticks: 0,
            remaining_ticks: 0,
            sector: 0,
            tag: Bytes32::EMPTY,
        }
    }
}

/// Fixed-capacity dead-drop table.
#[derive(Copy, Clone, Debug)]
pub struct DeadDropHeap {
    slots: [DropSlot; DEADDROP_MAX_DROPS],
    count: usize,
}

impl Default for DeadDropHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadDropHeap {
    pub const fn new() -> Self {
        Self {
            slots: [DropSlot::empty(); DEADDROP_MAX_DROPS],
            count: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// Drop a message for a recipient. The payload is stored encrypted via the
    /// Keller Vault. Returns the drop slot or None on full/oversize.
    pub fn drop(
        &mut self,
        owner: u32,
        tag: &Bytes32,
        message: &[u8],
        ttl_ticks: u32,
    ) -> Option<usize> {
        if self.count >= DEADDROP_MAX_DROPS || message.len() > vault::VAULT_SECTOR_SIZE {
            return None;
        }
        let sector = vault::store(message)?;
        let here = self.count;
        self.slots[here] = DropSlot {
            active: true,
            owner,
            ttl_ticks,
            remaining_ticks: ttl_ticks,
            sector,
            tag: *tag,
        };
        self.count += 1;
        Some(here)
    }

    /// Pick up a message by tag. The drop is removed (fire-and-forget).
    /// Returns the decrypted bytes or None if no matching drop / tampered.
    pub fn pickup(&mut self, tag: &Bytes32, out: &mut [u8]) -> Option<u32> {
        for i in 0..self.count {
            if self.slots[i].active && self.slots[i].tag == *tag {
                let slot = self.slots[i];
                if !vault::retrieve(slot.sector, out) {
                    // Tampered or disposed: purge.
                    vault::forget(slot.sector);
                    self.remove(i);
                    return None;
                }
                self.remove(i);
                return Some(slot.owner);
            }
        }
        None
    }

    /// Tick the clock once; expired drops are wiped from the vault.
    /// Returns how many drops expired this tick.
    pub fn sweep(&mut self) -> usize {
        let mut expired = 0usize;
        let mut i = 0usize;
        while i < self.count {
            let next_remaining = self.slots[i].remaining_ticks.saturating_sub(1);
            self.slots[i].remaining_ticks = next_remaining;
            if next_remaining == 0 {
                let sector = self.slots[i].sector;
                vault::forget(sector);
                self.remove(i);
                expired += 1;
            } else {
                i += 1;
            }
        }
        expired
    }

    fn remove(&mut self, i: usize) {
        if i >= self.count {
            return;
        }
        vault::forget(self.slots[i].sector);
        self.slots[i] = DropSlot::empty();
        // Compact.
        for j in i..self.count - 1 {
            self.slots[j] = self.slots[j + 1];
        }
        self.count -= 1;
    }
}

/// Capability required to place/pick up drops.
pub const REQUIRED_CAP: u32 = CAP_CITY_DEADDROP;

#[cfg(test)]
mod tests {
    use super::*;

    // The Keller Vault is a crate-level static shared by every deaddrop test;
    // serialize vault-affecting tests on the host so they don't race.
    static VAULT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn drop_pickup_roundtrip() {
        let _g = VAULT_GUARD.lock().unwrap();
        vault::dispose_all();
        let mut heap = DeadDropHeap::new();
        let tag = Bytes32::from_slice(b"lamp-42").unwrap();
        let msg = *b"pickup: coordinates cached";
        let slot = heap.drop(7, &tag, &msg, DEADDROP_TTL_TICKS).unwrap();
        assert_eq!(slot, 0);
        assert_eq!(heap.count(), 1);

        let mut out = [0u8; 26];
        let owner = heap.pickup(&tag, &mut out).unwrap();
        assert_eq!(owner, 7);
        assert_eq!(&out, &msg[..]);
        assert_eq!(heap.count(), 0);

        // Picking up again finds nothing.
        assert!(heap.pickup(&tag, &mut out).is_none());
        vault::dispose_all();
    }

    #[test]
    fn ttl_expires_drops() {
        let _g = VAULT_GUARD.lock().unwrap();
        vault::dispose_all();
        let mut heap = DeadDropHeap::new();
        let tag = Bytes32::from_slice(b"bus-stop").unwrap();
        let _ = heap.drop(3, &tag, b"e-ticket", 5);
        assert_eq!(heap.count(), 1);
        // Sweep 5 times -> expires.
        for _ in 0..5 {
            let e = heap.sweep();
            if e > 0 {
                break;
            }
        }
        assert_eq!(heap.count(), 0);
        vault::dispose_all();
    }
}
