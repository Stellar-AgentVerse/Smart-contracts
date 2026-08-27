# Repository Guidelines

## Project Structure

This repository is a Cargo workspace containing two Soroban contracts:

- `contracts/tokens/`: the `MyToken` fungible token, including contract API, token logic, storage types, events, and unit tests.
- `contracts/marketplace/`: the `PromptMarketplace`, including prompt storage, cross-contract calls, and tests.
- `scripts/`: Testnet integration and Mainnet deployment/verification scripts.
- `.github/workflows/`: CI configuration. `README.md` contains deployment and architecture details.

Keep contract-specific code and tests inside the relevant package. Test snapshots under each package’s `test_snapshots/` directory are generated artifacts; update them only when an intentional snapshot change is reviewed.

## Build, Test, and Development Commands

Install the Rust `wasm32v1-none` target and Stellar CLI before building. The repository requires the Spec Shaking environment variable:

```bash
make test HOST_TARGET=x86_64-unknown-linux-gnu  # build and run workspace tests
make build                                      # build release WASM artifacts
make fmt-check                                  # verify rustfmt compliance
make clippy HOST_TARGET=x86_64-unknown-linux-gnu # lint with warnings denied
cargo audit                                     # audit dependencies
```

Use `make test HOST_TARGET=$(rustc -vV | awk '/host:/ {print $2}')` locally when the host target differs. Run `bash scripts/integration-test.sh` only when deliberately testing against a configured Testnet deployment.

## Coding Style and Naming

Use Rust 2021 conventions, four-space indentation, `cargo fmt`, and idiomatic `snake_case` for functions/modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep Soroban contract entry points in `contract.rs`; place storage definitions in `storage/` and reusable token logic in `core/`. Preserve explicit authorization, storage, event, and cross-contract boundaries.

## Testing Guidelines

Unit tests use Rust’s `#[test]` framework with `soroban-sdk::Env`. Name tests by behavior, such as `test_buy_prompt_cross_contract` or `test_non_admin_cannot_register`. Add coverage for authorization failures, invalid inputs, state transitions, events, and cross-contract behavior. Run the full workspace suite before submitting changes.

## Commits and Pull Requests

Use concise Conventional Commits, for example `feat: add marketplace validation`, `fix: reject zero prices`, or `test: cover auth failures`. Do not add AI attribution. PRs should explain behavior and security impact, identify affected contracts, include test/format/lint results, and call out deployment or configuration changes. Never commit private keys or secrets; Mainnet scripts must receive credentials through environment variables or a secrets manager.
