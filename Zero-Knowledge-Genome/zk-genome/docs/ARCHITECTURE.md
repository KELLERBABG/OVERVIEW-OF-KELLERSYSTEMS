# Zero-Knowledge Genome — Architecture

> **Document Status:** v0.1.0-alpha release notes
> **Last Updated:** 2026-08-16

> **v0.1.0-alpha shipping state.** This document describes the system we are building.
> The subset that actually shipped in v0.1.0-alpha is:
>
> - `api/` Axum 0.8 server — researcher registration, encrypted queries, proof submission, aggregate stats (**shipped**, 8/8 unit tests pass)
> - `contracts/` Solidity 0.8.28 — ConsentRegistry, Payment, ProofRegistry, QueryBoard (**compiles**, no Foundry tests in this release)
> - `core/` 12 workspace crates — genomic, cryptographic, storage primitives (**shipped**, `cargo check --workspace` + `cargo test --workspace` clean)
> - `mobile/native` Rust FFI crate with `radio_shield.rs` stub (**shipped** as a library)
> - `sdk/python` and `sdk/typescript` (**shipped**)
> - `tools/cli` end-to-end demo (**shipped**)
>
> Components described further down that **are NOT in v0.1.0-alpha**: React Native app screens/services, on-device encrypted storage wiring, CRYSTALS-Dilithium signature integration, end-to-end zk-SNARK generation on mobile, full mobile radio-shield runtime. See **`.github/RELEASE_NOTES/v0.1.0-alpha.md`** for the full list.

---

## 1. System Architecture

### 1.1 High-Level Design

The system follows a **zero-knowledge, mobile-first architecture** with four primary layers:

1. **Mobile App (Genery)** — React Native frontend + Rust FFI core
2. **Rust Core Library** — All cryptography, genomics, and storage logic
3. **API Gateway** — Rust (Axum 0.8) backend for pharma integration — **Shipped in v0.1.0-alpha** (see `api/src/routes/`)
4. **Blockchain Layer** — L2 smart contracts for payment/settlement (Phase 3)

### 1.2 The 13 Components

| # | Component | Crate | Phase |
|---|-----------|-------|-------|
| 1 | Secure On-Device Storage | `storage-traits` | 1 |
| 2 | Zero-Knowledge Proofs (zk-SNARKs) | `zk-core` | 2 |
| 3 | Encrypted Query Matching | `query-engine` | 2 |
| 4 | Automated Micropayment Settlement | `contracts/` | 3 |
| 5 | Anonymized Statistics Aggregation | `api/` | 3 |
| 6 | Selective Disclosure Toggles | `mobile/` | 3 |
| 7 | Immutable Consent Management | (DID/VC) | 1 |
| 8 | Homomorphic Mutation Screening | `query-engine` | 2 |
| 9 | Differential Privacy Layer | `api/` | 2 |
| 10 | Verification Node Consensus | `contracts/` | 3 |
| 11 | Genetic Entropy Audit | `entropy-auditor` | 1 |
| 12 | Pharmaceutical Oracle Integration | `api/` | 3 |
| **13** | **Secure Laboratory Attestation & Ingestion** | **`genome-ingestion`** | **1** |

### 1.3 Ingestion Flow (Trust Anchor)

```
Certified Lab  ──→  Encrypted VCF  ──→  Signed (Dilithium + Ed25519)  ──→  Device
                                                                              │
                                         ┌────────────────────────────────────┴────────────┐
                                         │ 1. Verify lab certificate (TrustedLabRegistry) │
                                         │ 2. Verify hybrid signature (Ed25519+Dilithium) │
                                         │ 3. Verify SHA-3-512 integrity hash             │
                                         │ 4. Strip raw data (FASTQ/BAM discarded)        │
                                         │ 5. Store compressed variants encrypted         │
                                         │ 6. Update entropy audit chain                  │
                                         └────────────────────────────────────────────────┘
```

### 1.4 The One-Exit Principle

> **There is exactly one exit from Genery, and that exit is Global Ghost Net.**

Genery is a **zero-emission system** by design. No data, no signal, no electromagnetic radiation, no proof output, no aggregate statistic, no metadata, no telemetry, no crash report, no log line — nothing — leaves Genery through any path other than the explicitly designed Global Ghost Net bridging layer.

Until Global Ghost Net is operational and connected, **Genery has zero exits.** It is a completely air-gapped system that stores, processes, queries, and proves genomic data entirely on-device with no external communication path for any data originating from the genome.

## 2. Architectural Invariants

### 2.1 Post-Quantum Integrity

All integrity mechanisms use post-quantum secure primitives from day one:

| Integrity Domain | Mechanism | PQ Security |
|-----------------|-----------|-------------|
| Entropy Audit | SHA-3-512 hash chain | Grover: 256-bit |
| Consent Signatures | CRYSTALS-Dilithium + Ed25519 hybrid | Lattice-based |
| zk Proof Public Inputs | Poseidon hash + KZG | Post-quantum zk |
| On-Chain Proof Registry | SHA-3-256 Merkleized accumulator | Hash-based |
| **Lab Attestation Signatures** | **CRYSTALS-Dilithium + Ed25519 hybrid** | **Lattice-based** |
| **Ingestion Integrity** | **SHA-3-512 data hash** | **Grover: 256-bit** |

### 2.2 Signal Isolation

During any operation handling genomic plaintext:
1. App requests OS disable all radios (Wi-Fi, Cellular, Bluetooth, NFC)
2. System verifies radio-off state before proceeding
3. Continuous 500ms poll monitors interface state
4. Emergency buffer wipe + re-encrypt if radio detected active

### 2.3 Key Hierarchy

```
Secure Enclave
  └── Device Key (SE-backed, never exported)
        └── Database Encryption Key (wrapped by Device Key)
              └── Genome Data (AES-256-GCM in SQLCipher)
```

## 3. Mobile Architecture

### 3.1 Component Tree

```
App.tsx
├── AuthGate — Biometric authentication
├── GenomeScreen
│   ├── LabIngestion — Secure lab import (signed package verification)
│   ├── GenomeImport — Legacy VCF file picker
│   ├── GenomeBrowser — Chromosome list → variant counts
│   └── EntropyStatus — Integrity check display
├── ConsentScreen
│   ├── DIDManager — Key generation, identity display
│   └── ConsentList — Grant/revoke data permissions
├── QueryScreen (Phase 2)
│   ├── IncomingQueries — Encrypted query list
│   └── ProofStatus — Proof generation progress
└── SettingsScreen
    ├── SecuritySettings — Biometric, radio shield
    ├── BackupRecovery — Key export, migration
    └── About — Version, licenses
```

### 3.2 Service Layer

```
services/
├── SecureStorage.ts        — Keychain/Keystore abstraction
├── RustBridge.ts           — uniffi Rust FFI wrapper
├── RadioShield.ts          — Signal isolation native module
├── IngestionService.ts     — Lab attestation verification + import
├── GenomeService.ts        — Parse, query orchestration
├── ConsentService.ts       — DID lifecycle, credential issuance
└── EntropyService.ts       — Integrity check scheduling
```

## 4. Data Flow

### 4.1 Lab-Certified Genome Import

```
SignedGenomePackage from Lab
  │
  ├── TrustedLabRegistry.verify()
  │   ├── Is lab registered?          → UnrecognizedLab
  │   ├── Is assembly supported?      → UnsupportedAssembly
  │   ├── Does SHA-3-512 hash match?  → IntegrityMismatch
  │   └── Is hybrid signature valid?  → InvalidSignature
  │
  └── verification::ingest()
      ├── Raw data discarded (FASTQ/BAM never stored)
      ├── Compressed variants encrypted via SQLCipher
      ├── Lab attestation stored alongside
      └── Entropy audit chain updated with variant_data_hash
```

### 4.2 Encrypted Query (Phase 2)

```
Pharma Query (encrypted) → Device
  → Decrypt query locally (ECDH + HKDF shared secret)
  → Query Engine matches (pos, ref, alt) against stored variants
  → zk Proof Generator proves match without revealing genome
  → Proof output → Global Ghost Net (when connected)
```

## 5. Security Model

### 5.1 Ingestion Security

| Threat | Mitigation |
|--------|------------|
| Sybil attack (fake genome) | Lab signature verification via TrustedLabRegistry |
| Data tampered in transit | SHA-3-512 integrity hash + dual signature |
| Counterfeit lab identity | CLIA/ISO accreditation bound to Dilithium public key |
| Replay attack (same genome re-imported) | variant_data_hash duplicate detection |
| Raw data exposure | FASTQ/BAM discarded before device storage |

### 5.2 Threat Mitigation

See [THREAT_MODEL.md](THREAT_MODEL.md) for the full STRIDE analysis. Key mitigations:

- **Rooted device detection:** Runtime jailbreak detection with compile-time obfuscation
- **No cloud backup:** `NSURLIsExcludedFromBackupKey` / `android:allowBackup="false"`
- **Screen capture prevention:** `FLAG_SECURE` / snapshot redaction
- **Certificate pinning:** Embedded public key hashes for API connections
- **Minimum app version:** Server-side enforcement against downgrade attacks

### 5.3 Post-Quantum Cryptography

All cryptographic choices are post-quantum ready:
- **Hashing:** SHA-3 family only (SHA-3-512 for integrity, SHAKE256 for zk)
- **Signatures:** CRYSTALS-Dilithium (FIPS 203) hybrid with Ed25519
- **Key exchange:** CRYSTALS-Kyber (FIPS 204) when required
- **zk Proofs:** PLONK + KZG commitments (post-quantum trusted setup)
- **Lab attestation:** Ed25519 + Dilithium hybrid (both must verify)

## 6. Dependencies

### 6.1 Rust Crates (Core)

| Crate | Purpose |
|-------|---------|
| genome-core | Core data types, chromosome enums, variant structs |
| genome-parser | VCF v4.2 parser (streaming, memory-efficient) |
| genome-normalizer | VCF normalization (left-alignment, trimming) |
| genome-reference | hg19/hg38 reference genome coordinate mapping |
| genome-compress | Delta-encoding, bit-packing for variant storage |
| genome-annotate | rsID/gene/ClinVar annotation lookup |
| **genome-ingestion** | **Lab attestation verification, Sybil-resistant import** |
| query-engine | Encrypted query matching engine |
| entropy-auditor | SHA-3-512 hash chain integrity |
| storage-traits | Platform-abstracted encrypted storage |
| zk-core | zk-SNARK circuits and proof generation (Phase 2) |

### 6.2 External Dependencies

| Dependency | Use | Phase |
|------------|-----|-------|
| SQLCipher | Encrypted local database | 1 |
| uniffi | Rust to Kotlin/Swift FFI bindings | 1 |
| Circom + snarkjs | zk-SNARK circuit compilation | 2 |
| CRYSTALS-Dilithium | Post-quantum signatures | 1 |
| Axum | Rust HTTP API server | 3 |
| Solidity + Foundry | Smart contracts | 3 |

## 7. Development Environment

- **OS:** macOS (primary) / Linux / Windows (limited)
- **Rust:** stable (pinned via `zk-genome/rust-toolchain.toml`)
- **Node:** >= 18
- **Mobile:** iOS 17+ / Android 14+
- **Editor:** VS Code with rust-analyzer, ESLint, Prettier

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions.