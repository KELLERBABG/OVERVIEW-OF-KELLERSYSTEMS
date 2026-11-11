#!/bin/bash
set -euo pipefail

echo "🏗️ Building Fungal Compute Swarm..."
pnpm install
pnpm build
echo "✅ Build complete"