#!/usr/bin/env bash
# =============================================================================
# AgentVerse — Mainnet Deployment Script
# =============================================================================
# This script builds both Soroban contracts in release mode, computes and
# verifies WASM hashes, deploys them to Stellar mainnet, initializes them,
# runs post-deploy validation, and emits a machine-readable JSON summary.
#
# SECURITY NOTICE:
#   - NEVER source this script with secret keys inline.
#   - ALWAYS provide keys via environment variables injected at runtime by a
#     secrets manager (e.g. 1Password CLI, AWS Secrets Manager, HashiCorp Vault,
#     GitHub encrypted secrets, etc.).
#   - Prefer a hardware wallet / multisig account for mainnet admin operations.
#   - This script does NOT write private keys to disk.
#
# Required environment variables (inject securely):
#   MAINNET_DEPLOYER_SOURCE   Stellar CLI source alias/name for deployer account
#   MAINNET_ADMIN_SOURCE      Stellar CLI source alias/name for admin account
#   MAINNET_ADMIN_ADDR        Mainnet public address that will own the contracts
#
# Optional environment variables:
#   MAINNET_TOKEN_NAME        Token name  (default: "AgentVerse Token")
#   MAINNET_TOKEN_SYMBOL      Token symbol (default: "AVT")
#   MAINNET_TOKEN_DECIMALS    Token decimals (default: 7)
#   MAINNET_DEPLOY_OUT_DIR    Directory for deploy artifacts (default: ./deploy-artifacts)
#
# Example invocation:
#   MAINNET_DEPLOYER_SOURCE="deployer" \
#   MAINNET_ADMIN_SOURCE="admin" \
#   MAINNET_ADMIN_ADDR="G..." \
#   bash scripts/deploy-mainnet.sh
#
# Multisig / DAuthorization (v1 roadmap):
#   This initial version deploys from a single Stellar CLI source. If your
#   deployer account is a multisig account configured in the Stellar CLI,
#   the CLI will prompt/co-sign as usual. Future versions can integrate
#   explicit multi-signature workflows (e.g. signed transactions submitted
#   via separate signers) and DAuthorization (decentralized authorization)
#   modules.
# =============================================================================

set -euo pipefail

# ── Configuration ───────────────────────────────────────────────────────────
NETWORK="mainnet"
TARGET="wasm32v1-none"
RELEASE_DIR="target/${TARGET}/release"
TOKEN_WASM_NAME="my_token.wasm"
MKT_WASM_NAME="prompt_marketplace.wasm"

TOKEN_NAME="${MAINNET_TOKEN_NAME:-AgentVerse Token}"
TOKEN_SYMBOL="${MAINNET_TOKEN_SYMBOL:-AVT}"
TOKEN_DECIMALS="${MAINNET_TOKEN_DECIMALS:-7}"
OUT_DIR="${MAINNET_DEPLOY_OUT_DIR:-./deploy-artifacts}"
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

TOKEN_WASM="${RELEASE_DIR}/${TOKEN_WASM_NAME}"
MKT_WASM="${RELEASE_DIR}/${MKT_WASM_NAME}"

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

# Portable SHA-256 file hashing (Linux sha256sum or macOS shasum)
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

# Verify that a required environment variable is set and non-empty
require_env() {
  local var_name="$1"
  local var_value="${!var_name:-}"
  if [[ -z "$var_value" ]]; then
    fail "Missing required environment variable: ${var_name}"
  fi
}

# Verify that a binary is available in PATH
require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "Missing required command: $1. Please install it and add it to PATH."
  fi
}

# Invoke stellar contract with a small wrapper for readability
stellar_invoke() {
  local contract_id="$1"
  local source="$2"
  shift 2
  stellar contract invoke \
    --id "$contract_id" \
    --source "$source" \
    --network "$NETWORK" \
    -- "$@"
}

# ── Pre-flight checks ───────────────────────────────────────────────────────

log_section "AgentVerse Mainnet Deploy"
log_info "Network: ${NETWORK}"
log_info "Timestamp: ${TIMESTAMP}"

require_cmd cargo
require_cmd stellar

require_env MAINNET_DEPLOYER_SOURCE
require_env MAINNET_ADMIN_SOURCE
require_env MAINNET_ADMIN_ADDR

# Basic address format sanity check (Stellar public keys start with G and are 56 chars)
if [[ ! "${MAINNET_ADMIN_ADDR}" =~ ^G[A-Z0-9]{55}$ ]]; then
  fail "MAINNET_ADMIN_ADDR does not look like a valid Stellar public key: ${MAINNET_ADMIN_ADDR}"
fi

log_ok "Prerequisites satisfied"

# ── Build release ───────────────────────────────────────────────────────────

log_section "Build Release"
log_info "Target: ${TARGET}"
log_info "Profile: release (LTO, strip, overflow-checks)"

# The workspace profile.release already configures:
#   opt-level = "z", lto = true, strip = "symbols", overflow-checks = true
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build --target "${TARGET}" --release

if [[ ! -f "$TOKEN_WASM" ]]; then
  fail "Token WASM not found after build: ${TOKEN_WASM}"
fi
if [[ ! -f "$MKT_WASM" ]]; then
  fail "Marketplace WASM not found after build: ${MKT_WASM}"
fi

log_ok "Release build completed"

# ── Compute and store WASM hashes ───────────────────────────────────────────

log_section "Wasm Hashes"

TOKEN_WASM_HASH=$(sha256_file "$TOKEN_WASM")
MKT_WASM_HASH=$(sha256_file "$MKT_WASM")

log_info "MyToken      (${TOKEN_WASM_NAME}): ${TOKEN_WASM_HASH}"
log_info "Marketplace  (${MKT_WASM_NAME}): ${MKT_WASM_HASH}"

log_ok "WASM hashes computed"

# ── Deploy MyToken ──────────────────────────────────────────────────────────

log_section "Deploy MyToken"

MY_TOKEN_ID=$(stellar contract deploy \
  --wasm "$TOKEN_WASM" \
  --source "$MAINNET_DEPLOYER_SOURCE" \
  --network "$NETWORK")

if [[ -z "$MY_TOKEN_ID" ]]; then
  fail "MyToken deployment returned an empty contract ID"
fi

log_ok "MyToken deployed: ${MY_TOKEN_ID}"

# ── Initialize MyToken ──────────────────────────────────────────────────────

log_section "Initialize MyToken"

stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" \
  __constructor \
  --owner "$MAINNET_ADMIN_ADDR" \
  --name "$TOKEN_NAME" \
  --symbol "$TOKEN_SYMBOL" \
  --decimals "$TOKEN_DECIMALS"

log_ok "MyToken initialized"

# ── Deploy PromptMarketplace ────────────────────────────────────────────────

log_section "Deploy PromptMarketplace"

MARKETPLACE_ID=$(stellar contract deploy \
  --wasm "$MKT_WASM" \
  --source "$MAINNET_DEPLOYER_SOURCE" \
  --network "$NETWORK")

if [[ -z "$MARKETPLACE_ID" ]]; then
  fail "Marketplace deployment returned an empty contract ID"
fi

log_ok "PromptMarketplace deployed: ${MARKETPLACE_ID}"

# ── Initialize PromptMarketplace ────────────────────────────────────────────

log_section "Initialize PromptMarketplace"

stellar_invoke "$MARKETPLACE_ID" "$MAINNET_ADMIN_SOURCE" \
  __constructor \
  --admin "$MAINNET_ADMIN_ADDR" \
  --token "$MY_TOKEN_ID"

log_ok "PromptMarketplace initialized"

# ── Post-deploy validation ──────────────────────────────────────────────────

log_section "Post-Deploy Validation"

# 1. Verify MyToken metadata matches the requested values
TOKEN_NAME_ON_CHAIN=$(stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" name | tr -d '"')
TOKEN_SYMBOL_ON_CHAIN=$(stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" symbol | tr -d '"')
TOKEN_DECIMALS_ON_CHAIN=$(stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" decimals)
TOKEN_SUPPLY_ON_CHAIN=$(stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" total_supply)
TOKEN_OWNER_ON_CHAIN=$(stellar_invoke "$MY_TOKEN_ID" "$MAINNET_ADMIN_SOURCE" owner | tr -d '"')

if [[ "$TOKEN_NAME_ON_CHAIN" != "$TOKEN_NAME" ]]; then
  fail "Token name mismatch: expected '${TOKEN_NAME}', got '${TOKEN_NAME_ON_CHAIN}'"
fi
if [[ "$TOKEN_SYMBOL_ON_CHAIN" != "$TOKEN_SYMBOL" ]]; then
  fail "Token symbol mismatch: expected '${TOKEN_SYMBOL}', got '${TOKEN_SYMBOL_ON_CHAIN}'"
fi
if [[ "$TOKEN_DECIMALS_ON_CHAIN" != "$TOKEN_DECIMALS" ]]; then
  fail "Token decimals mismatch: expected '${TOKEN_DECIMALS}', got '${TOKEN_DECIMALS_ON_CHAIN}'"
fi
if [[ "$TOKEN_SUPPLY_ON_CHAIN" != "0" ]]; then
  fail "Initial token supply must be 0 at mainnet genesis, got '${TOKEN_SUPPLY_ON_CHAIN}'"
fi
if [[ "$TOKEN_OWNER_ON_CHAIN" != "$MAINNET_ADMIN_ADDR" ]]; then
  fail "Token owner mismatch: expected '${MAINNET_ADMIN_ADDR}', got '${TOKEN_OWNER_ON_CHAIN}'"
fi

log_ok "MyToken validation passed (name, symbol, decimals, supply=0, owner)"

# 2. Verify marketplace admin and token linkage
MKT_ADMIN_ON_CHAIN=$(stellar_invoke "$MARKETPLACE_ID" "$MAINNET_ADMIN_SOURCE" get_admin | tr -d '"')
MKT_TOKEN_ON_CHAIN=$(stellar_invoke "$MARKETPLACE_ID" "$MAINNET_ADMIN_SOURCE" get_token | tr -d '"')

if [[ "$MKT_ADMIN_ON_CHAIN" != "$MAINNET_ADMIN_ADDR" ]]; then
  fail "Marketplace admin mismatch: expected '${MAINNET_ADMIN_ADDR}', got '${MKT_ADMIN_ON_CHAIN}'"
fi
if [[ "$MKT_TOKEN_ON_CHAIN" != "$MY_TOKEN_ID" ]]; then
  fail "Marketplace token mismatch: expected '${MY_TOKEN_ID}', got '${MKT_TOKEN_ON_CHAIN}'"
fi

log_ok "PromptMarketplace validation passed (admin, token)"

# 3. Verify on-chain WASM hash matches the locally built artifact
verify_on_chain_hash() {
  local contract_id="$1"
  local expected_hash="$2"
  local label="$3"
  local fetched_wasm
  fetched_wasm="${OUT_DIR}/${label}_fetched.wasm"

  mkdir -p "$OUT_DIR"
  stellar contract fetch \
    --id "$contract_id" \
    --out-file "$fetched_wasm" \
    --network "$NETWORK"

  local fetched_hash
  fetched_hash=$(sha256_file "$fetched_wasm")

  if [[ "$fetched_hash" != "$expected_hash" ]]; then
    fail "${label} on-chain WASM hash mismatch. Expected: ${expected_hash}, got: ${fetched_hash}"
  fi

  log_ok "${label} on-chain WASM hash verified (${fetched_hash})"
}

verify_on_chain_hash "$MY_TOKEN_ID" "$TOKEN_WASM_HASH" "my_token"
verify_on_chain_hash "$MARKETPLACE_ID" "$MKT_WASM_HASH" "prompt_marketplace"

# ── Persist deploy artifacts ────────────────────────────────────────────────

log_section "Persisting Deploy Artifacts"

mkdir -p "$OUT_DIR"

SUMMARY_FILE="${OUT_DIR}/mainnet-deploy-summary-${TIMESTAMP}.json"
# Sanitize filename for Windows/Unix compatibility
SUMMARY_FILE="${SUMMARY_FILE//:/-}"

cat > "$SUMMARY_FILE" <<EOF
{
  "network": "${NETWORK}",
  "timestamp": "${TIMESTAMP}",
  "token": {
    "contract_id": "${MY_TOKEN_ID}",
    "name": "${TOKEN_NAME}",
    "symbol": "${TOKEN_SYMBOL}",
    "decimals": ${TOKEN_DECIMALS},
    "wasm_hash": "${TOKEN_WASM_HASH}",
    "wasm_path": "${TOKEN_WASM}",
    "owner": "${MAINNET_ADMIN_ADDR}",
    "initial_supply": "${TOKEN_SUPPLY_ON_CHAIN}"
  },
  "marketplace": {
    "contract_id": "${MARKETPLACE_ID}",
    "admin": "${MAINNET_ADMIN_ADDR}",
    "token_contract_id": "${MY_TOKEN_ID}",
    "wasm_hash": "${MKT_WASM_HASH}",
    "wasm_path": "${MKT_WASM}"
  },
  "deployer_source": "${MAINNET_DEPLOYER_SOURCE}",
  "admin_source": "${MAINNET_ADMIN_SOURCE}",
  "verification": {
    "token_metadata_ok": true,
    "token_owner_ok": true,
    "token_supply_ok": true,
    "marketplace_admin_ok": true,
    "marketplace_token_ok": true,
    "token_wasm_hash_on_chain_ok": true,
    "marketplace_wasm_hash_on_chain_ok": true
  }
}
EOF

log_ok "Deploy summary written to: ${SUMMARY_FILE}"

# ── Final human-readable summary ────────────────────────────────────────────

log_section "Deploy Summary"
cat <<EOF
Network:        ${NETWORK}
Timestamp:      ${TIMESTAMP}
MyToken:        ${MY_TOKEN_ID}
Marketplace:    ${MARKETPLACE_ID}
Admin:          ${MAINNET_ADMIN_ADDR}
Token Name:     ${TOKEN_NAME}
Token Symbol:   ${TOKEN_SYMBOL}
Token Decimals: ${TOKEN_DECIMALS}
Initial Supply: ${TOKEN_SUPPLY_ON_CHAIN}
Token Hash:     ${TOKEN_WASM_HASH}
Marketplace Hash: ${MKT_WASM_HASH}
Artifacts:      ${OUT_DIR}
EOF

log_ok "AgentVerse mainnet deployment completed successfully"
