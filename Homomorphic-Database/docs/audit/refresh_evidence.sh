#!/usr/bin/env bash
# Regenerate docs/audit/evidence.sha256 for the frozen artifacts.
#
# Run from anywhere; the script resolves the repository root itself. The
# resulting file is what an auditor checks with:
#
#   sha256sum -c docs/audit/evidence.sha256
#
# Only artifacts whose bytes are frozen for the review belong here. Do not add
# ordinary source files: their hashes go stale on every edit and turn the
# manifest into noise. The TFHE-rs `.crate` archive is fetched from crates.io
# rather than stored in the repository; its hash is recorded in EVIDENCE.md.
set -euo pipefail
cd "$(dirname "$0")/../.."

sha256sum \
  Cargo.lock \
  crates/fhe-core/tests/golden/v1_client_key.bin \
  crates/fhe-core/tests/golden/v1_envelope_u32_42.bin \
  vendor/tfhe/Cargo.toml \
  vendor/tfhe/Cargo.toml.orig \
  vendor/tfhe/.cargo_vcs_info.json \
  > docs/audit/evidence.sha256

echo "wrote docs/audit/evidence.sha256:"
cat docs/audit/evidence.sha256
