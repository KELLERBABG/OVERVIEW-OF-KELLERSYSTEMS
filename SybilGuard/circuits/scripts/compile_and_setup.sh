#!/usr/bin/env bash
# Phase 2 zk-SNARK circuit compilation and trusted setup script.
# Requires: circom, snarkjs, node, bash
# 
# Usage: bash circuits/scripts/compile_and_setup.sh

set -euo pipefail

echo "=== Sybil-Resistentes zk-SNARK Circuit Compilation ==="

CIRCUIT_DIR="circuits"
BUILD_DIR="${CIRCUIT_DIR}/build"
CIRCUIT_FILE="${CIRCUIT_DIR}/cluster_membership.circom"
CIRCUIT_NAME="cluster_membership"

mkdir -p "${BUILD_DIR}"

# 1. Compile the circuit
echo ""
echo "→ Compiling circuit with circom..."
circom "${CIRCUIT_FILE}" --r1cs --wasm --sym --output "${BUILD_DIR}" 2>&1

echo "  ✓ Circuit compiled: ${BUILD_DIR}/${CIRCUIT_NAME}.r1cs"
echo "  ✓ WASM generated: ${BUILD_DIR}/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm"

# 2. Print circuit statistics
echo ""
echo "→ Circuit statistics:"
snarkjs r1cs info "${BUILD_DIR}/${CIRCUIT_NAME}.r1cs" 2>&1

# 3. Powers of Tau ceremony (POT) — Phase 1
# In production this would be a multi-party ceremony.
# For dev/test, we generate locally.
echo ""
echo "→ Generating Powers of Tau (testing only!)..."
snarkjs powersoftau new bn128 12 "${BUILD_DIR}/pot12.ptau" -v 2>&1 | tail -3
snarkjs powersoftau contribute "${BUILD_DIR}/pot12.ptau" "${BUILD_DIR}/pot12_final.ptau" --name="dev contribution" -v 2>&1 | tail -3

# 4. Phase 2: Circuit-specific setup
echo ""
echo "→ Generating circuit-specific setup..."
snarkjs groth16 setup "${BUILD_DIR}/${CIRCUIT_NAME}.r1cs" "${BUILD_DIR}/pot12_final.ptau" "${BUILD_DIR}/${CIRCUIT_NAME}_0000.zkey" 2>&1 | tail -3

# 5. Contribute to Phase 2 (in production this would be multi-party)
echo ""
echo "→ Contributing to Phase 2..."
snarkjs zkey contribute "${BUILD_DIR}/${CIRCUIT_NAME}_0000.zkey" "${BUILD_DIR}/${CIRCUIT_NAME}_final.zkey" --name="dev" -v 2>&1 | tail -3

# 6. Export verification key
echo ""
echo "→ Exporting verification key..."
snarkjs zkey export verificationkey "${BUILD_DIR}/${CIRCUIT_NAME}_final.zkey" "${BUILD_DIR}/verification_key.json"

# 7. Export Solidity verifier (for on-chain verification)
echo ""
echo "→ Exporting Solidity verifier..."
snarkjs zkey export solidityverifier "${BUILD_DIR}/${CIRCUIT_NAME}_final.zkey" "${BUILD_DIR}/verifier.sol"

echo ""
echo "=== Setup complete ==="
echo "  Verification key:  ${BUILD_DIR}/verification_key.json"
echo "  Solidity verifier: ${BUILD_DIR}/verifier.sol"
echo "  Final zkey:        ${BUILD_DIR}/${CIRCUIT_NAME}_final.zkey"
echo ""
echo "To generate a proof:"
echo "  snarkjs groth16 prove ${BUILD_DIR}/${CIRCUIT_NAME}_final.zkey ${BUILD_DIR}/witness.wtns ${BUILD_DIR}/proof.json ${BUILD_DIR}/public.json"
echo ""
echo "To verify a proof:"
echo "  snarkjs groth16 verify ${BUILD_DIR}/verification_key.json ${BUILD_DIR}/public.json ${BUILD_DIR}/proof.json"