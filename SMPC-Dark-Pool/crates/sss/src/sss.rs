use crate::field::ScalarField;
use crate::polynomial::{Polynomial, lagrange_interpolate};
use crate::shares::Share;
use rand::thread_rng;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SssError {
    #[error("threshold t must be >= 1, got {0}")]
    InvalidThreshold(u64),
    #[error("number of shares n must be >= t, got n={n} t={t}")]
    InsufficientShares { n: u64, t: u64 },
    #[error("need at least t shares to reconstruct, got {0}")]
    NotEnoughShares(usize),
}

pub fn split_secret(secret: ScalarField, n: u64, t: u64) -> Result<Vec<Share>, SssError> {
    if t < 1 {
        return Err(SssError::InvalidThreshold(t));
    }
    if n < t {
        return Err(SssError::InsufficientShares { n, t });
    }
    let degree = (t - 1) as usize;
    let mut rng = ChaCha12Rng::from_rng(thread_rng()).expect("failed to seed RNG");
    let poly = Polynomial::random(degree, secret, &mut rng);
    let shares: Vec<Share> = (1..=n)
        .map(|i| Share::new(i, poly.evaluate(&ScalarField::from(i))))
        .collect();
    Ok(shares)
}

pub fn reconstruct(shares: &[Share]) -> Result<ScalarField, SssError> {
    if shares.is_empty() {
        return Err(SssError::NotEnoughShares(0));
    }
    let pairs: Vec<(ScalarField, ScalarField)> = shares.iter().map(|s| s.as_pair()).collect();
    Ok(lagrange_interpolate(&pairs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::random_scalar;
    use rand::thread_rng;

    #[test]
    fn test_split_and_reconstruct() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let shares = split_secret(secret, 5, 3).unwrap();
        assert_eq!(shares.len(), 5);
        let recovered = reconstruct(&shares[..3]).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_reconstruct_with_different_subset() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let shares = split_secret(secret, 5, 3).unwrap();
        let subset = vec![shares[1].clone(), shares[3].clone(), shares[4].clone()];
        let recovered = reconstruct(&subset).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_not_enough_shares() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let shares = split_secret(secret, 5, 3).unwrap();
        let result = reconstruct(&shares[..2]);
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), secret);
    }

    #[test]
    fn test_invalid_threshold() {
        let secret = random_scalar(&mut thread_rng());
        let result = split_secret(secret, 5, 0);
        assert!(matches!(result, Err(SssError::InvalidThreshold(0))));
    }

    #[test]
    fn test_insufficient_shares() {
        let secret = random_scalar(&mut thread_rng());
        let result = split_secret(secret, 2, 5);
        assert!(matches!(result, Err(SssError::InsufficientShares { .. })));
    }
}
