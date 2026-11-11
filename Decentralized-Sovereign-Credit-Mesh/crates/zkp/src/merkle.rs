use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf: String,
    pub path: Vec<(String, bool)>,
    pub root: String,
}

fn hash_node(left: &[u8], right: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    format!("{:x}", hasher.finalize())
}

fn hash_leaf(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"LEAF:v1:");
    hasher.update(value);
    format!("{:x}", hasher.finalize())
}

pub fn build_merkle_tree(values: &[Vec<u8>]) -> (Vec<Vec<String>>, String) {
    if values.is_empty() {
        return (vec![], String::new());
    }

    let leaves: Vec<String> = values.iter().map(|v| hash_leaf(v)).collect();
    let mut tree: Vec<Vec<String>> = vec![leaves.clone()];
    let mut level = leaves;

    while level.len() > 1 {
        let mut next_level = Vec::new();
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                next_level.push(hash_node(chunk[0].as_bytes(), chunk[1].as_bytes()));
            } else {
                next_level.push(chunk[0].clone());
            }
        }
        tree.push(next_level.clone());
        level = next_level;
    }

    let root = level.first().cloned().unwrap_or_default();
    (tree, root)
}

pub fn prove_membership(value: &[u8], tree: &[Vec<String>]) -> Option<MerkleProof> {
    if tree.is_empty() {
        return None;
    }

    let leaf = hash_leaf(value);
    let leaf_index = tree[0].iter().position(|l| l == &leaf)?;

    let mut path = Vec::new();
    let mut index = leaf_index;

    for level in tree.iter().take(tree.len() - 1) {
        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
        if sibling_index < level.len() {
            let is_right = index % 2 == 0;
            path.push((level[sibling_index].clone(), is_right));
        }
        index /= 2;
    }

    let root = tree
        .last()
        .and_then(|l| l.first())
        .cloned()
        .unwrap_or_default();

    Some(MerkleProof { leaf, path, root })
}

pub fn verify_membership(proof: &MerkleProof) -> bool {
    let mut current = proof.leaf.clone();

    for (sibling, is_right) in &proof.path {
        current = if *is_right {
            hash_node(current.as_bytes(), sibling.as_bytes())
        } else {
            hash_node(sibling.as_bytes(), current.as_bytes())
        };
    }

    current == proof.root
}
