# Format Freeze — external audit scope

**Frozen:** 2026-11-10, at the start of external-audit outreach (roadmap C5).
**Applies to:** the worktree described in [`EVIDENCE.md`](EVIDENCE.md), not to a
future commit until that commit re-runs the manifest.

An external reviewer must review a **fixed** surface. If the envelope frame,
the persisted log header, the parameter set, or the golden corpus drifts while
the review is open, the findings no longer describe the code being shipped and
the reviewer has to start over. This document names exactly what is frozen, how
the freeze is enforced, and how to change it deliberately.

---

## 1. What is frozen

| Surface | Frozen value | Where it lives | Enforcement |
|---|---|---|---|
| Ciphertext envelope version | `1` | `crates/fhe-core/src/ciphertext.rs` (`CURRENT_ENVELOPE_VERSION`) | `crates/fhe-core/tests/format_freeze.rs::envelope_version_is_frozen_at_v1` |
| Envelope frame | bincode `{ version: u8, width: ScalarWidth, payload: Vec<u8> }`, with a hand-parsed header validated (and payload lengths bounded) before the bincode decode | `ciphertext.rs` (`CiphertextEnvelope`, `parse_envelope_header`, `check_payload_lengths`) | Golden-corpus bytes + `golden_v1_envelope_decrypts_and_matches_current_version` |
| Golden corpus bytes | `v1_client_key.bin` = 31,267 bytes, SHA-256 `5cf6ff9d…a62b20`; `v1_envelope_u32_42.bin` = 263,465 bytes, SHA-256 `15054892…133d8` | `crates/fhe-core/tests/golden/` | `crates/fhe-core/tests/format_freeze.rs::golden_corpus_bytes_are_frozen`; full hashes in [`evidence.sha256`](evidence.sha256) |
| WAL magic / version / header | magic `FHELOG01`, version `1`, header 24 bytes (magic `u64`, version `u64`, snapshot length `u64`) | `crates/fhe-db/src/wal.rs` (`CURRENT_LOG_MAGIC`, `CURRENT_LOG_VERSION`, `LOG_HEADER_LEN`) | `crates/fhe-db/tests/format_freeze.rs` (constants and a fresh-log header byte check) |
| WAL record frame | `u32` little-endian length + bincode payload + `u32` CRC-32 (IEEE) | `wal.rs` (`Wal::append`, `scan_records`) | WAL unit tests (`torn_tail_is_truncated`, `corrupt_complete_record_is_rejected`) |
| TFHE parameter set | TFHE-rs 1.7.0 `PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128`: LWE 918, GLWE 1, polynomial 2048, PBS 2^23/1, KS 2^4/4, message/carry 4/4, max noise 5, `log2 p_fail = -129.581` | `crates/fhe-core/src/keys.rs` (`SecurityParameters::pinned_default`, `ClientKey::new`) | `format_freeze.rs::documented_security_parameters_are_frozen`; `default_parameters_match_documented_pinning` |
| Vendored TFHE-rs source | crates.io `tfhe 1.7.0` (SHA-256 of the `.crate` archive `f341a7a6…8da52`), unmodified except `Cargo.toml` `crate-type = ["lib"]` | `vendor/tfhe/`, `[patch.crates-io]` in the workspace `Cargo.toml` | [`EVIDENCE.md` §3](EVIDENCE.md#3-tfhe-rs-patch-verification) — re-runnable diff |
| HTTP error codes | the stable `code` strings listed in `AGENTS.md` §7 / `crates/fhe-server/src/error.rs` | `crates/fhe-server/src/error.rs` | `error_bodies_carry_stable_codes` |

The WAL header is not cryptographic, but it is part of the persistence-parsing
surface the reviewer inspects for memory-safety and corruption handling, so it
is frozen with the rest.

---

## 2. How the freeze is enforced

`cargo test --release --workspace` runs the freeze gates; CI runs the same
command, so a change cannot merge silently.

- `crates/fhe-core/tests/format_freeze.rs` — version, full golden-corpus
  SHA-256 + lengths, and every pinned parameter value.
- `crates/fhe-db/tests/format_freeze.rs` — log constants plus a byte-level
  check of a freshly created header.
- `crates/fhe-core/tests/golden_corpus.rs` — decrypts the committed envelope
  with the committed key and asserts `42`; a version bump without a corpus
  update fails here too.

---

## 3. Changing a frozen surface (deliberate procedure)

1. Write down the reason and the migration story: what old artifacts exist,
   how they are detected, and whether they are rejected or migrated.
2. Bump the version constant (`CURRENT_ENVELOPE_VERSION` and/or
   `CURRENT_LOG_VERSION`) — never change the frame while keeping the version.
3. Regenerate the golden corpus
   (`cargo test --release -p fhe-core --test golden_corpus
   regenerate_golden_corpus -- --ignored`) and derive the new hashes.
4. Update `format_freeze.rs` constants, this document (with date and reason),
   and [`evidence.sha256`](evidence.sha256) via
   `docs/audit/refresh_evidence.sh`.
5. Run the full quality gate and record the new numbers in
   `PROJECT_STATUS.md`.
6. Tell the reviewer before the change, not after. If a review is running,
   freeze the change until it closes.

---

## 4. What is *not* frozen

- Additive HTTP API changes (new endpoints, new fields on responses) that do
  not change existing paths, status codes, or error-code strings.
- Internal refactors that provably do not change the bytes above (for example
  moving the CRC-32 helper to another module).
- Documentation, tests, comments, and the `/metrics` counter set.
- Performance work inside `fhe-core`/`fhe-db` that keeps ciphertexts and log
  records byte-identical.

Changes in these areas still follow the normal quality gate; they just do not
require a version bump or an auditor notification.
