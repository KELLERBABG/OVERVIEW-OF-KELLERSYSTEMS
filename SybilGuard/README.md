<div align="center">

# &#128737;&#65039; SybilGuard

**Zero-Knowledge Proof-of-Personhood & Sybil-Resistant Identity Protocol**

<br>

[![Release](https://img.shields.io/badge/Release-v0.2.0-2563eb?style=flat-square)](https://github.com/KELLERBABG/SybilGuard/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-059669?style=flat-square)](LICENSE)
[![Language](https://img.shields.io/badge/Language-Rust%20%7C%20Circom-orange?style=flat-square)](https://www.rust-lang.org/)
[![ZK-Prover](https://img.shields.io/badge/Prover-Groth16%20%7C%20BN254-blueviolet?style=flat-square)](circuits/)
[![Network](https://img.shields.io/badge/Network-Citadel%20Mesh-blue?style=flat-square)](https://citadel.kellersystems.dev)

<br>

[**Protocol Specifications**](docs/) &bull; [**ZK Circuits**](circuits/) &bull; [**Live Citadel Interface**](https://citadel.kellersystems.dev) &bull; [**Mobile Enclave**](mobile/)

</div>

---

## What is SybilGuard?

**SybilGuard** is a decentralized, zero-knowledge identity and proof-of-personhood protocol. It enables sovereign individuals to prove uniqueness and human authenticity across decentralized networks without revealing biometric data, government credentials, or persistent pseudonyms.

Using hardware Secure Enclave attestation combined with Circom/Groth16 zk-SNARK circuits over BN254, SybilGuard generates deterministic nullifiers per application scope, preventing multi-identity exploitation and automated Sybil attacks while preserving complete mathematical anonymity.

---

## Why Use It?

* **Zero-Knowledge Uniqueness:** Generate mathematical proofs that you are a verified, unique human without disclosing real identity, biometric markers, or linking activity across domains.
* **Hardware Enclave Binding:** Attestations are rooted directly inside physical Secure Enclaves / TPMs, preventing proof duplication or credential extraction.
* **Deterministic Scope Nullifiers:** Guarantees one person, one vote/claim per application context through cryptographic nullifier derivation:
  $$\text{Nullifier} = \mathcal{H}(\text{IdentitySecret}, \text{ScopeID})$$
* **High-Throughput Sub-Second Verification:** Verified on-chain via smart contracts or off-chain via Rust batch verification crates.

---

## Architecture Overview

```
SybilGuard Protocol
├── circuits/           # Circom zk-SNARK circuits (Proof-of-Personhood & Nullifier derivation)
├── crates/             # Core Rust cryptographic primitives and proof verifiers
├── dashboard/          # Node operator & governance audit dashboard
├── mobile/             # Hardware enclave integration for iOS/Android
├── docs/               # Technical specs and threat models
├── scripts/            # Circuit compilation, trusted setup ceremonies, and tests
└── index.html          # Interactive Citadel portal explorer
```

---

## Quickstart

### Prerequisites
* Rust toolchain (1.75+)
* Circom 2.1.6+
* Node.js 18+

### Compile Circuits & Build Crates

```bash
git clone https://github.com/KELLERBABG/SybilGuard.git && cd SybilGuard && cargo build --release
```

Run circuit unit tests and witness generation:
```bash
cargo test --workspace
```

---

<div align="center">
&copy; 2026 Keller Systems &bull; Built for Sovereign Machine Verification
</div>
