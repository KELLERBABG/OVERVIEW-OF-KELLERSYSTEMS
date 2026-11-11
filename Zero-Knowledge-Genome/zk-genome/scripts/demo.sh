#!/usr/bin/env bash
# Run the ZK Genome end-to-end demo against the bundled sample VCF.
# Exits non-zero if `cargo run -p zk-genome-cli` does.
set -euo pipefail

cd "$(dirname "$0")/.."

SAMPLE="${SAMPLE:-data/samples/example.vcf}"

echo
echo "================================================="
echo "Zero-Knowledge Genome - end-to-end CLI demo"
echo "================================================="
echo

if [[ ! -f "$SAMPLE" ]]; then
  echo "Sample VCF missing: $SAMPLE" >&2
  echo "Falling back to: cargo run -p zk-genome-cli -- demo" >&2
  exec cargo run -p zk-genome-cli -- demo
fi

echo "1) Demo with no VCF (synthetic rs53576 / rs4680 / rs1800497):"
cargo run -q -p zk-genome-cli -- demo
echo

echo "2) Pipeline on $SAMPLE:"
cargo run -q -p zk-genome-cli -- pipeline "$SAMPLE"
echo

echo "3) Stats on $SAMPLE:"
cargo run -q -p zk-genome-cli -- stats "$SAMPLE"
echo

echo "4) Stub proofs for rs53576, rs4680, rs99999999 (last one will miss):"
cargo run -q -p zk-genome-cli -- prove-rsids "$SAMPLE" "rs53576,rs4680,rs99999999"
echo

echo "All done."
