use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Private signing-key store, kept separate from the serialized graph state.
///
/// State files (`mesh_state.json`, `mesh_server_state.json`) never contain secret
/// key material; the [`Keyring`] is the only place signing keys are persisted.
/// It is written with restrictive permissions (0600 on unix) so that reading the
/// state file does not leak any ed25519 seed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Keyring {
    keys: HashMap<String, Vec<u8>>,
}

impl Keyring {
    pub fn new() -> Self {
        Keyring::default()
    }

    pub fn insert(&mut self, identity_id: String, signing_key_bytes: Vec<u8>) {
        self.keys.insert(identity_id, signing_key_bytes);
    }

    pub fn get(&self, identity_id: &str) -> Option<&[u8]> {
        self.keys.get(identity_id).map(|v| v.as_slice())
    }

    pub fn contains(&self, identity_id: &str) -> bool {
        self.keys.contains_key(identity_id)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Load the keyring from `path`. A missing file yields an empty keyring;
    /// a corrupt file is an error (never silently reset — finding #7).
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Keyring::new());
        }
        let content = std::fs::read(path)
            .map_err(|e| format!("failed to read keyring '{}': {}", path.display(), e))?;
        serde_json::from_slice(&content)
            .map_err(|e| format!("keyring '{}' is corrupt: {}", path.display(), e))
    }

    /// Persist the keyring to `path` atomically (temp file + rename) with
    /// restrictive permissions where the platform supports it.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
            }
        }

        let content = serde_json::to_vec_pretty(self)
            .map_err(|e| format!("failed to serialize keyring: {}", e))?;

        let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&tmp, &content)
            .map_err(|e| format!("failed to write '{}': {}", tmp.display(), e))?;

        // Best-effort restrictive permissions (ignored on platforms without chmod).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }

        std::fs::rename(&tmp, path)
            .map_err(|e| format!("failed to finalize '{}': {}", path.display(), e))?;
        Ok(())
    }
}
