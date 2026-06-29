# Soroban Smart Contracts — AgentVerseStellar

[![Smart Contracts CI](https://github.com/Stellar-AgentVerse/Smart-contracts/actions/workflows/smart-contracts-ci.yml/badge.svg)](https://github.com/Stellar-AgentVerse/Smart-contracts/actions/workflows/smart-contracts-ci.yml)

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

### Unit tests (29)

```bash
# Todos los tests (29)
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test --workspace --target aarch64-apple-darwin

# Por paquete
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test -p my-token --target aarch64-apple-darwin
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo test -p prompt-marketplace --target aarch64-apple-darwin
```

> En Linux/CI usa `--target x86_64-unknown-linux-gnu` en vez de `aarch64-apple-darwin`. El target por defecto del workspace (`.cargo/config.toml`) es `wasm32v1-none`, que no soporta `cargo test` — siempre hay que pasar `--target` explícito para correr tests.

### Brecha de mock auth en Soroban v25 (y cómo se mitiga)

Soroban v25's mock auth no puede satisfacer un SEGUNDO `require_auth()` para la MISMA address dentro de un mismo árbol de invocación: si la invocación raíz llama `buyer.require_auth()` y luego una sub-invocación (marketplace → token vía `invoke_contract`) también llama `require_auth()` para `buyer`, el host rechaza con `Error(Auth, ExistingValue)` — mock auth no tiene forma de representar "esta address ya autorizó más arriba en el árbol".

**Mitigación adoptada en este código:** `sell_forwarded` y `mint_forwarded` (`contracts/tokens/src/contract.rs`) NO llaman `require_auth()` — confían en que la invocación raíz (`buy_prompt` / `remint`) ya autorizó la address correspondiente. Como resultado, sí es posible escribir tests con `mock_auths()` que mockeen solo el único `require_auth()` de la raíz y ejerzan la llamada cross-contract real de punta a punta (balances, eventos incluidos) — ver `test_buy_prompt_cross_contract`, `test_remint_cross_contract`, `test_buy_prompt_emits_event`, `test_has_access_after_buy` en `contracts/marketplace/src/tests.rs`. No se necesitan `sub_invokes` porque no hay un `require_auth()` anidado del lado del token.

**El riesgo que esto sigue implicando:** `sell_forwarded` / `mint_forwarded` no verifican autorización en absoluto — si algo pudiera invocarlos directamente (sin pasar por el marketplace), podría mintear o quemar balances arbitrarios. Ese límite de confianza se ejercita explícitamente, SIN ningún `mock_auths()`, en `contracts/tokens/src/tests.rs` (`test_sell_forwarded_updates_balance`, `test_mint_forwarded_mints_tokens`) — para probar que esas funciones realmente no requieren autorización, no solo el happy path.

`scripts/integration-test.sh` ejercita el mismo flujo de punta a punta contra testnet real con firmas genuinas (Soroban CLI), que es el único lugar donde un `require_auth()` anidado genuino (si se reintrodujera por error) sería detectado.

### Integration test (testnet)

Prueba el flujo completo contra testnet real: mint → register → buy (cross-contract) → verify → remint → verify.

```bash
# Con los contracts deployados actuales
TOKEN="CCHAUOEVX6TQD56VFZY2GI3N3HNF5W6QSRRKAGKDXV6S53T4BKD5PYQD" \
MKT="CA6RRLV4IBLKRRLPUDCVXZFDKRE77YHBNJFSXEFFLXV6EUAVVVS6HJUQ" \
bash scripts/integration-test.sh
```

### Token: 8 tests

| Test | Qué cubre |
|---|---|
| `test_token_mint_and_balance` | Mint vía storage API + balance + total_supply |
| `test_token_metadata` | name, symbol, decimals |
| `test_mint_multiple_same_user` | Mint acumulativo a la misma address |
| `test_mint_to_different_users` | Mint a múltiples usuarios, supply tracking |
| `test_mint_overflow_panics` | i128::MAX + 1 debe panic |
| `test_zero_balance_default` | Balance por defecto es 0 |
| `test_sell_forwarded_updates_balance` | `sell_forwarded` quema tokens y emite `SellEvent`, invocado SIN `mock_auths()` |
| `test_mint_forwarded_mints_tokens` | `mint_forwarded` mintea tokens y emite `MintEvent`, invocado SIN `mock_auths()` |

### Marketplace: 21 tests

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
| `test_buy_prompt_cross_contract` | E2E: register → buy_prompt (cross-contract real vía `invoke_contract`) → balance quemado |
| `test_has_access_after_buy` | has_access es false antes de comprar y true después de `buy_prompt` |
| `test_buy_prompt_emits_event` | `buy_prompt` emite `PromptPurchased` con buyer/prompt_id/price correctos |
| `test_remint_cross_contract` | E2E: `remint` (cross-contract real vía `invoke_contract`) → balance minteado |
| `test_remint_emits_event` | `remint` emite `TokensReminted` con admin/to/amount correctos |

---

## Build WASM

```bash
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 stellar contract build --package my-token
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 stellar contract build --package prompt-marketplace
```

Requiere el target `wasm32v1-none` y la env var `SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1` para Spec Shaking v2. Alternativamente, para mainnet se usa `cargo build --target wasm32v1-none --release` (ver sección Deploy en Mainnet).

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

## Deploy en Mainnet

> ⚠️ **ADVERTENCIA DE SEGURIDAD — LEER ANTES DE EJECUTAR**
>
> Mainnet implica valor real y transacciones irreversibles. Antes de deployar:
>
> 1. **No hardcodees ni hagas `source` de claves privadas.** Usa un secrets manager (1Password CLI, AWS Secrets Manager, HashiCorp Vault, GitHub encrypted secrets, etc.) para inyectar `MAINNET_DEPLOYER_SOURCE` y `MAINNET_ADMIN_SOURCE` en runtime.
> 2. **Verifica el código y el WASM antes de deployar.** Recompila desde una fuente confiable, calcula el hash SHA-256 y comparalo con el artefacto que vas a subir.
> 3. **Preferí cuentas multisig o hardware wallets** para la cuenta admin (`MAINNET_ADMIN_ADDR`). El deployer y el admin pueden ser distintas cuentas.
> 4. **Revisa los parámetros de constructor.** Un error en `owner`/`admin` o en el `token` del marketplace puede dejar los contratos incontrolables.
> 5. **Tené XLM suficiente** en la cuenta deployer para cubrir el rent/storage de ambos contratos en mainnet.

### Prerrequisitos

- Stellar CLI configurado para mainnet (`stellar network add mainnet ...` o variables de entorno equivalentes).
- Acceso a un RPC endpoint de mainnet (Stellar public RPC, Blockdaemon, etc.).
- Cuenta con XLM real para pagar fees y rent.
- Cuenta admin separada (recomendado) con su par de claves seguras.
- Target de compilación instalado: `rustup target add wasm32v1-none`.
- Variable de entorno para Spec Shaking V2:
  ```bash
  export SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1
  ```

### Scripts de mainnet

#### `scripts/deploy-mainnet.sh`

Build release, calcula hashes, deploya, inicializa y valida ambos contratos. Emite un resumen JSON en `deploy-artifacts/`.

```bash
MAINNET_DEPLOYER_SOURCE="deployer" \
MAINNET_ADMIN_SOURCE="admin" \
MAINNET_ADMIN_ADDR="G..." \
bash scripts/deploy-mainnet.sh
```

También podés usar el target de Makefile:

```bash
MAINNET_DEPLOYER_SOURCE="deployer" \
MAINNET_ADMIN_SOURCE="admin" \
MAINNET_ADMIN_ADDR="G..." \
make deploy-mainnet
```

Variables opcionales:

| Variable | Descripción | Default |
|---|---|---|
| `MAINNET_TOKEN_NAME` | Nombre del token | `AgentVerse Token` |
| `MAINNET_TOKEN_SYMBOL` | Símbolo del token | `AVT` |
| `MAINNET_TOKEN_DECIMALS` | Decimales | `7` |
| `MAINNET_DEPLOY_OUT_DIR` | Carpeta de artefactos | `./deploy-artifacts` |

El script:

1. Construye ambos contratos con `cargo build --target wasm32v1-none --release`.
2. Calcula el hash SHA-256 de cada WASM.
3. Deploya `MyToken` y `PromptMarketplace` desde `MAINNET_DEPLOYER_SOURCE`.
4. Inicializa el token con el admin, nombre, símbolo y decimales configurados.
5. Inicializa el marketplace con el admin y el `contract_id` del token recién deployado.
6. Valida post-deploy:
   - Metadata del token (`name`, `symbol`, `decimals`).
   - Supply inicial igual a `0`.
   - Owner del token igual a `MAINNET_ADMIN_ADDR`.
   - Admin del marketplace igual a `MAINNET_ADMIN_ADDR`.
   - Token del marketplace igual al `contract_id` del token deployado.
   - Hash WASM on-chain (fetcheado) coincide con el artefacto local.
7. Escribe `deploy-artifacts/mainnet-deploy-summary-<timestamp>.json`.

#### `scripts/verify-mainnet.sh`

Verifica un par de contratos ya deployados sin re-deployar. Útil para CI o validaciones periódicas.

```bash
MAINNET_TOKEN_ID="C..." \
MAINNET_MARKETPLACE_ID="C..." \
MAINNET_ADMIN_ADDR="G..." \
bash scripts/verify-mainnet.sh
```

O vía Makefile:

```bash
MAINNET_TOKEN_ID="C..." \
MAINNET_MARKETPLACE_ID="C..." \
MAINNET_ADMIN_ADDR="G..." \
make verify-mainnet
```

Verifica:

- Hash WASM on-chain vs. artefacto local.
- Admin/owner de ambos contratos.
- Link token ↔ marketplace.
- Supply inicial (`0`).
- Metadata del token.

Escribe `deploy-artifacts/mainnet-verify-report-<timestamp>.json`.

### Multisig / DAuthorization (roadmap v2)

La v1 actual soporta cuentas multisig configuradas en Stellar CLI (el CLI pedirá/co-firmará las transacciones). Para una v2 se documentará como mejora:

- Flujo de firmas separadas: el deployer crea la transacción, múltiples signers la firman off-line, y alguien la publica.
- DAuthorization: delegar privilegios admin a un módulo de gobernanza on-chain.

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
│       └── tests.rs             # 8 tests
└── marketplace/                 # PromptMarketplace
    ├── Cargo.toml
    └── src/
        ├── lib.rs               # module exports
        ├── contract.rs          # #[contract] PromptMarketplace — API pública
        ├── storage/
        │   └── types.rs         # DataKey, Prompt struct, errors
        └── tests.rs             # 21 tests
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

Ver [Brecha de mock auth en Soroban v25](#brecha-de-mock-auth-en-soroban-v25-y-cómo-se-mitiga) para cómo este patrón se prueba (y se vigila como riesgo de seguridad) en los tests.
