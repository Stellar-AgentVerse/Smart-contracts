# Soroban Smart Contracts — AgentVerseStellar

Dos contratos Soroban desplegados en **Testnet**, conectados entre sí:

- **MyToken** (`my-token`): token fungible SEP-0041 con mint gobernado por owner, sell con burn, pausable.
- **PromptMarketplace** (`prompt-marketplace`): marketplace donde admins registran prompts, usuarios compran (quemando tokens), y admins pueden re-mintear.

---

## Deploy en Testnet

| Contrato | ID | Wasm Hash | Tamaño |
|---|---|---|---|
| **MyToken** | `CCZQTMOMCASGZTUEEUNVNHEZSMS2NXGVEQ2MERBSEXMWTMWUXOAYZRJ4` | `a724013b...` | 9.7 KB |
| **PromptMarketplace** | `CA4XFMUJA5LHSJD4PQQSZFGZUPAJ5YKNQNRXODVNMBACUBSQ2AWDJ4EE` | `86efb8bb...` | 5.6 KB |

**Arquitectura**: el marketplace almacena el ID del token en `__constructor`. `buy_prompt` llama a `my_token::sell` via `env.invoke_contract` (cross-contract). `remint` llama a `my_token::mint`.

---

## Tests

```bash
# Todos los tests (22)
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test --lib --target aarch64-apple-darwin

# Por paquete
cargo test -p my-token --lib --target aarch64-apple-darwin
cargo test -p prompt-marketplace --lib --target aarch64-apple-darwin
```

### Token: 6 tests

| Test | Qué cubre |
|---|---|
| `test_token_mint_and_balance` | Mint vía storage API + balance + total_supply |
| `test_token_metadata` | name, symbol, decimals |
| `test_mint_multiple_same_user` | Mint acumulativo a la misma address |
| `test_mint_to_different_users` | Mint a múltiples usuarios, supply tracking |
| `test_mint_overflow_panics` | i128::MAX + 1 debe panic |
| `test_zero_balance_default` | Balance por defecto es 0 |

### Marketplace: 16 tests

| Test | Qué cubre |
|---|---|
| `test_register_and_query_prompt` | Happy path: register → get_price / get_owner |
| `test_duplicate_registration_panics` | Mismo ID no se puede registrar dos veces |
| `test_update_price` | Admin cambia precio |
| `test_remove_prompt` | Admin elimina prompt (idempotente) |
| `test_multiple_prompts_independent` | Prompts distintos no interfieren |
| `test_get_price_unregistered_panics` | Consultar precio de prompt inexistente |
| `test_non_admin_cannot_register` | No-admin no puede registrar |
| `test_non_admin_cannot_update_price` | No-admin no puede cambiar precio |
| `test_non_admin_cannot_remove` | No-admin no puede eliminar |
| `test_register_zero_price_panics` | Precio 0 es inválido |
| `test_update_price_zero_panics` | Actualizar a precio 0 es inválido |
| `test_register_max_price` | i128::MAX funciona como precio |
| `test_update_unregistered_prompt_panics` | Actualizar precio de prompt inexistente |
| `test_register_after_remove` | Re-registrar mismo ID post-eliminación |
| `test_has_access_unregistered` | has_access sin compra devuelve false |
| `test_token_mint_and_balance` | Mint vía storage en contexto del token |

---

## Build WASM

```bash
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 stellar contract build --package my-token
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 stellar contract build --package prompt-marketplace
```

Requiere el target `wasm32v1-none` y la env var para Spec SHAKING v2.

---

## Deploy

```bash
# Token
stellar contract deploy \
  --wasm target/wasm32v1-none/release/my_token.wasm \
  --source default \
  --network testnet \
  --alias my_token \
  -- \
  --owner "$(stellar keys address default)" \
  --name "PromptToken" \
  --symbol "PRMPT" \
  --decimals 7

# Marketplace (usar el ID del token deployado)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/prompt_marketplace.wasm \
  --source default \
  --network testnet \
  --alias prompt_marketplace \
  -- \
  --admin "$(stellar keys address default)" \
  --token "CCZQTMOMCASGZTUEEUNVNHEZSMS2NXGVEQ2MERBSEXMWTMWUXOAYZRJ4"
```

### Invocar

```bash
# Leer nombre del token
stellar contract invoke --source default --network testnet \
  --id CCZQTMOMCASGZTUEEUNVNHEZSMS2NXGVEQ2MERBSEXMWTMWUXOAYZRJ4 \
  -- name

# Registrar prompt
stellar contract invoke --source default --network testnet \
  --id CA4XFMUJA5LHSJD4PQQSZFGZUPAJ5YKNQNRXODVNMBACUBSQ2AWDJ4EE \
  --send=yes \
  -- register_prompt --prompt_id "alpha" --price 500 \
  --owner "$(stellar keys address default)"

# Consultar precio
stellar contract invoke --source default --network testnet \
  --id CA4XFMUJA5LHSJD4PQQSZFGZUPAJ5YKNQNRXODVNMBACUBSQ2AWDJ4EE \
  -- get_price --prompt_id "alpha"
```

---

## Estructura del proyecto

```
contracts/
├── tokens/                      # MyToken (token fungible)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs               # module exports, re-export público
│       ├── contract.rs          # #[contract] MyToken — API pública + traits
│       ├── events.rs            # #[contractevent] structs (MintEvent, SellEvent, etc.)
│       ├── core/
│       │   └── token.rs         # TokenManager — lógica de negocio
│       ├── storage/
│       │   └── types.rs         # DataKey, contracttype structs
│       └── tests.rs             # 6 tests
└── marketplace/                 # PromptMarketplace
    ├── Cargo.toml
    └── src/
        ├── lib.rs               # module exports
        ├── contract.rs          # #[contract] PromptMarketplace — API pública
        ├── storage/
        │   └── types.rs         # DataKey, Prompt struct, errors
        └── tests.rs             # 16 tests
Cargo.toml                       # workspace: tokens + marketplace
```

---

## Stack

- **Soroban SDK**: `25.3.0`
- **OZ Stellar Contracts**: `v0.7.1` (fungible token, ownable, pausable)
- **Stellar CLI**: `26.1.0`
- **Target**: `wasm32v1-none` (release), `aarch64-apple-darwin` (tests)

### Nota sobre autenticación en tests

`soroban-sdk` v25 no soporta `mock_auths` para `require_auth` anidados en llamadas cross-contract. Las funciones que llaman `require_auth()` internamente (`sell`, `transfer`, `burn`) no se pueden testear con `mock_auths` (recording mode) cuando el auth debe pasar a una sub-invocación. La solución adoptada:

- Marketplace: tests solo de la lógica propia, sin llamadas reales al token via `buy_prompt`/`remint`
- Token: tests de storage via `env.as_contract` + `TokenBase` (mint, balance), sin auth
- Las operaciones con auth se verifican por código (`#[only_owner]`, `require_auth()` explícito)
