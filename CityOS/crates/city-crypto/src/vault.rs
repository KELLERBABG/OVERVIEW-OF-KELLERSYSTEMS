//! Keller Vault (Phase 2.3): RAM-resident secret store with real AES-XTS
//! sector encryption and atomic Panic-Zeroing disposal.
//!
//! Each 32-byte sector is one AES-XTS data unit (2 blocks of 16 bytes):
//!   - `KEY_DATA` encrypts the plaintext
//!   - `KEY_TWEAK` derives the per-sector tweak (AES_k2(sector_index))
//!   - a per-sector integrity tag (FNV-1a over the ciphertext + sector)
//!     lets `retrieve` detect a flipped bit — tamper -> None (MAC fail).
//!
//! VERBATIM REUSE from KELLER OS (`sovereign-core/src/vault.rs`), adapted
//! only in that the City crate has no `crate::idt`/`crate::serial` deps.

use crate::aes;
use crate::spinlock::SpinLock;

pub const VAULT_SECTORS: usize = 16;
pub const VAULT_SECTOR_SIZE: usize = 32; // 2 AES blocks per sector

// Two independent AES-128 keys: data + tweak (XTS-style).
static KEY_DATA: [u8; 16] = [
    0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6, 0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C,
];
static KEY_TWEAK: [u8; 16] = [
    0x8E, 0x73, 0xB0, 0xF7, 0xDA, 0x0E, 0x64, 0x52, 0xC8, 0x10, 0xF3, 0x2B, 0x80, 0x90, 0x79, 0xE5,
];

/// FNV-1a over ciphertext bytes + sector number (integrity tag).
fn sector_tag(cipher: &[u8], sector: usize) -> u8 {
    let mut h: u32 = 0x811C9DC5;
    h ^= sector as u32;
    h = h.wrapping_mul(0x01000193);
    for &b in cipher {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    (h ^ (h >> 16) ^ (h >> 8)) as u8
}

pub static VAULT: SpinLock<Vault> = SpinLock::new(Vault::new());

pub struct Vault {
    sectors: [[u8; VAULT_SECTOR_SIZE]; VAULT_SECTORS],
    tags: [u8; VAULT_SECTORS],
    used: [bool; VAULT_SECTORS],
    disposed: bool,
    rk_data: aes::RoundKeys,
    rk_tweak: aes::RoundKeys,
    prepared: bool,
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}

impl Vault {
    pub const fn new() -> Self {
        Self {
            sectors: [[0; VAULT_SECTOR_SIZE]; VAULT_SECTORS],
            tags: [0; VAULT_SECTORS],
            used: [false; VAULT_SECTORS],
            disposed: false,
            rk_data: [[0; 16]; aes::ROUND_KEY_COUNT],
            rk_tweak: [[0; 16]; aes::ROUND_KEY_COUNT],
            prepared: false,
        }
    }

    /// Expand the key schedule (idempotent). The key schedule is wiped by
    /// `dispose` so it must be re-derived after a Panic-Zeroing event.
    fn ensure_keys(&mut self) {
        if !self.prepared {
            self.rk_data = aes::expand_key(&KEY_DATA);
            self.rk_tweak = aes::expand_key(&KEY_TWEAK);
            self.prepared = true;
        }
    }

    /// Store a secret into the vault (AES-XTS encrypted). Returns sector or None.
    pub fn store(&mut self, data: &[u8]) -> Option<usize> {
        self.disposed = false;
        self.ensure_keys();
        if data.len() > VAULT_SECTOR_SIZE {
            return None;
        }
        for (idx, used) in self.used.iter_mut().enumerate() {
            if !*used {
                let mut block = [0u8; VAULT_SECTOR_SIZE];
                block[..data.len()].copy_from_slice(data);

                aes::xts_encrypt_unit(&self.rk_data, &self.rk_tweak, idx as u32, &mut block);
                self.sectors[idx] = block;

                self.tags[idx] = sector_tag(&self.sectors[idx], idx);

                *used = true;
                return Some(idx);
            }
        }
        None
    }

    /// Retrieve and decrypt a secret. Returns false if invalid (tamper,
    /// disposed, out-of-range, or never used).
    pub fn retrieve(&self, sector: usize, out: &mut [u8]) -> bool {
        if self.disposed || sector >= VAULT_SECTORS || !self.used[sector] {
            return false;
        }
        let tag = sector_tag(&self.sectors[sector], sector);
        if tag != self.tags[sector] {
            return false;
        }
        let mut block = self.sectors[sector];
        aes::xts_decrypt_unit(&self.rk_data, &self.rk_tweak, sector as u32, &mut block);
        out.copy_from_slice(&block[..out.len()]);
        true
    }

    /// Wipe one sector (used on revocation).
    pub fn forget(&mut self, sector: usize) {
        if sector < VAULT_SECTORS {
            self.sectors[sector] = [0; VAULT_SECTOR_SIZE];
            self.tags[sector] = 0;
            self.used[sector] = false;
        }
    }

    /// Panic-Zeroing: atomically wipe everything INCLUDING the key schedule.
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        for s in self.sectors.iter_mut() {
            *s = [0; VAULT_SECTOR_SIZE];
        }
        for t in self.tags.iter_mut() {
            *t = 0;
        }
        for u in self.used.iter_mut() {
            *u = false;
        }
        for rk in self.rk_data.iter_mut() {
            *rk = [0; 16];
        }
        for rk in self.rk_tweak.iter_mut() {
            *rk = [0; 16];
        }
        self.prepared = false;
    }

    /// Whether the vault is currently disposed (panic-zeroing happened).
    pub fn is_disposed(&self) -> bool {
        self.disposed
    }
}

/// Test hook: flip one bit in a stored sector's ciphertext.
pub fn tamper_bit(sector: usize, bit: usize) -> bool {
    let mut v = VAULT.lock();
    if v.disposed || sector >= VAULT_SECTORS || !v.used[sector] || bit >= VAULT_SECTOR_SIZE * 8 {
        return false;
    }
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    v.sectors[sector][byte] ^= mask;
    true
}

/// Store a secret in the vault (kernel-friendly API).
pub fn store(data: &[u8]) -> Option<usize> {
    VAULT.lock().store(data)
}

/// Retrieve a secret.
pub fn retrieve(sector: usize, out: &mut [u8]) -> bool {
    VAULT.lock().retrieve(sector, out)
}

/// Forget one sector.
pub fn forget(sector: usize) {
    VAULT.lock().forget(sector)
}

/// Wipe all secrets (invoked by panic handler / exception path).
pub fn dispose_all() {
    VAULT.lock().dispose();
}
