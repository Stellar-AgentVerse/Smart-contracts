# Estado de issues y del proyecto

**Fecha de referencia:** 27 de agosto de 2026  
**Repositorio:** `Stellar-AgentVerse/Smart-contracts`

## Resumen ejecutivo

El proyecto es hoy un **prototipo funcional en Testnet**, no un MVP completo de extremo a extremo. El flujo básico de marketplace funciona y la vulnerabilidad crítica de autorización de tokens ([#27](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/27)) está corregida en la rama `fix/token-authorization-hardening`, ya integrada con el acceso privado que venía de `main`.

Mientras la rama no se mergee y despliegue, `main` sigue expuesto: ahí el mint y el burn cross-contract no piden autorización.

Falta la entrega cifrada del contenido en Backend y la coordinación completa entre capas. El registro de acceso privado on-chain ya existe, pero sólo cubre la parte on-chain del problema.

## Estado técnico real

### Implementado y validado

- El token sólo acepta operaciones cross-contract desde el marketplace configurado por el owner.
- La relación token → marketplace se configura una sola vez.
- Las llamadas directas externas de mint/burn son rechazadas.
- Los flujos legítimos `buy_prompt`, `buy_private_prompt` y `remint` siguen funcionando.
- Se validan montos positivos, pausa, replay, rollback y configuración incorrecta.
- El acceso privado usa commitments `BytesN<32>` según el ADR 0001: el ledger no ve `prompt_id` ni `content_uri`.
- `make verify` pasa: **48 tests** (13 de token, 34 de marketplace y 1 del prototipo), formato, Clippy y audit.
- `make verify-testnet` pasa usando despliegues efímeros y genera un reporte JSON en `deploy-artifacts/`.

El audit no reporta vulnerabilidades explotables, pero sí advierte dependencias sin mantenimiento (`derivative`, `paste`) y un paquete yanked (`spin`). Eso queda pendiente en la política de provenance de la issue #9.

### No implementado todavía

- La privacidad es parcial. Los commitments ocultan qué prompt se compra, pero el buyer, el precio y el momento siguen siendo observables, y el mismo hash es enlazable entre compras.
- El camino público `buy_prompt` sigue publicando `prompt_id` y `content_uri` en claro.
- No se implementaron nullifiers, relayers ni pruebas ZK.
- No hay un esquema aprobado de cifrado, entrega, retención, replay protection y auditoría de logs del Backend. El contrato sólo actúa como registro de autorización: la entrega depende de un servicio off-chain en el que hay que confiar.
- No debe asumirse un deployment Mainnet seguro. Los IDs históricos de Testnet están marcados como inseguros; los IDs generados por el validador son efímeros.

## Issues vigentes

| Repositorio | Issue | Estado | Significado |
|---|---|---|---|
| Smart-contracts | [#27](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/27) | Abierta / P0 | Mint y burn cross-contract sin autorización en `main`. Corregido en rama, pendiente de merge. |
| Smart-contracts | [#26](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/26) | Abierta | Evidencia de auditoría completa y rollout controlado a Mainnet. |
| Smart-contracts | [#25](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/25) | Abierta | Implementar el modelo de settlement para creadores, una vez decidido en #24. |
| Smart-contracts | [#24](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/24) | Abierta | Decidir el modelo de settlement y pago a creadores de Market V1. |
| Smart-contracts | [#9](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/9) | Abierta | Definir provenance de dependencias, revisión de `Cargo.lock` y auditoría reproducible. |
| Backend | [#8](https://github.com/Stellar-AgentVerse/Backend/issues/8) | Abierta | Trazabilidad de cifrado, entrega, logs, retención, replay y proveedor de IA. |

Las issues [#19](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/19) y [#20](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/20) están cerradas: el ADR 0001 fue aprobado y el registro de acceso privado quedó implementado. La [#18](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/18) se cerró como completada, pero la regresión que documenta la #27 muestra que el arreglo no llegó a `main`. Las issues antiguas #1 y #3 fueron reemplazadas por el ADR #19; #2 y #4 se consolidaron en #20; #5 se trasladó a Backend. Las de Backend [#2](https://github.com/Stellar-AgentVerse/Backend/issues/2) y [#3](https://github.com/Stellar-AgentVerse/Backend/issues/3) figuran cerradas, pero el flujo completo de privacidad todavía no está demostrado punta a punta.

## Orden recomendado para el MVP

1. Mergear la corrección P0 (#27) y publicar IDs nuevos de Testnet. Es lo que desbloquea todo lo demás.
2. Implementar y probar Backend #8 para entrega cifrada y trazabilidad controlada, que es lo que falta para que el acceso privado sirva de punta a punta.
3. Resolver #9 y repetir la auditoría antes de cualquier despliegue con usuarios o fondos reales.

## Comandos de evidencia

```bash
make verify
NETWORK=testnet \
VALIDATION_DEPLOYER_SOURCE=default \
VALIDATION_ADMIN_SOURCE=default \
VALIDATION_BUYER_SOURCE=buyer \
make verify-testnet
```

El segundo comando despliega contratos nuevos, ejecuta el flujo legítimo y ataques negativos, compara hashes WASM local/on-chain y falla con código distinto de cero ante cualquier diferencia. Testnet demuestra comportamiento reproducible; **no sustituye una auditoría ni prueba privacidad**.
