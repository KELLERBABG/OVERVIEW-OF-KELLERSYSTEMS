# ADR-003: zk-SNARKs (Groth16) for Proof System

## Status
Accepted

## Context
We need cryptographic proof of correct computation. Options: zk-SNARKs (small proofs, fast verification) or zk-STARKs (larger proofs, no trusted setup).

## Decision
Use Groth16 zk-SNARKs via snarkjs. Proofs are small (~200 bytes) and verify in milliseconds. Use a mock/fallback for environments without snarkjs.

## Consequences
- Requires trusted setup (Powers of Tau ceremony)
- Proofs are cheap to verify, expensive to generate
- Well-suited for browser-based proving/verification