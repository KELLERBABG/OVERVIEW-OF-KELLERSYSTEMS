//! On-device entropy: ADC noise → true-random bytes + session key.
//!
//! Port of the simulator's TRNG integration (`simulation/src/simulation.rs`):
//! the same `eh_sim_core::entropy::EntropyPipeline` (von Neumann + BLAKE3
//! sponge) runs on the MCU, fed by two independent ADC channels (PV ripple +
//! ELF preamplifier noise).

use esim_core::entropy::EntropyPipeline;

/// Thin wrapper around the simulator's entropy pipeline.
pub struct NodeEntropy {
    pipeline: EntropyPipeline,
    total_bytes: u64,
}

impl Default for NodeEntropy {
    fn default() -> Self {
        Self {
            pipeline: EntropyPipeline::new(),
            total_bytes: 0,
        }
    }
}

impl NodeEntropy {
    /// Feed one ADC sample pair (two independent noisy channels, e.g. PV
    /// voltage ripple and ELF preamplifier output). Feed as often as the
    /// sample loop allows; extracted entropy is accumulated in the sponge.
    pub fn feed(&mut self, sample_a: u16, sample_b: u16) {
        self.pipeline.feed_adc_samples(&[sample_a, sample_b]);
        self.total_bytes += 4; // two u16 inputs
    }

    /// Total entropy input bytes so far.
    pub fn input_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// 32-byte session key derived deterministically from the sponge state.
    /// Mirrors the simulator: key = sponge.squeeze(32) after feeding the
    /// first samples. NOTE: a real device should provision the key out-of-band
    /// once (stored in NVS); this derivation is for simulation parity.
    pub fn session_key(&mut self) -> [u8; 32] {
        let mut k = [0u8; 32];
        k.copy_from_slice(&self.pipeline.sponge.squeeze(32));
        k
    }

    /// Extract n random bytes for nonces / IVs.
    pub fn random_bytes(&mut self, n: usize) -> alloc::vec::Vec<u8> {
        self.pipeline.sponge.squeeze(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_produces_bytes_and_key() {
        let mut e = NodeEntropy::default();
        e.feed(0x1234, 0xABCD);
        e.feed(0x0F0F, 0xF0F0);
        let k = e.session_key();
        assert_eq!(k.len(), 32);
        // Key should not be all zeros (sponge has absorbed input).
        assert!(k.iter().any(|b| *b != 0));
        let r = e.random_bytes(16);
        assert_eq!(r.len(), 16);
    }

    #[test]
    fn same_input_same_key_deterministic() {
        let mut a = NodeEntropy::default();
        let mut b = NodeEntropy::default();
        a.feed(1, 2);
        b.feed(1, 2);
        assert_eq!(a.session_key(), b.session_key());
    }
}
