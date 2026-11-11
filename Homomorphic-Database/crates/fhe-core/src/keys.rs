//! Key generation and management.

use serde::{Deserialize, Serialize};

use bincode::Options as _;

use crate::error::{FheError, FheResult};

/// The secret client key.
///
/// This type wraps TFHE-rs' `ClientKey` and supports fresh key generation,
/// serialization to/from bytes for secure storage, encryption of plaintext
/// values (via [`crate::Encryptor`]), and decryption of ciphertexts.
#[derive(Clone, Serialize, Deserialize)]
pub struct ClientKey {
    pub(crate) inner: tfhe::ClientKey,
}

impl std::fmt::Debug for ClientKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the actual key material in debug output.
        f.debug_struct("ClientKey").finish_non_exhaustive()
    }
}

impl ClientKey {
    /// Generate a fresh client key + matching server key.
    ///
    /// Uses the default TFHE-rs configuration (the 2M128 parameter set; the
    /// vendor documents it as targeting IND-CPA^D, but it is unaudited) — the
    /// parameter set pinned by
    /// [`SecurityParameters::pinned_default`] and enforced by the gate test
    /// `default_parameters_match_documented_pinning`.
    ///
    /// # Returns
    ///
    /// `(client_key, server_key)` — the caller is responsible for sending
    /// only the server key to the compute party.
    pub fn new() -> (Self, ServerKey) {
        let config = tfhe::ConfigBuilder::default().build();
        Self::with_config(config)
    }

    /// Generate a keypair given an explicit TFHE configuration.
    pub fn with_config(config: tfhe::Config) -> (Self, ServerKey) {
        let (client_key, server_key) = tfhe::generate_keys(config);
        (Self { inner: client_key }, ServerKey { inner: server_key })
    }

    /// The concrete TFHE security parameters of this key.
    ///
    /// The values are read from the underlying TFHE-rs key and compared
    /// against [`SecurityParameters::pinned_default`] by the gate test, so a
    /// dependency upgrade that silently changes the security level fails the
    /// build instead of shipping unnoticed.
    pub fn computation_parameters(&self) -> SecurityParameters {
        let p = self.inner.computation_parameters();
        SecurityParameters {
            lwe_dimension: p.lwe_dimension().0,
            glwe_dimension: p.glwe_dimension().0,
            polynomial_size: p.polynomial_size().0,
            pbs_base_log: p.pbs_base_log().0,
            pbs_level: p.pbs_level().0,
            ks_base_log: p.ks_base_log().0,
            ks_level: p.ks_level().0,
            message_modulus: p.message_modulus().0,
            carry_modulus: p.carry_modulus().0,
            max_noise_level: p.max_noise_level().get(),
            log2_p_fail: p.log2_p_fail(),
        }
    }

    /// Serialize this client key to binary (bincode) bytes.
    ///
    /// The output should be stored somewhere **trusted and secret** (e.g. an
    /// HSM, OS keychain, or encrypted file).
    pub fn to_bytes(&self) -> FheResult<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize a client key from binary (bincode) bytes.
    ///
    /// The decode is bounded by the input length so a crafted byte stream
    /// cannot force bincode/serde to consume or preallocate beyond the
    /// supplied bytes (nested `Vec` lengths inside the TFHE key layout are
    /// additionally clamped by serde at 1 GiB per sequence — callers that
    /// accept keys from the wire should cap the input size, as
    /// `fhe-server` does for `POST /keys/server`).
    pub fn from_bytes(bytes: &[u8]) -> FheResult<Self> {
        bincode::options()
            .with_fixint_encoding()
            .with_limit(bytes.len() as u64)
            .reject_trailing_bytes()
            .deserialize(bytes)
            .map_err(|e| FheError::CorruptedKey(format!("failed to deserialize client key: {e}")))
    }
}

/// The concrete TFHE security parameters of a generated key.
///
/// This build pins TFHE-rs 1.7's **default integer configuration**,
/// `PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128` (see
/// `tfhe::shortint::parameters`): a 918-dimension LWE lattice with a single
/// 2048-degree GLWE polynomial, 4 message / 4 carry moduli (2-bit messages),
/// and an estimated correctness-failure probability of `2^-129.58`. TFHE-rs
/// names this set 2M128 and documents it as targeting 128-bit IND-CPA^D; the
/// `p_fail` value itself is a decryption-failure estimate, not a security
/// reduction, and this project has not reproduced the lattice estimate or had
/// it audited.
///
/// The gate test `default_parameters_match_documented_pinning` compares a
/// freshly generated key against [`Self::pinned_default`] so that a
/// dependency upgrade changing any of these values fails the build instead of
/// silently altering the security level. See `SECURITY.md` for the full
/// posture.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityParameters {
    /// LWE secret dimension.
    pub lwe_dimension: usize,
    /// GLWE secret dimension.
    pub glwe_dimension: usize,
    /// Polynomial size of the GLWE ring.
    pub polynomial_size: usize,
    /// PBS decomposition base logarithm.
    pub pbs_base_log: usize,
    /// PBS decomposition level count.
    pub pbs_level: usize,
    /// Key-switching decomposition base logarithm.
    pub ks_base_log: usize,
    /// Key-switching decomposition level count.
    pub ks_level: usize,
    /// Shortint message modulus.
    pub message_modulus: u64,
    /// Shortint carry modulus.
    pub carry_modulus: u64,
    /// Maximum noise level of the shortint ciphertexts.
    pub max_noise_level: u64,
    /// Base-2 logarithm of the failure probability (negative).
    pub log2_p_fail: f64,
}

impl SecurityParameters {
    /// The documented, pinned parameter set for this build.
    ///
    /// Values must match a key generated with [`ClientKey::new`] exactly
    /// (see the gate test). They are read from TFHE-rs 1.7.0's
    /// `PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128`.
    pub const fn pinned_default() -> Self {
        Self {
            lwe_dimension: 918,
            glwe_dimension: 1,
            polynomial_size: 2048,
            pbs_base_log: 23,
            pbs_level: 1,
            ks_base_log: 4,
            ks_level: 4,
            message_modulus: 4,
            carry_modulus: 4,
            max_noise_level: 5,
            log2_p_fail: -129.581,
        }
    }
}

/// The public evaluation (server) key.
///
/// This key is **not secret** — it is deliberately handed to the party that
/// performs homomorphic computations. It can only evaluate operations on
/// ciphertexts; it cannot decrypt them.
#[derive(Clone)]
pub struct ServerKey {
    pub(crate) inner: tfhe::ServerKey,
}

impl std::fmt::Debug for ServerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The server key is public, but it is huge; don't dump it.
        f.debug_struct("ServerKey").finish_non_exhaustive()
    }
}

impl Serialize for ServerKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;
        let bytes = bincode::serialize(&self.inner)
            .map_err(|e| S::Error::custom(format!("failed to serialize server key: {e}")))?;
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for ServerKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        let inner = bincode::options()
            .with_fixint_encoding()
            .with_limit(bytes.len() as u64)
            .reject_trailing_bytes()
            .deserialize(&bytes)
            .map_err(|e| D::Error::custom(format!("failed to deserialize server key: {e}")))?;
        Ok(Self { inner })
    }
}

impl ServerKey {
    /// Serialize this server key to binary (bincode) bytes.
    ///
    /// The server key is large (megabytes); this is expected. Callers
    /// transferring it over the network should compress if needed.
    pub fn to_bytes(&self) -> FheResult<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize a server key from binary (bincode) bytes.
    ///
    /// The decode is bounded by the input length (see
    /// [`ClientKey::from_bytes`] for the rationale); the wire endpoint caps
    /// the upload size separately.
    pub fn from_bytes(bytes: &[u8]) -> FheResult<Self> {
        bincode::options()
            .with_fixint_encoding()
            .with_limit(bytes.len() as u64)
            .reject_trailing_bytes()
            .deserialize(bytes)
            .map_err(|e| FheError::CorruptedKey(format!("failed to deserialize server key: {e}")))
    }

    /// Install this server key as the thread-local "active" key.
    ///
    /// TFHE-rs requires a server key to be set for the current thread before
    /// any homomorphic operations can run.
    pub fn set_as_global(&self) {
        tfhe::set_server_key(self.inner.clone());
    }

    /// Install this server key as a process-global fallback.
    ///
    /// When set, all threads inherit the key automatically. This is useful
    /// for batch/background processing. For request servers, prefer
    /// [`ServerKeyGuard`] for per-request key pinning.
    pub fn set_as_global_fallback(&self) -> FheResult<()> {
        let _ = GLOBAL_SERVER_KEY.set(self.inner.clone());
        Ok(())
    }
}

/// Thread-local guard that installs a server key for the duration of a scope.
///
/// TFHE-rs uses a thread-local server key. For request handlers that compute
/// on encrypted data, pin the key for the duration of the computation:
///
/// ```no_run
/// use fhe_core::keys::{ServerKey, ServerKeyGuard};
///
/// fn process(server_key: &ServerKey) {
///     let _guard = ServerKeyGuard::new(server_key);
///     // ... homomorphic operations now work on this thread ...
/// }
/// ```
pub struct ServerKeyGuard {
    _private: (),
}

impl ServerKeyGuard {
    /// Install the given server key on this thread.
    pub fn new(server_key: &ServerKey) -> Self {
        server_key.set_as_global();
        Self { _private: () }
    }
}

static GLOBAL_SERVER_KEY: std::sync::OnceLock<tfhe::ServerKey> = std::sync::OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate test: a freshly generated default key must match the documented,
    /// pinned parameters exactly. If this fails, a TFHE-rs upgrade (or an
    /// accidental config change) has silently altered the security level —
    /// update the pin deliberately, then re-document in `SECURITY.md`.
    #[test]
    fn default_parameters_match_documented_pinning() {
        let (client_key, _server_key) = ClientKey::new();
        let actual = client_key.computation_parameters();
        let pinned = SecurityParameters::pinned_default();

        assert_eq!(
            actual.lwe_dimension, pinned.lwe_dimension,
            "LWE dimension drifted from the documented pin"
        );
        assert_eq!(
            actual.glwe_dimension, pinned.glwe_dimension,
            "GLWE dimension drifted from the documented pin"
        );
        assert_eq!(
            actual.polynomial_size, pinned.polynomial_size,
            "polynomial size drifted from the documented pin"
        );
        assert_eq!(
            actual.pbs_base_log, pinned.pbs_base_log,
            "PBS base log drifted from the documented pin"
        );
        assert_eq!(
            actual.pbs_level, pinned.pbs_level,
            "PBS level drifted from the documented pin"
        );
        assert_eq!(
            actual.ks_base_log, pinned.ks_base_log,
            "KS base log drifted from the documented pin"
        );
        assert_eq!(
            actual.ks_level, pinned.ks_level,
            "KS level drifted from the documented pin"
        );
        assert_eq!(
            actual.message_modulus, pinned.message_modulus,
            "message modulus drifted from the documented pin"
        );
        assert_eq!(
            actual.carry_modulus, pinned.carry_modulus,
            "carry modulus drifted from the documented pin"
        );
        assert_eq!(
            actual.max_noise_level, pinned.max_noise_level,
            "max noise level drifted from the documented pin"
        );
        // Float tolerance: the pinned value is the exact constant in
        // TFHE-rs 1.7, but allow tiny representational drift.
        assert!(
            (actual.log2_p_fail - pinned.log2_p_fail).abs() < 1e-3,
            "log2_p_fail drifted: actual {} vs pinned {}",
            actual.log2_p_fail,
            pinned.log2_p_fail
        );
    }

    #[test]
    fn server_key_bytes_roundtrip() {
        let (_client_key, server_key) = ClientKey::new();
        let bytes = server_key.to_bytes().unwrap();
        let back = ServerKey::from_bytes(&bytes).unwrap();
        // Redacted Debug output must not leak key material.
        let dbg = format!("{back:?}");
        assert!(dbg.contains("ServerKey") && !dbg.contains("0x"));
    }
}
