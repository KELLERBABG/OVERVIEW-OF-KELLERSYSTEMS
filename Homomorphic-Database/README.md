<div align="center">

# &#128420; Homomorphic Database (FHE-DB)

**Encrypted aggregation and encrypted-weight neural inference over TFHE**

<br>

[![Version](https://img.shields.io/badge/Version-v0.1.0-2563eb?style=flat-square)](#)
[![License](https://img.shields.io/badge/License-BSD--3--Clause--Clear-059669?style=flat-square)](#)
[![Language](https://img.shields.io/badge/Language-Rust%20(Pure)-orange?style=flat-square)](https://www.rust-lang.org/)
[![Crypto](https://img.shields.io/badge/FHE-TFHE--rs-blueviolet?style=flat-square)](#)
[![Subdomain](https://img.shields.io/badge/Gateway-fhe.kellersystems.dev-0284c7?style=flat-square)](https://fhe.kellersystems.dev)

<br>

[**Status & Roadmap**](PROJECT_STATUS.md) &bull; [**Security Guidelines**](SECURITY.md) &bull; [**Internal Audit Package**](docs/audit/README.md) &bull; [**Live Gateway**](https://fhe.kellersystems.dev)

</div>

---

## Overview

**Homomorphic Database (FHE-DB)** is a Rust database and inference stack built
on [TFHE-rs](https://github.com/zama-ai/tfhe-rs). A client encrypts values
locally and sends only ciphertexts plus the public server key; the server stores
them and evaluates homomorphic operations without ever seeing value plaintext.
Only the holder of the secret key can decrypt results. Schema names, row
counts, access patterns, and the XNOR bias are metadata visible to the server;
they are not covered by the value-encryption guarantee. FHE provides
confidentiality, not verifiable computation: a malicious server can return a
wrong or replayed ciphertext, and this build has no result authentication.

The implemented surface is deliberately bounded:

- **Encrypted aggregation:** SUM, AVG and COUNT over encrypted columns.
- **Encrypted read-back:** stored ciphertexts returned to the client for local
  decryption.
- **Encrypted inference:** fixed-point dense/feed-forward layers over encrypted
  weights, including the XNOR demo and a real-dataset iris demo. Weight values
  are encrypted; bias values and layer shape are plaintext.
- **Transport hardening:** optional bearer-token auth and optional TLS, plus
  body/schema caps and a per-IP rate limiter (quota shared across connections).
  The limiter trusts the peer address, so a reverse proxy needs a trusted
  forwarded-for policy; see [docs/audit/README.md](docs/audit/README.md)
  finding F1 (fixed).

Filtering, joins, general SQL execution, SMPC, and GPU acceleration are **not**
implemented. See [PROJECT_STATUS.md](PROJECT_STATUS.md) for the measured
baseline, roadmap, and open gaps.

---

## Architectural Principles

* **TFHE over integer/fixed-point ciphertexts.** This build uses TFHE-rs only;
  it does not implement BGV or CKKS.
* **Columnar encrypted storage.** Homomorphic aggregation operates per column;
  row counts are public metadata.
* **Zero host value plaintext.** The server receives the public server key and
  ciphertext envelopes, never the client secret key or a plaintext field value.
  Metadata and the XNOR bias are outside this boundary.
* **Measured, not assumed.** Build, test, lint, format and demo results are
  recorded with the commands that produced them in
  [PROJECT_STATUS.md](PROJECT_STATUS.md).

---

## Repository Structure

```
.
├── apps/fhe-phone/     # Android-style webview shell (.exe) over an embedded server
├── crates/
│   ├── fhe-core/       # TFHE primitives, envelopes, algebra, neural layers
│   ├── fhe-db/         # Columnar encrypted database + WAL
│   ├── fhe-server/     # axum REST API
│   └── fhe-client/     # HTTP client SDK
├── examples/           # encrypted_sum, encrypted_nn, encrypted_iris
├── fuzz/               # cargo-fuzz targets for wire/snapshot decoding
├── scripts/            # benchmark regression helper
├── PROJECT_STATUS.md   # Authoritative status + measured baseline + roadmap
├── SECURITY.md         # Threat model, parameters, audit status
├── index.html          # Subdomain landing page
└── CNAME               # fhe.kellersystems.dev
```

---

## Quickstart

```bash
# Clone the repository
git clone https://github.com/KELLERBABG/Homomorphic-Database.git
cd Homomorphic-Database

# Build and test the workspace (release is the supported profile for FHE)
cargo build --release --workspace
cargo test --release --workspace

# End-to-end encrypted SUM / AVG / COUNT
cargo run --release -p encrypted_sum

# Encrypted XNOR inference with obfuscated weights
cargo run --release -p encrypted_nn

# Real-dataset encrypted inference (UCI iris)
cargo run --release -p encrypted_iris

# Android-style phone shell: embedded server + real client SDK + webview UI
cargo run --release -p fhe-phone
```

> **Status:** measured baseline recorded 2026-11-10. FHE-DB is **not
> production-ready**: an independent cryptographic audit and clean-machine
> packaging are still outstanding. See [PROJECT_STATUS.md](PROJECT_STATUS.md).

---

<div align="center">
  <sub>Maintained under the <b>Keller Systems</b> Sovereign Infrastructure Suite &bull; <a href="https://kellersystems.dev">kellersystems.dev</a></sub>
</div>
