# Planetary-Entropy-Mining — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Python`
> **Summary:** A Python + Circom/JS environmental entropy harvesting and zk-randomness beacon platform (`dsp/capture.py`, `dsp/analysis.py`, `dsp/nist800_90b.py`, `dsp/validation.py`, `erasure/reedsolomon.py`, `hardening/gf256.py`, `circuits/scripts/entropy_poly_prove.mjs`, `cli/demo.py`, `cli/validate.py`). Implements NIST SP 800-90B min-entropy estimation and lattice-style polynomial hardening.

## Task Queue

- [ ] **TASK-01: Implement NIST SP 800-90B online health tests (RCT and APT) in dsp/nist800_90b.py**
  - **Target Files:** `dsp/nist800_90b.py`, `dsp/validation.py`
  - **Context Files:** `dsp/capture.py`, `tests/test_analysis.py`
  - **Verify:** `pytest tests/test_analysis.py -q`
  - **Spec:** Harden `dsp/nist800_90b.py` with continuous online health tests per NIST SP 800-90B Section 4.4.

1. Implement the Repetition Count Test (RCT): fail immediately if identical consecutive raw ADC samples exceed cutoff $C = 1 + \lceil -\log_2(W) / H_\infty \rceil$ (false positive probability $W = 2^{-20}$).
2. Implement the Adaptive Proportion Test (APT): within a rolling window of $W = 1024$ samples, fail if any sample value appears more than $C$ times.
3. If either test fails, flag `EntropyQuality::UnsafeNoiseFloor` and refuse to emit seeds.
4. Add unit tests in `tests/test_analysis.py` covering stuck-bit ADC faults and low-entropy harmonic inputs.

- [ ] **TASK-02: Harden GF(256) / polynomial lattice conditioning against adversarial bias**
  - **Target Files:** `hardening/gf256.py`, `cli/zk_witness.py`
  - **Context Files:** `erasure/reedsolomon.py`, `tests/test_certificate.py`
  - **Verify:** `pytest tests/test_certificate.py -q`
  - **Spec:** Enhance `hardening/gf256.py` and `cli/zk_witness.py` to ensure raw entropy is passed through a deterministic Leftover Hash Lemma / 2-universal hash extractor.

1. Ensure min-entropy extraction guarantees that even if an attacker controls up to $30\%$ of the raw noise bits (e.g. via RF carrier injection), the conditioned 256-bit output remains indistinguishable from uniform with statistical distance $\epsilon < 2^{-128}$.
2. Add unit tests in `tests/test_certificate.py`.

- [ ] **TASK-03: Verify end-to-end zk-SNARK randomness certificate generation and contract settlement**
  - **Target Files:** `contracts/settlement_sim.py`, `cli/e2e.py`
  - **Context Files:** `cli/validate.py`, `tests/test_darkpool.py`
  - **Verify:** `pytest tests/test_darkpool.py -q`
  - **Spec:** Wire `contracts/settlement_sim.py` and `cli/e2e.py` so that a miner's randomness submission must provide a valid zero-knowledge proof verifying that the claimed entropy certificate was derived from a valid NIST-compliant sample window without disclosing the underlying ambient seed bytes.

1. Add end-to-end integration tests proving valid payouts for compliant entropy and zero payout for failing proofs.
