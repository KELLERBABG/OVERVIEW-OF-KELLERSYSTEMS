use chrono::{Duration, Utc};
use sybil_core::{
    constants::*, NodeId, SybilError, SybilResult, VouchRecord,
};

/// The vouching engine manages issuance, verification, and revocation of vouches.
#[derive(Debug, Clone)]
pub struct VouchingEngine {
    /// In-memory store of vouches (node_id -> list of issued vouches).
    vouches: std::collections::HashMap<NodeId, Vec<VouchRecord>>,
    /// Remaining vouching power per node.
    power: std::collections::HashMap<NodeId, u32>,
}

impl VouchingEngine {
    pub fn new() -> Self {
        Self {
            vouches: std::collections::HashMap::new(),
            power: std::collections::HashMap::new(),
        }
    }

    /// Issue a vouch from `voucher` for `subject`.
    ///
    /// Validates:
    /// - Not self-vouching
    /// - Voucher has remaining vouching power
    /// - Subject is not already vouched by this voucher
    pub fn vouch(
        &mut self,
        voucher: NodeId,
        subject: NodeId,
        depth: u32,
    ) -> SybilResult<VouchRecord> {
        if voucher == subject {
            return Err(SybilError::SelfVouchNotAllowed);
        }

        let remaining = self.power.get(&voucher).copied().unwrap_or(MAX_VOUCHES_PER_NODE);
        if remaining == 0 {
            return Err(SybilError::VouchingPowerExhausted(voucher.to_string()));
        }

        // Check for existing vouch
        if let Some(issued) = self.vouches.get(&voucher) {
            if issued.iter().any(|v| v.subject == subject) {
                return Err(SybilError::Vouching(format!(
                    "{} already vouched for {}",
                    voucher, subject
                )));
            }
        }

        if depth > MAX_TRUST_CHAIN_DEPTH {
            return Err(SybilError::MaxVouchingDepthExceeded {
                limit: MAX_TRUST_CHAIN_DEPTH,
                attempted: depth,
            });
        }

        let record = VouchRecord {
            voucher: voucher.clone(),
            subject: subject.clone(),
            signature: String::new(), // caller sets this after signing
            depth,
            power_consumed: 1,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(VOUCH_EXPIRY_DAYS),
        };

        self.vouches.entry(voucher.clone()).or_default().push(record.clone());
        self.power.insert(voucher, remaining - 1);

        Ok(record)
    }

    /// Revoke a vouch.
    pub fn revoke(&mut self, voucher: &NodeId, subject: &NodeId) -> SybilResult<()> {
        if let Some(issued) = self.vouches.get_mut(voucher) {
            let pos = issued.iter().position(|v| v.subject == *subject)
                .ok_or_else(|| SybilError::Vouching(format!("no vouch from {} to {}", voucher, subject)))?;
            issued.remove(pos);
            // Restore vouching power
            let power = self.power.get(voucher).copied().unwrap_or(0);
            self.power.insert(voucher.clone(), power + 1);
            Ok(())
        } else {
            Err(SybilError::Vouching(format!("no vouches from {}", voucher)))
        }
    }

    /// Get all vouches issued by a node.
    pub fn vouches_by(&self, node: &NodeId) -> Vec<VouchRecord> {
        self.vouches.get(node).cloned().unwrap_or_default()
    }

    /// Get all vouches received by a node.
    pub fn vouches_for(&self, node: &NodeId) -> Vec<VouchRecord> {
        self.vouches
            .values()
            .flatten()
            .filter(|v| v.subject == *node)
            .cloned()
            .collect()
    }

    /// Check if a vouch is still valid.
    pub fn is_vouch_valid(&self, vouch: &VouchRecord) -> bool {
        Utc::now() < vouch.expires_at
    }

    /// Get remaining vouching power for a node.
    pub fn remaining_power(&self, node: &NodeId) -> u32 {
        self.power.get(node).copied().unwrap_or(MAX_VOUCHES_PER_NODE)
    }

    /// Reset vouching power for a node (e.g., when reputation changes).
    pub fn set_vouching_power(&mut self, node: NodeId, power: u32) {
        self.power.insert(node, power.min(MAX_VOUCHES_PER_NODE));
    }
}

impl Default for VouchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vouch_basic() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        let b = NodeId::new();
        let vouch = engine.vouch(a.clone(), b.clone(), 1).unwrap();
        assert_eq!(vouch.voucher, a);
        assert_eq!(vouch.subject, b);
        assert_eq!(vouch.depth, 1);
    }

    #[test]
    fn test_self_vouch_rejected() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        assert!(matches!(
            engine.vouch(a.clone(), a, 1),
            Err(SybilError::SelfVouchNotAllowed)
        ));
    }

    #[test]
    fn test_duplicate_vouch_rejected() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        let b = NodeId::new();
        engine.vouch(a.clone(), b.clone(), 1).unwrap();
        assert!(engine.vouch(a, b, 1).is_err());
    }

    #[test]
    fn test_revoke() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        let b = NodeId::new();
        engine.vouch(a.clone(), b.clone(), 1).unwrap();
        assert!(!engine.vouches_by(&a).is_empty());
        engine.revoke(&a, &b).unwrap();
        assert!(engine.vouches_by(&a).is_empty());
    }

    #[test]
    fn test_vouching_power_exhaustion() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        // Exhaust by issuing max vouches
        for _ in 0..MAX_VOUCHES_PER_NODE {
            let target = NodeId::new();
            engine.vouch(a.clone(), target, 1).unwrap();
        }
        let extra = NodeId::new();
        assert!(engine.vouch(a, extra, 1).is_err());
    }

    #[test]
    fn test_depth_limit() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        let b = NodeId::new();
        assert!(engine.vouch(a, b, MAX_TRUST_CHAIN_DEPTH + 1).is_err());
    }

    #[test]
    fn test_vouch_expiry() {
        let mut engine = VouchingEngine::new();
        let a = NodeId::new();
        let b = NodeId::new();
        let vouch = engine.vouch(a, b, 1).unwrap();
        assert!(engine.is_vouch_valid(&vouch));
        assert!(vouch.expires_at > Utc::now());
    }
}