//! FHE-friendly neural layers with **encrypted weight obfuscation**.
//!
//! In a zero-trust setting the cloud must not learn the input data or the
//! sensitive weights. [`DenseLayer`] therefore takes *ciphertext* inputs and
//! *ciphertext* weights, so the server evaluates `W·x + b` while seeing
//! neither input nor weight plaintext.
//!
//! Biases stay plaintext (they are small, system-level constants that leak
//! almost nothing); the sensitive model knowledge — the weight matrix — is
//! fully encrypted. [`FeedForward`] chains dense layers with polynomial
//! activations so a whole encrypted network can be evaluated server-side.

use crate::algebra::CiphertextAlgebra;
use crate::ciphertext::Ciphertext;
use crate::error::{FheError, FheResult};
use crate::nn::activation::Activation;
use crate::nn::fixed_point::FixedPoint;

/// A fully-connected layer whose **weights are encrypted**.
///
/// Layout: `output[j] = activation(bias[j] + sum_i weights[i][j] * input[i])`.
/// The server computes the dot products homomorphically over the encrypted
/// weight matrix (`obfuscated_weights`) and encrypted input; only the
/// plaintext bias is added trivially.
///
/// Deliberately does **not** derive `Debug` or `Copy`: the weights are
/// ciphertexts, which never expose plaintext via debug output, and
/// `Ciphertext` contains heap-allocated TFHE state that cannot be `Copy`.
#[derive(Clone)]
pub struct DenseLayer<'a> {
    /// Row-major encrypted weights: `[input_count][output_count]`.
    pub obfuscated_weights: &'a [Vec<Ciphertext>],
    /// Plaintext bias per output neuron, in the fixed-point scale.
    pub bias: &'a [i64],
    /// Activation applied to each neuron.
    pub activation: &'a Activation,
}

impl DenseLayer<'_> {
    /// Evaluate this dense layer over an encrypted input vector.
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptySlice`] if the input or weights are empty.
    /// - [`FheError::LengthMismatch`] if the input length differs from the
    ///   weight row length, or the output count differs from the bias length.
    /// - [`FheError::WidthMismatch`] on width mismatches.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn eval(
        &self,
        algebra: &CiphertextAlgebra,
        fixed: &FixedPoint,
        input: &[Ciphertext],
    ) -> FheResult<Vec<Ciphertext>> {
        if input.is_empty() || self.obfuscated_weights.is_empty() {
            return Err(FheError::EmptySlice);
        }
        let output_count = self.obfuscated_weights[0].len();
        if self.bias.len() != output_count {
            return Err(FheError::LengthMismatch {
                expected: output_count,
                found: self.bias.len(),
            });
        }
        for row in self.obfuscated_weights {
            if row.len() != output_count {
                return Err(FheError::LengthMismatch {
                    expected: output_count,
                    found: row.len(),
                });
            }
        }

        let mut outputs = Vec::with_capacity(output_count);
        for j in 0..output_count {
            // Gather the j-th weight from every row: `weights[i][j]`.
            let terms: Vec<Ciphertext> = self
                .obfuscated_weights
                .iter()
                .map(|row| row[j].clone())
                .collect();

            // Weighted sum over the encrypted input x encrypted weight.
            let weighted: Vec<Ciphertext> = input
                .iter()
                .zip(terms.iter())
                .map(|(x, w)| algebra.mul(x, w))
                .collect::<FheResult<_>>()?;
            let mut acc = algebra.sum(&weighted)?;

            // Trivial keyless bias addition (plaintext constant).
            if self.bias[j] != 0 {
                let encoded = fixed.encode_scaled(self.bias[j])?;
                acc = algebra.add_scalar(&acc, encoded)?;
            }

            let activated = self.activation.eval(algebra, &acc, fixed)?;
            outputs.push(activated);
        }
        Ok(outputs)
    }
}

/// A feed-forward network: an ordered list of dense layers.
///
/// Each layer's output feeds the next layer's input. This is the executable
/// structure the server runs for privacy-preserving inference.
///
/// Deliberately does **not** derive `Debug` or `Copy` for the same
/// ciphertext-security reasons as [`DenseLayer`].
#[derive(Clone)]
pub struct FeedForward<'a> {
    /// Dense layers, first to last.
    pub layers: &'a [DenseLayer<'a>],
}

impl FeedForward<'_> {
    /// Run the full network over an encrypted input.
    ///
    /// # Errors
    ///
    /// Propagates layer evaluation errors (see [`DenseLayer::eval`]).
    pub fn eval(
        &self,
        algebra: &CiphertextAlgebra,
        fixed: &FixedPoint,
        input: &[Ciphertext],
    ) -> FheResult<Vec<Ciphertext>> {
        let mut current_input = input.to_vec();
        for layer in self.layers {
            current_input = layer.eval(algebra, fixed, &current_input)?;
        }
        Ok(current_input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};

    /// Q15.0: pure-integer fixed point. Weights and biases are ordinary
    /// signed integers, so every product and sum stays exactly in the
    /// 15-bit signed range. The layer encodes the signed bias via
    /// `FixedPoint::encode_scaled`; callers pass the signed value.
    fn setup() -> (ClientKey, CiphertextAlgebra, FixedPoint) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let algebra = CiphertextAlgebra;
        let fixed = FixedPoint::new(ScalarWidth::U16, 0).unwrap();
        (client_key, algebra, fixed)
    }

    fn enc_row(ck: &ClientKey, fixed: &FixedPoint, row: &[f64]) -> Vec<Ciphertext> {
        row.iter()
            .map(|v| {
                Encryptor::new(ck)
                    .encrypt_u64(fixed.encode(*v).unwrap(), fixed.width)
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn dense_matches_plaintext_math() {
        let (client_key, algebra, fixed) = setup();
        // 2 inputs -> 2 outputs, identity activation, zero bias, integer
        // weights under Q15.0.
        let weights = vec![
            enc_row(&client_key, &fixed, &[1.0, -1.0]),
            enc_row(&client_key, &fixed, &[2.0, 3.0]),
        ];
        let bias = [0i64, 0];
        let layer = DenseLayer {
            obfuscated_weights: &weights,
            bias: &bias,
            activation: &Activation::identity(),
        };

        let input = enc_row(&client_key, &fixed, &[2.0, 3.0]);
        let out = layer.eval(&algebra, &fixed, &input).unwrap();
        let v0 = fixed.decode(out[0].decrypt(&client_key).as_u64());
        let v1 = fixed.decode(out[1].decrypt(&client_key).as_u64());
        // w0: 1*2 + 2*3 = 8 ; w1: -1*2 + 3*3 = 7
        assert!((v0 - 8.0).abs() < 1e-6, "v0={v0}");
        assert!((v1 - 7.0).abs() < 1e-6, "v1={v1}");
    }

    #[test]
    fn dense_applies_bias() {
        let (client_key, algebra, fixed) = setup();
        let weights = vec![enc_row(&client_key, &fixed, &[1.0])];
        // Signed bias in fixed-point scale (=1 under Q15.0): the layer
        // encodes it with `FixedPoint::encode_scaled` internally.
        let bias = [3i64];
        let layer = DenseLayer {
            obfuscated_weights: &weights,
            bias: &bias,
            activation: &Activation::identity(),
        };
        let input = enc_row(&client_key, &fixed, &[2.0]);
        let out = layer.eval(&algebra, &fixed, &input).unwrap();
        let v = fixed.decode(out[0].decrypt(&client_key).as_u64());
        assert!((v - 5.0).abs() < 1e-6, "v={v}");
    }

    #[test]
    fn xnand_network_using_square_activation() {
        let (client_key, algebra, fixed) = setup();
        // h = (x0 + x1 - 1)^2 maps {0,1}x{0,1} -> 0 for dissimilar, 1 for
        // identical inputs, i.e. XNOR. Encrypted weights w=[1,1], signed
        // bias -1, square activation.
        let weights = vec![
            enc_row(&client_key, &fixed, &[1.0]),
            enc_row(&client_key, &fixed, &[1.0]),
        ];
        let bias = [-1i64];
        let hidden = DenseLayer {
            obfuscated_weights: &weights,
            bias: &bias,
            activation: &Activation::poly_square(),
        };

        for (a, b, expected) in [(0, 0, 1.0), (0, 1, 0.0), (1, 0, 0.0), (1, 1, 1.0)] {
            let input = vec![
                Encryptor::new(&client_key)
                    .encrypt_u64(fixed.encode(a as f64).unwrap(), fixed.width)
                    .unwrap(),
                Encryptor::new(&client_key)
                    .encrypt_u64(fixed.encode(b as f64).unwrap(), fixed.width)
                    .unwrap(),
            ];
            let out = hidden.eval(&algebra, &fixed, &input).unwrap();
            let v = fixed.decode(out[0].decrypt(&client_key).as_u64());
            assert!(
                (v - expected).abs() < 1e-6,
                "XNOR({a},{b}): v={v}, expected={expected}"
            );
        }
    }
}
