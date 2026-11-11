#!/usr/bin/env bash
#
# start-cluster.sh
#
# Starts the SMPC Dark Pool cluster with Docker Compose.
# Builds and starts all 5 MPC nodes plus a Ganache Ethereum testnet.
#
# Usage:
#   ./scripts/start-cluster.sh
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "Starting SMPC Dark Pool cluster..."
echo "Nodes: node0, node1, node2, node3, node4"
echo "Ethereum testnet: ganache (chain ID 1337)"
echo ""

docker compose up --build "$@"
