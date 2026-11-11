//! Cryptographic Tamper-Evident Shift Audit Log.

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub worker_pseudonym_id: Uuid,
    pub pm25_ug_m3: f64,
    pub cumulative_dose_ug_h_m3: f64,
    pub alert_tier: String,
    pub previous_hash: String,
    pub entry_hash: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    signing_key: SigningKey,
}

impl AuditLog {
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            entries: Vec::new(),
            signing_key,
        }
    }

    pub fn append_entry(
        &mut self,
        worker_id: Uuid,
        pm25: f64,
        dose: f64,
        tier: &str,
    ) -> &AuditEntry {
        let seq = self.entries.len() as u64;
        let ts = Utc::now();
        let prev_hash = self.entries.last().map(|e| e.entry_hash.clone()).unwrap_or_else(|| "0000000000000000000000000000000000000000000000000000000000000000".to_string());

        let mut hasher = Sha256::new();
        hasher.update(seq.to_be_bytes());
        hasher.update(ts.to_rfc3339().as_bytes());
        hasher.update(worker_id.as_bytes());
        hasher.update(pm25.to_be_bytes());
        hasher.update(dose.to_be_bytes());
        hasher.update(tier.as_bytes());
        hasher.update(prev_hash.as_bytes());
        let hash_bytes = hasher.finalize();
        let entry_hash = hex::encode(hash_bytes);

        let signature = self.signing_key.sign(entry_hash.as_bytes());
        let signature_hex = hex::encode(signature.to_bytes());

        let entry = AuditEntry {
            sequence: seq,
            timestamp: ts,
            worker_pseudonym_id: worker_id,
            pm25_ug_m3: pm25,
            cumulative_dose_ug_h_m3: dose,
            alert_tier: tier.to_string(),
            previous_hash: prev_hash,
            entry_hash,
            signature_hex,
        };

        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    pub fn verify_chain(&self, verifying_key: &VerifyingKey) -> Result<(), String> {
        use ed25519_dalek::Verifier;

        for (i, entry) in self.entries.iter().enumerate() {
            if let Some(prev_idx) = i.checked_sub(1) {
                if entry.previous_hash != self.entries[prev_idx].entry_hash {
                    return Err(format!("Broken hash chain at sequence {}", entry.sequence));
                }
            }

            let sig_bytes = hex::decode(&entry.signature_hex).map_err(|e| e.to_string())?;
            let sig = ed25519_dalek::Signature::from_slice(&sig_bytes).map_err(|e| e.to_string())?;
            verifying_key.verify(entry.entry_hash.as_bytes(), &sig).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }
}
