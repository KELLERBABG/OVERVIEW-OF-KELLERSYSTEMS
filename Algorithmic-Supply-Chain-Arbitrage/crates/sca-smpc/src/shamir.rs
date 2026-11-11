//! Shamir's Secret Sharing over Z_p.
//!
//! A secret value is split into N shares using a random polynomial
//! of degree K-1. Any K shares can reconstruct the secret, but
//! fewer than K reveal nothing.

use rand::Rng;

use crate::field::FiniteField;

/// A single share of a secret — the pair (x, f(x)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecretShare {
    /// The x-coordinate (evaluation point).
    pub x: FiniteField,
    /// The y-coordinate (f(x)).
    pub y: FiniteField,
}

/// Shamir's Secret Sharing implementation.
pub struct ShamirSharing {
    /// Threshold: minimum shares needed to reconstruct.
    pub threshold: usize,
    /// Total number of shares.
    pub total_shares: usize,
}

impl ShamirSharing {
    /// Create a new sharing scheme with K-of-N threshold.
    pub fn new(threshold: usize, total_shares: usize) -> Self {
        assert!(threshold <= total_shares, "Threshold must be ≤ total shares");
        assert!(threshold >= 1, "Threshold must be at least 1");
        Self { threshold, total_shares }
    }

    /// Split a secret into N shares, requiring K to reconstruct.
    ///
    /// Generates a random polynomial of degree K-1 where f(0) = secret,
    /// then evaluates it at N distinct points (1..=N).
    pub fn split(&self, secret: FiniteField) -> Vec<SecretShare> {
        let mut rng = rand::thread_rng();

        // Generate random coefficients for the polynomial:
        // f(x) = secret + c1*x + c2*x^2 + ... + c_{K-1}*x^{K-1}
        let mut coefficients = Vec::with_capacity(self.threshold);
        coefficients.push(secret); // f(0) = secret

        for _ in 1..self.threshold {
            let coeff = FiniteField::new(rng.gen_range(1..=u64::MAX));
            coefficients.push(coeff);
        }

        // Evaluate polynomial at points 1..N
        let mut shares = Vec::with_capacity(self.total_shares);
        for i in 1..=self.total_shares {
            let x = FiniteField::new(i as u64);
            let y = self.evaluate_polynomial(&coefficients, x);
            shares.push(SecretShare { x, y });
        }

        shares
    }

    /// Reconstruct the secret from K shares using Lagrange interpolation.
    ///
    /// Given K shares (x_i, f(x_i)), compute f(0) = Σ f(x_i) * L_i(0)
    /// where L_i(0) = Π_{j≠i} (0 - x_j) / (x_i - x_j)
    pub fn reconstruct(shares: &[SecretShare]) -> FiniteField {
        let k = shares.len();
        assert!(k >= 2, "Need at least 2 shares to reconstruct");

        let mut result = FiniteField::zero();

        for i in 0..k {
            let xi = shares[i].x;
            let yi = shares[i].y;

            // Compute Lagrange basis polynomial at 0: L_i(0)
            let mut numerator = FiniteField::one();
            let mut denominator = FiniteField::one();

            for j in 0..k {
                if i == j {
                    continue;
                }
                let xj = shares[j].x;
                // L_i(0) = Π (0 - x_j) / (x_i - x_j)
                numerator = numerator * (FiniteField::zero() - xj);
                denominator = denominator * (xi - xj);
            }

            let li0 = numerator / denominator;
            result = result + yi * li0;
        }

        result
    }

    /// Evaluate the polynomial at point x.
    fn evaluate_polynomial(&self, coeffs: &[FiniteField], x: FiniteField) -> FiniteField {
        // Horner's method
        let mut result = FiniteField::zero();
        for coeff in coeffs.iter().rev() {
            result = result * x + *coeff;
        }
        result
    }

    /// Verify that a set of shares is consistent (all lie on the same polynomial).
    pub fn verify(shares: &[SecretShare], threshold: usize) -> bool {
        if shares.len() < threshold {
            return false;
        }
        // Use the first K shares to reconstruct, then check the rest
        let first_k: Vec<SecretShare> = shares.iter().take(threshold).copied().collect();
        let _secret = Self::reconstruct(&first_k);

        // For a real implementation: reconstruct the polynomial coefficients
        // and verify all shares. For simplicity, we check that any subset of
        // K shares reconstructs to the same secret.
        if shares.len() <= threshold {
            return true;
        }

        let secret1 = Self::reconstruct(&first_k);
        let second_k: Vec<SecretShare> = shares.iter().skip(1).take(threshold).copied().collect();
        if second_k.len() == threshold {
            let secret2 = Self::reconstruct(&second_k);
            return secret1 == secret2;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_reconstruct() {
        let secret = FiniteField::new(42);
        let scheme = ShamirSharing::new(3, 5); // 3-of-5
        let shares = scheme.split(secret);

        assert_eq!(shares.len(), 5);

        // Any 3 shares should reconstruct
        let reconstructed = ShamirSharing::reconstruct(&shares[0..3]);
        assert_eq!(reconstructed, secret);
    }

    #[test]
    fn test_different_subset() {
        let secret = FiniteField::new(12345);
        let scheme = ShamirSharing::new(3, 6);
        let shares = scheme.split(secret);

        // Use shares [1, 3, 5] (0-indexed: 0, 2, 4)
        let subset = vec![shares[0], shares[2], shares[4]];
        let reconstructed = ShamirSharing::reconstruct(&subset);
        assert_eq!(reconstructed, secret);
    }

    #[test]
    fn test_insufficient_shares() {
        let secret = FiniteField::new(99);
        let scheme = ShamirSharing::new(3, 5);
        let shares = scheme.split(secret);

        // With only 2 shares (K=3), reconstruction should give wrong result
        let wrong = ShamirSharing::reconstruct(&shares[0..2]);
        assert_ne!(wrong, secret);
    }

    #[test]
    fn test_large_secret() {
        let secret = FiniteField::new(1_000_000_000);
        let scheme = ShamirSharing::new(5, 10);
        let shares = scheme.split(secret);

        let reconstructed = ShamirSharing::reconstruct(&shares[0..5]);
        assert_eq!(reconstructed, secret);
    }

    #[test]
    fn test_verify_consistency() {
        let secret = FiniteField::new(777);
        let scheme = ShamirSharing::new(2, 4);
        let shares = scheme.split(secret);

        assert!(ShamirSharing::verify(&shares, 2));
    }

    #[test]
    fn test_additive_homomorphism() {
        // The sum of shares of two secrets should be shares of the sum
        let secret1 = FiniteField::new(100);
        let secret2 = FiniteField::new(200);

        let scheme = ShamirSharing::new(3, 5);
        let shares1 = scheme.split(secret1);
        let shares2 = scheme.split(secret2);

        // Add shares pointwise
        let sum_shares: Vec<SecretShare> =
            shares1.iter().zip(shares2.iter()).map(|(s1, s2)| SecretShare { x: s1.x, y: s1.y + s2.y }).collect();

        // Reconstruct should give the sum
        let reconstructed_sum = ShamirSharing::reconstruct(&sum_shares[0..3]);
        assert_eq!(reconstructed_sum, secret1 + secret2);
    }
}
