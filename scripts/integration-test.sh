#!/usr/bin/env bash
# Integration test: full testnet flow for MyToken + PromptMarketplace
#
# Usage:
#   export TOKEN=<contract-id> MKT=<contract-id>
#   bash scripts/integration-test.sh
#
# If TOKEN and MKT are not set, defaults to the latest deployed contracts.

set -euo pipefail

# ── Config ──────────────────────────────────────────────────────────────
TOKEN="${TOKEN:-}"
MKT="${MKT:-}"
ADMIN="${ADMIN:-$(stellar keys address default)}"
BUYER="${BUYER:-$(stellar keys address buyer)}"
NETWORK="${NETWORK:-testnet}"

if [[ -z "$TOKEN" || -z "$MKT" ]]; then
  echo "❌ TOKEN and MKT must be set or deployed via scripts/deploy.sh"
  exit 1
fi

# ── Helpers ─────────────────────────────────────────────────────────────
invoke() { stellar contract invoke --id "$1" --source "$2" --network "$NETWORK" "${@:3}"; }
invoke_ro() { stellar contract invoke --id "$1" --source default --network "$NETWORK" -- "$@"; }
send_yes() { invoke "$1" "$2" --send=yes -- "${@:3}"; }

pass()  { echo "  ✅ $*"; }
fail()  { echo "  ❌ $*"; exit 1; }

check_balance() {
  local label="$1" expected="$2"
  local got
  got=$(invoke_ro "$TOKEN" balance --account "$BUYER" | tr -d '"')
  if [[ "$got" == "$expected" ]]; then
    pass "$label: balance = $got"
  else
    fail "$label: expected $expected, got $got"
  fi
}

# ── Test Suite ──────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Integration Test — MyToken + PromptMarketplace"
echo "  Token ID : $TOKEN"
echo "  Mkt  ID  : $MKT"
echo "  Admin    : $ADMIN"
echo "  Buyer    : $BUYER"
echo "═══════════════════════════════════════════════════════"
echo ""

# ── 1. Admin mints 5000 tokens to buyer ─────────────────────────────────
echo "─── Step 1: Mint 5000 tokens to buyer ───"
send_yes "$TOKEN" default mint --to "$BUYER" --amount 5000
pass "Mint succeeded"
echo ""

# ── 2. Admin registers a prompt ─────────────────────────────────────────
echo "─── Step 2: Register prompt at price 500 ───"
send_yes "$MKT" default register_prompt \
  --prompt_id "test-prompt-1" \
  --price 500 \
  --owner "$BUYER" \
  --content_uri "ipfs://QmIntegrationTest"
pass "Prompt registered"
echo ""

# ── 3. Buyer buys the prompt (cross-contract auth forwarding) ───────────
echo "─── Step 3: Buyer buys prompt ───"
send_yes "$MKT" buyer buy_prompt \
  --buyer "$BUYER" \
  --prompt_id "test-prompt-1"
pass "Buy succeeded (auth forwarding works!)"
echo ""

# ── 4. Verify access + balance ──────────────────────────────────────────
echo "─── Step 4: Verify access and balance ───"
access=$(invoke_ro "$MKT" has_access --user "$BUYER" --prompt_id "test-prompt-1")
if [[ "$access" == "true" ]]; then
  pass "has_access = true"
else
  fail "has_access expected true, got $access"
fi
check_balance "Buyer balance after purchase" "4500"
echo ""

# ── 5. Admin remints tokens via marketplace ─────────────────────────────
echo "─── Step 5: Admin remints 2000 tokens ───"
send_yes "$MKT" default remint --to "$BUYER" --amount 2000
pass "Remint succeeded (cross-contract mint works!)"
echo ""

# ── 6. Verify final balance ─────────────────────────────────────────────
echo "─── Step 6: Final balance check ───"
check_balance "Final balance" "6500"
echo ""

echo "═══════════════════════════════════════════════════════"
echo "  ✅ ALL INTEGRATION TESTS PASSED"
echo "═══════════════════════════════════════════════════════"
