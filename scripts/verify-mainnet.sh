#!/usr/bin/env bash
# =============================================================================
# AgentVerse — Mainnet Verification Script
# =============================================================================
# Independently verifies deployed mainnet contracts against locally built WASM
# artifacts and expected configuration. Use this after deployment, in CI, or
# after a contract upgrade to ensure on-chain state matches expectations.
#
# Required environment variables:
#   MAINNET_TOKEN_ID            MyToken contract ID on mainnet
#   MAINNET_MARKETPLACE_ID      PromptMarketplace contract ID on mainnet
#   MAINNET_ADMIN_ADDR          Expected admin/owner address
#
# Optional environment variables:
#   MAINNET_TOKEN_NAME          Expected token name (default: "AgentVerse Token")
#   MAINNET_TOKEN_SYMBOL        Expected token symbol (default: "AVT")
#   MAINNET_TOKEN_DECIMALS      Expected token decimals (default: 7)
#   MAINNET_TOKEN_WASM          Path to local MyToken wasm (default: target/wasm32v1-none/release/my_token.wasm)
#   MAINNET_MARKETPLACE_WASM    Path to local Marketplace wasm (default: target/wasm32v1-none/release/prompt_marketplace.wasm)
#   MAINNET_VERIFY_SOURCE       Stellar CLI source for read-only invocations (default: admin source or default)
#   MAINNET_VERIFY_OUT_DIR      Directory for fetched wasm artifacts (default: ./deploy-artifacts)
#
# Example invocation:
#   MAINNET_TOKEN_ID="C..." \
#   MAINNET_MARKETPLACE_ID="C..." \
#   MAINNET_ADMIN_ADDR="G..." \
#   bash scripts/verify-mainnet.sh
#
# Exit codes:
#   0  All verifications passed
#   1  One or more verifications failed
# =============================================================================

set -euo pipefail

# ── Configuration ───────────────────────────────────────────────────────────
NETWORK="mainnet"
TARGET="wasm32v1-none"
RELEASE_DIR="target/${TARGET}/release"

TOKEN_ID="${MAINNET_TOKEN_ID:-}"
MKT_ID="${MAINNET_MARKETPLACE_ID:-}"
ADMIN_ADDR="${MAINNET_ADMIN_ADDR:-}"

TOKEN_NAME="${MAINNET_TOKEN_NAME:-AgentVerse Token}"
TOKEN_SYMBOL="${MAINNET_TOKEN_SYMBOL:-AVT}"
TOKEN_DECIMALS="${MAINNET_TOKEN_DECIMALS:-7}"

TOKEN_WASM="${MAINNET_TOKEN_WASM:-${RELEASE_DIR}/my_token.wasm}"
MKT_WASM="${MAINNET_MARKETPLACE_WASM:-${RELEASE_DIR}/prompt_marketplace.wasm}"
VERIFY_SOURCE="${MAINNET_VERIFY_SOURCE:-${MAINNET_ADMIN_SOURCE:-default}}"
OUT_DIR="${MAINNET_VERIFY_OUT_DIR:-./deploy-artifacts}"
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# ── Helpers ─────────────────────────────────────────────────────────────────

log_section() {
  echo ""
  echo "=== $1 ==="
}

log_info() {
  echo "  ℹ️  $*"
}

log_ok() {
  echo "  ✅ $*"
}

log_error() {
  echo "  ❌ $*" >&2
}

fail() {
  log_error "$*"
  exit 1
}

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    fail "No SHA-256 tool found. Install sha256sum (Linux) or shasum (macOS)."
  fi
}

require_env() {
  local var_name="$1"
  local var_value="${!var_name:-}"
  if [[ -z "$var_value" ]]; then
    fail "Missing required environment variable: ${var_name}"
  fi
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "Missing required command: $1. Please install it and add it to PATH."
  fi
}

stellar_invoke() {
  local contract_id="$1"
  shift 1
  stellar contract invoke \
    --id "$contract_id" \
    --source "$VERIFY_SOURCE" \
    --network "$NETWORK" \
    -- "$@"
}

# ── Pre-flight checks ───────────────────────────────────────────────────────

log_section "AgentVerse Mainnet Verification"
log_info "Network: ${NETWORK}"
log_info "Timestamp: ${TIMESTAMP}"

require_cmd stellar

require_env MAINNET_TOKEN_ID
require_env MAINNET_MARKETPLACE_ID
require_env MAINNET_ADMIN_ADDR

if [[ ! -f "$TOKEN_WASM" ]]; then
  fail "Local MyToken WASM not found: ${TOKEN_WASM}. Build with: cargo build --target ${TARGET} --release"
fi
if [[ ! -f "$MKT_WASM" ]]; then
  fail "Local Marketplace WASM not found: ${MKT_WASM}. Build with: cargo build --target ${TARGET} --release"
fi

log_ok "Inputs validated"

# ── Compute local WASM hashes ───────────────────────────────────────────────

log_section "Local WASM Hashes"

LOCAL_TOKEN_HASH=$(sha256_file "$TOKEN_WASM")
LOCAL_MKT_HASH=$(sha256_file "$MKT_WASM")

log_info "MyToken local hash:      ${LOCAL_TOKEN_HASH}"
log_info "Marketplace local hash:  ${LOCAL_MKT_HASH}"

# ── Verify on-chain WASM hashes ─────────────────────────────────────────────

log_section "On-Chain WASM Hash Verification"

mkdir -p "$OUT_DIR"

verify_fetched_hash() {
  local contract_id="$1"
  local expected_hash="$2"
  local label="$3"
  local fetched_wasm="${OUT_DIR}/${label}_fetched.wasm"

  log_info "Fetching ${label} WASM from mainnet..."
  stellar contract fetch \
    --id "$contract_id" \
    --out-file "$fetched_wasm" \
    --network "$NETWORK"

  local fetched_hash
  fetched_hash=$(sha256_file "$fetched_wasm")

  if [[ "$fetched_hash" != "$expected_hash" ]]; then
    fail "${label} WASM hash mismatch. Local: ${expected_hash}, On-chain: ${fetched_hash}"
  fi

  log_ok "${label} on-chain WASM hash matches local artifact"
}

verify_fetched_hash "$TOKEN_ID" "$LOCAL_TOKEN_HASH" "my_token"
verify_fetched_hash "$MKT_ID" "$LOCAL_MKT_HASH" "prompt_marketplace"

# ── Verify MyToken state ────────────────────────────────────────────────────

log_section "MyToken State Verification"

TOKEN_NAME_CHAIN=$(stellar_invoke "$TOKEN_ID" name | tr -d '"')
TOKEN_SYMBOL_CHAIN=$(stellar_invoke "$TOKEN_ID" symbol | tr -d '"')
TOKEN_DECIMALS_CHAIN=$(stellar_invoke "$TOKEN_ID" decimals)
TOKEN_SUPPLY_CHAIN=$(stellar_invoke "$TOKEN_ID" total_supply)
TOKEN_OWNER_CHAIN=$(stellar_invoke "$TOKEN_ID" owner | tr -d '"')

if [[ "$TOKEN_NAME_CHAIN" != "$TOKEN_NAME" ]]; then
  fail "Token name mismatch. Expected: ${TOKEN_NAME}, Got: ${TOKEN_NAME_CHAIN}"
fi
log_ok "Token name: ${TOKEN_NAME_CHAIN}"

if [[ "$TOKEN_SYMBOL_CHAIN" != "$TOKEN_SYMBOL" ]]; then
  fail "Token symbol mismatch. Expected: ${TOKEN_SYMBOL}, Got: ${TOKEN_SYMBOL_CHAIN}"
fi
log_ok "Token symbol: ${TOKEN_SYMBOL_CHAIN}"

if [[ "$TOKEN_DECIMALS_CHAIN" != "$TOKEN_DECIMALS" ]]; then
  fail "Token decimals mismatch. Expected: ${TOKEN_DECIMALS}, Got: ${TOKEN_DECIMALS_CHAIN}"
fi
log_ok "Token decimals: ${TOKEN_DECIMALS_CHAIN}"

if [[ "$TOKEN_SUPPLY_CHAIN" != "0" ]]; then
  fail "Token initial supply mismatch. Expected: 0, Got: ${TOKEN_SUPPLY_CHAIN}"
fi
log_ok "Token initial supply: ${TOKEN_SUPPLY_CHAIN}"

if [[ "$TOKEN_OWNER_CHAIN" != "$ADMIN_ADDR" ]]; then
  fail "Token owner mismatch. Expected: ${ADMIN_ADDR}, Got: ${TOKEN_OWNER_CHAIN}"
fi
log_ok "Token owner: ${TOKEN_OWNER_CHAIN}"

# ── Verify Marketplace state ────────────────────────────────────────────────

log_section "PromptMarketplace State Verification"

MKT_ADMIN_CHAIN=$(stellar_invoke "$MKT_ID" get_admin | tr -d '"')
MKT_TOKEN_CHAIN=$(stellar_invoke "$MKT_ID" get_token | tr -d '"')

if [[ "$MKT_ADMIN_CHAIN" != "$ADMIN_ADDR" ]]; then
  fail "Marketplace admin mismatch. Expected: ${ADMIN_ADDR}, Got: ${MKT_ADMIN_CHAIN}"
fi
log_ok "Marketplace admin: ${MKT_ADMIN_CHAIN}"

if [[ "$MKT_TOKEN_CHAIN" != "$TOKEN_ID" ]]; then
  fail "Marketplace token mismatch. Expected: ${TOKEN_ID}, Got: ${MKT_TOKEN_CHAIN}"
fi
log_ok "Marketplace token: ${MKT_TOKEN_CHAIN}"

# ── Persist verification report ─────────────────────────────────────────────

log_section "Verification Report"

REPORT_FILE="${OUT_DIR}/mainnet-verify-report-${TIMESTAMP}.json"
REPORT_FILE="${REPORT_FILE//:/-}"

cat > "$REPORT_FILE" <<EOF
{
  "network": "${NETWORK}",
  "timestamp": "${TIMESTAMP}",
  "token": {
    "contract_id": "${TOKEN_ID}",
    "expected_admin": "${ADMIN_ADDR}",
    "actual_admin": "${TOKEN_OWNER_CHAIN}",
    "expected_name": "${TOKEN_NAME}",
    "actual_name": "${TOKEN_NAME_CHAIN}",
    "expected_symbol": "${TOKEN_SYMBOL}",
    "actual_symbol": "${TOKEN_SYMBOL_CHAIN}",
    "expected_decimals": ${TOKEN_DECIMALS},
    "actual_decimals": ${TOKEN_DECIMALS_CHAIN},
    "expected_supply": "0",
    "actual_supply": "${TOKEN_SUPPLY_CHAIN}",
    "local_wasm_hash": "${LOCAL_TOKEN_HASH}",
    "wasm_path": "${TOKEN_WASM}"
  },
  "marketplace": {
    "contract_id": "${MKT_ID}",
    "expected_admin": "${ADMIN_ADDR}",
    "actual_admin": "${MKT_ADMIN_CHAIN}",
    "expected_token": "${TOKEN_ID}",
    "actual_token": "${MKT_TOKEN_CHAIN}",
    "local_wasm_hash": "${LOCAL_MKT_HASH}",
    "wasm_path": "${MKT_WASM}"
  },
  "verification": {
    "token_wasm_hash_ok": true,
    "marketplace_wasm_hash_ok": true,
    "token_name_ok": true,
    "token_symbol_ok": true,
    "token_decimals_ok": true,
    "token_supply_ok": true,
    "token_owner_ok": true,
    "marketplace_admin_ok": true,
    "marketplace_token_ok": true
  },
  "status": "PASS"
}
EOF

log_ok "Verification report written to: ${REPORT_FILE}"

cat <<EOF

═══════════════════════════════════════════════════════════════
  ✅ ALL MAINNET VERIFICATIONS PASSED
═══════════════════════════════════════════════════════════════
  Token:       ${TOKEN_ID}
  Marketplace: ${MKT_ID}
  Admin:       ${ADMIN_ADDR}
  Supply:      ${TOKEN_SUPPLY_CHAIN}
  Report:      ${REPORT_FILE}
═══════════════════════════════════════════════════════════════
EOF
