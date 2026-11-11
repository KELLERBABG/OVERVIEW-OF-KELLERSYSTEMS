# TFHE Parameter Dossier

**Scope:** the parameter set used by `fhe_core::ClientKey::new`, what the
repository actually pins and tests, and which security statements are vendor
claims rather than independent evidence. **Status:** internal review,
2026-11-10.

## 1. Provenance

| Item | Value | Evidence |
|---|---|---|
| TFHE-rs version | `1.7.0` | `Cargo.lock:3626-3627` |
| Integration | `tfhe` is a workspace dependency of `fhe-core` only | `crates/fhe-core/Cargo.toml`; `cargo tree -i tfhe` |
| Vendor patch | `vendor/tfhe` 1.7.0 source with `crate-type = ["lib"]` (removes the impossible PE cdylib) | workspace `Cargo.toml` `[patch.crates-io]` |
| Config used | `tfhe::ConfigBuilder::default().build()` | `crates/fhe-core/src/keys.rs:39-41` |
| Resolved shortint set | `PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128` | `crates/fhe-core/src/keys.rs:148-161`; vendor `.../v1_4/.../ks_pbs.rs:29-43` |
| Project pin test | `default_parameters_match_documented_pinning` | `crates/fhe-core/src/keys.rs:289-345` |

The vendor source defines the set as:

| Field | Value |
|---|---|
| `lwe_dimension` | 918 |
| `glwe_dimension` | 1 |
| `polynomial_size` | 2048 |
| `lwe_noise_distribution` | t-uniform, 45 |
| `glwe_noise_distribution` | t-uniform, 17 |
| `pbs_base_log` / `pbs_level` | 23 / 1 |
| `ks_base_log` / `ks_level` | 4 / 4 |
| `message_modulus` / `carry_modulus` | 4 / 4 |
| `max_noise_level` | 5 |
| `log2_p_fail` | −129.581 |
| encryption key choice | `Big` |
| modulus-switch reduction | centered mean noise reduction |

Source: `vendor/tfhe/src/shortint/parameters/v1_4/classic/tuniform/p_fail_2_minus_128/ks_pbs.rs:29-43`.

## 2. What `log2_p_fail` means

`log2_p_fail = −129.581` means TFHE-rs estimates the probability of a
**correctness failure** — a homomorphic evaluation whose noise exceeds the
budget, so decryption no longer returns the intended value — at about
`2^-129.581` for the modeled operation. It is a property of the noise analysis
and the modeled circuit, not an adversary's distinguishing advantage.

The 128-bit **IND-CPA^D** label comes from TFHE-rs naming and documentation.
TFHE-rs states:

> "By default, the parameter sets used in the High-Level API have a failure
> probability ≤ 2^-128 to securely work in the IND-CPA^D model using the
> algorithmic techniques provided in our code base."

and says its security estimations use the
[Lattice Estimator](https://github.com/malb/lattice-estimator) with
`red_cost_model = reduction.RC.BDGL16` (`vendor/tfhe/README.md:198-208`).
That is a vendor methodology statement, not a proof included in this
repository.

**Do not read "2M128" or "p_fail = 2^-129.581" as "this project proved
128-bit security."** The project pins the values; it does not reproduce the
lattice estimate, model the adversary, or hold an independent audit.

## 3. What this repository proves

| Proposition | Evidence | Strength |
|---|---|---|
| A fresh default key resolves to the documented parameter values. | Gate test `default_parameters_match_documented_pinning` (`keys.rs:289-345`) | Strong regression guard for value drift. |
| Encryption, envelope round-trips, and homomorphic SUM/AVG/COUNT work on the tested inputs. | `fhe-core`, `fhe-db`, `fhe-server`, `encrypted_sum` tests and demo | Functional evidence only. |
| The default key's `log2_p_fail` equals the pinned constant. | Same gate test, float tolerance `1e-3` | Pins the library value; does not validate it. |
| Only `fhe-core` depends on `tfhe`. | `cargo tree -i tfhe --workspace --edges normal` | Boundary holds. |

## 4. What this repository does not prove

- That the Lattice Estimator result for these parameters is correct, or that
  BDGL16 is the right cost model for the intended adversary.
- That the IND-CPA^D reduction applies to this composition (key switching,
  PBS, modulus switching, and the high-level integer API), not just the
  shortint parameter set.
- That the vendored `crate-type` patch is behavior-preserving in every build
  configuration.
- That the implementation is free of side-channel leakage. TFHE-rs's README
  says: "Mitigation for side-channel attacks has not yet been implemented in
  TFHE-rs" (`vendor/tfhe/README.md:215-219`). This project adds no
  constant-time key handling, memory locking, or zeroization.
- That the noise budget survives the documented maximum workload. The
  aggregation cap is 1,000,000 rows (`crates/fhe-server/src/routes.rs:92`),
  but the tests aggregate small columns; there is no 1M-row decryption test,
  no noise-budget measurement, and no bootstrapping in the aggregation path.
  The cap is a DoS guard, not a proven correctness bound.
- That the XNOR/Q15.0 inference path stays within the signed range for
  arbitrary weights or deeper networks; the documented exactness constraint is
  Q15.0 only (`crates/fhe-core/src/nn/mod.rs`).
