# Estado de issues y del proyecto

**Fecha de referencia:** 19 de julio de 2026  
**Repositorio:** `Stellar-AgentVerse/Smart-contracts`

## Resumen ejecutivo

El proyecto es hoy un **prototipo funcional en Testnet**, no un MVP completo de extremo a extremo. El flujo básico de marketplace funciona y la vulnerabilidad crítica de autorización de tokens fue corregida en el árbol de trabajo. La corrección fue validada con tests locales y con un despliegue fresco en Testnet.

Todavía faltan la entrega privada del contenido, el modelo de amenazas aprobado y la coordinación completa con Backend. Los cambios actuales aún deben revisarse y confirmarse en un commit antes de tratarlos como una release.

## Estado técnico real

### Implementado y validado

- El token sólo acepta operaciones cross-contract desde el marketplace configurado por el owner.
- La relación token → marketplace se configura una sola vez.
- Las llamadas directas externas de mint/burn son rechazadas.
- Los flujos legítimos `buy_prompt` y `remint` siguen funcionando.
- Se validan montos positivos, pausa, replay, rollback y configuración incorrecta.
- `make verify` pasa: **38 tests** (13 de token y 25 de marketplace), formato, Clippy y audit.
- `make verify-testnet` pasa usando despliegues efímeros y genera un reporte JSON en `deploy-artifacts/`.

El audit no reporta vulnerabilidades explotables, pero sí advierte dependencias sin mantenimiento (`derivative`, `paste`) y un paquete yanked (`spin`). Eso queda pendiente en la política de provenance de la issue #9.

### No implementado todavía

- No existe privacidad on-chain ni off-chain completa.
- El prompt no se almacena on-chain, pero el comprador y la metadata/URI de la compra son observables.
- No se implementaron commitments, nullifiers, relayers ni pruebas ZK.
- No hay un esquema aprobado de cifrado, entrega, retención, replay protection y auditoría de logs del Backend.
- No debe asumirse un deployment Mainnet seguro. Los IDs históricos de Testnet están marcados como inseguros; los IDs generados por el validador son efímeros.

## Issues vigentes

| Repositorio | Issue | Estado | Significado |
|---|---|---|---|
| Smart-contracts | [#9](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/9) | Abierta | Definir provenance de dependencias, revisión de `Cargo.lock` y auditoría reproducible. |
| Smart-contracts | [#19](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/19) | Abierta | ADR del modelo de amenazas y viabilidad de privacidad en Stellar. Es el gate de diseño. |
| Smart-contracts | [#20](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/20) | Abierta/bloqueada | Diseñar el registro de acceso privado sólo después de aprobar el ADR #19. |
| Backend | [#8](https://github.com/Stellar-AgentVerse/Backend/issues/8) | Abierta | Trazabilidad de cifrado, entrega, logs, retención, replay y proveedor de IA. |

La issue [#18](https://github.com/Stellar-AgentVerse/Smart-contracts/issues/18) fue cerrada como completada al resolver el P0. Las issues antiguas #1 y #3 fueron reemplazadas por el ADR #19; #2 y #4 fueron consolidadas en #20; #5 se trasladó a Backend. Las issues de implementación de llamadas y firma de Backend [#2](https://github.com/Stellar-AgentVerse/Backend/issues/2) y [#3](https://github.com/Stellar-AgentVerse/Backend/issues/3) figuran cerradas, pero el flujo completo de privacidad todavía no está demostrado.

## Orden recomendado para el MVP

1. Revisar y commitear la corrección P0; luego publicar IDs nuevos de Testnet.
2. Aprobar el [ADR de privacidad](adr/0001-private-access-threat-model.md) (#19), sin asumir una tecnología antes de comparar sus garantías y costos.
3. Implementar y probar Backend #8 para entrega cifrada y trazabilidad controlada.
4. Ejecutar #20 sólo si el ADR confirma que el registro privado es necesario y viable.
5. Resolver #9 y repetir la auditoría antes de cualquier despliegue con usuarios o fondos reales.

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
