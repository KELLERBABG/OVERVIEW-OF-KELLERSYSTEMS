<div align="center">

# &#9878;&#65039; FluidIP

**Decoupled Fluid Intellectual Property & Formal Patent Routing Network**

<br>

[![Release](https://img.shields.io/badge/Release-v0.1.0-2563eb?style=flat-square)](https://github.com/KELLERBABG/FluidIP/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-059669?style=flat-square)](LICENSE)
[![Language](https://img.shields.io/badge/Language-Rust%20(Pure)-orange?style=flat-square)](https://www.rust-lang.org/)
[![Runtime](https://img.shields.io/badge/Runtime-Tokio%20Async-blue?style=flat-square)](https://tokio.rs/)
[![Verification](https://img.shields.io/badge/Verification-SMT%20%7C%20Z3-purple?style=flat-square)](WHITEPAPER.md)

<br>

[**Protocol Whitepaper**](WHITEPAPER.md) &bull; [**Step-by-Step Architecture**](Step-by-Step.txt) &bull; [**Live Interface**](https://patent.kellersystems.dev) &bull; [**API Server**](api/)

</div>

---

## What is FluidIP?

**FluidIP** (Decoupled Fluid IP Network) is an autonomous, trustless intellectual property management and verification protocol written in Rust. It replaces ambiguous natural-language patent text with machine-verifiable formal mathematical specifications (SMT/FOL) and continuous dependency graphs.

By decoupling patent claims into directed acyclic dependency graphs (DAGs), FluidIP automatically tracks IP lineage, resolves claim infringement mathematically using SMT solvers, and executes automated micro-royalty attribution via smart contracts.

---

## Why Use It?

* **Machine-Verifiable Patent Claims:** Transforms subjective prose into First-Order Logic (FOL) formulas, verified via automated Z3 SMT solvers to prove novelty and detect prior art without human litigation bias.
* **Continuous Lineage Graphs:** Every invention is modeled as a verified node in an immutable IP graph. Derivative innovations automatically attribute micro-royalties to foundational claim owners through verifiable flow graphs.
* **Taint Tracking & Zero-Knowledge Disclosure:** Inspect potential code or schematic infringements via homomorphic taint tracking without revealing raw source blueprints to untrusted counterparties.
* **Sub-Second Claim Resolution:** High-throughput Rust REST API and CLI toolchain for instant claim registration, verification, and license clearing.

---

## Architecture Overview

```
FluidIP Ecosystem
├── api/                # High-throughput Axum/Tokio HTTP REST API daemon
├── cli/                # Command-line interface for claim compilation & verification
├── core/               # Core domain models: PatentGraph, Node, Edge, Claim
├── contracts/          # Smart contracts for on-chain royalty escrow & registry
├── formal-verifier/    # SMT/Z3 claim consistency & novelty validation engine
├── taint-tracking/     # Information flow analysis & dependency taint auditor
├── web/                # Frontend dashboard & interactive patent graph explorer
└── WHITEPAPER.md       # Complete formal protocol specification
```

---

## Quickstart

### Prerequisites
* Rust toolchain (1.75+)
* Z3 Theorem Prover (4.12+)

### Building and Running

```bash
git clone https://github.com/KELLERBABG/FluidIP.git && cd FluidIP && cargo build --release
```

Run the API service:
```bash
cargo run --release -p fluidip-api
```

Run formal claim verifications across local claims:
```bash
cargo test --workspace
```

---

## Protocol Verification & Whitepaper

For deep mathematical formulations, SMT encodings, and economic payout algorithms, see [WHITEPAPER.md](WHITEPAPER.md).

---

<div align="center">
&copy; 2026 Keller Systems &bull; Built for Sovereign Machine Verification
</div>
