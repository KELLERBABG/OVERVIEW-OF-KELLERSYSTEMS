//! FHE-optimized neural network layers.
//!
//! This module brings **homomorphic AI inference** to FHE-DB. It implements
//! the homomorphic inference stack:
//!
//! - **FHE-optimized neural layers** — [`layers::DenseLayer`] and
//!   [`layers::FeedForward`] evaluated entirely over ciphertexts.
//! - **Polynomial activations** — [`activation::Activation`] (square,
//!   polynomial sigmoid, identity) because only polynomial functions are
//!   homomorphically computable.
//! - **Encrypted weight obfuscation** — a [`DenseLayer`]'s weights are
//!   `Ciphertext`s. The server never sees weight plaintext; plaintext bias
//!   and the layer shape are visible.
//! - **Fixed-point arithmetic** — [`fixed_point::FixedPoint`] encodes signed
//!   reals into width-tagged two's-complement integers that TFHE-rs can
//!   process.
//!
//! ## Zero-trust inference flow
//!
//! 1. The model owner encodes weights with [`FixedPoint::encode`] and
//!    encrypts them with [`crate::Encryptor`].
//! 2. A client encrypts its input with the same format.
//! 3. The server evaluates [`layers::FeedForward::eval`] — it sees only
//!    ciphertexts, so it never sees input or weight plaintext. Plaintext bias
//!    and the model shape are visible to the server.
//! 4. The client decrypts the output and decodes it with
//!    [`FixedPoint::decode_plaintext`].
//!
//! ## Noise awareness
//!
//! Every homomorphic multiplication adds noise. Low-degree activations
//! (degree ≤ 5 here) keep the circuit shallow enough for TFHE's default
//! parameters without bootstrapping in this phase. Deeper networks will need
//! bootstrapping-aware training (roadmap stage 3) — see `PROJECT_STATUS.md`.
//!
//! ## Width & scale strategy (the wider-width path)
//!
//! The current layers are exact only at **Q15.0** (`FixedPoint { width: U16,
//! frac_bits: 0 }`): [`Activation::eval`] and [`DenseLayer::eval`] treat
//! coefficients and inputs as plain integers, so any `frac_bits > 0` would
//! silently mis-scale polynomial terms (products of encoded values carry
//! `scale²`). The documented path to fractional/wider formats is:
//!
//! 1. **Quantize training to the inference scale** — train in `f64`, then
//!    round weights/inputs to the exact Q15.0 integers used at inference
//!    (what `examples/encrypted_iris` does; this is the quantization-aware /
//!    bootstrapping-aware principle applied at the demo scale).
//! 2. **Scale planning for fractions** — before enabling `frac_bits > 0`,
//!    every layer must re-derive the per-term scale so cross-scale products
//!    are renormalized (a scale-corrected `Activation::eval`). Input
//!    magnitude × weight magnitude × fan-in must stay inside the signed
//!    range; larger nets need U32/U64 or multi-slot packing.
//! 3. **Multi-slot packing** (planned) — pack several scalar values into one
//!    ciphertext with TFHE-rs's packed/integer CRT modes to amortize
//!    bootstrapping across a batch of neurons.
//!
//! The golden rule: never raise `frac_bits` without a scale-corrected
//! activation and a range audit of every intermediate product.

pub mod activation;
pub mod fixed_point;
pub mod layers;

pub use activation::Activation;
pub use fixed_point::FixedPoint;
pub use layers::{DenseLayer, FeedForward};
