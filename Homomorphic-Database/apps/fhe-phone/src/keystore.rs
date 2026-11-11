//! Encrypted-at-rest key storage for the phone.
//!
//! The secret [`fhe_core::ClientKey`] (and the public
//! [`fhe_core::ServerKey`], which the embedded server needs on unlock) are
//! persisted to `%APPDATA%\FHE-DB` (or `~/.fhe-db-phone`), **encrypted with
//! AES-256-GCM** under a key derived from the user's PIN via **Argon2id**.
//!
//! # On-disk format (v2)
//!
//! A single `vault.json` holds the KDF parameters, the Argon2id PHC hash of
//! the PIN, the encrypted key pair, and the unlock-failure state. One file is
//! one atomic commit: it is written to `vault.json.tmp`, flushed with
//! `sync_all`, then renamed over `vault.json`. A crash or I/O error therefore
//! leaves either the complete old vault or the complete new one — never a new
//! PIN hash next to ciphertext encrypted under the old PIN.
//!
//! Version 1 used two files (`pin.json` + `keys.json`) and is migrated on
//! first open: the v1 records are combined into `vault.json` (written
//! atomically), then the legacy files are removed. The v1 ciphertext uses the
//! old JSON payload layout, which is recorded as [`PayloadFormat::JsonV1`] so
//! it stays decryptable; new writes always use [`PayloadFormat::BinaryV2`].
//!
//! # Security properties
//!
//! - The PIN is never stored in plaintext: only an Argon2id PHC hash is kept.
//! - The Argon2id parameters are **pinned** ([`KdfParams::PINNED`]) and
//!   recorded in the vault. They no longer depend on the `argon2` crate's
//!   defaults, so a dependency default change cannot strand existing keys.
//! - A wrong PIN fails GCM authentication: unlocking returns an
//!   authentication error instead of decrypting garbage.
//! - Failed unlocks are persisted and rate-limited (5 free attempts, then an
//!   exponential lockout capped at one hour). This bounds **online** guessing
//!   at the lock screen; it does not stop an attacker who copies the vault
//!   and guesses offline.
//! - Derived keys and plaintext key buffers are zeroized after use
//!   ([`zeroize::Zeroizing`]). TFHE-rs key types do not expose zeroization, so
//!   the decrypted `ClientKey` itself is only dropped, not explicitly wiped;
//!   see `SECURITY.md`.
//! - PIN change, backup import, and key destruction write/remove the single
//!   vault atomically. There is no intermediate state in which the PIN and
//!   the ciphertext disagree.
//!
//! All files are JSON (not bincode) so they stay inspectable and portable;
//! only the ciphertext payload is opaque.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Context as _};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use fhe_core::{ClientKey, ServerKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Current on-disk format version; bumped only with deliberate migration.
const STORE_VERSION: u32 = 2;

/// Legacy (v1) on-disk format version, still accepted for migration/import.
const LEGACY_STORE_VERSION: u32 = 1;

/// The active vault file.
const VAULT_FILE: &str = "vault.json";
/// Legacy PIN hash file (v1), migrated on first open.
const LEGACY_PIN_FILE: &str = "pin.json";
/// Legacy encrypted-key file (v1), migrated on first open.
const LEGACY_KEYS_FILE: &str = "keys.json";

/// Minimum / maximum PIN length (characters).
///
/// The lock-screen keypad only emits digits, so the policy is digits-only:
/// a PIN that could be set but not entered would be a lockout defect.
const PIN_MIN: usize = 6;
const PIN_MAX: usize = 12;

/// Free unlock attempts before the persisted lockout begins.
const MAX_FREE_ATTEMPTS: u32 = 5;
/// First lockout after [`MAX_FREE_ATTEMPTS`], doubling per further failure.
const BASE_LOCKOUT_SECS: u64 = 30;
/// Cap on the exponential lockout.
const MAX_LOCKOUT_SECS: u64 = 3600;

/// Argon2 version 0x13 (`v=19` in a PHC string).
const ARGON2_VERSION: u32 = 19;

/// AES-256-GCM salt / nonce / key sizes.
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Argon2id parameters, pinned and persisted with the vault.
///
/// Values are the OWASP baseline (19 MiB, 2 iterations, 1 lane) and match the
/// `argon2` 0.5 crate defaults, which is what v1 stores already used — so
/// migrating a v1 ciphertext does not require re-encryption. Unlike v1, the
/// numbers are now authoritative on disk: future crate default changes do not
/// affect existing vaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct KdfParams {
    /// Memory cost in KiB.
    m_cost: u32,
    /// Iterations.
    t_cost: u32,
    /// Parallelism lanes.
    p_cost: u32,
    /// Derived key length in bytes.
    output_len: u32,
}

impl KdfParams {
    /// Parameters used for every new vault and every PHC PIN hash.
    const PINNED: Self = Self {
        m_cost: 19_456,
        t_cost: 2,
        p_cost: 1,
        output_len: KEY_LEN as u32,
    };

    /// Parameters v1 used implicitly through `Argon2::default()`.
    ///
    /// Pinned here so migration stays independent of the dependency's
    /// defaults. A test asserts these produce the same derivation as
    /// `Argon2::default()` in this build.
    const LEGACY_V1: Self = Self {
        m_cost: 19_456,
        t_cost: 2,
        p_cost: 1,
        output_len: KEY_LEN as u32,
    };

    /// Reject absurd or unsupported parameters before allocating memory.
    ///
    /// This matters for imported backups and hand-edited vault files: an
    /// `m_cost` of `u32::MAX` would otherwise attempt a multi-terabyte
    /// allocation.
    fn validate(&self) -> anyhow::Result<()> {
        if !(8..=262_144).contains(&self.m_cost) {
            return Err(anyhow!(
                "unsupported Argon2 memory cost {} KiB (expected 8..=262144)",
                self.m_cost
            ));
        }
        if !(1..=10).contains(&self.t_cost) {
            return Err(anyhow!(
                "unsupported Argon2 iteration count {} (expected 1..=10)",
                self.t_cost
            ));
        }
        if !(1..=4).contains(&self.p_cost) {
            return Err(anyhow!(
                "unsupported Argon2 parallelism {} (expected 1..=4)",
                self.p_cost
            ));
        }
        if self.output_len != KEY_LEN as u32 {
            return Err(anyhow!(
                "unsupported Argon2 output length {} (expected {KEY_LEN})",
                self.output_len
            ));
        }
        Params::new(
            self.m_cost,
            self.t_cost,
            self.p_cost,
            Some(self.output_len as usize),
        )
        .map_err(|e| anyhow!("invalid Argon2 parameters: {e}"))?;
        Ok(())
    }

    /// Build the pinned Argon2id instance for these parameters.
    fn argon2(&self) -> anyhow::Result<Argon2<'static>> {
        self.validate()?;
        let params = Params::new(
            self.m_cost,
            self.t_cost,
            self.p_cost,
            Some(self.output_len as usize),
        )
        .map_err(|e| anyhow!("invalid Argon2 parameters: {e}"))?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
}

/// How the decrypted key payload is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum PayloadFormat {
    /// v1: `{"client_key": "<base64>", "server_key": "<base64>"}`.
    JsonV1,
    /// v2: length-prefixed raw client/server key bytes.
    #[default]
    BinaryV2,
}

/// AES-256-GCM-encrypted key pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedKeys {
    /// Argon2id salt for the AES key (base64).
    salt: String,
    /// GCM nonce (base64).
    nonce: String,
    /// Encrypted key payload (base64).
    ciphertext: String,
}

/// The single-file vault (format v2).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    /// [`STORE_VERSION`].
    version: u32,
    /// Argon2id parameters used to derive the AES key from the PIN.
    kdf: KdfParams,
    /// Argon2id PHC string for PIN verification.
    pin_hash: String,
    /// Payload encoding inside the GCM ciphertext.
    #[serde(default)]
    payload_format: PayloadFormat,
    /// Encrypted key pair; `None` until key generation.
    #[serde(default)]
    keys: Option<EncryptedKeys>,
    /// Consecutive failed unlock attempts (persisted across restarts).
    #[serde(default)]
    failed_attempts: u32,
    /// Unix timestamp (seconds) before which unlock is refused.
    #[serde(default)]
    locked_until_unix: u64,
}

impl VaultFile {
    fn validate(&self) -> anyhow::Result<()> {
        if self.version != STORE_VERSION {
            return Err(anyhow!(
                "unsupported key store version {} (expected {STORE_VERSION})",
                self.version
            ));
        }
        self.kdf.validate()?;
        validate_phc(&self.pin_hash)?;
        if let Some(keys) = &self.keys {
            let salt = b64_decode(&keys.salt, "key store salt")?;
            if salt.len() != SALT_LEN {
                return Err(anyhow!(
                    "key store salt is {} bytes (expected {SALT_LEN})",
                    salt.len()
                ));
            }
            let nonce = b64_decode(&keys.nonce, "key store nonce")?;
            if nonce.len() != NONCE_LEN {
                return Err(anyhow!(
                    "key store nonce is {} bytes (expected {NONCE_LEN})",
                    nonce.len()
                ));
            }
            if b64_decode(&keys.ciphertext, "key store ciphertext")?.is_empty() {
                return Err(anyhow!("key store ciphertext is empty"));
            }
        }
        Ok(())
    }
}

/// Legacy v1 `pin.json`.
#[derive(Debug, Deserialize)]
struct LegacyPinFile {
    version: u32,
    hash: String,
}

/// Legacy v1 `keys.json`.
#[derive(Debug, Deserialize)]
struct LegacyKeysFile {
    version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// Outcome of checking a PIN (without decrypting the key pair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinCheck {
    /// The PIN is correct.
    Ok,
    /// The PIN is wrong; `attempts_left` free attempts remain before a lockout.
    Wrong {
        /// Free attempts remaining after this failure.
        attempts_left: u32,
    },
    /// Unlock is temporarily refused after repeated failures.
    Locked {
        /// Whole seconds until another attempt will be evaluated.
        retry_after_secs: u64,
    },
    /// No PIN has been configured yet.
    NotConfigured,
}

/// Outcome of an unlock attempt.
#[derive(Debug)]
pub enum Unlock {
    /// PIN accepted. `Some` when a key pair was stored and decrypted.
    Unlocked(Option<Box<StoredKeys>>),
    /// PIN rejected; see [`PinCheck::Wrong`].
    Wrong {
        /// Free attempts remaining after this failure.
        attempts_left: u32,
    },
    /// Temporarily refused after repeated failures.
    Locked {
        /// Whole seconds until another attempt will be evaluated.
        retry_after_secs: u64,
    },
    /// No PIN has been configured yet.
    NotConfigured,
}

/// Zeroizing byte buffer for key material.
type KeyBytes = Zeroizing<Vec<u8>>;

/// The persisted key pair, decrypted.
#[derive(Debug)]
pub struct StoredKeys {
    /// Secret client key.
    pub client_key: ClientKey,
    /// Public server key.
    pub server_key: ServerKey,
}

/// The phone's encrypted key store on disk.
#[derive(Debug, Clone)]
pub struct KeyStore {
    /// Directory holding `vault.json` (and, transiently, legacy v1 files).
    dir: PathBuf,
}

fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64_decode(s: &str, what: &str) -> anyhow::Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .with_context(|| format!("corrupt base64 in {what}"))
}

/// Current Unix time in seconds (0 if the clock is before the epoch).
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl KeyStore {
    /// Open (or create) the store in the platform data directory and migrate
    /// a legacy v1 store if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if no writable data directory can be determined, the
    /// directory cannot be created, or a legacy store cannot be migrated.
    pub fn open() -> anyhow::Result<Self> {
        let dir = data_dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create key store directory {}", dir.display()))?;
        let store = Self { dir };
        store.migrate_legacy()?;
        Ok(store)
    }

    /// Open a store rooted at an explicit directory (used by tests) and
    /// migrate a legacy v1 store if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or a legacy store
    /// cannot be migrated.
    pub fn open_at(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let store = Self { dir };
        store.migrate_legacy()?;
        Ok(store)
    }

    /// The store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn vault_path(&self) -> PathBuf {
        self.dir.join(VAULT_FILE)
    }

    fn legacy_pin_path(&self) -> PathBuf {
        self.dir.join(LEGACY_PIN_FILE)
    }

    fn legacy_keys_path(&self) -> PathBuf {
        self.dir.join(LEGACY_KEYS_FILE)
    }

    fn has_legacy(&self) -> bool {
        self.legacy_pin_path().is_file() || self.legacy_keys_path().is_file()
    }

    /// Whether a PIN has been configured.
    pub fn has_pin(&self) -> bool {
        self.vault_path().is_file()
    }

    /// Whether a key pair has been stored.
    pub fn has_keys(&self) -> bool {
        self.read_vault().map(|v| v.keys.is_some()).unwrap_or(false)
    }

    /// Validate a PIN against the local policy: 6–12 ASCII digits.
    ///
    /// # Errors
    ///
    /// Returns an error when the PIN is too short, too long, or contains a
    /// non-digit. The lock-screen keypad only emits digits, so accepting
    /// other characters would create a PIN that cannot be entered.
    pub fn validate_pin(pin: &str) -> anyhow::Result<()> {
        let len = pin.chars().count();
        if len < PIN_MIN {
            return Err(anyhow!("PIN must be at least {PIN_MIN} digits (got {len})"));
        }
        if len > PIN_MAX {
            return Err(anyhow!("PIN must be at most {PIN_MAX} digits (got {len})"));
        }
        if !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow!("PIN must contain digits only (0–9)"));
        }
        Ok(())
    }

    fn read_vault(&self) -> anyhow::Result<VaultFile> {
        let bytes = std::fs::read(self.vault_path())
            .with_context(|| format!("failed to read {}", self.vault_path().display()))?;
        let vault: VaultFile = serde_json::from_slice(&bytes)
            .with_context(|| format!("corrupt JSON in {}", self.vault_path().display()))?;
        vault.validate()?;
        Ok(vault)
    }

    fn write_vault(&self, vault: &VaultFile) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec_pretty(vault)?;
        atomic_write(&self.vault_path(), &bytes)
    }

    /// Set the initial PIN. Fails if a PIN already exists; use
    /// [`KeyStore::change_pin`] to replace one (which re-encrypts the key pair
    /// in the same atomic write).
    ///
    /// # Errors
    ///
    /// Returns an error if the PIN is invalid, a PIN already exists, or the
    /// vault cannot be written.
    pub fn set_pin(&self, pin: &str) -> anyhow::Result<()> {
        Self::validate_pin(pin)?;
        if self.has_pin() {
            return Err(anyhow!(
                "A PIN is already set. Use Settings → Security → Change PIN."
            ));
        }
        if self.has_legacy() {
            return Err(anyhow!(
                "A legacy key store could not be migrated; refusing to overwrite it."
            ));
        }
        let vault = VaultFile {
            version: STORE_VERSION,
            kdf: KdfParams::PINNED,
            pin_hash: hash_pin(pin)?,
            payload_format: PayloadFormat::BinaryV2,
            keys: None,
            failed_attempts: 0,
            locked_until_unix: 0,
        };
        self.write_vault(&vault)
    }

    /// Check a PIN, enforcing the persisted unlock lockout.
    ///
    /// This does **not** decrypt the key pair (use [`KeyStore::unlock`] for
    /// that); it is the right call for a destructive action that only needs
    /// authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the vault is corrupt or the PIN hash cannot be
    /// parsed. A wrong PIN is **not** an error; it is
    /// [`PinCheck::Wrong`].
    pub fn check_pin(&self, pin: &str) -> anyhow::Result<PinCheck> {
        self.check_pin_at(pin, now_unix())
    }

    fn check_pin_at(&self, pin: &str, now: u64) -> anyhow::Result<PinCheck> {
        if !self.has_pin() {
            return Ok(PinCheck::NotConfigured);
        }
        let mut vault = self.read_vault()?;
        if vault.locked_until_unix > now {
            return Ok(PinCheck::Locked {
                retry_after_secs: vault.locked_until_unix - now,
            });
        }
        if verify_pin_hash(pin, &vault.pin_hash)? {
            if vault.failed_attempts != 0 || vault.locked_until_unix != 0 {
                vault.failed_attempts = 0;
                vault.locked_until_unix = 0;
                // A failure to persist the reset must not turn a correct PIN
                // into a rejection; the next success retries the write.
                if let Err(err) = self.write_vault(&vault) {
                    tracing::warn!("failed to persist unlock-success state: {err}");
                }
            }
            return Ok(PinCheck::Ok);
        }

        vault.failed_attempts = vault.failed_attempts.saturating_add(1);
        if vault.failed_attempts >= MAX_FREE_ATTEMPTS {
            let lockout = lockout_seconds(vault.failed_attempts);
            vault.locked_until_unix = now.saturating_add(lockout);
            if let Err(err) = self.write_vault(&vault) {
                tracing::warn!("failed to persist unlock lockout: {err}");
            }
            return Ok(PinCheck::Locked {
                retry_after_secs: lockout,
            });
        }
        if let Err(err) = self.write_vault(&vault) {
            tracing::warn!("failed to persist failed-attempt count: {err}");
        }
        Ok(PinCheck::Wrong {
            attempts_left: MAX_FREE_ATTEMPTS - vault.failed_attempts,
        })
    }

    /// Verify the PIN and decrypt the stored key pair, enforcing the lockout.
    ///
    /// # Errors
    ///
    /// Returns an error if the vault is corrupt, the recorded KDF parameters
    /// are invalid, or the ciphertext fails to decrypt **after** a correct
    /// PIN (a corrupt/tampered key record, not a wrong PIN).
    pub fn unlock(&self, pin: &str) -> anyhow::Result<Unlock> {
        self.unlock_at(pin, now_unix())
    }

    fn unlock_at(&self, pin: &str, now: u64) -> anyhow::Result<Unlock> {
        match self.check_pin_at(pin, now)? {
            PinCheck::Ok => {}
            PinCheck::Wrong { attempts_left } => return Ok(Unlock::Wrong { attempts_left }),
            PinCheck::Locked { retry_after_secs } => {
                return Ok(Unlock::Locked { retry_after_secs })
            }
            PinCheck::NotConfigured => return Ok(Unlock::NotConfigured),
        }
        let vault = self.read_vault()?;
        let keys = match &vault.keys {
            Some(encrypted) => Some(decrypt_keys(
                encrypted,
                pin,
                &vault.kdf,
                vault.payload_format,
            )?),
            None => None,
        };
        Ok(Unlock::Unlocked(keys.map(Box::new)))
    }

    /// Atomically change the PIN, re-encrypting the stored key pair under it.
    ///
    /// The old PIN is verified with the same lockout policy as an unlock. The
    /// new PIN hash and the re-encrypted key pair are written in **one**
    /// atomic vault write, so no crash can leave the new PIN next to
    /// ciphertext encrypted under the old one.
    ///
    /// # Errors
    ///
    /// Returns an error if the old PIN is wrong or locked out, the new PIN
    /// violates the policy, or the vault cannot be written.
    pub fn change_pin(&self, old: &str, new: &str) -> anyhow::Result<()> {
        Self::validate_pin(new)?;
        if !self.has_pin() {
            return Err(anyhow!("No PIN is configured; set one first."));
        }
        match self.check_pin_at(old, now_unix())? {
            PinCheck::Ok => {}
            PinCheck::Wrong { attempts_left } => {
                return Err(anyhow!(
                    "Wrong current PIN. {attempts_left} attempt(s) left before a temporary lockout."
                ))
            }
            PinCheck::Locked { retry_after_secs } => {
                return Err(anyhow!(
                    "Too many failed attempts. Try again in {retry_after_secs} s."
                ))
            }
            PinCheck::NotConfigured => return Err(anyhow!("No PIN is configured.")),
        }

        let mut vault = self.read_vault()?;
        let stored = match &vault.keys {
            Some(encrypted) => Some(decrypt_keys(
                encrypted,
                old,
                &vault.kdf,
                vault.payload_format,
            )?),
            None => None,
        };
        vault.pin_hash = hash_pin(new)?;
        vault.kdf = KdfParams::PINNED;
        vault.payload_format = PayloadFormat::BinaryV2;
        vault.keys = match &stored {
            Some(keys) => Some(encrypt_keys(
                &keys.client_key,
                &keys.server_key,
                new,
                &KdfParams::PINNED,
            )?),
            None => None,
        };
        vault.failed_attempts = 0;
        vault.locked_until_unix = 0;
        self.write_vault(&vault)
    }

    /// Encrypt and persist the key pair under the configured PIN.
    ///
    /// # Errors
    ///
    /// Returns an error if no PIN is configured, the PIN is wrong, the keys
    /// fail to serialize, or the vault cannot be written.
    pub fn save_keys(
        &self,
        client_key: &ClientKey,
        server_key: &ServerKey,
        pin: &str,
    ) -> anyhow::Result<()> {
        Self::validate_pin(pin)?;
        if !self.has_pin() {
            return Err(anyhow!(
                "Set a PIN first: the key pair is stored encrypted at rest under it."
            ));
        }
        let mut vault = self.read_vault()?;
        // Not an authentication boundary (the caller has already unlocked);
        // this only prevents saving under a PIN the user mistyped.
        if !verify_pin_hash(pin, &vault.pin_hash)? {
            return Err(anyhow!("Wrong PIN — keys not saved."));
        }
        vault.kdf = KdfParams::PINNED;
        vault.payload_format = PayloadFormat::BinaryV2;
        vault.keys = Some(encrypt_keys(
            client_key,
            server_key,
            pin,
            &KdfParams::PINNED,
        )?);
        self.write_vault(&vault)
    }

    /// Export a portable backup containing the entire vault (PIN hash +
    /// encrypted key pair + failure state reset on import).
    ///
    /// The backup is copyable, but its confidentiality is only as strong as
    /// the PIN against offline guessing, and its integrity is not
    /// authenticated. Treat an imported backup as a new root of trust.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or the backup written.
    pub fn export_backup(&self, dest: impl AsRef<Path>) -> anyhow::Result<()> {
        let vault = self.read_vault()?;
        let backup = serde_json::json!({
            "version": STORE_VERSION,
            "vault": vault,
        });
        let bytes = serde_json::to_vec_pretty(&backup)?;
        let dest = dest.as_ref();
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        atomic_write(dest, &bytes)
    }

    /// Import a v2 backup (or a legacy v1 backup), replacing the current
    /// vault in one atomic write.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not a valid backup, its KDF
    /// parameters are out of bounds, or the vault cannot be written.
    pub fn import_backup(&self, src: impl AsRef<Path>) -> anyhow::Result<()> {
        let bytes = std::fs::read(src.as_ref())?;
        let backup: serde_json::Value = serde_json::from_slice(&bytes)?;
        let version = backup.get("version").and_then(|v| v.as_u64()).unwrap_or(0);

        let vault = match version {
            v if v == STORE_VERSION as u64 => {
                let value = backup
                    .get("vault")
                    .cloned()
                    .ok_or_else(|| anyhow!("corrupt backup: missing `vault`"))?;
                let mut vault: VaultFile = serde_json::from_value(value)?;
                vault.validate()?;
                vault.failed_attempts = 0;
                vault.locked_until_unix = 0;
                Some(vault)
            }
            v if v == LEGACY_STORE_VERSION as u64 => legacy_backup_to_vault(&backup)?,
            other => {
                return Err(anyhow!(
                    "unsupported backup version {other} (expected {STORE_VERSION} or {LEGACY_STORE_VERSION})"
                ))
            }
        };

        match vault {
            Some(vault) => {
                self.write_vault(&vault)?;
                self.remove_legacy_files();
            }
            None => {
                // A valid empty backup (no PIN, no keys): clear the lock.
                self.destroy()?;
            }
        }
        Ok(())
    }

    /// Destroy all local key material: the vault and any legacy v1 files.
    ///
    /// The ciphertexts already stored by the server remain, but without the
    /// key pair they are undecryptable (crypto-shredding). Exported backups,
    /// OS file-system copies, and the server's ciphertext store are **not**
    /// touched; the caller must communicate that.
    ///
    /// # Errors
    ///
    /// Returns an error if a file exists but cannot be removed.
    pub fn destroy(&self) -> anyhow::Result<()> {
        for path in [
            self.vault_path(),
            self.legacy_pin_path(),
            self.legacy_keys_path(),
        ] {
            remove_if_exists(&path)?;
        }
        // Best-effort: a leftover temp file is harmless (never read), but
        // removing it shrinks the window in which ciphertext lingers.
        let _ = remove_if_exists(&temp_path(&self.vault_path()));
        Ok(())
    }

    /// Migrate the legacy two-file v1 store, if present.
    ///
    /// Ordering: build the complete v2 vault, write it atomically, then
    /// delete `pin.json` / `keys.json`. A crash before the rename leaves the
    /// v1 files untouched; a crash after it leaves `vault.json` authoritative
    /// and the legacy files are cleaned up on the next open.
    fn migrate_legacy(&self) -> anyhow::Result<()> {
        if self.vault_path().is_file() {
            self.remove_legacy_files();
            return Ok(());
        }
        if !self.has_legacy() {
            return Ok(());
        }
        if !self.legacy_pin_path().is_file() {
            return Err(anyhow!(
                "legacy `{}` exists without `{}`; cannot migrate a key store with no PIN",
                LEGACY_KEYS_FILE,
                LEGACY_PIN_FILE
            ));
        }

        let pin_bytes = std::fs::read(self.legacy_pin_path())?;
        let pin: LegacyPinFile =
            serde_json::from_slice(&pin_bytes).context("corrupt legacy pin.json")?;
        if pin.version != LEGACY_STORE_VERSION {
            return Err(anyhow!(
                "unsupported legacy pin.json version {} (expected {LEGACY_STORE_VERSION})",
                pin.version
            ));
        }
        validate_phc(&pin.hash)?;

        let keys = if self.legacy_keys_path().is_file() {
            let key_bytes = std::fs::read(self.legacy_keys_path())?;
            let legacy: LegacyKeysFile =
                serde_json::from_slice(&key_bytes).context("corrupt legacy keys.json")?;
            if legacy.version != LEGACY_STORE_VERSION {
                return Err(anyhow!(
                    "unsupported legacy keys.json version {} (expected {LEGACY_STORE_VERSION})",
                    legacy.version
                ));
            }
            Some(EncryptedKeys {
                salt: legacy.salt,
                nonce: legacy.nonce,
                ciphertext: legacy.ciphertext,
            })
        } else {
            None
        };

        let vault = VaultFile {
            version: STORE_VERSION,
            kdf: KdfParams::LEGACY_V1,
            pin_hash: pin.hash,
            payload_format: PayloadFormat::JsonV1,
            keys,
            failed_attempts: 0,
            locked_until_unix: 0,
        };
        vault.validate()?;
        self.write_vault(&vault)?;
        self.remove_legacy_files();
        tracing::info!(
            event = "keystore.migrate",
            from_version = LEGACY_STORE_VERSION,
            to_version = STORE_VERSION,
            had_keys = vault.keys.is_some(),
            "migrated the phone key store to the single-file vault"
        );
        Ok(())
    }

    fn remove_legacy_files(&self) {
        for path in [self.legacy_pin_path(), self.legacy_keys_path()] {
            if let Err(err) = remove_if_exists(&path) {
                tracing::warn!("failed to remove legacy {}", path.display());
                tracing::debug!("legacy removal error: {err}");
            }
        }
    }
}

/// Exponential backoff after the free attempts are exhausted.
fn lockout_seconds(failed_attempts: u32) -> u64 {
    let steps = failed_attempts.saturating_sub(MAX_FREE_ATTEMPTS).min(7);
    BASE_LOCKOUT_SECS
        .saturating_mul(1u64 << steps)
        .min(MAX_LOCKOUT_SECS)
}

/// Hash a PIN with the pinned Argon2id parameters.
fn hash_pin(pin: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    KdfParams::PINNED
        .argon2()?
        .hash_password(pin.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| anyhow!("Argon2id hashing failed: {e}"))
}

/// Parse and bounds-check a PHC string without verifying a PIN.
fn validate_phc(phc: &str) -> anyhow::Result<()> {
    let parsed =
        PasswordHash::new(phc).map_err(|e| anyhow!("corrupt PIN hash (not a PHC string): {e}"))?;
    if parsed.algorithm.as_str() != "argon2id" {
        return Err(anyhow!(
            "unsupported PIN hash algorithm `{}` (expected argon2id)",
            parsed.algorithm
        ));
    }
    if parsed.version != Some(ARGON2_VERSION) {
        return Err(anyhow!("unsupported Argon2 version in the PIN hash"));
    }
    phc_kdf_params(&parsed).validate()?;
    Ok(())
}

/// Extract the Argon2 cost parameters from a parsed PHC string.
fn phc_kdf_params(parsed: &PasswordHash<'_>) -> KdfParams {
    let text = parsed.params.to_string();
    let get = |name: &str| -> u32 {
        text.split(',')
            .find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                (key == name).then(|| value.parse::<u32>().ok()).flatten()
            })
            .unwrap_or(0)
    };
    KdfParams {
        m_cost: get("m"),
        t_cost: get("t"),
        p_cost: get("p"),
        // The output length is implicit in the stored hash. Verification uses
        // the hash's own bytes; bounding it to the AES key size keeps the
        // accepted parameter set narrow.
        output_len: KEY_LEN as u32,
    }
}

/// Verify a PIN against a PHC hash after bounds-checking its parameters.
fn verify_pin_hash(pin: &str, phc: &str) -> anyhow::Result<bool> {
    validate_phc(phc)?;
    let parsed =
        PasswordHash::new(phc).map_err(|e| anyhow!("corrupt PIN hash (not a PHC string): {e}"))?;
    Ok(phc_kdf_params(&parsed)
        .argon2()?
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok())
}

/// Derive the AES-256 key from the PIN + per-file salt.
fn derive_key(pin: &str, salt: &[u8], params: &KdfParams) -> anyhow::Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    params
        .argon2()?
        .hash_password_into(pin.as_bytes(), salt, &mut key[..])
        .map_err(|e| anyhow!("Argon2id key derivation failed: {e}"))?;
    Ok(key)
}

/// Encode the key pair for encryption.
fn encode_payload(client: &ClientKey, server: &ServerKey) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let client_bytes = Zeroizing::new(client.to_bytes()?);
    let server_bytes = Zeroizing::new(server.to_bytes()?);
    let mut payload = Zeroizing::new(Vec::with_capacity(
        8 + client_bytes.len() + server_bytes.len(),
    ));
    payload.extend_from_slice(&(client_bytes.len() as u32).to_le_bytes());
    payload.extend_from_slice(&client_bytes);
    payload.extend_from_slice(&(server_bytes.len() as u32).to_le_bytes());
    payload.extend_from_slice(&server_bytes);
    Ok(payload)
}

/// Decode a v2 binary payload, zeroizing both key buffers.
fn decode_payload(plaintext: &[u8]) -> anyhow::Result<(KeyBytes, KeyBytes)> {
    let mut offset = 0;
    let client = read_chunk(plaintext, &mut offset, "client key")?;
    let server = read_chunk(plaintext, &mut offset, "server key")?;
    if offset != plaintext.len() {
        return Err(anyhow!("corrupt key payload: trailing bytes"));
    }
    Ok((client, server))
}

fn read_chunk(buf: &[u8], offset: &mut usize, what: &str) -> anyhow::Result<KeyBytes> {
    if buf.len().saturating_sub(*offset) < 4 {
        return Err(anyhow!("corrupt key payload: missing {what} length"));
    }
    let len =
        u32::from_le_bytes(buf[*offset..*offset + 4].try_into().expect("4-byte slice")) as usize;
    *offset += 4;
    if buf.len().saturating_sub(*offset) < len {
        return Err(anyhow!("corrupt key payload: truncated {what}"));
    }
    let chunk = Zeroizing::new(buf[*offset..*offset + len].to_vec());
    *offset += len;
    Ok(chunk)
}

/// Encrypt the key pair under `pin`.
fn encrypt_keys(
    client: &ClientKey,
    server: &ServerKey,
    pin: &str,
    params: &KdfParams,
) -> anyhow::Result<EncryptedKeys> {
    let plaintext = encode_payload(client, server)?;
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(pin, &salt, params)?;
    let cipher = Aes256Gcm::new_from_slice(&key[..])
        .map_err(|e| anyhow!("AES-256-GCM key setup failed: {e}"))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|_| anyhow!("AES-GCM encryption failed"))?;
    Ok(EncryptedKeys {
        salt: b64_encode(&salt),
        nonce: b64_encode(&nonce_bytes),
        ciphertext: b64_encode(&ciphertext),
    })
}

/// Decrypt the stored key pair under `pin`.
fn decrypt_keys(
    encrypted: &EncryptedKeys,
    pin: &str,
    params: &KdfParams,
    format: PayloadFormat,
) -> anyhow::Result<StoredKeys> {
    let salt = b64_decode(&encrypted.salt, "key store salt")?;
    let nonce = b64_decode(&encrypted.nonce, "key store nonce")?;
    let ciphertext = b64_decode(&encrypted.ciphertext, "key store ciphertext")?;
    if salt.len() != SALT_LEN || nonce.len() != NONCE_LEN {
        return Err(anyhow!("corrupt key store: bad salt or nonce length"));
    }
    let key = derive_key(pin, &salt, params)?;
    let cipher = Aes256Gcm::new_from_slice(&key[..])
        .map_err(|e| anyhow!("AES-256-GCM key setup failed: {e}"))?;
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| {
                anyhow!("Key store authentication failed (wrong PIN or tampered ciphertext).")
            })?,
    );

    let (client_bytes, server_bytes) = match format {
        PayloadFormat::BinaryV2 => decode_payload(&plaintext)?,
        PayloadFormat::JsonV1 => decode_legacy_payload(&plaintext)?,
    };
    let client_key = ClientKey::from_bytes(client_bytes.as_slice())?;
    let server_key = ServerKey::from_bytes(server_bytes.as_slice())?;
    Ok(StoredKeys {
        client_key,
        server_key,
    })
}

/// Decode the v1 JSON payload (`{"client_key": "<base64>", ...}`).
fn decode_legacy_payload(plaintext: &[u8]) -> anyhow::Result<(KeyBytes, KeyBytes)> {
    let payload: serde_json::Value =
        serde_json::from_slice(plaintext).context("corrupt legacy key payload")?;
    let client = payload
        .get("client_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("corrupt key payload: missing client_key"))?;
    let server = payload
        .get("server_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("corrupt key payload: missing server_key"))?;
    Ok((
        Zeroizing::new(b64_decode(client, "client key")?),
        Zeroizing::new(b64_decode(server, "server key")?),
    ))
}

/// Convert a legacy v1 backup object into a v2 vault.
fn legacy_backup_to_vault(backup: &serde_json::Value) -> anyhow::Result<Option<VaultFile>> {
    let pin = backup.get("pin").and_then(|v| v.as_str());
    let keys = backup.get("keys").and_then(|v| v.as_str());
    match (pin, keys) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(anyhow!(
            "corrupt v1 backup: keys are present but the PIN hash is missing"
        )),
        (Some(pin_text), keys_text) => {
            let pin: LegacyPinFile =
                serde_json::from_str(pin_text).context("corrupt v1 backup pin")?;
            if pin.version != LEGACY_STORE_VERSION {
                return Err(anyhow!("unsupported v1 backup PIN version {}", pin.version));
            }
            validate_phc(&pin.hash)?;
            let keys = match keys_text {
                Some(text) => {
                    let legacy: LegacyKeysFile =
                        serde_json::from_str(text).context("corrupt v1 backup keys")?;
                    if legacy.version != LEGACY_STORE_VERSION {
                        return Err(anyhow!(
                            "unsupported v1 backup key version {}",
                            legacy.version
                        ));
                    }
                    Some(EncryptedKeys {
                        salt: legacy.salt,
                        nonce: legacy.nonce,
                        ciphertext: legacy.ciphertext,
                    })
                }
                None => None,
            };
            let vault = VaultFile {
                version: STORE_VERSION,
                kdf: KdfParams::LEGACY_V1,
                pin_hash: pin.hash,
                payload_format: PayloadFormat::JsonV1,
                keys,
                failed_attempts: 0,
                locked_until_unix: 0,
            };
            vault.validate()?;
            Ok(Some(vault))
        }
    }
}

fn temp_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ))
}

/// Write a file atomically: temp in the same directory, `sync_all`, rename.
///
/// A failure at any step leaves the previous file intact.
fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = temp_path(path);
    // A stale temp file from an interrupted write is never authoritative.
    let _ = std::fs::remove_file(&tmp);
    let result = (|| -> anyhow::Result<()> {
        let mut file = std::fs::File::create(&tmp)
            .with_context(|| format!("failed to create {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to flush {}", tmp.display()))?;
        drop(file);
        std::fs::rename(&tmp, path)
            .with_context(|| format!("failed to commit {} -> {}", tmp.display(), path.display()))?;
        // Persist the directory entry where the platform supports it. Windows
        // does not allow opening a directory as a File; the rename itself is
        // the durable commit there.
        #[cfg(unix)]
        if let Some(dir) = path.parent() {
            if let Ok(handle) = std::fs::File::open(dir) {
                let _ = handle.sync_all();
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn remove_if_exists(path: &Path) -> anyhow::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {}", path.display())),
    }
}

/// The platform data directory for the key store.
fn data_dir() -> anyhow::Result<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Ok(PathBuf::from(appdata).join("FHE-DB"));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(home).join(".fhe-db-phone"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".fhe-db-phone"));
    }
    Err(anyhow!(
        "cannot determine a data directory (APPDATA/USERPROFILE/HOME unset)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp dir per test so parallel tests never collide.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fhe_phone_keystore_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Build a real legacy v1 store on disk using the old JSON payload.
    fn write_legacy_store(dir: &Path, pin: &str, client: &ClientKey, server: &ServerKey) {
        use serde_json::json;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(pin.as_bytes(), &salt)
            .unwrap()
            .to_string();
        std::fs::write(
            dir.join(LEGACY_PIN_FILE),
            serde_json::to_vec(&json!({ "version": 1, "hash": hash })).unwrap(),
        )
        .unwrap();

        let salt_bytes = [7u8; SALT_LEN];
        let nonce_bytes = [9u8; NONCE_LEN];
        let payload = json!({
            "client_key": b64_encode(&client.to_bytes().unwrap()),
            "server_key": b64_encode(&server.to_bytes().unwrap()),
        });
        let plaintext = serde_json::to_vec(&payload).unwrap();
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(pin.as_bytes(), &salt_bytes, &mut key)
            .unwrap();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
            .unwrap();
        std::fs::write(
            dir.join(LEGACY_KEYS_FILE),
            serde_json::to_vec(&json!({
                "version": 1,
                "salt": b64_encode(&salt_bytes),
                "nonce": b64_encode(&nonce_bytes),
                "ciphertext": b64_encode(&ciphertext),
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn pin_policy_requires_six_to_twelve_digits() {
        let dir = temp_dir("pinpolicy");
        let store = KeyStore::open_at(&dir).unwrap();
        for bad in ["12345", "1234567890123", "12345a", "", "12 3456"] {
            assert!(store.set_pin(bad).is_err(), "PIN {bad:?} must be rejected");
            assert!(!store.has_pin());
        }
        store.set_pin("123456").unwrap();
        assert!(store.check_pin("123456").unwrap() == PinCheck::Ok);
        assert!(KeyStore::validate_pin("123456789012").is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pin_roundtrip_and_wrong_pin_rejected() {
        let dir = temp_dir("pin");
        let store = KeyStore::open_at(&dir).unwrap();
        assert!(!store.has_pin());
        assert_eq!(store.check_pin("123456").unwrap(), PinCheck::NotConfigured);
        store.set_pin("123456").unwrap();
        assert_eq!(store.check_pin("123456").unwrap(), PinCheck::Ok);
        assert_eq!(
            store.check_pin("999999").unwrap(),
            PinCheck::Wrong { attempts_left: 4 }
        );
        assert!(matches!(
            store.unlock("123456").unwrap(),
            Unlock::Unlocked(None)
        ));
        // A second set is refused without destroying the existing PIN.
        assert!(store.set_pin("654321").is_err());
        assert_eq!(store.check_pin("123456").unwrap(), PinCheck::Ok);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kdf_parameters_are_pinned_and_recorded() {
        let dir = temp_dir("kdf");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("246810").unwrap();
        let raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(VAULT_FILE)).unwrap()).unwrap();
        assert_eq!(raw["version"], STORE_VERSION);
        assert_eq!(raw["kdf"]["m_cost"], 19_456u32);
        assert_eq!(raw["kdf"]["t_cost"], 2u32);
        assert_eq!(raw["kdf"]["p_cost"], 1u32);
        assert_eq!(raw["kdf"]["output_len"], 32u32);
        assert!(!dir.join(LEGACY_PIN_FILE).exists());
        assert!(!dir.join(LEGACY_KEYS_FILE).exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_kdf_defaults_match_the_pinned_migration_record() {
        let salt = [0u8; SALT_LEN];
        let mut default_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(b"123456", &salt, &mut default_key)
            .unwrap();
        let mut pinned_key = [0u8; 32];
        KdfParams::LEGACY_V1
            .argon2()
            .unwrap()
            .hash_password_into(b"123456", &salt, &mut pinned_key)
            .unwrap();
        assert_eq!(
            default_key, pinned_key,
            "the recorded v1 parameters must reproduce Argon2::default()"
        );
    }

    #[test]
    fn keys_save_unlock_roundtrip_and_wrong_pin() {
        let dir = temp_dir("keys");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("246810").unwrap();

        let (client_key, server_key) = ClientKey::new();
        store.save_keys(&client_key, &server_key, "246810").unwrap();
        assert!(store.has_keys());

        let stored = match store.unlock("246810").unwrap() {
            Unlock::Unlocked(Some(keys)) => keys,
            other => panic!("expected stored keys, got {other:?}"),
        };
        // The restored client key must decrypt a value encrypted by the
        // original key — proving the exact key material survived.
        let encryptor = fhe_core::Encryptor::new(&stored.client_key);
        let ct = encryptor
            .encrypt_u64(42, fhe_core::ScalarWidth::U32)
            .unwrap();
        assert_eq!(ct.decrypt(&client_key).as_u64(), 42);

        assert!(matches!(
            store.unlock("999999").unwrap(),
            Unlock::Wrong { .. }
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pin_change_is_atomic_and_preserves_keys() {
        let dir = temp_dir("changepin");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("246810").unwrap();
        let (client_key, server_key) = ClientKey::new();
        store.save_keys(&client_key, &server_key, "246810").unwrap();

        // Make the atomic commit fail: the temp path is a directory, so the
        // write cannot replace the existing vault.
        std::fs::create_dir_all(dir.join("vault.json.tmp")).unwrap();
        let err = store.change_pin("246810", "135790").unwrap_err();
        assert!(!err.to_string().is_empty());
        std::fs::remove_dir_all(dir.join("vault.json.tmp")).unwrap();

        // The old vault is still complete: old PIN verifies and decrypts.
        assert_eq!(store.check_pin("246810").unwrap(), PinCheck::Ok);
        assert!(matches!(
            store.unlock("246810").unwrap(),
            Unlock::Unlocked(Some(_))
        ));
        assert!(store.check_pin("135790").unwrap() != PinCheck::Ok);

        // The successful change commits both the hash and the ciphertext.
        store.change_pin("246810", "135790").unwrap();
        assert!(matches!(
            store.unlock("135790").unwrap(),
            Unlock::Unlocked(Some(keys)) if keys.server_key.to_bytes().is_ok()
        ));
        assert!(matches!(
            store.unlock("246810").unwrap(),
            Unlock::Wrong { .. }
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failures_are_persisted_and_lock_out_then_recover() {
        let dir = temp_dir("lockout");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("246810").unwrap();

        for expected_left in [4, 3, 2, 1] {
            assert_eq!(
                store.check_pin_at("000000", 1_000).unwrap(),
                PinCheck::Wrong {
                    attempts_left: expected_left
                }
            );
        }
        assert_eq!(
            store.check_pin_at("000000", 1_000).unwrap(),
            PinCheck::Locked {
                retry_after_secs: 30
            }
        );
        // Persisted across instances and restarts.
        let reopened = KeyStore::open_at(&dir).unwrap();
        assert_eq!(
            reopened.check_pin_at("246810", 1_010).unwrap(),
            PinCheck::Locked {
                retry_after_secs: 20
            }
        );
        // Correct PIN is accepted after the lockout and resets the counter.
        assert_eq!(
            reopened.check_pin_at("246810", 1_030).unwrap(),
            PinCheck::Ok
        );
        let vault = reopened.read_vault().unwrap();
        assert_eq!(vault.failed_attempts, 0);
        assert_eq!(vault.locked_until_unix, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn destroy_removes_local_key_material() {
        let dir = temp_dir("destroy");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("246810").unwrap();
        let (client_key, server_key) = ClientKey::new();
        store.save_keys(&client_key, &server_key, "246810").unwrap();

        assert_eq!(
            store.check_pin("000000").unwrap(),
            PinCheck::Wrong { attempts_left: 4 }
        );
        store.destroy().unwrap();
        assert!(!store.has_pin());
        assert!(!store.has_keys());
        assert!(!dir.join(VAULT_FILE).exists());
        assert_eq!(store.check_pin("246810").unwrap(), PinCheck::NotConfigured);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_import_roundtrip_and_v1_backup_import() {
        let dir = temp_dir("backup");
        let store = KeyStore::open_at(&dir).unwrap();
        store.set_pin("135790").unwrap();
        let (client_key, server_key) = ClientKey::new();
        store.save_keys(&client_key, &server_key, "135790").unwrap();

        let backup = dir.join("backup.json");
        store.export_backup(&backup).unwrap();

        let dir2 = temp_dir("backup2");
        let store2 = KeyStore::open_at(&dir2).unwrap();
        store2.import_backup(&backup).unwrap();
        assert!(store2.has_pin());
        assert!(matches!(
            store2.unlock("135790").unwrap(),
            Unlock::Unlocked(Some(_))
        ));

        // A v1 backup (two embedded file strings) still imports.
        let legacy_dir = temp_dir("backupv1");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        write_legacy_store(&legacy_dir, "864209", &client_key, &server_key);
        let pin_text = std::fs::read_to_string(legacy_dir.join(LEGACY_PIN_FILE)).unwrap();
        let keys_text = std::fs::read_to_string(legacy_dir.join(LEGACY_KEYS_FILE)).unwrap();
        let v1_backup = dir2.join("backup-v1.json");
        std::fs::write(
            &v1_backup,
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "pin": pin_text,
                "keys": keys_text,
            }))
            .unwrap(),
        )
        .unwrap();
        store2.import_backup(&v1_backup).unwrap();
        assert!(matches!(
            store2.unlock("864209").unwrap(),
            Unlock::Unlocked(Some(_))
        ));

        // A tampered backup is rejected without touching the live vault.
        let mut bad: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
        bad["vault"]["kdf"]["m_cost"] = serde_json::json!(4_000_000_000u64);
        let bad_path = dir2.join("bad.json");
        std::fs::write(&bad_path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(store2.import_backup(&bad_path).is_err());
        assert!(matches!(
            store2.unlock("864209").unwrap(),
            Unlock::Unlocked(Some(_))
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&dir2).ok();
        std::fs::remove_dir_all(&legacy_dir).ok();
    }

    #[test]
    fn legacy_two_file_store_is_migrated() {
        let dir = temp_dir("migrate");
        std::fs::create_dir_all(&dir).unwrap();
        let (client_key, server_key) = ClientKey::new();
        write_legacy_store(&dir, "975310", &client_key, &server_key);

        let store = KeyStore::open_at(&dir).unwrap();
        assert!(dir.join(VAULT_FILE).is_file(), "v2 vault written");
        assert!(!dir.join(LEGACY_PIN_FILE).exists(), "legacy PIN removed");
        assert!(!dir.join(LEGACY_KEYS_FILE).exists(), "legacy keys removed");

        let vault = store.read_vault().unwrap();
        assert_eq!(vault.payload_format, PayloadFormat::JsonV1);

        let stored = match store.unlock("975310").unwrap() {
            Unlock::Unlocked(Some(keys)) => keys,
            other => panic!("expected migrated keys, got {other:?}"),
        };
        let encryptor = fhe_core::Encryptor::new(&stored.client_key);
        let ct = encryptor
            .encrypt_u64(7, fhe_core::ScalarWidth::U32)
            .unwrap();
        assert_eq!(ct.decrypt(&client_key).as_u64(), 7);
        std::fs::remove_dir_all(&dir).ok();
    }
}
