# Zero-Knowledge Genome — Privacy Policy & Data Handling

> **Document Status:** v0.1.0-alpha release notes
> **Last Updated:** 2026-08-16

> **Read this first.** Most of this document describes the design we are committed to.
> v0.1.0-alpha ships only a small subset of it; the rest is future work tracked in
> **`.github/RELEASE_NOTES/v0.1.0-alpha.md`**.
>
> **In v0.1.0-alpha the privacy boundary is:**
>
> - **No genomic data enters the system anywhere.** The `api/` server has zero surface to ingest genomes. It only ever sees (a) opaque proof blobs from researchers and (b) bearer-token-authenticated researcher → user queries and stats.
> - **No on-disk persistence.** `AppState` is an in-memory `Arc<RwLock<AppStateInner>>` that resets on server restart. There is no SQLCipher, no DB.
> - **No telemetry, no analytics, no third-party SDKs.**
> - **Dilithium is designed, not integrated.** Auth uses bearer tokens with SHA-3-512 hashing in this release. Hybrid Dilithium + Ed25519 signatures are designed but not deployed.
>
> Sections that describe on-device app-storage, Keychain/Keystore key wrapping, ECDH-based query encryption, and differential-privacy aggregate release refer to the system as it WILL be built, **not** what is in this tagged commit.

---

## 1. Core Privacy Principles

### 1.1 Zero-Knowledge by Design

Zero-Knowledge Genome operates on a **data minimization** and **zero-knowledge** principle:

- **Genomic data never leaves your device.** All storage, processing, and query matching happens locally in your phone's Secure Enclave/Keystore.
- **No telemetry.** The app contains no analytics SDK, no crash reporter that sends genome-context data, no usage tracking.
- **No cloud dependency.** There are no API calls carrying genome-derived data. The system does not depend on any external server for genome operations.
- **Signal isolation.** All radios are programmatically disabled during genome operations to prevent any data exfiltration.

### 1.2 Legal Basis (GDPR Art. 9)

Processing of genetic data is prohibited under GDPR Art. 9(1) unless explicit consent is given under Art. 9(2)(a). This system is designed so that:

1. **No processing occurs on any server** — all processing is local on the user's device
2. **The user retains full control** via DID-based consent management
3. **Consent is granular, revocable, and auditable** — each data use permission is individually issued and can be revoked independently
4. **Data portability** — the user can export their genome at any time (encrypted)

## 2. Data We Collect

### 2.1 What We Store (On-Device Only)

| Data Type | Storage | Purpose | Retention |
|-----------|---------|---------|-----------|
| Full genome (VCF-derived variants) | Encrypted SQLCipher on-device | Local query matching, proof generation | Until user deletes |
| Encryption keys | iOS Keychain / Android Keystore | Genome encryption/decryption | Until user revokes |
| Consent credentials | Local DID wallet | Consent audit trail | Until revoked |
| Entropy audit chain | Encrypted local storage | Integrity verification | Permanent on-device |

### 2.2 What We NEVER Store or Transmit

| Data Type | Reason |
|-----------|--------|
| Raw genomic data (anywhere outside device) | Zero-knowledge principle |
| IP addresses | No logging policy |
| Usage analytics | No telemetry policy |
| Crash reports with genome context | No genome data in crash context |
| Advertising identifiers | No advertising |
| Location data | No location permission requested |
| Contacts | No contacts permission requested |

## 3. Data Processing

### 3.1 Genome Import

When you import a VCF file:
1. File is read on-device only
2. Variants are extracted, normalized, compressed
3. Data is encrypted with AES-256-GCM via SQLCipher
4. Encryption keys are stored in hardware-backed Secure Enclave
5. Original file can be deleted after import

### 3.2 Query Matching (Phase 2)

When a pharmaceutical query arrives:
1. Query is encrypted end-to-end
2. Decrypted only on-device
3. Matched locally against your stored genome
4. A zk-SNARK proof is generated that proves the match without revealing your genome
5. Only the proof (not your data) leaves the device

### 3.3 Differential Privacy (Phase 2)

For aggregate statistics:
1. Laplace noise is added to all aggregate outputs
2. Privacy budget (ε) is tracked per user per period
3. k-anonymity check is performed before any release
4. No individual-level data is ever included in aggregates

## 4. Your Rights

### 4.1 Access
You can view all stored genomic data on-device at any time.

### 4.2 Rectification
You can delete and re-import your genome data.

### 4.3 Erasure ("Right to be Forgotten")
- Local genome data: Deleted immediately on request
- On-chain consent proofs: Key revocation cryptographically renders them inaccessible
- Proof hashes on-chain: Immutable but contain no personal data (only hash of proof)

### 4.4 Data Portability
You can export your genome in VCF format at any time.

### 4.5 Withdraw Consent
Revoke individual consent credentials at any time via the app. Future queries requiring that permission will be denied.

## 5. Third-Party Data Sharing

### 5.1 Current (Phase 1)
**No third party receives any data.** The system is completely air-gapped.

### 5.2 Future (Phase 3 — Marketplace)
When the marketplace is active:
- Pharma receives only **zk-SNARK proofs** (not raw genome data)
- Proofs verify mutation presence without revealing identity or other variants
- All transactions are recorded on-chain but contain only proof hashes
- No personal data ever enters the blockchain

## 6. Security Measures

- **Encryption at rest:** AES-256-GCM via SQLCipher
- **Key management:** iOS Keychain / Android Keystore (hardware-backed)
- **Signal isolation:** All radios disabled during genome operations
- **Post-quantum hashing:** SHA-3-512 for integrity checks
- **No cloud backup:** App data excluded from iCloud/Google Drive
- **Screen capture prevention:** Sensitive views are redacted
- **Minimum app version:** Server-side enforcement against downgrade attacks
- **Certificate pinning:** HTTPS connections verify pinned certificates

## 7. Compliance

### 7.1 GDPR (EU)
- Art. 9(2)(a): Explicit consent for genetic data processing
- Art. 17: Right to erasure (data is on-device, deletable)
- Art. 20: Right to data portability
- Art. 25: Data protection by design and by default
- Art. 32: Security of processing
- Art. 33: Breach notification (no central data store to breach)

### 7.2 HIPAA (US)
The system is designed as HIPAA-compliant when used in a healthcare context:
- All PHI remains on-device
- Encryption standards exceed HIPAA requirements
- Access controls (biometric + device passcode)
- Audit controls (entropy audit trail)

### 7.3 GINA (US)
- No employment discrimination possible — employers never see genomic data
- No health insurance discrimination — insurers never see genomic data
- User retains complete control over disclosure

## 8. Data Breach Protocol

In the event of a security incident:
1. **No central data store exists** — genome data is distributed across individual devices
2. If a device-specific vulnerability is discovered, an app update with forced minimum version will be issued
3. Users will be notified via in-app alert and push notification
4. Affected users can revoke all consent credentials and re-encrypt with new keys

## 9. Contact

For privacy questions or data requests:
- **Email:** privacy@zeroknowledgegenome.io
- **Data Protection Officer:** Available upon request
- **Supervisory Authority:** [Your Lead DPA]

---

> **Last Updated:** 2026-07-28  
> **Next Review:** At Phase 1 completion (Sprint 19-20 regulatory prep)