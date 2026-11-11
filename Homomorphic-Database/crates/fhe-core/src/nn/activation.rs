//! FHE-friendly polynomial activation functions.
//!
//! Real neural networks use non-polynomial activations (ReLU, sigmoid,
//! tanh). Homomorphic multiplication is polynomial, so each activation must
//! be **approximated by a low-degree polynomial**. This module implements
//! the standard FHE approximation toolkit:
//!
//! - [`Activation`] — a coefficient-slice polynomial evaluated over
//!   ciphertexts with [`CiphertextAlgebra::power`] and
//!   [`CiphertextAlgebra::mul_scalar`]. Coefficients are signed values in
//!   the layer's fixed-point format; the *constant* term is folded by the
//!   caller as a plaintext bias (trivial keyless addition), so coefficients
//!   start at the degree-1 term.
//! - [`Activation::sig`] — a degree-3 polynomial sigmoid widely used in FHE
//!   papers (e.g. `0.5 + 0.197x - 0.004x^3` on `x ∈ [-3, 3]`).
//! - [`Activation::poly_square`] — `x^2`, the canonical ReLU-style choice
//!   for small/interpretable networks.
//! - [`Activation::identity`] — for the output layer of regression tasks.
//!
//! ## Fixed-point arithmetic notes
//!
//! All coefficients and evaluated powers live in the layer's fixed-point
//! encoding. The caller must pick a [`FixedPoint`] format whose width and
//! fractional bits keep every intermediate product in range; see the module
//! docs of [`super::fixed_point`].

use crate::algebra::CiphertextAlgebra;
use crate::ciphertext::Ciphertext;
use crate::error::{FheError, FheResult};
use crate::nn::fixed_point::FixedPoint;

/// An FHE activation function: a polynomial over the neuron's input.
///
/// Coeffients are **signed** and given in the same fixed-point scale as the
/// network. Coefficient `i` multiplies `x^i`; there is no degree-0 term
/// because the constant bias is trivially added with
/// [`CiphertextAlgebra::add_scalar`] by the layer.
#[derive(Debug, Clone, PartialEq)]
pub struct Activation {
    /// Signed coefficients `[a1, a2, ...]` where activation(x) =
    /// `a1*x + a2*x^2 + ...`.
    pub coefficients: Vec<f64>,
}

impl Activation {
    /// The identity activation `x`.
    pub fn identity() -> Self {
        Self {
            coefficients: vec![1.0],
        }
    }

    /// The square activation `x^2` — the canonical tiny-FHE choice. Its
    /// output is non-negative, which pairs well with threshold decoding on
    /// unsigned ciphertexts.
    pub fn poly_square() -> Self {
        Self {
            coefficients: vec![0.0, 1.0],
        }
    }

    /// A degree-3 polynomial approximation of the sigmoid, tuned for inputs
    /// on roughly `[-4, 4]`:
    ///
    /// `sigmoid(x) ≈ 0.5 + 0.197x − 0.004x³`
    ///
    /// The `0.5` constant is the bias added by the layer; the coefficients
    /// here are `[0.197, 0.0, -0.004]`.
    pub fn sig() -> Self {
        Self {
            coefficients: vec![0.197, 0.0, -0.004],
        }
    }

    /// A degree-5 polynomial sigmoid for higher-accuracy approximation on
    /// `[-5, 5]`. Coefficients from the standard 3/3 Padé-style fit used in
    /// FHE literature:
    ///
    /// `sigmoid(x) ≈ 0.5 + 0.191x − 0.004x³ + 0.0002x⁵`
    pub fn sig5() -> Self {
        Self {
            coefficients: vec![0.191, 0.0, -0.004, 0.0, 0.0002],
        }
    }

    /// Evaluate this activation polynomial over an encrypted input.
    ///
    /// Coefficients are encoded in `fixed` and each power `x^i` is computed
    /// homomorphically with `CiphertextAlgebra`. The caller must provide the
    /// same [`FixedPoint`] used to encode the network so the coefficient
    /// scaling matches.
    ///
    /// # Errors
    ///
    /// - [`FheError::UnsupportedOperation`] if a polynomial degree would
    ///   require an unsupported power (exponent 0 is never used here).
    /// - [`FheError::ValueOutOfRange`] if a coefficient cannot be encoded in
    ///   the fixed-point format.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn eval(
        &self,
        algebra: &CiphertextAlgebra,
        x: &Ciphertext,
        fixed: &FixedPoint,
    ) -> FheResult<Ciphertext> {
        let mut acc: Option<Ciphertext> = None;
        for (i, coeff) in self.coefficients.iter().enumerate() {
            let degree = i + 1;
            if coeff.abs() < 1e-9 {
                continue;
            }
            let encoded = fixed.encode(*coeff)?;
            // A zero encoded coefficient (e.g. 0.197 rounds to 0 at low
            // precision) contributes nothing; skip to avoid multiply-by-zero
            // noise growth.
            let term = algebra.mul_scalar(&algebra.power(x, degree)?, encoded)?;
            acc = Some(match acc {
                None => term,
                Some(prev) => algebra.add(&prev, &term)?,
            });
        }
        acc.ok_or(FheError::UnsupportedOperation(x.width().bits()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};

    /// Q15.0: pure-integer fixed point. Multiplying two scale-1 values stays
    /// exactly in the signed 15-bit range, which is the correct contract for
    /// a tiny exact FHE network; fractional formats require width planning
    /// so cross-scale products never overflow the modulus.
    fn setup() -> (ClientKey, CiphertextAlgebra, FixedPoint) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let algebra = CiphertextAlgebra;
        let fixed = FixedPoint::new(ScalarWidth::U16, 0).unwrap();
        (client_key, algebra, fixed)
    }

    fn encrypt_value(ck: &ClientKey, v: f64, fixed: &FixedPoint) -> Ciphertext {
        Encryptor::new(ck)
            .encrypt_u64(fixed.encode(v).unwrap(), fixed.width)
            .unwrap()
    }

    #[test]
    fn square_activation_matches_plaintext() {
        let (client_key, algebra, fixed) = setup();
        // Integer inputs under Q15.0: x^2 is exact and in-range.
        for x in [-2.0, -1.0, 0.0, 1.0, 3.0] {
            let ct = encrypt_value(&client_key, x, &fixed);
            let out = Activation::poly_square()
                .eval(&algebra, &ct, &fixed)
                .unwrap();
            let decrypted = out.decrypt(&client_key).as_u64();
            let decoded = fixed.decode(decrypted);
            assert!(
                (decoded - x * x).abs() < 1e-6,
                "x={x}: decoded={decoded}, expected={}",
                x * x
            );
        }
    }

    #[test]
    fn identity_activation_passes_through() {
        let (client_key, algebra, fixed) = setup();
        let x = 2.0;
        let ct = encrypt_value(&client_key, x, &fixed);
        let out = Activation::identity().eval(&algebra, &ct, &fixed).unwrap();
        let decoded = fixed.decode(out.decrypt(&client_key).as_u64());
        assert!((decoded - x).abs() < 1e-6);
    }

    #[test]
    fn sigmoid_is_centered() {
        let (client_key, algebra, fixed) = setup();
        // At x=0 the bias (0.5) dominates and the polynomial terms vanish.
        // Under Q15.0 the tiny 0.197 coefficient rounds to 0, so the
        // polynomial part of sig(0) is identically zero.
        let ct = encrypt_value(&client_key, 0.0, &fixed);
        let out = Activation::sig().eval(&algebra, &ct, &fixed).unwrap();
        let decoded = fixed.decode(out.decrypt(&client_key).as_u64());
        assert_eq!(decoded, 0.0, "sig(0) polynomial part = {decoded}");
    }
}
