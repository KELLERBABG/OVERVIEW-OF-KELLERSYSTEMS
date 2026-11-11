//! OTA (Over-The-Air) firmware update module.
//!
//! Checks a remote manifest (JSON) for newer firmware versions, downloads
//! the firmware bundle, verifies it with an Ed25519 signature and SHA-256
//! checksum, then stages it into the configured staging directory for the
//! bootloader to apply on next restart.
//!
//! Manifest format:
//! ```json
//! {
//!   "version": "0.3.0",
//!   "url": "https://.../firmware-0.3.0.bin",
//!   "sha256": "<hex>",
//!   "signature_hex": "<ed25519 signature over the bundle bytes, hex>"
//! }
//! ```

use crate::config::OtaConfig;
use serde::Deserialize;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Current firmware version of this build.
pub const CURRENT_VERSION: &str = "0.2.0";

/// Ed25519 signature length in bytes.
const ED25519_SIGNATURE_LEN: usize = 64;
/// Ed25519 public key length in bytes.
const ED25519_PUBLIC_KEY_LEN: usize = 32;

/// Errors returned by OTA operations.
#[derive(Debug, thiserror::Error)]
pub enum OtaError {
    #[error("OTA not configured (no manifest URL)")]
    NotConfigured,
    #[error("manifest request failed: {0}")]
    ManifestFetch(String),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("firmware download failed: {0}")]
    Download(String),
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("SHA-256 checksum mismatch")]
    ChecksumMismatch,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// OTA manifest describing the latest available firmware.
#[derive(Debug, Clone, Deserialize)]
pub struct OtaManifest {
    /// Target firmware version (e.g. "0.3.0").
    pub version: String,
    /// Direct URL to the firmware bundle.
    #[serde(default)]
    pub url: Option<String>,
    /// SHA-256 checksum of the bundle (hex).
    #[serde(default)]
    pub sha256: Option<String>,
    /// Ed25519 signature over the bundle bytes (hex).
    #[serde(default)]
    pub signature_hex: Option<String>,
}

/// Parse "major.minor.patch" (optional leading 'v') into comparable numbers.
fn parse_version(version: &str) -> (u64, u64, u64) {
    let digits: Vec<u64> = version
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    (
        digits.first().copied().unwrap_or(0),
        digits.get(1).copied().unwrap_or(0),
        digits.get(2).copied().unwrap_or(0),
    )
}

/// Returns true when `remote` is a newer version than `current`.
fn is_newer(remote: &str, current: &str) -> bool {
    parse_version(remote) > parse_version(current)
}

/// Keep only characters that are safe in a filename / URL path segment.
/// Defense against path traversal via a malicious manifest `version` field.
/// Dots are stripped too (the version string is preserved in logs and the
/// manifest; only the staged *filename* is sanitized, e.g. `firmware-030.bin`).
fn sanitize_version(version: &str) -> String {
    version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

/// Check the remote manifest and return `Ok(Some(version))` when an update
/// is available, or `Ok(None)` when the device is up-to-date.
pub async fn check_for_update(cfg: &OtaConfig) -> Result<Option<String>, OtaError> {
    let Some(manifest_url) = &cfg.manifest_url else {
        return Ok(None);
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(format!("AmbientThermalArbitrage/{}", CURRENT_VERSION))
        .build()
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?;

    let url = manifest_url.replace("{version}", CURRENT_VERSION);
    let text = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?
        .error_for_status()
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?
        .text()
        .await
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?;

    let manifest: OtaManifest = serde_json::from_str(&text)
        .map_err(|e| OtaError::InvalidManifest(e.to_string()))?;

    crate::metrics::inc_ota("check");

    if is_newer(&manifest.version, CURRENT_VERSION) {
        log::info!("📦 OTA: update available {} (current {})", manifest.version, CURRENT_VERSION);
        Ok(Some(manifest.version))
    } else {
        log::info!("📦 OTA: up-to-date (current {})", CURRENT_VERSION);
        Ok(None)
    }
}

/// Download, verify, and stage a firmware bundle for the given version.
pub async fn fetch_and_apply_update(cfg: &OtaConfig, version: &str) -> Result<(), OtaError> {
    let Some(manifest_url) = &cfg.manifest_url else {
        return Err(OtaError::NotConfigured);
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent(format!("AmbientThermalArbitrage/{}", CURRENT_VERSION))
        .build()
        .map_err(|e| OtaError::Download(e.to_string()))?;

    // Fetch the manifest for the target version to locate the bundle.
    let safe_version = sanitize_version(version);
    let manifest_text = client
        .get(manifest_url.replace("{version}", &safe_version))
        .send()
        .await
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?
        .error_for_status()
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?
        .text()
        .await
        .map_err(|e| OtaError::ManifestFetch(e.to_string()))?;

    let manifest: OtaManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| OtaError::InvalidManifest(e.to_string()))?;

    let download_url = manifest
        .url
        .as_deref()
        .ok_or_else(|| OtaError::InvalidManifest("manifest does not include a bundle url".into()))?;

    let bundle = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| OtaError::Download(e.to_string()))?
        .error_for_status()
        .map_err(|e| OtaError::Download(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| OtaError::Download(e.to_string()))?;

    // Verify Ed25519 signature (fail-closed when a key is configured).
    if let Some(sig_hex) = &manifest.signature_hex {
        if cfg.verify_key_hex.is_empty() {
            log::warn!("📦 OTA: manifest signed but no verify key configured — skipping verification");
        } else if !verify_signature(&bundle, sig_hex, &cfg.verify_key_hex) {
            return Err(OtaError::SignatureVerificationFailed);
        }
    } else if !cfg.verify_key_hex.is_empty() {
        // Key configured but bundle unsigned — fail closed.
        return Err(OtaError::SignatureVerificationFailed);
    }

    // Verify SHA-256 checksum when provided.
    if let Some(expected_sha) = &manifest.sha256 {
        let actual = sha256_hex(&bundle);
        if !actual.eq_ignore_ascii_case(expected_sha.trim()) {
            return Err(OtaError::ChecksumMismatch);
        }
    }

    // Stage the bundle for the bootloader (sanitized version: no path traversal).
    std::fs::create_dir_all(&cfg.staging_dir)?;
    // Prune previously staged bundles so the staging dir can't grow forever on
    // a 24/7 device (SD-card wear) — only the newest firmware is needed.
    if let Ok(entries) = std::fs::read_dir(&cfg.staging_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(n) = name.to_str() {
                if n.starts_with("firmware-") && n.ends_with(".bin") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
    let staged_path = std::path::Path::new(&cfg.staging_dir)
        .join(format!("firmware-{}.bin", sanitize_version(version)));
    std::fs::write(&staged_path, &bundle)?;

    log::info!(
        "📦 OTA: firmware {} staged to {} ({} bytes)",
        version,
        staged_path.display(),
        bundle.len()
    );
    crate::metrics::inc_ota("stage");
    Ok(())
}

/// Verify an Ed25519 signature over a payload using the configured public key.
pub fn verify_signature(payload: &[u8], signature_hex: &str, public_key_hex: &str) -> bool {
    let signature = match hex_decode(signature_hex) {
        Some(sig) if sig.len() == ED25519_SIGNATURE_LEN => sig,
        _ => return false,
    };
    let public_key = match hex_decode(public_key_hex) {
        Some(key) if key.len() == ED25519_PUBLIC_KEY_LEN => key,
        _ => return false,
    };

    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key)
        .verify(payload, &signature)
        .is_ok()
}

/// Run the periodic OTA check loop in the background.
///
/// Checks the manifest on the configured interval and stages any available
/// update. The first check runs immediately.
pub fn run_ota_loop(cfg: OtaConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let period = Duration::from_secs_f64(cfg.check_interval_sec.max(60.0));
        log::info!("📦 OTA loop started (check every {}s)", period.as_secs());

        let mut timer = tokio::time::interval(period);
        loop {
            timer.tick().await; // first tick fires immediately
            match check_for_update(&cfg).await {
                Ok(Some(version)) => {
                    log::info!("📦 OTA update available: {}", version);
                    match fetch_and_apply_update(&cfg, &version).await {
                        Ok(()) => crate::metrics::inc_ota("apply_success"),
                        Err(e) => {
                            log::error!("📦 OTA apply failed: {}", e);
                            crate::metrics::inc_ota("apply_failure");
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => log::warn!("📦 OTA check failed: {}", e),
            }
        }
    })
}

/// Compute the SHA-256 of a payload as lowercase hex.
fn sha256_hex(payload: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode a hex string into bytes, ignoring whitespace.
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() || !cleaned.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    let chars: Vec<char> = cleaned.chars().collect();
    for pair in chars.chunks(2) {
        let hi = pair[0].to_digit(16)?;
        let lo = pair[1].to_digit(16)?;
        bytes.push(((hi << 4) | lo) as u8);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert!(is_newer("0.3.0", "0.2.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("v0.2.1", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn test_sanitize_version_blocks_path_traversal() {
        // Malicious manifest version must not escape the staging dir:
        // no path separators, dots, or control chars may survive.
        for evil in ["../../etc/cron.d/evil", "..\\..\\boot", "0.3.0\n", "a/b\\c", ".."] {
            let safe = sanitize_version(evil);
            assert!(
                !safe.contains(['/', '\\', '.', '\n']),
                "unsafe chars survived sanitize_version({evil:?}) -> {safe:?}"
            );
        }
        // Legit semver becomes a safe single filename segment (dots stripped).
        assert_eq!(sanitize_version("1.0.0"), "100");
        assert_eq!(sanitize_version("v0.2.1-rc1"), "v021-rc1");
    }

    #[test]
    fn test_hex_decode() {
        assert_eq!(hex_decode("00ff10"), Some(vec![0x00, 0xff, 0x10]));
        assert_eq!(hex_decode("0A"), Some(vec![10]));
        assert_eq!(hex_decode("0a 0b"), Some(vec![0x0a, 0x0b]));
        assert!(hex_decode("0").is_none());
        assert!(hex_decode("zz").is_none());
        assert!(hex_decode("").is_none());
    }

    #[test]
    fn test_sha256_hex() {
        // SHA-256("") — well-known test vector.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_verify_signature_rejects_bad_key() {
        let sig = "00".repeat(64);
        let key = "00".repeat(32);
        assert!(!verify_signature(b"hello", &sig, &key));
    }

    #[test]
    fn test_parse_manifest() {
        let manifest: OtaManifest = serde_json::from_str(
            r#"{"version":"0.3.0","url":"https://example.com/fw.bin","sha256":"abc","signature_hex":"def"}"#,
        )
        .unwrap();
        assert_eq!(manifest.version, "0.3.0");
        assert_eq!(manifest.url.as_deref(), Some("https://example.com/fw.bin"));
    }
}

