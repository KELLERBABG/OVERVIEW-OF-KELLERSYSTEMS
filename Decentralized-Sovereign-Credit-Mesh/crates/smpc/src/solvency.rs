use serde::{Deserialize, Serialize};

use crate::secret_sharing::{recover_secret, split_secret, Share};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetShares {
    pub shares: Vec<Share>,
    pub num_parties: u8,
    pub threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiabilityShares {
    pub shares: Vec<Share>,
    pub num_parties: u8,
    pub threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolvencyStatement {
    pub entity_id: String,
    pub asset_commitment: String,
    pub liability_commitment: String,
    pub num_parties: u8,
    pub threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyShare {
    pub party_index: u8,
    pub asset_share: u64,
    pub liability_share: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolvencyResult {
    pub entity_id: String,
    pub net_position: i64,
    pub solvent: bool,
    pub parties_used: u8,
}

pub fn prepare_solvency_shares(
    entity_id: &str,
    total_assets: u64,
    total_liabilities: u64,
    num_parties: u8,
    threshold: u8,
) -> Result<(Vec<PartyShare>, SolvencyStatement), String> {
    if total_liabilities > total_assets {
        return Err(format!(
            "entity {} is insolvent: liabilities {} > assets {}",
            entity_id, total_liabilities, total_assets
        ));
    }

    let asset_shares = split_secret(total_assets, num_parties, threshold)?;
    let liability_shares = split_secret(total_liabilities, num_parties, threshold)?;

    use sha2::{Digest, Sha256};
    let asset_commit = format!("{:x}", Sha256::digest(serialize_shares(&asset_shares)));
    let liability_commit = format!("{:x}", Sha256::digest(serialize_shares(&liability_shares)));

    let parties: Vec<PartyShare> = (0..num_parties)
        .map(|i| PartyShare {
            party_index: i,
            asset_share: asset_shares[i as usize].value,
            liability_share: liability_shares[i as usize].value,
        })
        .collect();

    let statement = SolvencyStatement {
        entity_id: entity_id.to_string(),
        asset_commitment: asset_commit,
        liability_commitment: liability_commit,
        num_parties,
        threshold,
    };

    Ok((parties, statement))
}

pub fn compute_net_position(parties: &[PartyShare], threshold: u8) -> Result<i64, String> {
    // Validate the share set: indices must be unique and in a sane range so the
    // Lagrange interpolation below cannot hit a zero denominator or be confused
    // by duplicate parties (finding #12 hardening).
    let mut seen = std::collections::HashSet::new();
    for p in parties {
        if !seen.insert(p.party_index) {
            return Err(format!(
                "duplicate party_index {} in share file",
                p.party_index
            ));
        }
        if p.party_index == 255 {
            return Err(format!("party_index {} out of range", p.party_index));
        }
    }

    let asset_shares: Vec<Share> = parties
        .iter()
        .map(|p| Share {
            x: (p.party_index + 1) as u64,
            value: p.asset_share,
        })
        .collect();

    let liability_shares: Vec<Share> = parties
        .iter()
        .map(|p| Share {
            x: (p.party_index + 1) as u64,
            value: p.liability_share,
        })
        .collect();

    if asset_shares.len() < threshold as usize {
        return Err(format!(
            "need {} parties for threshold, got {}",
            threshold,
            asset_shares.len()
        ));
    }

    let assets = recover_secret(&asset_shares, threshold)?;
    let liabilities = recover_secret(&liability_shares, threshold)?;

    let net = assets as i64 - liabilities as i64;
    Ok(net)
}

pub fn verify_solvency(
    entity_id: &str,
    parties: &[PartyShare],
    threshold: u8,
) -> Result<SolvencyResult, String> {
    let net = compute_net_position(parties, threshold)?;
    let solvent = net >= 0;

    Ok(SolvencyResult {
        entity_id: entity_id.to_string(),
        net_position: net,
        solvent,
        parties_used: parties.len() as u8,
    })
}

fn serialize_shares(shares: &[Share]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(shares.len() * 16);
    for share in shares {
        bytes.extend_from_slice(&share.x.to_le_bytes());
        bytes.extend_from_slice(&share.value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solvent_entity() {
        let (parties, stmt) = prepare_solvency_shares("alice", 10000, 5000, 5, 3).unwrap();
        let result = verify_solvency(&stmt.entity_id, &parties[..3], 3).unwrap();
        assert!(result.solvent);
        assert_eq!(result.entity_id, "alice");
        assert_eq!(result.net_position, 5000);
    }

    #[test]
    fn test_insolvent_rejected() {
        let result = prepare_solvency_shares("bob", 3000, 5000, 5, 3);
        assert!(result.is_err());
    }
}
