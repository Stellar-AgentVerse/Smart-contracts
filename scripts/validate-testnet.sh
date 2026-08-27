#!/usr/bin/env bash
# Deploys fresh Testnet contracts and validates the security-critical flows.
# This script is intentionally Testnet-only and never reuses contract IDs.

set -euo pipefail

NETWORK="${NETWORK:-}"
DEPLOYER_SOURCE="${VALIDATION_DEPLOYER_SOURCE:-}"
ADMIN_SOURCE="${VALIDATION_ADMIN_SOURCE:-}"
BUYER_SOURCE="${VALIDATION_BUYER_SOURCE:-}"
OUT_DIR="${VALIDATION_OUT_DIR:-./deploy-artifacts}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-$$"

fail() {
  echo "❌ $*" >&2
  exit 1
}

pass() {
  echo "✅ $*"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing command: $1"
}

require_value() {
  [[ -n "$1" ]] || fail "Missing required value: $2"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    fail "Install sha256sum or shasum"
  fi
}

invoke() {
  local contract_id="$1"
  local source="$2"
  shift 2
  stellar contract invoke \
    --id "$contract_id" \
    --source "$source" \
    --network "$NETWORK" \
    --send=yes \
    -- "$@"
}

read_contract() {
  local contract_id="$1"
  shift
  stellar contract invoke \
    --id "$contract_id" \
    --source "$ADMIN_SOURCE" \
    --network "$NETWORK" \
    -- "$@"
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >/tmp/validate-testnet-failure.log 2>&1; then
    cat /tmp/validate-testnet-failure.log >&2
    fail "$label unexpectedly succeeded"
  fi
  pass "$label rejected"
}

require_cmd cargo
require_cmd stellar
require_value "$NETWORK" NETWORK
require_value "$DEPLOYER_SOURCE" VALIDATION_DEPLOYER_SOURCE
require_value "$ADMIN_SOURCE" VALIDATION_ADMIN_SOURCE
require_value "$BUYER_SOURCE" VALIDATION_BUYER_SOURCE

[[ "$NETWORK" == "testnet" ]] || fail "This validator only permits NETWORK=testnet"

ADMIN_ADDR="${VALIDATION_ADMIN_ADDR:-$(stellar keys address "$ADMIN_SOURCE")}"
BUYER_ADDR="${VALIDATION_BUYER_ADDR:-$(stellar keys address "$BUYER_SOURCE")}"
require_value "$ADMIN_ADDR" VALIDATION_ADMIN_ADDR
require_value "$BUYER_ADDR" VALIDATION_BUYER_ADDR

mkdir -p "$OUT_DIR"
TOKEN_WASM="target/wasm32v1-none/release/my_token.wasm"
MARKETPLACE_WASM="target/wasm32v1-none/release/prompt_marketplace.wasm"

echo "=== Fresh Testnet validation: $RUN_ID ==="

SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  stellar contract build
[[ -f "$TOKEN_WASM" ]] || fail "Missing $TOKEN_WASM"
[[ -f "$MARKETPLACE_WASM" ]] || fail "Missing $MARKETPLACE_WASM"

LOCAL_TOKEN_HASH="$(sha256_file "$TOKEN_WASM")"
LOCAL_MARKETPLACE_HASH="$(sha256_file "$MARKETPLACE_WASM")"

TOKEN_ID="$(stellar contract deploy \
  --wasm "$TOKEN_WASM" \
  --source "$DEPLOYER_SOURCE" \
  --network "$NETWORK" \
  -- \
  --owner "$ADMIN_ADDR" \
  --name "ValidationToken-$RUN_ID" \
  --symbol "VTEST" \
  --decimals 7)"
require_value "$TOKEN_ID" TOKEN_ID

MARKETPLACE_ID="$(stellar contract deploy \
  --wasm "$MARKETPLACE_WASM" \
  --source "$DEPLOYER_SOURCE" \
  --network "$NETWORK" \
  -- \
  --admin "$ADMIN_ADDR" \
  --token "$TOKEN_ID")"
require_value "$MARKETPLACE_ID" MARKETPLACE_ID

invoke "$TOKEN_ID" "$ADMIN_SOURCE" set_marketplace --marketplace "$MARKETPLACE_ID"
pass "Token trusts the newly deployed marketplace"

invoke "$TOKEN_ID" "$ADMIN_SOURCE" mint --to "$BUYER_ADDR" --amount 5000
PROMPT_ID="validation-$RUN_ID"
invoke "$MARKETPLACE_ID" "$ADMIN_SOURCE" register_prompt \
  --prompt_id "$PROMPT_ID" \
  --price 500 \
  --owner "$BUYER_ADDR" \
  --content_uri "enc://validation/$RUN_ID"
pass "Admin setup completed"

BEFORE_BALANCE="$(read_contract "$TOKEN_ID" balance --account "$BUYER_ADDR" | tr -d '"[:space:]')"
[[ "$BEFORE_BALANCE" == "5000" ]] || fail "Expected initial buyer balance 5000, got $BEFORE_BALANCE"

invoke "$MARKETPLACE_ID" "$BUYER_SOURCE" buy_prompt \
  --buyer "$BUYER_ADDR" \
  --prompt_id "$PROMPT_ID"
ACCESS="$(read_contract "$MARKETPLACE_ID" has_access --user "$BUYER_ADDR" --prompt_id "$PROMPT_ID" | tr -d '"[:space:]')"
AFTER_BUY_BALANCE="$(read_contract "$TOKEN_ID" balance --account "$BUYER_ADDR" | tr -d '"[:space:]')"
[[ "$ACCESS" == "true" ]] || fail "Expected access=true, got $ACCESS"
[[ "$AFTER_BUY_BALANCE" == "4500" ]] || fail "Expected balance 4500 after buy, got $AFTER_BUY_BALANCE"
pass "Legitimate purchase burned exactly 500 tokens"

expect_failure "Direct marketplace_mint attack" invoke "$TOKEN_ID" "$BUYER_SOURCE" marketplace_mint --to "$BUYER_ADDR" --amount 1
expect_failure "Direct marketplace_burn attack" invoke "$TOKEN_ID" "$BUYER_SOURCE" marketplace_burn --seller "$BUYER_ADDR" --amount 1

AFTER_ATTACK_BALANCE="$(read_contract "$TOKEN_ID" balance --account "$BUYER_ADDR" | tr -d '"[:space:]')"
[[ "$AFTER_ATTACK_BALANCE" == "4500" ]] || fail "Attack changed buyer balance: $AFTER_ATTACK_BALANCE"

expect_failure "Purchase replay" invoke "$MARKETPLACE_ID" "$BUYER_SOURCE" buy_prompt \
  --buyer "$BUYER_ADDR" \
  --prompt_id "$PROMPT_ID"

invoke "$MARKETPLACE_ID" "$ADMIN_SOURCE" remint --to "$BUYER_ADDR" --amount 2000
FINAL_BALANCE="$(read_contract "$TOKEN_ID" balance --account "$BUYER_ADDR" | tr -d '"[:space:]')"
[[ "$FINAL_BALANCE" == "6500" ]] || fail "Expected final balance 6500, got $FINAL_BALANCE"

CONFIGURED_MARKETPLACE="$(read_contract "$TOKEN_ID" get_marketplace | tr -d '"[:space:]')"
LINKED_TOKEN="$(read_contract "$MARKETPLACE_ID" get_token | tr -d '"[:space:]')"
[[ "$CONFIGURED_MARKETPLACE" == "$MARKETPLACE_ID" ]] || fail "Token marketplace link mismatch"
[[ "$LINKED_TOKEN" == "$TOKEN_ID" ]] || fail "Marketplace token link mismatch"

FETCHED_TOKEN="$OUT_DIR/validate-${RUN_ID}-token.wasm"
FETCHED_MARKETPLACE="$OUT_DIR/validate-${RUN_ID}-marketplace.wasm"
stellar contract fetch --id "$TOKEN_ID" --out-file "$FETCHED_TOKEN" --network "$NETWORK"
stellar contract fetch --id "$MARKETPLACE_ID" --out-file "$FETCHED_MARKETPLACE" --network "$NETWORK"
ONCHAIN_TOKEN_HASH="$(sha256_file "$FETCHED_TOKEN")"
ONCHAIN_MARKETPLACE_HASH="$(sha256_file "$FETCHED_MARKETPLACE")"
[[ "$ONCHAIN_TOKEN_HASH" == "$LOCAL_TOKEN_HASH" ]] || fail "Token WASM hash mismatch"
[[ "$ONCHAIN_MARKETPLACE_HASH" == "$LOCAL_MARKETPLACE_HASH" ]] || fail "Marketplace WASM hash mismatch"

REPORT="$OUT_DIR/validate-testnet-${RUN_ID}.json"
cat > "$REPORT" <<EOF
{
  "network": "$NETWORK",
  "run_id": "$RUN_ID",
  "token_id": "$TOKEN_ID",
  "marketplace_id": "$MARKETPLACE_ID",
  "prompt_id": "$PROMPT_ID",
  "local_token_wasm_sha256": "$LOCAL_TOKEN_HASH",
  "onchain_token_wasm_sha256": "$ONCHAIN_TOKEN_HASH",
  "local_marketplace_wasm_sha256": "$LOCAL_MARKETPLACE_HASH",
  "onchain_marketplace_wasm_sha256": "$ONCHAIN_MARKETPLACE_HASH",
  "cases": {
    "trusted_marketplace_configured": true,
    "legitimate_purchase": true,
    "direct_mint_rejected": true,
    "direct_burn_rejected": true,
    "purchase_replay_rejected": true,
    "state_and_links_verified": true,
    "wasm_hashes_verified": true
  },
  "status": "PASS"
}
EOF

echo "=== PASS ==="
echo "Fresh token:       $TOKEN_ID"
echo "Fresh marketplace: $MARKETPLACE_ID"
echo "Evidence report:   $REPORT"
