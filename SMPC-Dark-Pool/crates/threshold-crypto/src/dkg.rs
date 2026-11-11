use crate::bls::generate_keypair;
use crate::types::{KeyShare, PublicKey, SecretKey};
use ark_bn254::G2Projective;
use ark_ec::Group;
use sss::sss::split_secret;

pub struct DistributedKeyGeneration {
    pub threshold: u64,
    pub total_nodes: u64,
}

impl DistributedKeyGeneration {
    pub fn new(threshold: u64, total_nodes: u64) -> Self {
        Self {
            threshold,
            total_nodes,
        }
    }

    pub fn generate_shares(&self) -> (Vec<KeyShare>, PublicKey) {
        let (master_sk, _master_pk) = generate_keypair();
        self.generate_shares_from_secret(master_sk)
    }

    pub fn generate_shares_from_secret(&self, master_sk: SecretKey) -> (Vec<KeyShare>, PublicKey) {
        let master_pk = PublicKey {
            point: G2Projective::generator() * master_sk.scalar,
        };
        let sk_shares = split_secret(master_sk.scalar, self.total_nodes, self.threshold)
            .expect("failed to split secret key");
        let key_shares: Vec<KeyShare> = sk_shares
            .into_iter()
            .map(|share| {
                let sk = SecretKey {
                    scalar: share.value,
                };
                let pk = PublicKey {
                    point: G2Projective::generator() * sk.scalar,
                };
                KeyShare {
                    index: share.index,
                    secret: sk,
                    public: pk,
                }
            })
            .collect();
        (key_shares, master_pk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::Group;

    #[test]
    fn test_dkg_generates_correct_number_of_shares() {
        let dkg = DistributedKeyGeneration::new(3, 5);
        let (shares, _) = dkg.generate_shares();
        assert_eq!(shares.len(), 5);
    }

    #[test]
    fn test_dkg_shares_have_public_keys() {
        let dkg = DistributedKeyGeneration::new(2, 4);
        let (shares, _) = dkg.generate_shares();
        for share in &shares {
            let expected_pk = G2Projective::generator() * share.secret.scalar;
            assert_eq!(share.public.point, expected_pk);
        }
    }
}
