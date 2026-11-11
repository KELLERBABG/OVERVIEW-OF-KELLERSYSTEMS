use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub display_name: String,
    pub public_key_bytes: Vec<u8>,

    /// Signing key bytes. NEVER serialized to state files — secrets live only in
    /// the separate [`crate::keyring::Keyring`] store (see PROD_READINESS_FINDINGS #1).
    #[serde(skip)]
    pub signing_key_bytes: Option<Vec<u8>>,
}

impl Identity {
    pub fn generate(id: String, display_name: String) -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let secret = secret_bytes;
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();
        Identity {
            id,
            display_name,
            public_key_bytes: verifying_key.to_bytes().to_vec(),
            signing_key_bytes: Some(signing_key.to_bytes().to_vec()),
        }
    }

    pub fn public_only(id: String, display_name: String, public_key_bytes: Vec<u8>) -> Self {
        Identity {
            id,
            display_name,
            public_key_bytes,
            signing_key_bytes: None,
        }
    }

    pub fn signing_key(&self) -> Option<SigningKey> {
        self.signing_key_bytes.as_ref().map(|bytes| {
            let mut secret_bytes = [0u8; 32];
            secret_bytes.copy_from_slice(bytes);
            let secret = secret_bytes;
            SigningKey::from_bytes(&secret)
        })
    }

    pub fn verifying_key(&self) -> Result<VerifyingKey, ed25519_dalek::SignatureError> {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.public_key_bytes);
        VerifyingKey::from_bytes(&bytes)
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, String> {
        let sk = self
            .signing_key()
            .ok_or_else(|| "signing key not available (this is not your identity)".to_string())?;
        let signature = sk.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    pub fn verify(&self, message: &[u8], signature_bytes: &[u8]) -> Result<bool, String> {
        let vk = self
            .verifying_key()
            .map_err(|e| format!("invalid public key: {}", e))?;
        let mut sig_bytes = [0u8; 64];
        if signature_bytes.len() != 64 {
            return Err("invalid signature length".to_string());
        }
        sig_bytes.copy_from_slice(signature_bytes);
        let signature = Signature::from_bytes(&sig_bytes);
        match vk.verify(message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(&self.public_key_bytes);
        hasher.finalize().to_vec()
    }

    pub fn nullifier(&self, nonce: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.hash());
        hasher.update(nonce);
        hasher.finalize().to_vec()
    }
}
