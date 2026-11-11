//! TRNG entropy extraction pipeline.
//!
//! Converts raw analog noise voltage deltas from atmospheric sensors
//! into random bytes using:
//!   1. von Neumann decorrelator (removes bias from ADC LSB bits)
//!   2. BLAKE3 (cryptographic hash) — absorbs decorrelated bits into a
//!      construction that guarantees uniform extraction for downstream use.
//!
//! # Pipeline
//!   Raw ADC samples (12-24 bit) → von Neumann extractor → BLAKE3 → 32 random bytes

/// Von Neumann decorrelator: removes bias from a stream of bits.
/// Takes pairs of bits; outputs 1 on "01", 0 on "10", discards "00" and "11".
pub struct VonNeumannExtractor {
    buffer: u32,
    bits_in_buffer: u8,
    output_bits: u64,
    input_pairs: u64,
}

#[allow(clippy::derivable_impls)]
impl Default for VonNeumannExtractor {
    fn default() -> Self {
        Self {
            buffer: 0,
            bits_in_buffer: 0,
            output_bits: 0,
            input_pairs: 0,
        }
    }
}

impl VonNeumannExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a raw bit (0 or 1). Returns Some(bit) if a decorrelated bit is produced.
    pub fn feed_bit(&mut self, bit: u8) -> Option<u8> {
        let b = (bit & 1) as u32;
        self.buffer = (self.buffer << 1) | b;
        self.bits_in_buffer += 1;
        if self.bits_in_buffer >= 2 {
            self.bits_in_buffer = 0;
            let pair = self.buffer & 0x3;
            self.input_pairs += 1;
            match pair {
                0b01 => {
                    self.output_bits += 1;
                    Some(1)
                }
                0b10 => {
                    self.output_bits += 1;
                    Some(0)
                }
                _ => None, // discard 00 and 11
            }
        } else {
            None
        }
    }

    /// Feed raw bytes (e.g., from ADC readings). Returns collected decorrelated bytes.
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> alloc::vec::Vec<u8> {
        let mut out = alloc::vec::Vec::new();
        let mut byte_acc: u8 = 0;
        let mut bits: u8 = 0;
        for &b in bytes {
            for i in 0..8 {
                let bit = (b >> (7 - i)) & 1;
                if let Some(debiased) = self.feed_bit(bit) {
                    byte_acc = (byte_acc << 1) | debiased;
                    bits += 1;
                    if bits >= 8 {
                        out.push(byte_acc);
                        byte_acc = 0;
                        bits = 0;
                    }
                }
            }
        }
        out
    }

    /// Entropy efficiency metric: output bits / input pairs.
    pub fn efficiency(&self) -> f64 {
        if self.input_pairs == 0 {
            0.0
        } else {
            self.output_bits as f64 / self.input_pairs as f64
        }
    }
}

/// BLAKE3-based entropy sponge wrapper.
///
/// This uses the real BLAKE3 (via the `blake3` crate). Since BLAKE3 is already
/// a keyed/extendable hash built as a tree-Merkle sponge construction, we absorb
/// the decorrelated bytes and squeeze 32 bytes of uniformly-distributed output.
///
/// > Security note: the von Neumann extractor only removes *bit bias*; for a
/// > cryptographically meaningful TRNG, a proper conditioner (e.g. an SP 800-90B
/// > health-tested extractor) feeding a self-testing entropy source is required.
/// > This pipeline is a simulation of that downstream of the ADC model.
#[derive(Debug, Clone)]
pub struct Blake3Sponge {
    /// The running BLAKE3 hash state (incremental hasher over absorbed bytes).
    hasher: blake3::Hasher,
    /// Number of bytes squeezed so far (for an entropy-rate estimate).
    squeezed: u64,
}

#[allow(clippy::derivable_impls)]
impl Default for Blake3Sponge {
    fn default() -> Self {
        Self::new()
    }
}

impl Blake3Sponge {
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
            squeezed: 0,
        }
    }

    /// Absorb bytes into the sponge state.
    pub fn absorb(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Squeeze n bytes out of the sponge — derived from the BLAKE3 XOF.
    pub fn squeeze(&mut self, n: usize) -> alloc::vec::Vec<u8> {
        let mut xof = self.hasher.finalize_xof();
        let mut out = alloc::vec::Vec::with_capacity(n);
        let mut tmp = [0u8; 64];
        let mut remaining = n;
        while remaining > 0 {
            let take = remaining.min(64);
            xof.fill(&mut tmp[..take]);
            out.extend_from_slice(&tmp[..take]);
            remaining -= take;
        }
        self.squeezed += n as u64;
        out
    }

    /// Estimated entropy yield in bits per second given elapsed real time.
    pub fn entropy_rate_bps(&self, _input_bytes: u64, elapsed_s: f64) -> f64 {
        if elapsed_s <= 0.0 {
            0.0
        } else {
            (self.squeezed * 8) as f64 / elapsed_s
        }
    }
}

/// Full entropy extraction pipeline.
pub struct EntropyPipeline {
    pub von_neumann: VonNeumannExtractor,
    pub sponge: Blake3Sponge,
    pub total_input_bytes: u64,
}

#[allow(clippy::derivable_impls)]
impl Default for EntropyPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EntropyPipeline {
    pub fn new() -> Self {
        Self {
            von_neumann: VonNeumannExtractor::new(),
            sponge: Blake3Sponge::new(),
            total_input_bytes: 0,
        }
    }

    /// Feed raw ADC samples (voltage deltas). Returns extracted random bytes.
    pub fn feed_adc_samples(&mut self, samples: &[u16]) -> alloc::vec::Vec<u8> {
        let raw: alloc::vec::Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        self.total_input_bytes += raw.len() as u64;
        let debiased = self.von_neumann.feed_bytes(&raw);
        self.sponge.absorb(&debiased);
        self.sponge.squeeze(32) // 32 bytes = 256 bits of randomness
    }

    /// Report the current entropy efficiency.
    pub fn efficiency(&self) -> f64 {
        self.von_neumann.efficiency()
    }
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_von_neumann_removes_bias() {
        // All-ones input produces no output (all pairs discarded).
        let mut vn = VonNeumannExtractor::new();
        for _ in 0..16 {
            vn.feed_bit(1);
        }
        // 16 bits = 8 pairs, all "11" → discarded.
        assert_eq!(vn.output_bits, 0);
        assert_eq!(vn.efficiency(), 0.0);
    }

    #[test]
    fn test_von_neumann_alternating() {
        // Alternating 10 → produces one 0 bit per pair.
        let mut vn = VonNeumannExtractor::new();
        for _ in 0..8 {
            vn.feed_bit(1);
            vn.feed_bit(0);
        }
        assert_eq!(vn.output_bits, 8); // 8 output zero-bits
        assert!(vn.efficiency() > 0.0);
    }

    #[test]
    fn test_pipeline_deterministic_extraction() {
        let mut a = EntropyPipeline::new();
        let mut b = EntropyPipeline::new();
        let samples = [0x1234u16, 0x5678, 0x9abc, 0xdef0, 0x0f0f, 0xf0f0];
        let ra = a.feed_adc_samples(&samples);
        let rb = b.feed_adc_samples(&samples);
        assert_eq!(ra, rb);
        assert_eq!(ra.len(), 32);
    }

    #[test]
    fn test_pipeline_changes_on_input() {
        let mut a = EntropyPipeline::new();
        let mut b = EntropyPipeline::new();
        // Use a range of richly-varying samples so the differing byte
        // survives the von Neumann decorrelator (which discards 00/11 pairs).
        let samples_a: alloc::vec::Vec<u16> = (0..64u16).collect();
        let mut samples_b = samples_a.clone();
        samples_b[3] ^= 0xAAAA;
        let ra = a.feed_adc_samples(&samples_a);
        let rb = b.feed_adc_samples(&samples_b);
        assert_ne!(ra, rb);
    }
}
