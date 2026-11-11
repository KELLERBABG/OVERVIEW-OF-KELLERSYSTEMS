//! # encrypted_nn — End-to-End Encrypted Neural Inference Demo
//!
//! Demonstrates Phase 2 of the FHE-DB roadmap: **homomorphic AI inference**
//! with **encrypted weight obfuscation**.
//!
//! The demo runs a 1-neuron network. Both the input and the weights are
//! ciphertexts, so a server evaluating the network never sees input or weight
//! plaintext. The bias and layer shape are plaintext. Only the client, which
//! holds the secret key, decrypts the output.
//!
//! ```text
//! h = ((w0*x0 + w1*x1) + bias)^2     with w0=1, w1=1, bias=-1, square act
//! ```
//!
//! For binary inputs, `(x0 + x1 - 1)^2` equals 1 for `(0,0)` and `(1,1)`, and
//! 0 for `(0,1)` and `(1,0)` — exactly the **XNOR** gate.
//!
//! Run with `cargo run --release -p encrypted_nn`.

use std::sync::Once;

use anyhow::Context as _;
use fhe_core::nn::{Activation, DenseLayer, FixedPoint};
use fhe_core::{CiphertextAlgebra, ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};
use tracing::info;

/// Fixed-point format: Q15.0 over 16-bit storage — pure-integer encoding.
///
/// Multiplying two Q15.0 values stays exactly in the signed 15-bit range, so
/// a tiny exact network like this one needs no fractional rescaling. The
/// layer's `encode_scaled` performs the signed two's-complement encoding.
const FIXED: FixedPoint = FixedPoint {
    width: ScalarWidth::U16,
    frac_bits: 0,
};

fn init_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .init();
    });
}

/// Encrypt a real value using the fixed-point encoding.
fn encrypt(ck: &ClientKey, value: f64) -> anyhow::Result<fhe_core::Ciphertext> {
    let encoded = FIXED.encode(value)?;
    Ok(Encryptor::new(ck).encrypt_u64(encoded, FIXED.width)?)
}

fn main() -> anyhow::Result<()> {
    init_tracing();

    // ------------------------------------------------------------------
    // 1. Keys. The server key is the *only* key material that leaves the
    //    client; it cannot decrypt.
    // ------------------------------------------------------------------
    let (client_key, server_key) = ClientKey::new();
    let _guard = ServerKeyGuard::new(&server_key);
    info!("generated client key (secret) + server key (public, can't decrypt)");

    let algebra = CiphertextAlgebra;

    // ------------------------------------------------------------------
    // 2. Model owner encrypts the weights. This is the obfuscation step:
    //    the plaintext weights never leave this process.
    // ------------------------------------------------------------------
    // h = (w0*x0 + w1*x1 + b)^2 with w0=w1=1, b = -1.
    let weight_0 = encrypt(&client_key, 1.0).context("encrypt w0")?;
    let weight_1 = encrypt(&client_key, 1.0).context("encrypt w1")?;
    // Bias is a small public system constant; it stays plaintext as a signed
    // value in the fixed-point scale. The layer encodes it internally.
    let bias = [-1i64];

    let hidden = DenseLayer {
        obfuscated_weights: &[vec![weight_0], vec![weight_1]],
        bias: &bias,
        activation: &Activation::poly_square(),
    };

    info!("model encrypted: 2 inputs -> 1 neuron (square activation)");
    info!("server holds ciphertexts of w0=1, w1=1, and any input; bias and shape are plaintext");

    // ------------------------------------------------------------------
    // 3. Clients encrypt inputs and submit them for inference.
    // ------------------------------------------------------------------
    let cases: [(f64, f64, f64); 4] = [
        (0.0, 0.0, 1.0), // XNOR(0,0) = 1
        (0.0, 1.0, 0.0), // XNOR(0,1) = 0
        (1.0, 0.0, 0.0), // XNOR(1,0) = 0
        (1.0, 1.0, 1.0), // XNOR(1,1) = 1
    ];

    info!("--- encrypted inference over the whole truth table ---");
    for (x0, x1, expected) in cases {
        let encrypted_x0 = encrypt(&client_key, x0).context("encrypt x0")?;
        let encrypted_x1 = encrypt(&client_key, x1).context("encrypt x1")?;

        // Server-side: evaluate the network over ciphertexts only.
        let outputs = hidden
            .eval(&algebra, &FIXED, &[encrypted_x0, encrypted_x1])
            .context("evaluate encrypted network")?;

        // Client-side: decrypt + decode.
        let decrypted = outputs[0].decrypt(&client_key).as_u64();
        let decoded = FIXED.decode(decrypted);

        // A rounded output near 0 => predict 0; near 1 => predict 1.
        let predicted = (decoded + 0.5).floor();
        info!(
            "  XNOR({x0:.0},{x1:.0}) -> h={decoded:.4}, predicted={predicted:.0}, expected={expected:.0}"
        );
        let ok = (predicted - expected).abs() < 1e-6;
        assert!(
            ok,
            "mismatch at ({x0},{x1}): got {predicted}, expected {expected}"
        );
    }

    info!("[PASS] encrypted XNOR inference verified with obfuscated weights");
    info!(
        "note: the server saw only ciphertexts of the weights and inputs; the \
         plaintext bias and layer shape are visible to it."
    );

    Ok(())
}
