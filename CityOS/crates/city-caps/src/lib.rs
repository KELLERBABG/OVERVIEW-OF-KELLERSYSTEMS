//! City OS capability table.
//!
//! ALIGNED WITH KELLER OS (`sovereign-core/src/caps.rs`): the first seven
//! slots match the kernel exactly (CAP_SERIAL..=CAP_IO), and the revocation
//! model is identical — a handle is a 32-bit index; clearing its `valid` bit
//! takes effect at the *next* dispatch (O(1) runtime revocation).
//!
//! City OS extends the table with the service slots the City node exposes to
//! its applications (DAG, BRP auth, dead-drops, shards, resource allocation).

#![no_std]

#[cfg(test)]
extern crate std;

use city_types::CapSlot;

/// KELLER kernel slots (must match sovereign-core `caps.rs` exactly).
pub const CAP_SERIAL: CapSlot = 0;
pub const CAP_KEYBOARD: CapSlot = 1;
pub const CAP_TIMER: CapSlot = 2;
pub const CAP_VAULT: CapSlot = 3;
pub const CAP_IPC: CapSlot = 4;
pub const CAP_SHELL: CapSlot = 5;
pub const CAP_IO: CapSlot = 6;

/// City OS service slots (extensions beyond the KELLER kernel table).
pub const CAP_CITY_DAG: CapSlot = 7;
pub const CAP_CITY_BRP: CapSlot = 8;
pub const CAP_CITY_DEADDROP: CapSlot = 9;
pub const CAP_CITY_SHARD: CapSlot = 10;
pub const CAP_CITY_ALLOC: CapSlot = 11;
pub const CAP_CITY_COUNT: usize = 12;

/// Maximum handles (same bound as KELLER's table).
pub const CAP_MAX_HANDLES: usize = 64;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub slot: CapSlot,
    pub tag: u32,
    pub valid: u32,
    pub owner: u32,
}

impl Capability {
    pub const fn new(slot: CapSlot, tag: u32, owner: u32) -> Self {
        Self {
            slot,
            tag,
            valid: 1,
            owner,
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.valid & 1 != 0
    }

    pub fn revoke(&mut self) {
        self.valid = 0;
        self.tag = 0;
    }
}

/// Human-readable slot names (for the node dashboard / shell).
pub fn slot_name(slot: CapSlot) -> &'static str {
    match slot {
        CAP_SERIAL => "serial",
        CAP_KEYBOARD => "keyboard",
        CAP_TIMER => "timer",
        CAP_VAULT => "vault",
        CAP_IPC => "ipc",
        CAP_SHELL => "shell",
        CAP_IO => "io",
        CAP_CITY_DAG => "city-dag",
        CAP_CITY_BRP => "city-brp",
        CAP_CITY_DEADDROP => "city-deaddrop",
        CAP_CITY_SHARD => "city-shard",
        CAP_CITY_ALLOC => "city-alloc",
        _ => "unknown",
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapTable {
    caps: [Option<Capability>; CAP_MAX_HANDLES],
    next_handle: usize,
}

impl Default for CapTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CapTable {
    pub const fn new() -> Self {
        Self {
            caps: [None; CAP_MAX_HANDLES],
            next_handle: 1,
        }
    }

    /// Issue a fresh handle for a slot. Returns None if the table is full.
    /// Same linear-scan + golden-ratio tag scheme as KELLER.
    pub fn issue(&mut self, slot: CapSlot, owner: u32) -> Option<usize> {
        let start = self.next_handle % CAP_MAX_HANDLES;
        let mut i = start;
        loop {
            if self.caps[i].is_none() {
                let tag = slot.wrapping_mul(0x9E37_79B9).wrapping_add(i as u32);
                self.caps[i] = Some(Capability::new(slot, tag, owner));
                self.next_handle = (i + 1) % CAP_MAX_HANDLES;
                return Some(i);
            }
            i = (i + 1) % CAP_MAX_HANDLES;
            if i == start {
                return None;
            }
        }
    }

    /// Check that `handle` is valid and points at `slot`.
    pub fn check(&self, handle: usize, slot: CapSlot) -> bool {
        match self.caps.get(handle) {
            Some(Some(cap)) => cap.is_valid() && cap.slot == slot,
            _ => false,
        }
    }

    /// Revoke a handle (O(1) runtime revocation). Returns true if revoked.
    pub fn revoke(&mut self, handle: usize) -> bool {
        match self.caps.get_mut(handle) {
            Some(Some(cap)) => {
                cap.revoke();
                true
            }
            _ => false,
        }
    }

    /// Wipe every handle — used by Panic-Zeroing.
    pub fn wipe(&mut self) {
        for c in self.caps.iter_mut() {
            *c = None;
        }
    }

    /// Count currently-valid handles (dashboard / audit).
    pub fn live_count(&self) -> usize {
        self.caps
            .iter()
            .filter(|c| c.is_some_and(|cap| cap.is_valid()))
            .count()
    }
}

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// Tiny spin-lock wrapper (no_std; the single-CPU kernel model).
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

pub static TABLE: SpinLock<CapTable> = SpinLock::new(CapTable::new());

/// Issue the kernel's own service handles at boot (owner 0).
pub fn init() {
    let mut t = TABLE.lock();
    let _ = t.issue(CAP_SERIAL, 0);
    let _ = t.issue(CAP_KEYBOARD, 0);
    let _ = t.issue(CAP_TIMER, 0);
    let _ = t.issue(CAP_VAULT, 0);
    let _ = t.issue(CAP_IPC, 0);
    let _ = t.issue(CAP_SHELL, 0);
    let _ = t.issue(CAP_IO, 0);
    let _ = t.issue(CAP_CITY_DAG, 0);
    let _ = t.issue(CAP_CITY_BRP, 0);
    let _ = t.issue(CAP_CITY_DEADDROP, 0);
    let _ = t.issue(CAP_CITY_SHARD, 0);
    let _ = t.issue(CAP_CITY_ALLOC, 0);
}

/// Issue a handle for a process (City service bootstrap).
pub fn issue_for_process(slot: CapSlot, pid: u32) -> Option<usize> {
    TABLE.lock().issue(slot, pid)
}

/// Revoke a handle (O(1) runtime revocation).
pub fn revoke_handle(handle: usize) -> bool {
    TABLE.lock().revoke(handle)
}

/// Check validity.
pub fn check_handle(handle: usize, slot: CapSlot) -> bool {
    TABLE.lock().check(handle, slot)
}

/// Panic-zeroing: wipe all capabilities.
pub fn dispose_all() {
    TABLE.lock().wipe();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_check_revoke_default() {
        let mut t = CapTable::new();
        let h = t.issue(CAP_CITY_DAG, 7).unwrap();
        assert!(t.check(h, CAP_CITY_DAG));
        assert!(!t.check(h, CAP_CITY_BRP));
        assert!(t.revoke(h));
        assert!(!t.check(h, CAP_CITY_DAG));
        // KELLER semantics: revoke on an occupied slot is idempotent and
        // still reports true; validity remains cleared.
        assert!(t.revoke(h));
        assert!(!t.check(h, CAP_CITY_DAG));
        // A handle that was never issued reports false.
        assert!(!t.revoke(0));
    }

    #[test]
    fn slot_names_cover_city_services() {
        assert_eq!(slot_name(CAP_CITY_DAG), "city-dag");
        assert_eq!(slot_name(CAP_CITY_ALLOC), "city-alloc");
        assert_eq!(slot_name(CAP_SERIAL), "serial");
    }

    #[test]
    fn wipe_zeroes_everything() {
        let mut t = CapTable::new();
        let h = t.issue(CAP_IPC, 1).unwrap();
        t.wipe();
        assert!(!t.check(h, CAP_IPC));
        assert_eq!(t.live_count(), 0);
    }
}
