use sybil_core::{DidDocument, SybilResult};

/// DID Resolver that retrieves DID Documents by their DID string.
pub struct DidResolver;

impl DidResolver {
    pub fn new() -> Self {
        Self
    }

    /// Resolve a DID to its document (delegates to a storage backend).
    pub fn resolve(&self, did: &str) -> SybilResult<DidDocument> {
        // In production, this would query a DLT, IPFS, or database.
        // For now, it requires the DID to be a `did:sybil:` format.
        Err(sybil_core::SybilError::NotImplemented(format!(
            "DID resolution for {} - requires storage backend integration",
            did
        )))
    }
}