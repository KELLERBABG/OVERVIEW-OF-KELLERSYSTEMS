use crate::types::{
    AggregatedSignature, PartialSignature, PublicKey, SecretKey, Signature, ThresholdConfig,
    ThresholdError,
};
use ark_bn254::{Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::Group;
use ark_ec::pairing::Pairing;
use ark_ff::{Field, PrimeField, UniformRand};
use rand::thread_rng;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let mut rng = ChaCha12Rng::from_rng(thread_rng()).expect("failed to seed RNG");
    let scalar = Fr::rand(&mut rng);
    let point = G2Projective::generator() * scalar;
    (SecretKey { scalar }, PublicKey { point })
}

pub fn hash_to_g1(msg: &[u8]) -> G1Projective {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    let hash_bytes = hasher.finalize();
    let scalar = Fr::from_le_bytes_mod_order(&hash_bytes);
    G1Projective::generator() * scalar
}

pub fn sign(sk: &SecretKey, msg: &[u8]) -> Signature {
    let h = hash_to_g1(msg);
    Signature {
        point: h * sk.scalar,
    }
}

pub fn verify(pk: &PublicKey, msg: &[u8], sig: &Signature) -> bool {
    let h = hash_to_g1(msg);
    let g2 = G2Projective::generator();
    let lhs = ark_bn254::Bn254::pairing(G1Affine::from(sig.point), G2Affine::from(g2));
    let rhs = ark_bn254::Bn254::pairing(G1Affine::from(h), G2Affine::from(pk.point));
    lhs == rhs
}

pub fn aggregate_signatures(
    partials: &[PartialSignature],
    config: &ThresholdConfig,
    indices: &[u64],
) -> Result<AggregatedSignature, ThresholdError> {
    if partials.len() < config.threshold {
        return Err(ThresholdError::InsufficientSignatures {
            need: config.threshold,
            got: partials.len(),
        });
    }
    let mut result = G1Projective::default();
    for ps in partials.iter() {
        let (num, den) = lagrange_basis(ps.share_index, indices, config.threshold);
        let lambda = num * den.inverse().unwrap();
        result += ps.signature.point * lambda;
    }
    Ok(AggregatedSignature {
        signature: Signature { point: result },
    })
}

pub fn verify_aggregate(master_pk: &PublicKey, msg: &[u8], agg_sig: &AggregatedSignature) -> bool {
    verify(master_pk, msg, &agg_sig.signature)
}

fn lagrange_basis(i: u64, all_indices: &[u64], threshold: usize) -> (Fr, Fr) {
    let xi = Fr::from(i);
    let mut num = Fr::ONE;
    let mut den = Fr::ONE;
    for &j in all_indices.iter().take(threshold) {
        if j != i {
            let xj = Fr::from(j);
            num *= xj;
            den *= xj - xi;
        }
    }
    (num, den)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SecretKey;
    use sss::Share;
    use sss::sss::{reconstruct, split_secret};

    fn setup_key_shares(n: u64, t: u64) -> Vec<crate::types::KeyShare> {
        let (master_sk, _master_pk) = generate_keypair();
        let sk_shares = split_secret(master_sk.scalar, n, t).unwrap();
        sk_shares
            .into_iter()
            .map(|share| {
                let sk = SecretKey {
                    scalar: share.value,
                };
                let pk = PublicKey {
                    point: G2Projective::generator() * sk.scalar,
                };
                crate::types::KeyShare {
                    index: share.index,
                    secret: sk,
                    public: pk,
                }
            })
            .collect()
    }

    #[test]
    fn test_sign_verify() {
        let (sk, pk) = generate_keypair();
        let msg = b"trade_order_42";
        let sig = sign(&sk, msg);
        assert!(verify(&pk, msg, &sig));
    }

    #[test]
    fn test_sign_verify_wrong_message() {
        let (sk, pk) = generate_keypair();
        let sig = sign(&sk, b"real_message");
        assert!(!verify(&pk, b"wrong_message", &sig));
    }

    #[test]
    fn test_threshold_sign_aggregate() {
        let shares = setup_key_shares(5, 3);
        let msg = b"trade_123";
        let partials: Vec<PartialSignature> = shares[..3]
            .iter()
            .map(|ks| {
                let sig = sign(&ks.secret, msg);
                PartialSignature {
                    share_index: ks.index,
                    signature: sig,
                }
            })
            .collect();
        let config = ThresholdConfig::new(3, 5);
        let indices: Vec<u64> = partials.iter().map(|p| p.share_index).collect();
        let agg = aggregate_signatures(&partials, &config, &indices).unwrap();
        let master_sk_coeffs: Vec<Share> = shares[..3]
            .iter()
            .map(|ks| Share::new(ks.index, ks.secret.scalar))
            .collect();
        let master_sk_secret = reconstruct(&master_sk_coeffs).unwrap();
        let master_sk = SecretKey {
            scalar: master_sk_secret,
        };
        let master_pk = PublicKey {
            point: G2Projective::generator() * master_sk.scalar,
        };
        assert!(verify_aggregate(&master_pk, msg, &agg));
    }
}
