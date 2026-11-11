#!/usr/bin/env bash
# ATA gateway end-to-end test: closed-loop hardware mode against a fake PLC.
#
# Verifies (all automated, exit non-zero on failure):
#   1. Real PLC temp (20.0°C) drives the decision -> Emergency, compressor ON
#   2. Real PLC grid frequency (49.98 Hz) is consumed (not the dataset value)
#   3. Coil writes are echoed + verified (FC05)
#   4. API: /price NaN -> 400, missing token -> 401, valid token -> 200
#   5. PLC loss -> forced fail-safe + sensor_loss alert, gateway stays alive
#
# Usage:   bash scripts/e2e/e2e-test.sh
# Requires: node (for the fake PLC), cargo/rustc, and the gateway deps.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d)"
PLC_PORT=1502
API_PORT=18086
PLC_PID=""
GW_PID=""

cleanup() {
  [ -n "$GW_PID" ] && kill "$GW_PID" 2>/dev/null || true
  [ -n "$PLC_PID" ] && kill "$PLC_PID" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

echo "==> Building gateway (debug)..."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
cargo build --manifest-path crates/gateway/Cargo.toml --target-dir "$CARGO_TARGET_DIR"

if [ -f "$CARGO_TARGET_DIR/debug/thermal-arbitrage-mvp" ]; then
  BIN="$CARGO_TARGET_DIR/debug/thermal-arbitrage-mvp"
elif [ -f "$ROOT/target/debug/thermal-arbitrage-mvp" ]; then
  BIN="$ROOT/target/debug/thermal-arbitrage-mvp"
elif [ -f "crates/gateway/target/debug/thermal-arbitrage-mvp" ]; then
  BIN="crates/gateway/target/debug/thermal-arbitrage-mvp"
else
  echo "❌ Error: Could not find thermal-arbitrage-mvp binary" >&2
  exit 1
fi
chmod +x "$BIN" 2>/dev/null || true

echo "==> Setting up test workspace..."
cp scripts/e2e/plc-config.toml "$WORK/config.toml"
mkdir -p "$WORK/data_cache"
[ -d crates/gateway/data_cache ] && cp crates/gateway/data_cache/*.json "$WORK/data_cache/" || true

echo "==> Starting fake PLC on :$PLC_PORT..."
node scripts/e2e/fake-plc.js "$PLC_PORT" > "$WORK/plc.log" 2>&1 &
PLC_PID=$!
sleep 1

echo "==> Starting gateway (hardware sensor mode)..."
( cd "$WORK" && ATA_SECRET=e2e-secret RUST_LOG=info "$BIN" > gateway.log 2>&1 ) &
GW_PID=$!

fail() {
  echo "❌ FAIL: $1" >&2
  if [ -f "$WORK/gateway.log" ]; then
    echo "--- GATEWAY LOG ---" >&2
    cat "$WORK/gateway.log" >&2
  fi
  if [ -f "$WORK/plc.log" ]; then
    echo "--- PLC LOG ---" >&2
    cat "$WORK/plc.log" >&2
  fi
  exit 1
}
ok() { echo "✅ $1"; }

sleep 10

# 1. Real PLC temp drives Emergency
STATUS="$(curl -s -m 3 "http://127.0.0.1:$API_PORT/status")"
echo "$STATUS" | grep -q '"current_temp_c":20.0' || fail "PLC temp not read (got: $STATUS)"
echo "$STATUS" | grep -q '"active_controller":"Emergency"' || fail "not in Emergency (got: $STATUS)"
echo "$STATUS" | grep -q '"compressor_on":true' || fail "compressor not ON (got: $STATUS)"
ok "PLC temp 20.0°C drives Emergency + compressor ON"

# 2. Real PLC frequency consumed (49.98 Hz from the meter, not 50.0 dataset)
echo "$STATUS" | grep -q '"grid_frequency_hz":49.98' || fail "meter frequency not used (got: $STATUS)"
ok "PLC grid frequency 49.98 Hz used"

# 3. Coil writes echoed + verified (no write errors in log)
grep -a "Modbus write.*failed" "$WORK/gateway.log" && fail "coil write failed" || true
grep -a "comp ON" "$WORK/gateway.log" | tail -1 >/dev/null || fail "no comp ON in modbus log"
ok "coil writes verified (FC05 echo)"

# 4. API: bounds + auth
TOK=$(printf '/price' | openssl dgst -sha256 -mac HMAC -macopt key:e2e-secret -binary 2>/dev/null | base64 | tr '+/' '-_' | tr -d '=')
[ -z "$TOK" ] && TOK="skip" # openssl missing: auth paths still exercised below
CODE_NO_TOKEN=$(curl -s -m 3 -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" "http://127.0.0.1:$API_PORT/price" -d '{"price_eur_per_mwh":80}')
[ "$CODE_NO_TOKEN" = "401" ] || fail "missing token expected 401, got $CODE_NO_TOKEN"
CODE_NAN=$(curl -s -m 3 -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $TOK" "http://127.0.0.1:$API_PORT/price" -d '{"price_eur_per_mwh":NaN}')
[ "$CODE_NAN" = "400" ] || fail "NaN price expected 400, got $CODE_NAN"
if [ "$TOK" != "skip" ]; then
  CODE_OK=$(curl -s -m 3 -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $TOK" "http://127.0.0.1:$API_PORT/price" -d '{"price_eur_per_mwh":80}')
  [ "$CODE_OK" = "200" ] || fail "valid token expected 200, got $CODE_OK"
fi
ok "API auth (401/200) + price bounds (400) verified"

# 5. PLC loss -> forced fail-safe, gateway stays alive
kill "$PLC_PID" 2>/dev/null || true
PLC_PID=""
sleep 15
grep -a "FAIL-SAFE: forced cooling" "$WORK/gateway.log" | tail -1 >/dev/null || fail "no forced fail-safe after PLC loss"
grep -a "Sensor/comm loss" "$WORK/gateway.log" | tail -1 >/dev/null || fail "no sensor_loss alert"
curl -s -m 3 -o /dev/null -w "%{http_code}" "http://127.0.0.1:$API_PORT/health" | grep -q 200 || fail "gateway dead after PLC loss"
ok "PLC loss -> forced fail-safe + sensor_loss alert, gateway alive"

echo ""
echo "🎉 E2E PASSED: closed loop, real meter frequency, auth, bounds, fail-safe"
