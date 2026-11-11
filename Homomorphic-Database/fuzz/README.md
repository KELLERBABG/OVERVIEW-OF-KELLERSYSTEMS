# Fuzzing (cargo-fuzz)

The `fuzz/` crate holds libFuzzer targets for the untrusted deserialization
paths:

- `fuzz_envelope` — `fhe_core::Ciphertext::from_envelope_bytes` (the wire
  entry point for every ciphertext a client uploads). Invariant: arbitrary
  bytes yield a valid ciphertext or a typed error — never a panic, never an
  absurd allocation (length-prefix bombs).
- `fuzz_db_snapshot` — `fhe_db::EncryptedDatabase::from_bytes` (boot-time
  snapshot/WAL recovery). Invariant: corrupt bytes yield
  `DbError::CorruptedDatabase`, never a panic or an absurd allocation.

## Requirements

libFuzzer needs a C toolchain (`clang`), so fuzzing runs on CI (ubuntu) or
any machine with clang installed — not on this repo's Windows-GNU build host
(no C compiler). The crate is its own workspace (`[workspace]` in
`fuzz/Cargo.toml`) so `cargo build --workspace` / `cargo test --workspace`
from the root are unaffected.

## Running

```bash
cargo install cargo-fuzz --locked
cargo fuzz build                    # compile all targets (first run builds tfhe: 10+ min)
cargo fuzz run fuzz_envelope        # run forever; Ctrl-C to stop
cargo fuzz run fuzz_envelope -- -runs=50000 -max_len=4096   # bounded smoke run
cargo fuzz run fuzz_db_snapshot -- -runs=20000 -max_len=8192
```

Any crash is written to `fuzz/artifacts/`; minimize it with
`cargo fuzz tmin <target> <artifact>` and add a regression test.

## Relationship to unit tests

The same invariants are pinned on machines without clang by:

- `fhe_core::tests::envelope_rejects_length_prefix_bomb` and
  `envelope_rejects_oversized_claim` — the specific bomb class this
  hardening fixes.
- `fhe_core/tests/properties.rs::envelope_deserialization_never_panics` —
  proptest over arbitrary 0..4096-byte inputs.
