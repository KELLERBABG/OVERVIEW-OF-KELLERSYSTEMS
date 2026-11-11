use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::commitment::{commit, generate_blinding, Commitment};
use crate::merkle::{build_merkle_tree, prove_membership, verify_membership, MerkleProof};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRangeProof {
    pub commitment: Commitment,
    pub min_reputation: f64,
    pub range_proof: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMembershipProof {
    pub identity_id: String,
    pub identity_commitment: Commitment,
    pub merkle_proof: MerkleProof,
    pub reputation_proof: ReputationRangeProof,
    pub cluster_size: usize,
    pub avg_reputation: f64,
    pub timestamp: i64,
}

impl ClusterMembershipProof {
    pub fn prove(
        identity_id: &str,
        identity_secret: &[u8],
        cluster_members: &[Vec<u8>],
        reputation_scores: &[f64],
        min_reputation: f64,
    ) -> Option<Self> {
        let mut member_data: Vec<Vec<u8>> = cluster_members.to_vec();
        member_data.sort();
        member_data.dedup();

        let (tree, _root) = build_merkle_tree(&member_data);

        let _identity_index = member_data.iter().position(|m| m == identity_secret)?;

        let blinding = generate_blinding();
        let identity_commitment = commit(identity_secret, &blinding);

        let merkle_proof = prove_membership(identity_secret, &tree)?;

        let cluster_size = member_data.len();
        let avg_reputation = if reputation_scores.is_empty() {
            0.0
        } else {
            reputation_scores.iter().sum::<f64>() / reputation_scores.len() as f64
        };

        let range_msg = format!(
            "reputation:{}:min:{}:cluster:{}",
            avg_reputation, min_reputation, cluster_size
        );
        let range_proof = format!("{:x}", Sha256::digest(range_msg.as_bytes()));

        let rep_commit = commit(
            format!("{}", avg_reputation).as_bytes(),
            format!("{}", min_reputation).as_bytes(),
        );

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Some(ClusterMembershipProof {
            identity_id: identity_id.to_string(),
            identity_commitment,
            merkle_proof,
            reputation_proof: ReputationRangeProof {
                commitment: rep_commit,
                min_reputation,
                range_proof,
            },
            cluster_size,
            avg_reputation,
            timestamp,
        })
    }

    pub fn verify(&self, min_cluster_size: usize, min_reputation: f64) -> Result<bool, String> {
        if self.cluster_size < min_cluster_size {
            return Ok(false);
        }

        let merkle_valid = verify_membership(&self.merkle_proof);
        if !merkle_valid {
            return Ok(false);
        }

        let stored_min = self.reputation_proof.min_reputation;
        if stored_min < min_reputation {
            return Ok(false);
        }

        let range_msg = format!(
            "reputation:{}:min:{}:cluster:{}",
            self.avg_reputation, stored_min, self.cluster_size
        );
        let expected_range = format!("{:x}", Sha256::digest(range_msg.as_bytes()));
        if self.reputation_proof.range_proof != expected_range {
            return Ok(false);
        }

        if self.avg_reputation < min_reputation {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn summary(&self) -> String {
        format!(
            "ClusterProof[id={}, size={}, avg_rep={:.4}, min_rep={}]",
            self.identity_id,
            self.cluster_size,
            self.avg_reputation,
            self.reputation_proof.min_reputation
        )
    }
}
