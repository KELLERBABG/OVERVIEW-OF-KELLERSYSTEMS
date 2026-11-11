#!/bin/bash
set -euo pipefail

echo "🧪 Running all tests..."
pnpm test
echo "✅ All tests passed"