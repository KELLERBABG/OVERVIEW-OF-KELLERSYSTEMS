use crate::types::{Scalar, SecretShare};
use ark_ff::Field;
use sss::Share;

pub fn secure_add(a: &SecretShare, b: &SecretShare) -> SecretShare {
    let sum = a.value() + b.value();
    SecretShare::new(Share::new(a.index(), sum))
}

pub fn secure_add_constant(a: &SecretShare, c: Scalar) -> SecretShare {
    let sum = *a.value() + c;
    SecretShare::new(Share::new(a.index(), sum))
}

pub fn batch_secure_add(shares: &[SecretShare]) -> SecretShare {
    shares.iter().fold(
        SecretShare::new(Share::new(shares[0].index(), Scalar::ZERO)),
        |acc, s| secure_add(&acc, s),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::Field;
    use rand::thread_rng;
    use sss::field::random_scalar;
    use sss::sss::{reconstruct, split_secret};

    #[test]
    fn test_add_reconstruct() {
        let mut rng = thread_rng();
        let a = random_scalar(&mut rng);
        let b = random_scalar(&mut rng);
        let a_shares = split_secret(a, 5, 3).unwrap();
        let b_shares = split_secret(b, 5, 3).unwrap();
        let sum_shares: Vec<SecretShare> = a_shares
            .into_iter()
            .zip(b_shares)
            .map(|(a, b)| secure_add(&SecretShare::new(a), &SecretShare::new(b)))
            .collect();
        let raw_shares: Vec<Share> = sum_shares.into_iter().map(|s| s.share).collect();
        let recovered = reconstruct(&raw_shares[..3]).unwrap();
        assert_eq!(recovered, a + b);
    }

    #[test]
    fn test_add_constant() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let constant = random_scalar(&mut rng);
        let shares = split_secret(secret, 5, 3).unwrap();
        let added: Vec<Share> = shares
            .into_iter()
            .map(|s| secure_add_constant(&SecretShare::new(s), constant).share)
            .collect();
        let recovered = reconstruct(&added[..3]).unwrap();
        assert_eq!(recovered, secret + constant);
    }

    #[test]
    fn test_batch_add() {
        let mut rng = thread_rng();
        let vals: Vec<_> = (0..4).map(|_| random_scalar(&mut rng)).collect();
        let all_shares: Vec<Vec<Share>> = vals
            .iter()
            .map(|v| split_secret(*v, 5, 3).unwrap())
            .collect();
        let batch: Vec<SecretShare> = (0..5)
            .map(|i| {
                let shares_at_i: Vec<SecretShare> = all_shares
                    .iter()
                    .map(|s| SecretShare::new(s[i as usize].clone()))
                    .collect();
                batch_secure_add(&shares_at_i)
            })
            .collect();
        let expected: Scalar = vals.iter().fold(Scalar::ZERO, |a, b| a + b);
        let recovered =
            reconstruct(&batch.into_iter().map(|s| s.share).collect::<Vec<_>>()[..3]).unwrap();
        assert_eq!(recovered, expected);
    }
}
