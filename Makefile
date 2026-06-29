
# Configurable variables
WASM ?= target/wasm32v1-none/release/my_token.wasm
SOURCE ?= alice
NETWORK ?= testnet
HOST_TARGET ?= x86_64-unknown-linux-gnu

default: build

all: test

test: build
	cargo test --workspace --target $(HOST_TARGET)

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

clean:
	cargo clean
