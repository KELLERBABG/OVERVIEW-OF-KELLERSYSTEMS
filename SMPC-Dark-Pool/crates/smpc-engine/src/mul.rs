use crate::types::{Scalar, SecretShare};
use sss::Share;

pub fn secure_mul(a: &SecretShare, b: &SecretShare) -> SecretShare {
    let prod = *a.value() * *b.value();
    SecretShare::new(Share::new(a.index(), prod))
}

pub fn secure_mul_constant(a: &SecretShare, c: Scalar) -> SecretShare {
    let prod = *a.value() * c;
    SecretShare::new(Share::new(a.index(), prod))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use sss::field::random_scalar;
    use sss::sss::{reconstruct, split_secret};

    #[test]
    fn test_mul_constant_reconstruct() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let constant = Scalar::from(3u64);
        let shares = split_secret(secret, 5, 3).unwrap();
        let multiplied: Vec<Share> = shares
            .into_iter()
            .map(|s| secure_mul_constant(&SecretShare::new(s), constant).share)
            .collect();
        let recovered = reconstruct(&multiplied[..3]).unwrap();
        assert_eq!(recovered, secret * constant);
    }
}
