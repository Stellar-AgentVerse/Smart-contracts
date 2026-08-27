#!/usr/bin/env bash
# Compatibility entrypoint. Use validate-testnet.sh for fresh, repeatable
# validation; persistent contract IDs are intentionally not accepted anymore.

set -euo pipefail

echo "integration-test.sh is deprecated; running fresh Testnet validation."
exec bash "$(dirname "$0")/validate-testnet.sh"
