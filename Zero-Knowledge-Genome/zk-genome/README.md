# Zero-Knowledge Genome 🔬🔐

> **Privacy-preserving genomic data marketplace.**  
> Store your genome on-device. Prove mutations without revealing your DNA. Get paid for research.

[![Release](https://github.com/KELLERBABG/ZKG/actions/workflows/release.yml/badge.svg)](https://github.com/KELLERBABG/ZKG/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

> **v0.1.0-alpha release notes.** This README describes the entire system as designed. For an
> honest snapshot of what ships in v0.1.0-alpha vs. what is future work, see
> **`.github/RELEASE_NOTES/v0.1.0-alpha.md`**.

---

---

## Overview

Zero-Knowledge Genome is a **privacy-first platform** where individuals store their genomic data **exclusively on their mobile device** and can cryptographically prove the presence of specific genetic variants to pharmaceutical researchers — without ever revealing their identity, their full genome, or any other private data.

### The Problem

- Genomic data is the **most sensitive biometric identifier** — it identifies you, your family, and your predispositions for life
- Current genomic databases are centralized honeypots: 23andMe, AncestryDNA breaches have exposed millions of genetic profiles
- Pharmaceutical research needs access to genomic data, but the current model forces users to choose between privacy and participation

### The Solution

| Component | Technology |
|-----------|------------|
| **Secure Storage** | On-device Secure Enclave + SQLCipher encrypted database |
| **Zero-Knowledge Proofs** | Circom + snarkjs zk-SNARKs (Groth16/PLONK) on-device |
| **Encrypted Queries** | Pharma queries never see raw data — matched locally on-device |
| **Consent Management** | Decentralized Identity (DID) + W3C Verifiable Credentials |
| **Data Integrity** | Post-quantum SHA-3-512 hash chains (entropy audit) |
| **Privacy Layer** | Differential privacy (Laplace mechanism) for aggregate statistics |
| **Payments** | Automated micropayment settlement via L2 smart contracts |
| **Signal Isolation** | Hardware-level radio shutdown during genome operations |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        MOBILE APP (Genery)                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌──────────┐  │
│  │ Secure      │  │ Genomic      │  │ zk Proof    │  │ Consent  │  │
│  │ Enclave     │  │ Pipeline     │  │ Generator   │  │ Manager  │  │
│  │ (Keychain/  │  │ (Rust: VCF → │  │ (Circom +   │  │ (DID/VC) │  │
│  │  TEE)       │  │  variants)   │  │  wasm)      │  │          │  │
│  └──────┬──────┘  └──────┬───────┘  └──────┬──────┘  └────┬─────┘  │
│         │                │                 │              │         │
│  ┌──────┴────────────────┴─────────────────┴──────────────┴─────┐  │
│  │                    Rust Core Library (FFI)                     │  │
│  │  • VCF parsing + normalization      • Hash chain integrity    │  │
│  │  • Encrypted query matching          • Differential privacy   │  │
│  │  • Lightweight searchable encryption  • Entropy audit         │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                          ┌─────────┴──────────┐
                          │                    │
                    ┌─────▼─────┐       ┌──────▼──────┐
                    │ Blockchain │       │ Pharma      │
                    │ (L2)      │       │ Dashboard   │
                    │ • Payment │       │ (Web App)   │
                    │ • Consent │       │ • Queries   │
                    │ • Proofs  │       │ • Bids      │
                    └───────────┘       │ • Analytics │
                                         └─────────────┘
```

## Project Structure

```
zk-genome/
├── api/                     # Axum 0.8 HTTP server — researcher routes (auth · queries · proofs · stats)
├── contracts/               # Solidity 0.8.28 — ConsentRegistry · Payment · ProofRegistry · QueryBoard
├── core/                    # Rust workspace — 12 crates of cryptography & genomics
│   ├── genome-core/         #   Core data types (Chromosome, Variant, Genotype)
│   ├── genome-parser/       #   VCF v4.2 parser
│   ├── genome-normalizer/   #   Left-aligned VCF normalization
│   ├── genome-reference/    #   hg19/hg38 reference support
│   ├── genome-compress/     #   Delta-encode + bit-pack variant positions
│   ├── genome-annotate/     #   rsID, gene symbol, ClinVar annotation
│   ├── genome-dtc-import/   #   23andMe / AncestryDNA / MyHeritage raw ingest
│   ├── genome-ingestion/    #   Lab attestation & signed VCF imports
│   ├── query-engine/        #   Encrypted query matching engine
│   ├── entropy-auditor/     #   SHA-3-512 hash chain integrity auditor
│   ├── storage-traits/      #   Platform-abstracted encrypted storage
│   └── zk-core/             #   zk-SNARK circuits (Circom) + prover + verifier
├── mobile/                  # Mobile FFI — Rust crate ships; React Native UI is future work
│   └── native/              #   Rust mobile library (uniffi bindings + RadioShield)
├── sdk/                     # Researcher-facing SDKs
│   ├── python/              #   Python SDK (zk_genome_sdk.py)
│   └── typescript/          #   TypeScript SDK
├── tools/                   # Internal tooling
│   └── cli/                 #   Demo CLI that runs end-to-end against core/
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      #   System architecture deep-dive
│   ├── THREAT_MODEL.md      #   STRIDE threat model
│   ├── PRIVACY.md           #   Data handling & privacy policy
│   ├── whitepaper/          #   Privacy-preserving genome marketplace whitepaper
│   └── v1.1-engineering-challenges.md
├── global-ghost-net/        # ⚠️ Placeholder — single exit point (not yet implemented)
└── scripts/                 # Build & dev helper scripts (used to lay down the MVP)
```

## Development

### Prerequisites

- Rust (stable — pinned via `zk-genome/rust-toolchain.toml`)
- Foundry (only needed for `zk-genome/contracts/` Solidity tests)
- The React Native mobile UI is **not** part of v0.1.0-alpha; the `mobile/native` crate ships as a library only

### Quick Start

```bash
# Clone and enter
git clone https://github.com/KELLERBABG/ZKG.git
cd ZKG/zk-genome

# Build all Rust crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Check code quality
cargo clippy --workspace
cargo fmt --all --check
```

## Phases

The four-phase plan from `DEVELOPMENT_PLAN.md` describes a 56-month solo-dev roadmap.
The shipping state as of v0.1.0-alpha is:

| Phase | Scope | State in v0.1.0-alpha |
|-------|-------|-----------------------|
| 1 | Genomic foundation + crate wiring | ✅ 12 `core/` crates compile and pass unit tests; `mobile/native` FFI crate ships as a library |
| 2 | zk-SNARKs + query pipeline | 🚧 Circom circuits defined; prover/verifier shells present; end-to-end pairing is future work |
| 3 | Marketplace & settlement | ✅ `api/` ships the `/register`, `/query`, `/proof`, `/stats` routes (8/8 unit tests pass); ✅ 4 Solidity contracts compile (Foundry tests are future) |
| 4 | Clinical integration & adoption | 📋 Planned — `genome-dtc-import` is the seed; React Native frontend is future |

See **`.github/RELEASE_NOTES/v0.1.0-alpha.md`** for the per-component honesty table.

## Security

The cryptography stack is **designed for post-quantum** security and is rolled out progressively:

| Domain | v0.1.0-alpha state | Future target |
|--------|--------------------|---------------|
| Authentication | Bearer-token (`Authorization: Bearer <api_key>`) with SHA-3-512 hashing | CRYSTALS-Dilithium (FIPS 203) hybrid signatures with Ed25519 |
| Integrity audit | SHA-3-512 hash chain in `entropy-auditor` crate | Same |
| On-device storage | `storage-traits` crate compiles; no SQLCipher wiring yet | SQLCipher with AES-256-GCM, keys in Secure Enclave |
| Signal isolation | `mobile/native/radio_shield.rs` is a stub | Hardware-level radio shield enforced from the React Native app |
| Privacy telemetry | Zero analytics SDKs, zero crash reporters | Same |

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) and [THREAT_MODEL.md](docs/THREAT_MODEL.md) for the full design and the honest limitations of v0.1.0-alpha.

## License

MIT