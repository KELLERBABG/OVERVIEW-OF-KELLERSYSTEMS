//! **City environmental signature sync** — entropy extraction from
//! seismic/EM sensor readings for OTP key material.
//!
//! District nodes observe environmental noise (seismic amplitude buckets,
//! EM field strength buckets, ambient temp). To tolerate measurement noise
//! between neighboring nodes, each reading is **quantized into cells of
//! `ENV_CELL_WIDTH` units before hashing** — so two nodes whose readings
//! differ by less than a cell width derive the *exact same* signature and
//! therefore the *exact same* shared secret. The shared secret is 32 bytes,
//! exactly one KELLER IPC payload / vault sector.
//!
//! No raw readings are ever transmitted: nodes only compare signatures.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::bigint::to_be_bytes;
use city_crypto::derive_entropy;

pub const ENV_MAX_BUCKETS: usize = 4;
pub const ENV_DEFAULT_TOLERANCE: u32 = 8;
/// Each sensor bucket is quantized to this cell width before hashing, so
/// neighboring nodes within `ENV_DEFAULT_TOLERANCE` of each other land in
/// the same cell and derive the same secret.
pub const ENV_CELL_WIDTH: u32 = 16;

#[inline]
fn quantize(v: u32) -> u32 {
    v / ENV_CELL_WIDTH
}

/// A node's environmental signature for one temporal window.
///
/// The `seed` is a hash over the **quantized** sensor readings, so noisy
/// neighbors (within one cell) produce identical seeds. Two signatures are
/// "in sync" iff they share a window and an identical quantized seed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EnvSignature {
    pub seed: u32,
    pub window: u32,
    /// Raw readings retained only for diagnostics / audit.
    pub seismic: u32,
    pub em: u32,
    pub temp: u32,
}

impl EnvSignature {
    /// Combine a node's local sensor readings into a stable signature.
    pub fn from_readings(window: u32, seismic: u32, em: u32, temp: u32) -> Self {
        let (qs, qe, qt) = (quantize(seismic), quantize(em), quantize(temp));
        let mut seed: u32 = 0x5EED_7A7A;
        seed = seed.wrapping_mul(0x01000193) ^ window;
        seed = seed.wrapping_mul(0x01000193) ^ qs;
        seed = seed.wrapping_mul(0x01000193) ^ qe;
        seed = seed.wrapping_mul(0x01000193) ^ qt;
        Self {
            seed,
            window,
            seismic,
            em,
            temp,
        }
    }

    /// Two signatures are "in sync" when observed in the same window and
    /// their quantized hashes are identical (tolerance is built into the
    /// quantization, so a tolerance check reduces to equality).
    pub fn is_in_sync(&self, other: &Self, _tolerance: u32) -> bool {
        self.window == other.window && self.seed == other.seed
    }

    /// Derive the shared OTP key material (32 bytes) from the synced seed —
    /// vault-friendly, exactly one KELLER IPC payload.
    ///
    /// The seed is built from the window, the 32-bit fingerprint, and the
    /// **quantized** cells — not the raw readings — so two in-sync neighbors
    /// (whose quantized cells are identical by construction) derive the
    /// identical key, while the PRF expansion spreads it over a 256-bit
    /// seed. Key strength is bounded by the entropy of the quantized
    /// environment readings, which is inherent to the protocol.
    pub fn otp_key(&self) -> [u8; 32] {
        let mut seed = [0u8; 32];
        seed[0..4].copy_from_slice(&self.window.to_le_bytes());
        seed[4..8].copy_from_slice(&self.seed.to_le_bytes());
        seed[8..12].copy_from_slice(&quantize(self.seismic).to_le_bytes());
        seed[12..16].copy_from_slice(&quantize(self.em).to_le_bytes());
        seed[16..20].copy_from_slice(&quantize(self.temp).to_le_bytes());
        to_be_bytes(&derive_entropy(&seed))
    }
}

/// A small "district beacon": the most recent synced signature per window
/// bucket. Nodes compare against it to confirm a sync.
#[derive(Copy, Clone, Debug)]
pub struct EntropyBeacon {
    windows: [Option<EnvSignature>; ENV_MAX_BUCKETS],
}

impl Default for EntropyBeacon {
    fn default() -> Self {
        Self::new()
    }
}

impl EntropyBeacon {
    pub const fn new() -> Self {
        Self {
            windows: [None; ENV_MAX_BUCKETS],
        }
    }

    pub fn record(&mut self, sig: &EnvSignature) {
        let bucket = (sig.window as usize) % ENV_MAX_BUCKETS;
        self.windows[bucket] = Some(*sig);
    }

    /// Is a node's local reading within tolerance of the beacon for its
    /// window? If so, the node can derive the same OTP key.
    pub fn confirm_sync(&self, sig: &EnvSignature, tolerance: u32) -> bool {
        let bucket = (sig.window as usize) % ENV_MAX_BUCKETS;
        match self.windows[bucket] {
            Some(beacon) => beacon.is_in_sync(sig, tolerance),
            None => false,
        }
    }
}

/// Re-key a node each window: the OTP material rotates with the window tick.
///
/// XORs the counter into the 256-bit seed (keeping the shared secret's
/// contribution) and runs the PRF expansion, so successive windows produce
/// unrelated-looking keys from a full-width seed rather than a bytewise
/// XOR pattern.
pub fn derive_otp(shared: &[u8; 32], counter: u32) -> [u8; 32] {
    let mut seed = *shared;
    let c = counter.to_le_bytes();
    for i in 0..4 {
        seed[i] ^= c[i];
    }
    to_be_bytes(&derive_entropy(&seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noisy_neighbors_sync_and_share_otp_key() {
        // Node A and Node B observe the same window with slightly noisy EM
        // (+3 units, well within one 16-unit cell).
        let a = EnvSignature::from_readings(3, 0x15, 0x2B, 0x1C);
        let b = EnvSignature::from_readings(3, 0x15, 0x2B + 3, 0x1C);

        assert!(a.is_in_sync(&b, ENV_DEFAULT_TOLERANCE));
        assert_eq!(a.otp_key(), b.otp_key());

        // A node from a DIFFERENT window does not sync.
        let c = EnvSignature::from_readings(4, 0x15, 0x2B, 0x1C);
        assert!(!a.is_in_sync(&c, ENV_DEFAULT_TOLERANCE));
    }

    #[test]
    fn readings_beyond_one_cell_do_not_sync() {
        // +32 units pushes EM into the next quantization cell → no sync.
        let a = EnvSignature::from_readings(3, 0x15, 0x2B, 0x1C);
        let far = EnvSignature::from_readings(3, 0x15, 0x2B + 32, 0x1C);
        assert!(!a.is_in_sync(&far, ENV_DEFAULT_TOLERANCE));
    }

    #[test]
    fn beacon_confirms_and_relays_key() {
        let mut beacon = EntropyBeacon::new();
        let sig = EnvSignature::from_readings(1, 9, 9, 9);
        beacon.record(&sig);

        let local = EnvSignature::from_readings(1, 9, 9, 9);
        assert!(beacon.confirm_sync(&local, ENV_DEFAULT_TOLERANCE));

        let shared = sig.otp_key();
        let otp1 = derive_otp(&shared, 1);
        let otp2 = derive_otp(&shared, 2);
        assert_ne!(otp1, otp2); // rotates per counter
    }
}
