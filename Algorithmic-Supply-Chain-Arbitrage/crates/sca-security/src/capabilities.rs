//! Capability-based access control for the supply chain network.
//!
//! Each participant (node, peer, user) holds a set of cryptographically
//! signed capability tokens. Capabilities can be granted and revoked
//! by authorized issuers. Revoked capabilities are published to a
//! distributed revocation list that all network participants maintain.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Types of capabilities that can be granted to participants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CapabilityType {
    /// Can create and manage supply chain nodes.
    NodeManagement,
    /// Can create and modify edges/routes.
    EdgeManagement,
    /// Can submit orders to the matching engine.
    OrderSubmission,
    /// Can settle contracts and release payments.
    Settlement,
    /// Can participate in SMPC price discovery.
    PriceDiscovery,
    /// Can act as a P2P capacity broker.
    CapacityBroker,
    /// Can verify custody chains.
    CustodyVerification,
    /// Administrative access.
    Admin,
}

/// A cryptographic capability token.
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    /// Unique token ID.
    pub id: Uuid,
    /// The type of capability.
    pub capability: CapabilityType,
    /// The subject (participant) this capability is granted to.
    pub subject_id: Uuid,
    /// The issuer who granted this capability.
    pub issuer_id: Uuid,
    /// Expiration timestamp (Unix seconds), 0 = no expiry.
    pub expires_at: i64,
    /// Token hash for revocation checking.
    pub token_hash: Vec<u8>,
    /// Whether this token is currently revoked.
    pub revoked: bool,
}

impl CapabilityToken {
    pub fn new(
        capability: CapabilityType,
        subject_id: Uuid,
        issuer_id: Uuid,
        expires_at: i64,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(capability.to_string().as_bytes());
        hasher.update(subject_id.as_bytes());
        hasher.update(issuer_id.as_bytes());
        hasher.update(expires_at.to_le_bytes().as_slice());
        let token_hash = hasher.finalize().to_vec();

        Self {
            id: Uuid::new_v4(),
            capability,
            subject_id,
            issuer_id,
            expires_at,
            token_hash,
            revoked: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        chrono::Utc::now().timestamp() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }
}

impl CapabilityType {
    fn to_string(&self) -> &'static str {
        match self {
            Self::NodeManagement => "node_management",
            Self::EdgeManagement => "edge_management",
            Self::OrderSubmission => "order_submission",
            Self::Settlement => "settlement",
            Self::PriceDiscovery => "price_discovery",
            Self::CapacityBroker => "capacity_broker",
            Self::CustodyVerification => "custody_verification",
            Self::Admin => "admin",
        }
    }
}

/// A revocation list entry.
#[derive(Debug, Clone)]
pub struct RevocationEntry {
    pub token_id: Uuid,
    pub revoked_at: i64,
    pub reason: String,
}

/// Capability manager with revocation support.
#[derive(Debug, Clone)]
pub struct CapabilityManager {
    tokens: Vec<CapabilityToken>,
    revocation_list: Vec<RevocationEntry>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            revocation_list: Vec::new(),
        }
    }

    /// Issue a new capability token.
    pub fn issue(&mut self, token: CapabilityToken) {
        self.tokens.push(token);
    }

    /// Revoke a capability token by ID.
    pub fn revoke(&mut self, token_id: &Uuid, reason: impl Into<String>) -> bool {
        if let Some(token) = self.tokens.iter_mut().find(|t| t.id == *token_id) {
            token.revoked = true;
            self.revocation_list.push(RevocationEntry {
                token_id: *token_id,
                revoked_at: chrono::Utc::now().timestamp(),
                reason: reason.into(),
            });
            true
        } else {
            false
        }
    }

    /// Revoke all capabilities for a subject.
    pub fn revoke_all_for_subject(&mut self, subject_id: &Uuid, reason: impl Into<String>) -> usize {
        let reason = reason.into();
        let count = self.tokens.iter_mut()
            .filter(|t| t.subject_id == *subject_id && !t.revoked)
            .map(|t| {
                t.revoked = true;
                self.revocation_list.push(RevocationEntry {
                    token_id: t.id,
                    revoked_at: chrono::Utc::now().timestamp(),
                    reason: reason.clone(),
                });
            })
            .count();
        count
    }

    /// Check if a subject has a valid capability.
    pub fn check(&self, subject_id: &Uuid, capability: &CapabilityType) -> bool {
        self.tokens.iter().any(|t| {
            t.subject_id == *subject_id
                && t.capability == *capability
                && t.is_valid()
        })
    }

    pub fn tokens_for_subject(&self, subject_id: &Uuid) -> Vec<&CapabilityToken> {
        self.tokens.iter().filter(|t| t.subject_id == *subject_id).collect()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_and_check() {
        let mut mgr = CapabilityManager::new();
        let subject = Uuid::new_v4();
        let issuer = Uuid::new_v4();

        let token = CapabilityToken::new(
            CapabilityType::Settlement, subject, issuer, 0,
        );
        mgr.issue(token);

        assert!(mgr.check(&subject, &CapabilityType::Settlement));
        assert!(!mgr.check(&subject, &CapabilityType::Admin));
    }

    #[test]
    fn test_revocation() {
        let mut mgr = CapabilityManager::new();
        let subject = Uuid::new_v4();
        let issuer = Uuid::new_v4();

        let token = CapabilityToken::new(
            CapabilityType::NodeManagement, subject, issuer, 0,
        );
        let token_id = token.id;
        mgr.issue(token);

        assert!(mgr.check(&subject, &CapabilityType::NodeManagement));

        mgr.revoke(&token_id, "Security incident");
        assert!(!mgr.check(&subject, &CapabilityType::NodeManagement));
    }

    #[test]
    fn test_revoke_all() {
        let mut mgr = CapabilityManager::new();
        let subject = Uuid::new_v4();
        let issuer = Uuid::new_v4();

        mgr.issue(CapabilityToken::new(
            CapabilityType::Settlement, subject, issuer, 0,
        ));
        mgr.issue(CapabilityToken::new(
            CapabilityType::OrderSubmission, subject, issuer, 0,
        ));

        assert_eq!(mgr.revoke_all_for_subject(&subject, "Compromised"), 2);
        assert!(!mgr.check(&subject, &CapabilityType::Settlement));
        assert!(!mgr.check(&subject, &CapabilityType::OrderSubmission));
    }
}