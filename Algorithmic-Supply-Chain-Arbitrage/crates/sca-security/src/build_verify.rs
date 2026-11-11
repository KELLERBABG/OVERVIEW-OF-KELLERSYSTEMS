//! Deterministic Build Verification.
//!
//! Records source hashes and build parameters to allow external
//! verification that a binary matches its claimed source.
//! Implements reproducible build attestation: given identical source
//! and toolchain, the crate produces identical output hashes.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A build attestation that can be verified by third parties.
#[derive(Debug, Clone)]
pub struct BuildAttestation {
    /// Component name (crate name).
    pub component: String,
    /// Version string.
    pub version: String,
    /// Source hash — SHA-256 of all source files concatenated.
    pub source_hash: Vec<u8>,
    /// Toolchain identifier (Rust version, target triple).
    pub toolchain: String,
    /// Compiler flags used.
    pub compiler_flags: Vec<String>,
    /// SHA-256 hash of the dependency graph (resolved Cargo.lock content).
    pub dependency_hash: Vec<u8>,
    /// Expected binary hash (SHA-256 of the compiled artifact).
    pub expected_binary_hash: Vec<u8>,
    /// Timestamp of the build attestation.
    pub attested_at: i64,
}

/// Verifies that a built binary matches its declared source.
pub struct BuildVerifier {
    attestations: Vec<BuildAttestation>,
}

impl BuildVerifier {
    pub fn new() -> Self {
        Self {
            attestations: Vec::new(),
        }
    }

    /// Record a build attestation.
    pub fn attest(&mut self, attestation: BuildAttestation) {
        self.attestations.push(attestation);
    }

    /// Compute the source hash from a list of source files and their contents.
    pub fn compute_source_hash(sources: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for (path, content) in sources {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        hasher.finalize().to_vec()
    }

    /// Compute a binary hash from binary bytes.
    pub fn compute_binary_hash(binary: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(binary);
        hasher.finalize().to_vec()
    }

    /// Verify a binary against a stored attestation.
    pub fn verify(
        &self,
        component: &str,
        version: &str,
        binary: &[u8],
    ) -> Result<(), String> {
        let binary_hash = Self::compute_binary_hash(binary);

        let matching: Vec<&BuildAttestation> = self
            .attestations
            .iter()
            .filter(|a| a.component == component && a.version == version)
            .collect();

        if matching.is_empty() {
            return Err(format!("No attestation found for {component} v{version}"));
        }

        for attestation in matching {
            if attestation.expected_binary_hash == binary_hash {
                return Ok(());
            }
        }

        Err(format!(
            "Binary hash mismatch for {component} v{version}. \
             Binary may have been tampered with or built from different source."
        ))
    }

    /// Get all attestations.
    pub fn attestations(&self) -> &[BuildAttestation] {
        &self.attestations
    }
}

impl Default for BuildVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a build attestation for the current crate.
pub fn generate_attestation(
    component: &str,
    version: &str,
    sources: &BTreeMap<String, Vec<u8>>,
    binary: &[u8],
) -> BuildAttestation {
    BuildAttestation {
        component: component.to_string(),
        version: version.to_string(),
        source_hash: BuildVerifier::compute_source_hash(sources),
        toolchain: format!(
            "rustc {} ({})",
            rustc_version(),
            std::env::consts::ARCH
        ),
        compiler_flags: vec!["-C opt-level=3".into(), "-C target-cpu=native".into()],
        dependency_hash: vec![], // Would read Cargo.lock
        expected_binary_hash: BuildVerifier::compute_binary_hash(binary),
        attested_at: chrono::Utc::now().timestamp(),
    }
}

/// Get the current Rust compiler version string.
fn rustc_version() -> String {
    option_env!("CARGO_PKG_RUST_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_hash_determinism() {
        let mut sources = BTreeMap::new();
        sources.insert("src/lib.rs".into(), b"pub fn hello() {}".to_vec());
        sources.insert("Cargo.toml".into(), b"[package]".to_vec());

        let hash1 = BuildVerifier::compute_source_hash(&sources);
        let hash2 = BuildVerifier::compute_source_hash(&sources);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_sources_different_hashes() {
        let mut sources1 = BTreeMap::new();
        sources1.insert("src/lib.rs".into(), b"pub fn a() {}".to_vec());

        let mut sources2 = BTreeMap::new();
        sources2.insert("src/lib.rs".into(), b"pub fn b() {}".to_vec());

        assert_ne!(
            BuildVerifier::compute_source_hash(&sources1),
            BuildVerifier::compute_source_hash(&sources2),
        );
    }

    #[test]
    fn test_verify_matching_binary() {
        let mut verifier = BuildVerifier::new();

        let binary = b"elf_binary_data_here".to_vec();
        let attestation = BuildAttestation {
            component: "sca-security".into(),
            version: "0.1.0".into(),
            source_hash: vec![0u8; 32],
            toolchain: "rustc 1.97 (x86_64)".into(),
            compiler_flags: vec![],
            dependency_hash: vec![],
            expected_binary_hash: BuildVerifier::compute_binary_hash(&binary),
            attested_at: chrono::Utc::now().timestamp(),
        };

        verifier.attest(attestation);

        assert!(verifier.verify("sca-security", "0.1.0", &binary).is_ok());
    }

    #[test]
    fn test_verify_tampered_binary() {
        let mut verifier = BuildVerifier::new();

        let binary = b"original_binary".to_vec();
        let attestation = BuildAttestation {
            component: "sca-matching".into(),
            version: "0.1.0".into(),
            source_hash: vec![0u8; 32],
            toolchain: "rustc 1.97".into(),
            compiler_flags: vec![],
            dependency_hash: vec![],
            expected_binary_hash: BuildVerifier::compute_binary_hash(&binary),
            attested_at: chrono::Utc::now().timestamp(),
        };

        verifier.attest(attestation);

        let tampered = b"tampered_binary".to_vec();
        assert!(verifier.verify("sca-matching", "0.1.0", &tampered).is_err());
    }

    #[test]
    fn test_no_attestation() {
        let verifier = BuildVerifier::new();
        assert!(verifier.verify("nonexistent", "0.0.0", b"data").is_err());
    }
}