# Zero-Knowledge Genome — Threat Model

> **Document Status:** v0.1.0-alpha release notes
> **Methodology:** STRIDE per component
> **Last Updated:** 2026-08-16

> **Threat-model scope for v0.1.0-alpha.** The deployed surface is the **Axum `api/` server** plus the **four Solidity contracts in `contracts/`**. Anything else referenced in this document (mobile app code, on-device encryption, hybrid Dilithium signatures, full ECDH query matching) is **not part of the deployable v0.1.0-alpha**. STRIDE mitigations marked "deployed" below refer to that subset; everything else is design intent tracked for future releases.
>
> **Honest limitations of v0.1.0-alpha based on the actual `api/` + `contracts/` shipping surface:**
>
> - Auth = bearer tokens, not full Dilithium hybrid. Bearer tokens leak via log lines unless explicitly suppressed; `routes/auth.rs` does not log them and downstream code paths must not.
> - `AppState` is in-memory and unprotected — anyone with network access to the API daemon can hit `/register`, `/query`, `/proof` once they have a token. There is no DB-level isolation.
> - Replay protection for proofs is enforced at the API server via `submitted_proofs` keyed on `(query_id, sha3_512(blob))`; duplicate replay on-chain prevention still requires ProofRegistry.sol integration.
> - Smart contracts compile but have **no Foundry tests** in this release. They should not be deployed to a public chain without an external audit.

---

## 1. System Overview

This threat model covers the Zero-Knowledge Genome system — a mobile-first, privacy-preserving genomic data platform. The system stores full human genomes on-device with no cloud storage, processes queries via encrypted matching, and generates zk-SNARK proofs of mutation presence without revealing the genome.

### 1.1 Assets

| Asset | Sensitivity | Location |
|-------|-------------|----------|
| Full genome sequence (3B+ base pairs) | **Extreme** — identifies individual, family, predispositions | On-device encrypted SQLCipher |
| Variant set (1M+ SNPs/indels) | **Extreme** — sufficient for re-identification | On-device encrypted SQLCipher |
| Encryption keys | **Critical** — controls access to genome | iOS Keychain / Android Keystore |
| Consent credentials (DID keys) | **High** — controls data sharing permissions | On-device DID wallet |
| zk-SNARK proofs | **Low** — provably zero-knowledge | Transmitted to pharma (Phase 3) |
| Aggregate statistics | **Medium** — differential privacy applied | Transmitted via DP layer (Phase 3) |

### 1.2 Trust Boundaries

1. **Device boundary** — App sandbox vs. OS vs. other apps
2. **Secure Enclave boundary** — SE cannot store genome directly (too large)
3. **Network boundary** — App is air-gapped (Phase 1) / limited to proof output (Phase 3)
4. **Blockchain boundary** — Smart contracts only see proof hashes (Phase 3)

## 2. STRIDE Analysis

### 2.1 Spoofing

| Threat | Risk | Mitigation |
|--------|------|------------|
| Fake app replaces legitimate app | Medium | Code signing, app store verification, startup integrity hash |
| Malicious pharma impersonates legitimate researcher | Medium | DID-based authentication, API keys, certificate pinning (Phase 3) |
| Biometric bypass via deepfake/spoof | Low-Medium | Multi-factor: biometric + device passcode, rate-limiting |

### 2.2 Tampering

| Threat | Risk | Mitigation |
|--------|------|------------|
| Genome data modified on-device | Medium | Entropy audit: SHA-3-512 hash chain, periodic integrity checks |
| Consent credential modified | Medium | DID-based signatures (CRYSTALS-Dilithium hybrid) |
| zk proof modified in transit | Low | Proof verification on-chain, KZG commitments |
| App binary modified (re-packaged) | Medium | Code integrity hash at startup, app store notarization |

### 2.3 Repudiation

| Threat | Risk | Mitigation |
|--------|------|------------|
| User denies consenting to query | Low | Immutable consent audit log with DID signatures |
| Pharma denies submitting query | Low | On-chain query registry (Phase 3) |
| User denies receiving payment | Low | On-chain payment settlement (Phase 3) |

### 2.4 Information Disclosure

| Threat | Risk | Mitigation |
|--------|------|------------|
| Genome extracted from device storage | High | SQLCipher AES-256-GCM, keys in Secure Enclave |
| Genome exfiltrated via network | High | Signal isolation: radios disabled during genome operations |
| Genome leaked via cloud backup | High | Explicit opt-out of iCloud/Google Drive backup |
| Genome captured via screen recording | Medium | FLAG_SECURE, snapshot redaction, clipboard disabled |
| Memory dump during genome processing | Medium | Zero sensitive data after processing, mmap with mlock |
| Side-channel leakage (timing, power) | Low | Constant-time crypto primitives |

### 2.5 Denial of Service

| Threat | Risk | Mitigation |
|--------|------|------------|
| Battery drain from proof generation | Medium | Throttle: limit proofs on low battery, checkpoint-resume |
| Storage exhaustion from entropy chain | Low | Configurable retention, periodic pruning |
| Smart contract griefing (Phase 3) | Low | Staking/slashing for validators |

### 2.6 Elevation of Privilege

| Threat | Risk | Mitigation |
|--------|------|------------|
| Root/jailbreak bypasses SE protection | Medium | Runtime jailbreak detection, refuse decryption on rooted devices |
| Malware with AccessibilityService reads genome | Medium | Never display full genome as scrollable text, paginated summaries only |
| TEE firmware vulnerability | Medium | StrongBox requirement, device compatibility matrix |

## 3. Attack Tree: Genome Extraction

```
Root: Extract plaintext genome from device
├── 1. Physical device theft
│   ├── 1.1 Crack weak passcode
│   │   └── Countermeasure: Rate-limiting, auto-wipe after N failures
│   └── 1.2 Extract keys from SE
│       └── Countermeasure: SE is hardware-bound, key never exported
├── 2. Malware on device
│   ├── 2.1 Root/jailbreak exploit
│   │   └── Countermeasure: Detection at runtime, refuse decryption
│   ├── 2.2 Screen recording
│   │   └── Countermeasure: FLAG_SECURE, blur on background
│   └── 2.3 Memory dump
│       └── Countermeasure: Zero buffers after use, mlock
├── 3. Network exfiltration
│   ├── 3.1 Malicious app with network access
│   │   └── Countermeasure: Signal isolation, radio shutdown
│   ├── 3.2 Cloud backup sync
│   │   └── Countermeasure: Explicit backup exclusion
│   └── 3.3 Phishing/Social engineering
│       └── Countermeasure: User education in app
└── 4. Cryptographic attacks
    ├── 4.1 Quantum decryption of stored genome
    │   └── Countermeasure: AES-256-GCM (post-quantum safe)
    └── 4.2 Side-channel key extraction
        └── Countermeasure: Constant-time operations
```

## 4. Mobile-Specific Threats

### 4.1 iOS

| Threat | Mitigation |
|--------|------------|
| Checkm8/Jailbreak | Runtime detection, refuse decryption |
| iCloud backup | `NSURLIsExcludedFromBackupKey` on genome directory |
| App snapshot in app switcher | Override `applicationDidEnterBackground` to blur |
| Universal link hijack | Verify source application, HTTPS only |

### 4.2 Android

| Threat | Mitigation |
|--------|------------|
| Fragmented TEE implementations | StrongBox requirement, fallback to passphrase-derived key |
| AccessibilityService abuse | Never display full genome text, disable clipboard |
| APK repackaging | Code integrity verification, Google Play Signing |
| Task snapshot | `FLAG_SECURE` on all sensitive activities |

## 5. Cryptographic Threat Analysis

| Primitive | Classical Security | Post-Quantum Security | Notes |
|-----------|-------------------|----------------------|-------|
| AES-256-GCM | 256-bit | 128-bit (Grover) | Sufficient for 70-year horizon |
| SHA-3-512 | 512-bit | 256-bit (Grover) | Chosen for PQ margin |
| Ed25519 | 128-bit | **Broken** (Shor) | Hybrid with Dilithium |
| CRYSTALS-Dilithium | N/A | 128-bit (Lattice) | FIPS 203 compliant |
| Poseidon hash | 128-bit | 128-bit (ZK-friendly) | Used in circuits only |

**Decision:** All signatures use Dilithium + Ed25519 hybrid mode. No pure classical signatures for integrity-sensitive operations.

## 6. Trust Assumptions

1. **The mobile OS is trustworthy** — we rely on iOS/Android kernel security
2. **The Secure Enclave/TEE is secure** — hardware-backed key isolation
3. **The app store review process catches obvious malware** — but not zero-days
4. **The user's biometric is reasonably unique** — not perfect, but sufficient when combined with passcode
5. **Pharma actors follow the protocol** — economic incentives via staking (Phase 3)

## 7. Risk Register

| Risk | Probability | Impact | Response |
|------|------------|--------|----------|
| Device compromised via 0-day | Low | Critical | Remote wipe, forced app update |
| User loses biometric + passcode | Medium | High | Recovery key (12-word seed) |
| Android TEE vulnerability | Medium | High | StrongBox requirement, compatibility list |
| Quantum computer breaks RSA/ECC | Medium (15yr) | Critical | Already migrated to PQ crypto |
| App store policy changes | Low-Medium | High | Pre-submit concept, compliance docs ready |

---

> **Review Cycle:** Every sprint during Phase 1, quarterly thereafter  
> **Next Review:** Sprint 17-18 (Security Hardening)