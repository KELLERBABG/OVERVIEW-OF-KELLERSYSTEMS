use serde::{Deserialize, Serialize};

use crate::field::{Fp, FIELD_PRIME};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub x: u64,
    pub value: u64,
}

pub fn evaluate_polynomial(coefficients: &[Fp], x: Fp) -> Fp {
    let mut result = Fp::zero();
    let mut x_pow = Fp::one();
    for &coeff in coefficients {
        result = result + coeff * x_pow;
        x_pow = x_pow * x;
    }
    result
}

pub fn generate_shares(secret: u64, num_shares: u8, threshold: u8) -> Vec<Share> {
    let k = threshold.max(2) as usize;
    let n = num_shares.max(k as u8);

    let mut coefficients = Vec::with_capacity(k);
    coefficients.push(Fp::new(secret));
    for _ in 1..k {
        coefficients.push(Fp::random());
    }

    let mut shares = Vec::with_capacity(n as usize);
    for i in 1..=n {
        let x = Fp::new(i as u64);
        let value = evaluate_polynomial(&coefficients, x);
        shares.push(Share {
            x: i as u64,
            value: value.value(),
        });
    }
    shares
}

pub fn lagrange_basis(x: u64, other_xs: &[u64], target_x: u64) -> Fp {
    let x_fp = Fp::new(x);
    let target_fp = Fp::new(target_x);
    let mut result = Fp::one();

    for &other in other_xs {
        if other != x {
            let other_fp = Fp::new(other);
            let num = target_fp - other_fp;
            let den = x_fp - other_fp;
            result = result * (num / den);
        }
    }
    result
}

pub fn recover_secret(shares: &[Share], threshold: u8) -> Result<u64, String> {
    let k = threshold as usize;
    if shares.len() < k {
        return Err(format!(
            "need at least {} shares to recover secret, got {}",
            k,
            shares.len()
        ));
    }

    let used_shares = &shares[..k];
    let xs: Vec<u64> = used_shares.iter().map(|s| s.x).collect();

    let mut secret = Fp::zero();
    for share in used_shares {
        let basis = lagrange_basis(share.x, &xs, 0);
        let value = Fp::new(share.value);
        secret = secret + value * basis;
    }

    Ok(secret.value())
}

pub fn split_secret(secret: u64, num_shares: u8, threshold: u8) -> Result<Vec<Share>, String> {
    if secret >= FIELD_PRIME {
        return Err(format!(
            "secret {} exceeds field prime {}",
            secret, FIELD_PRIME
        ));
    }
    if threshold < 2 {
        return Err("threshold must be at least 2".to_string());
    }
    if num_shares < threshold {
        return Err("num_shares must be >= threshold".to_string());
    }
    if num_shares == 0 {
        return Err("num_shares must be positive".to_string());
    }
    Ok(generate_shares(secret, num_shares, threshold))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shamir_roundtrip() {
        let secret = 12345u64;
        let shares = split_secret(secret, 5, 3).unwrap();
        let recovered = recover_secret(&shares[..3], 3).unwrap();
        assert_eq!(secret, recovered);
    }

    #[test]
    fn test_shamir_threshold() {
        let secret = 99999u64;
        let shares = split_secret(secret, 5, 3).unwrap();
        let result = recover_secret(&shares[..2], 3);
        assert!(result.is_err());
    }
}
