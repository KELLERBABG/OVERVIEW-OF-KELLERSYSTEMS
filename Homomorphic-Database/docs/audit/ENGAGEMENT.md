# External Audit — engagement plan (roadmap C5)

**Status:** ready to send. **No reviewer has been engaged or quoted.** Sending
the request, signing an NDA/contract, and paying for the review are owner
actions; everything the reviewer needs is prepared here and in
[`EVIDENCE.md`](EVIDENCE.md).

This document exists so the engagement does not stall on missing scope,
missing evidence, or an unclear ask. It is deliberately narrow: the review is
of the cryptography and its integration, not of the phone UI or the business
model.

---

## 1. Objective

Obtain an independent written assessment of:

1. the security of the pinned TFHE parameter set as used here, and the validity
   of the "128-bit IND-CPA^D" claim for this deployment;
2. the correctness and safety of the TFHE-rs integration — key handling,
   encryption, ciphertext algebra, envelope framing/parsing, and the vendored
   patch;
3. the client-side key lifecycle as implemented (generation, storage, backup,
   restore, rotation, destruction), including the F4/F8 regressions and their
documented residuals;
4. whether the documented threat model and security claims match the code.

The review is **not** a general penetration test and **not** a certification.
A clean report does not make the project production-ready; it is the gate that
lets the remaining release work proceed honestly.

---

## 2. Scope of work

### In scope

| Area | What to review | Entry point |
|---|---|---|
| Parameters | TFHE-rs 1.7.0 `PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128`: LWE 918 / GLWE 1 / N 2048, 2-bit messages, `p_fail = 2^-129.58`. Reproduce or challenge the Lattice-Estimator result and the IND-CPA^D label. | `crates/fhe-core/src/keys.rs`; `vendor/tfhe/src/shortint/parameters/.../ks_pbs.rs` |
| TFHE-rs integration | Key generation, client/server key split, encryption, homomorphic add/mul/sum/average, `ServerKeyGuard` lifecycle. | `crates/fhe-core/src/{keys,encrypt,algebra}.rs` |
| Wire format | Envelope v1 framing and parsing, length-prefix bounds, version/width handling, deserialization DoS. | `crates/fhe-core/src/ciphertext.rs`; `fuzz/` |
| Persistence parsing | WAL v1 header/record framing, CRC handling, torn-tail truncation, replay validation (memory safety and corruption behaviour, not cryptographic confidentiality). | `crates/fhe-db/src/wal.rs` |
| Key lifecycle | Phone keystore: AES-256-GCM, Argon2id parameters, PIN policy, atomicity, backup/restore, destruction. | `apps/fhe-phone/src/keystore.rs`; `docs/audit/KEY_LIFECYCLE.md` |
| Claims and threat model | Whether `SECURITY.md`, `README.md`, `docs/audit/THREAT_MODEL.md`, and the rustdoc match the implementation; flag overclaims. | `docs/audit/CLAIMS.md` |

### Out of scope (separate engagements)

- UI/UX, installer, code signing, clean-machine packaging (stage 4).
- Network/TLS deployment hardening beyond noting it (the TLS feature is
  CI-built; the default posture is loopback).
- Authorization/tenancy design (F9 is accepted only for the single-user local
  vault; [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md)).
- Performance, benchmark methodology, GPU/SMPC/cloud work.
- Formal verification or a proof of the scheme. Parameter analysis is in scope;
  a new security proof is not.

### Explicitly disclosed limitations

The reviewer is told up front, in the evidence pack, that:

- the "128-bit" label is a **vendor claim**, not a project proof (F5);
- computation results are **not authenticated** — a malicious server can
  return a wrong ciphertext that decrypts without detection;
- side-channel mitigations are **not implemented** (F7), and offline guessing
  against the phone PIN remains possible (F4 residual);
- schema, row counts, batch sizes, and the XNOR bias are **plaintext
  metadata** (F3/F6);
- the server has **no authorization model**; F9 is accepted only for the
  single-user local-vault scope recorded in [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md).

A review that confirms these are honestly disclosed is as valuable as one that
finds something new.

---

## 3. Deliverables required from the reviewer

1. Written report: finding, severity, affected file/line, exploit scenario,
   and remediation guidance.
2. Parameter analysis: the estimator version/configuration used, the claimed
   security level, and any discrepancy with the documented `p_fail`.
3. A statement on each disclosed limitation in §2: confirmed, overstated, or
   understated.
4. Retest of fixes within an agreed window (price this separately).
5. An attestation letter naming the reviewed commit/artifact hashes, suitable
   for publishing next to the release.
6. Permission to publish the report (or a summary) — preferred, not mandatory.

---

## 4. Request for quote — template

Send with [`EVIDENCE.md`](EVIDENCE.md), [`FORMAT_FREEZE.md`](FORMAT_FREEZE.md),
`SECURITY.md`, and repository access. Ask for:

1. Confirmation that the §2 scope is in your practice and any suggested
   changes to it.
2. Fixed price and calendar duration, split into (a) parameter analysis,
   (b) implementation review, (c) key-lifecycle review, (d) retest.
3. The named reviewers and their relevant lattice/FHE work.
4. Which estimator/tooling you use for LWE parameters, and whether you will
   reproduce the `2^-129.58` figure independently.
5. Your standard report format and two public sample reports.
6. Assumptions: repository access method, NDA requirements, communication
   cadence, and what you need from us during the review.
7. Retest policy and price, and the validity window of the quote.

Useful context to include in the request:

> FHE-DB is a local-first encrypted database built on TFHE-rs 1.7.0. The
> server stores and aggregates ciphertexts and never holds the client key. The
> review covers the pinned parameter set, the TFHE-rs integration and vendored
> patch, the v1 envelope and WAL parsers, and the client key lifecycle. The
> repository is pre-release, has no external audit, and does not claim
> production readiness. The internal audit package, threat model, claim review,
> and a reproducible evidence pack are ready and will be sent with the
> repository.

---

## 5. Suggested outreach set

These firms publicly describe cryptography practices that include FHE or
lattice-based review. **None has been contacted, quoted, or vetted by this
project** — verify scope, references, and availability yourself, and get at
least three quotes.

| Firm | Relevant public focus | Link |
|---|---|---|
| zkSecurity | Cryptography audits including FHE, MPC, and post-quantum; cryptographer-staffed | <https://zksecurity.xyz> |
| Trail of Bits | Cryptography practice: protocols, primitives, implementations, post-quantum | <https://trailofbits.com/services/cryptography> |
| Least Authority | Cryptographic protocol and privacy-system audits; public reports | <https://leastauthority.com> |
| NCC Group — Cryptography Services | Cryptographic design and implementation review at enterprise scale | <https://www.nccgroup.com> |
| Kudelski Security | Cryptography audits and quantum-readiness assessments | <https://kudelskisecurity.com> |
| Hexens | Publishes an explicit FHE implementation review service (TFHE/BGV/BFV/CKKS) | <https://hexens.io/services/cryptography-security> |

Selection criteria, in order: demonstrated lattice/FHE work with named
reviewers → willingness to reproduce the parameter estimate → fixed price and
timeline → retest policy → publication permission.

---

## 6. Parallel work while the review is open

The review has the longest lead time; it does not block everything else.

| Work | Constraint |
|---|---|
| Phone keystore D7/D8 (Argon2 pinning, atomic PIN change, attempt limiting, zeroization) | Changes `apps/fhe-phone`, not the frozen cryptographic surface. Safe to do in parallel. |
| Stage-4 packaging (installer, signing, WebView2, clean machine) | Independent of the review. Safe to do in parallel. |
| Any envelope, parameter, key-format, or WAL-format change | **Frozen** — see [`FORMAT_FREEZE.md`](FORMAT_FREEZE.md). Do not change during the review. |
| Fixes for findings | Land after the report, then request the retest. Keep each fix on its own commit so the retest can be scoped. |

---

## 7. Engagement log (owner fills in)

| Date | Firm | Action | Outcome / next step |
|---|---|---|---|
| | | Request sent | |
| | | Quote received | |
| | | Selected | |
| | | NDA signed | |
| | | Review started (commit hash: ______) | |
| | | Report received | |
| | | Fixes landed / retest requested | |
| | | Attestation received | |

When a review starts, record the reviewed commit hash and the
`docs/audit/evidence.sha256` hash in the log. That pair is what the attestation
must name.
