
# Configurable variables
WASM ?= target/wasm32v1-none/release/my_contract.wasm
SOURCE ?= alice
NETWORK ?= testnet
HOST_TARGET ?= x86_64-unknown-linux-gnu

default: build

all: test

test: build
	cargo test --workspace --target $(HOST_TARGET)

build:
	stellar contract build
	@ls -l target/wasm32v1-none/release/*.wasm

# Deploy the built wasm to Stellar (uses `stellar` CLI)
# Usage: make deploy WASM=path/to/wasm SOURCE=alice NETWORK=testnet
deploy: build
	stellar contract deploy --wasm $(WASM) --source $(SOURCE) --network $(NETWORK)

# Convenience target: deploy and show contract id (stdout from CLI)
deploy-show: deploy
	@echo "Deployed. Check output above for CONTRACT_ID"

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --target $(HOST_TARGET) -- -D warnings

clean:
	cargo clean
