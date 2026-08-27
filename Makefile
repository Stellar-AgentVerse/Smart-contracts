
# Configurable variables
WASM ?= target/wasm32v1-none/release/my_token.wasm
SOURCE ?= alice
NETWORK ?= testnet
HOST_TARGET ?= $(shell rustc -vV | grep 'host:' | awk '{print $$2}')
# Fallback: if detection fails, default to x86_64-unknown-linux-gnu
ifeq ($(HOST_TARGET),)
HOST_TARGET := x86_64-unknown-linux-gnu
endif

default: build

all: test

test: build
	SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test --workspace --target $(HOST_TARGET)

build:
	SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 stellar contract build
	@ls -l target/wasm32v1-none/release/*.wasm

# Deploy the built wasm to Stellar (uses `stellar` CLI)
# Usage: make deploy WASM=path/to/wasm SOURCE=alice NETWORK=testnet
deploy: build
	stellar contract deploy --wasm $(WASM) --source $(SOURCE) --network $(NETWORK)

# Convenience target: deploy and show contract id (stdout from CLI)
deploy-show: deploy
	@echo "Deployed. Check output above for CONTRACT_ID"

# Mainnet deployment for AgentVerse (MyToken + PromptMarketplace)
# Requires MAINNET_DEPLOYER_SOURCE, MAINNET_ADMIN_SOURCE, MAINNET_ADMIN_ADDR
# Usage:
#   MAINNET_DEPLOYER_SOURCE=deployer \
#   MAINNET_ADMIN_SOURCE=admin \
#   MAINNET_ADMIN_ADDR=G... \
#   make deploy-mainnet
deploy-mainnet:
	bash scripts/deploy-mainnet.sh

# Mainnet verification of deployed contracts
# Usage:
#   MAINNET_TOKEN_ID=C... \
#   MAINNET_MARKETPLACE_ID=C... \
#   MAINNET_ADMIN_ADDR=G... \
#   make verify-mainnet
verify-mainnet:
	bash scripts/verify-mainnet.sh

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --target $(HOST_TARGET) -- -D warnings

audit:
	@command -v cargo-audit >/dev/null 2>&1 || (echo "cargo-audit is required. Install it with: cargo install cargo-audit"; exit 1)
	@if [ -n "$(CARGO_AUDIT_HOME)" ]; then CARGO_HOME="$(CARGO_AUDIT_HOME)" cargo audit; else cargo audit; fi

verify: test fmt-check clippy audit

# Deploy fresh contracts and run positive and adversarial Testnet checks.
# Required: NETWORK=testnet VALIDATION_DEPLOYER_SOURCE=... \
#           VALIDATION_ADMIN_SOURCE=... VALIDATION_BUYER_SOURCE=...
verify-testnet:
	bash scripts/validate-testnet.sh

clean:
	cargo clean
