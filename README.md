# Soroban Smart Contracts — AgentVerseStellar

Dos contratos Soroban desplegados en **Testnet**, conectados entre sí:

- **MyToken** (`my-token`): token fungible SEP-0041 con mint gobernado por owner, sell con burn, pausable.
- **PromptMarketplace** (`prompt-marketplace`): marketplace donde admins registran prompts, usuarios compran (quemando tokens), y admins pueden re-mintear.

---

## Deploy en Testnet

| Contrato | ID | Wasm Hash | Tamaño |
|---|---|---|---|---|
| **MyToken** | `CCHAUOEVX6TQD56VFZY2GI3N3HNF5W6QSRRKAGKDXV6S53T4BKD5PYQD` | `6613593d...` | 9.7 KB |
| **PromptMarketplace** | `CA6RRLV4IBLKRRLPUDCVXZFDKRE77YHBNJFSXEFFLXV6EUAVVVS6HJUQ` | `f31af684...` | 5.6 KB |

**Arquitectura**: el marketplace almacena el ID del token en `__constructor`. `buy_prompt` llama a `my_token::sell_forwarded` via `env.invoke_contract` (cross-contract). `remint` llama a `my_token::mint_forwarded`. Ambos contratos emiten eventos `#[contractevent]` (PromptRegistered, PromptPriceUpdated, PromptRemoved, PromptPurchased, TokensReminted).

> ⚠️ **Cross-contract auth forwarding**: las funciones `sell_forwarded` y `mint_forwarded` no verifican `require_auth()` internamente. Confían en que la root invocation (marketplace) ya verificó la autorización. Esto evita el error `Error(Auth, ExistingValue)` de Soroban cuando se llama `require_auth()` para la misma address en root + sub-invocation.

---

## Tests

### Unit tests (22)

```bash
# Todos los tests (22)
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test --lib --target aarch64-apple-darwin

# Por paquete
cargo test -p my-token --lib --target aarch64-apple-darwin
cargo test -p prompt-marketplace --lib --target aarch64-apple-darwin
```

### Integration test (testnet)

Prueba el flujo completo contra testnet real: mint → register → buy (cross-contract) → verify → remint → verify.

```bash
# Con los contracts deployados actuales
TOKEN="CCHAUOEVX6TQD56VFZY2GI3N3HNF5W6QSRRKAGKDXV6S53T4BKD5PYQD" \
MKT="CA6RRLV4IBLKRRLPUDCVXZFDKRE77YHBNJFSXEFFLXV6EUAVVVS6HJUQ" \
bash scripts/integration-test.sh
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
  --token "CCHAUOEVX6TQD56VFZY2GI3N3HNF5W6QSRRKAGKDXV6S53T4BKD5PYQD"
```

### Invocar

```bash
# Leer nombre del token
stellar contract invoke --source default --network testnet \
  --id CCHAUOEVX6TQD56VFZY2GI3N3HNF5W6QSRRKAGKDXV6S53T4BKD5PYQD \
  -- name

# Registrar prompt
stellar contract invoke --source default --network testnet \
  --id CA6RRLV4IBLKRRLPUDCVXZFDKRE77YHBNJFSXEFFLXV6EUAVVVS6HJUQ \
  --send=yes \
  -- register_prompt --prompt_id "alpha" --price 500 \
  --owner "$(stellar keys address default)"

# Consultar precio
stellar contract invoke --source default --network testnet \
  --id CA6RRLV4IBLKRRLPUDCVXZFDKRE77YHBNJFSXEFFLXV6EUAVVVS6HJUQ \
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

### Nota sobre autenticación cross-contract

`soroban-sdk` v25 tiene una limitación: `require_auth()` para una misma dirección solo puede ejecutarse UNA VEZ por árbol de llamadas. Llamarlo en la root invocation y luego en una sub-invocación (via `invoke_contract`) falla con `Error(Auth, ExistingValue)`.

**Patrón adoptado:**

- La función raíz (ej. `buy_prompt`, `remint`) llama `require_auth()` para la/s dirección/es involucradas.
- Las sub-invocaciones al token usan variantes `*_forwarded` (`sell_forwarded`, `mint_forwarded`) que **no** verifican `require_auth()` internamente.
- Para operaciones directas (sin marketplace), el token expone `sell` (con `require_auth`) y `mint` (con `#[only_owner]`).

Esto aplica también a `TokenManager::sell` que usa `Base::update` en vez de `Base::burn` para evitar el doble `require_auth` de `Base::burn`.
