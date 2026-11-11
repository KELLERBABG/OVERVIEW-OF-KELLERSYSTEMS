//! Encrypted iris classification — a **real dataset**, encrypted weights.
//!
//! This is the Phase-2 "real dataset demo" that replaces the toy XNOR with a
//! small real classification problem. It runs the full zero-trust flow:
//!
//! 1. **Train on the client** — a linear softmax classifier over the iris
//!    dataset (150 samples, 4 features, 3 classes), embedded verbatim in
//!    `data/iris.csv` (UCI Machine Learning Repository).
//! 2. **Quantize quantization-aware** — the trained `f64` weights are rounded
//!    to the *exact* integer scale used at inference (`FixedPoint`, Q15.0),
//!    and the plaintext-quantized accuracy is measured. Training and
//!    inference share the same fixed-point arithmetic, the core idea of
//!    bootstrapping-aware/quantization-aware training.
//! 3. **Encrypt the weights** — only the weight and input values are
//!    ciphertexts; the bias and layer shape are plaintext. The "server"
//!    (evaluated locally via [`fhe_core::nn::FeedForward`]) never sees input
//!    or weight plaintext.
//! 4. **Homomorphic inference + client-side argmax** — each encrypted test
//!    sample produces three encrypted scores; the client decrypts them and
//!    picks the argmax class.
//!
//! The encrypted accuracy must equal the plaintext-quantized accuracy
//! exactly (same integer math, now done over ciphertexts); both are printed
//! and asserted.
//!
//! Run: `cargo run --release -p encrypted_iris`

use anyhow::{anyhow, Context as _};
use fhe_core::nn::{Activation, DenseLayer, FeedForward, FixedPoint};
use fhe_core::{Ciphertext, CiphertextAlgebra, ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};

/// The real iris dataset (150 rows), embedded so the demo is reproducible
/// offline. Source: UCI Machine Learning Repository, iris.data.
const IRIS_CSV: &str = include_str!("../data/iris.csv");

const FEATURES: usize = 4;
const CLASSES: usize = 3;
const CLASS_NAMES: [&str; CLASSES] = ["setosa", "versicolor", "virginica"];

/// Input quantization: raw cm × 10, rounded (5.1 → 51). All features stay
/// small positive integers, so Q15.0 products never approach the modulus.
const INPUT_SCALE: f64 = 10.0;
/// Weight quantization: `w_int = round(w * WEIGHT_SCALE)`. Applied uniformly
/// to every weight and bias so the argmax is scale-invariant.
const WEIGHT_SCALE: f64 = 100.0;

/// Fixed-point format for the whole network: Q15.0 (pure-integer scale).
/// This is the only format the layer/activation algebra implements exactly
/// (see the `nn` module docs); training quantizes to it up front.
const FIXED: FixedPoint = FixedPoint {
    width: ScalarWidth::U16,
    frac_bits: 0,
};

struct Sample {
    features: [f64; FEATURES],
    label: usize,
}

fn main() -> anyhow::Result<()> {
    let samples = load_iris()?;
    let (train, test) = split(&samples);

    // --- 1. Train the linear softmax classifier on quantized inputs. -------
    // Inputs are quantized exactly as they will be encrypted, so training
    // sees the same values the server will process.
    let x_train: Vec<Vec<f64>> = train.iter().map(|s| quantize_input(&s.features)).collect();
    let y_train: Vec<usize> = train.iter().map(|s| s.label).collect();
    let model = train_softmax(&x_train, &y_train, 1200, 0.03);

    // --- 2. Quantize to Q15.0 and measure the plaintext accuracy. ----------
    let w_int = quantize_matrix(&model.w);
    let b_int = quantize_vec(&model.b);
    let x_test: Vec<Vec<f64>> = test.iter().map(|s| quantize_input(&s.features)).collect();
    let y_test: Vec<usize> = test.iter().map(|s| s.label).collect();

    let plaintext_correct = x_test
        .iter()
        .zip(&y_test)
        .filter(|(x, y)| predict_int(x, &w_int, &b_int) == **y)
        .count();
    let plaintext_acc = plaintext_correct as f64 / x_test.len() as f64;
    println!(
        "plaintext  : {plaintext_correct}/{} correct ({:.1}%) on quantized Q15.0 model",
        x_test.len(),
        plaintext_acc * 100.0
    );

    // --- 3. Encrypt the weights. -------------------------------------------
    let (client_key, server_key) = ClientKey::new();
    let _guard = ServerKeyGuard::new(&server_key);
    let encryptor = Encryptor::new(&client_key);
    let algebra = CiphertextAlgebra;

    let encrypted_weights: Vec<Vec<Ciphertext>> = (0..FEATURES)
        .map(|i| {
            (0..CLASSES)
                .map(|j| {
                    encryptor
                        .encrypt_u64(
                            FIXED.encode_scaled(w_int[i][j]).expect("weight fits"),
                            FIXED.width,
                        )
                        .expect("encrypt weight")
                })
                .collect()
        })
        .collect();
    let layer = DenseLayer {
        obfuscated_weights: &encrypted_weights,
        bias: &b_int,
        activation: &Activation::identity(),
    };
    let network = FeedForward { layers: &[layer] };

    // --- 4. Homomorphic inference over the encrypted test set. -------------
    let mut encrypted_correct = 0usize;
    let mut confusion = [[0usize; CLASSES]; CLASSES];
    for (x, y) in x_test.iter().zip(&y_test) {
        let input: Vec<Ciphertext> = x
            .iter()
            .map(|v| {
                encryptor
                    .encrypt_u64(FIXED.encode(*v).expect("input fits"), FIXED.width)
                    .expect("encrypt input")
            })
            .collect();
        let scores = network
            .eval(&algebra, &FIXED, &input)
            .context("homomorphic inference failed")?;
        let scores_int: Vec<i64> = scores
            .iter()
            .map(|ct| FIXED.decode(ct.decrypt(&client_key).as_u64()) as i64)
            .collect();
        let pred = argmax(&scores_int);
        confusion[*y][pred] += 1;
        if pred == *y {
            encrypted_correct += 1;
        }
    }
    let encrypted_acc = encrypted_correct as f64 / x_test.len() as f64;
    println!(
        "encrypted  : {encrypted_correct}/{} correct ({:.1}%) — scores decrypted client-side",
        x_test.len(),
        encrypted_acc * 100.0
    );
    print_confusion(&confusion);

    // The encrypted result must be *exactly* the plaintext-quantized result:
    // the same integer arithmetic, evaluated over ciphertexts.
    if plaintext_correct != encrypted_correct {
        return Err(anyhow!(
            "encrypted accuracy {encrypted_correct} != plaintext quantized {plaintext_correct} — the homomorphic path diverged from plaintext math"
        ));
    }
    if encrypted_acc < 0.8 {
        return Err(anyhow!(
            "test accuracy {encrypted_acc:.2} is below the 0.80 floor — the demo regressed"
        ));
    }

    println!(
        "[PASS] encrypted iris inference matches plaintext exactly ({encrypted_correct}/{})",
        x_test.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Dataset
// ---------------------------------------------------------------------------

fn load_iris() -> anyhow::Result<Vec<Sample>> {
    let mut samples = Vec::new();
    for (line_no, line) in IRIS_CSV.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split(',');
        let mut features = [0.0f64; FEATURES];
        for f in features.iter_mut() {
            let token = parts
                .next()
                .ok_or_else(|| anyhow!("line {}: missing feature", line_no + 1))?;
            *f = token
                .parse()
                .with_context(|| format!("line {}: invalid feature `{token}`", line_no + 1))?;
        }
        let cls = parts
            .next()
            .ok_or_else(|| anyhow!("line {}: missing class", line_no + 1))?;
        let label = CLASS_NAMES
            .iter()
            .position(|c| cls == format!("Iris-{c}"))
            .ok_or_else(|| anyhow!("line {}: unknown class `{cls}`", line_no + 1))?;
        samples.push(Sample { features, label });
    }
    if samples.len() != 150 {
        return Err(anyhow!(
            "expected 150 iris samples, got {} — the embedded CSV is corrupt",
            samples.len()
        ));
    }
    Ok(samples)
}

/// Deterministic train/test split: every 5th sample (indices 4, 9, 14, ...)
/// is held out → 120 train / 30 test, 10 per class. No randomness, so the
/// reported numbers are reproducible.
fn split(samples: &[Sample]) -> (Vec<&Sample>, Vec<&Sample>) {
    let mut train = Vec::new();
    let mut test = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        if i % 5 == 4 {
            test.push(s);
        } else {
            train.push(s);
        }
    }
    (train, test)
}

/// Quantize a raw feature vector to the integer scale used at inference.
fn quantize_input(features: &[f64; FEATURES]) -> Vec<f64> {
    features.iter().map(|v| (v * INPUT_SCALE).round()).collect()
}

// ---------------------------------------------------------------------------
// Plaintext training (softmax cross-entropy, full-batch gradient descent)
// ---------------------------------------------------------------------------

struct Model {
    /// `w[i][j]` — weight from input `i` to class `j`.
    w: Vec<Vec<f64>>,
    /// `b[j]` — bias of class `j`.
    b: Vec<f64>,
}

fn softmax(z: &[f64]) -> Vec<f64> {
    let max = z.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = z.iter().map(|v| (v - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|e| e / sum).collect()
}

fn train_softmax(x: &[Vec<f64>], y: &[usize], epochs: usize, lr: f64) -> Model {
    let n = x.len();
    let dim = x[0].len();
    let mut w = vec![vec![0.0f64; CLASSES]; dim];
    let mut b = [0.0f64; CLASSES];

    for _ in 0..epochs {
        let mut dw = vec![vec![0.0f64; CLASSES]; dim];
        let mut db = [0.0f64; CLASSES];
        for (xi, yi) in x.iter().zip(y) {
            let scores: Vec<f64> = (0..CLASSES)
                .map(|j| b[j] + (0..dim).map(|i| w[i][j] * xi[i]).sum::<f64>())
                .collect();
            let probs = softmax(&scores);
            for j in 0..CLASSES {
                let err = probs[j] - if *yi == j { 1.0 } else { 0.0 };
                for i in 0..dim {
                    dw[i][j] += err * xi[i];
                }
                db[j] += err;
            }
        }
        for i in 0..dim {
            for j in 0..CLASSES {
                w[i][j] -= lr * dw[i][j] / n as f64;
            }
        }
        for j in 0..CLASSES {
            b[j] -= lr * db[j] / n as f64;
        }
    }
    Model { w, b: b.to_vec() }
}

// ---------------------------------------------------------------------------
// Quantization (to the exact inference scale) and integer prediction
// ---------------------------------------------------------------------------

fn quantize_matrix(w: &[Vec<f64>]) -> Vec<Vec<i64>> {
    w.iter()
        .map(|row| {
            row.iter()
                .map(|v| (v * WEIGHT_SCALE).round() as i64)
                .collect()
        })
        .collect()
}

fn quantize_vec(b: &[f64]) -> Vec<i64> {
    b.iter()
        .map(|v| (v * WEIGHT_SCALE).round() as i64)
        .collect()
}

fn predict_int(x: &[f64], w: &[Vec<i64>], b: &[i64]) -> usize {
    let scores: Vec<i64> = (0..CLASSES)
        .map(|j| b[j] + (0..FEATURES).map(|i| w[i][j] * x[i] as i64).sum::<i64>())
        .collect();
    argmax(&scores)
}

fn argmax(scores: &[i64]) -> usize {
    scores
        .iter()
        .enumerate()
        .max_by_key(|(_, v)| *v)
        .map(|(i, _)| i)
        .expect("non-empty scores")
}

fn print_confusion(confusion: &[[usize; CLASSES]; CLASSES]) {
    println!("confusion (rows=truth, cols=predicted):");
    for (i, row) in confusion.iter().enumerate() {
        let row_str: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        println!("  {:>10}: {}", CLASS_NAMES[i], row_str.join(" "));
    }
}
