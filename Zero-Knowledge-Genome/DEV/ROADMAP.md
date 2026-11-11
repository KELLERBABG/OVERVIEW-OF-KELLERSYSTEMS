# Zero-Knowledge-Genome — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 10-crate Rust workspace (`zk-genome/core/genome-core`, `genome-parser`, `genome-compress`, `genome-annotate`, `entropy-auditor`, `query-engine`, `storage-traits`, `zk-core`, `api`) implementing on-device genetic sequence compression, 23andMe/VCF parsing, encrypted query matching, and zero-knowledge phenotypic trait proofs.

## Task Queue

- [ ] **TASK-01: Harden zk-SNARK genomic trait proofs and Merkle chromosome commitment in zk-core**
  - **Target Files:** `zk-genome/core/zk-core/src/lib.rs`, `zk-genome/api/src/verify.rs`
  - **Context Files:** `zk-genome/core/genome-core/src/lib.rs`, `zk-genome/api/src/models/mod.rs`
  - **Verify:** `cargo test --manifest-path zk-genome/Cargo.toml --workspace --quiet`
  - **Spec:** Harden `zk-genome/core/zk-core/src/lib.rs` and `api/src/verify.rs` so that a user proves possession of a specific single-nucleotide polymorphism (SNP) via Merkle inclusion proof against a verified reference assembly.

1. The proof must reveal only `(nullifier, target_snp_rsid, presence_bit)` while keeping all other chromosomal positions and patient identity concealed.
2. Reject any proof whose Merkle root does not match the signed laboratory sequencing attestation.
3. Add unit tests in `zk-core` verifying valid proof generation and rejection of forged SNP assertions.

- [ ] **TASK-02: Implement differential privacy noise injection on pharmaceutical query aggregation**
  - **Target Files:** `zk-genome/core/query-engine/src/lib.rs`, `zk-genome/api/src/routes/queries.rs`
  - **Context Files:** `zk-genome/core/genome-core/src/lib.rs`
  - **Verify:** `cargo test --manifest-path zk-genome/Cargo.toml --workspace --quiet`
  - **Spec:** Update `zk-genome/core/query-engine/src/lib.rs` and `api/src/routes/queries.rs` so that pharmaceutical research queries querying population counts inject Laplace noise calibrated to global sensitivity $\Delta f = 1$ and privacy budget $\epsilon = 0.5$.

1. Enforce a minimum cohort size ($N_{\min} \ge 50$); queries matching fewer individuals return an empty/error result to prevent re-identification of rare genetic diseases.
2. Add unit tests verifying differential privacy noise and small-cohort blocking.
