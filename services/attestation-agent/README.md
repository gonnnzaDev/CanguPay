# Agente de atestación · CanguPay (P0-07)

Servicio que lee el estado real del contrato, produce un reporte `PASS/FAIL` reproducible,
firma `attest(outcome, report_hash)` desde la cuenta engine y vigila el contrato como keeper.

Implementado en **Rust** con las librerías oficiales de Stellar (`stellar-xdr`,
`stellar-rpc-client` v27), en el mismo protocolo que el contrato (`soroban-sdk 27.0.6`).

## Comandos

```bash
cargo run -p attestation-agent --bin cangu-attest -- --help

# Verifica fixtures, clave y red. No envía nada.
cargo run -p attestation-agent --bin cangu-attest -- selfcheck

# Reporte PASS/FAIL de un bundle local (sale con 1 si es FAIL).
cargo run -p attestation-agent --bin cangu-attest -- evaluate \
    --bundle evidence.json --amount 1000 --currency CPUSD

# Estado real del contrato.
cargo run -p attestation-agent --bin cangu-attest -- status \
    --rpc https://soroban-testnet.stellar.org --network "Test SDF Network ; September 2015" \
    --contract C...

# Firma y envía attest() tras comprobar estado, plazo y hash.
cargo run -p attestation-agent --bin cangu-attest -- attest --bundle evidence.json --amount 1000

# Keeper: vigila y atesta cuando la evidencia está lista.
cargo run -p attestation-agent --bin cangu-attest -- keeper --bundle evidence.json --amount 1000 --poll 10
```

Códigos de salida: `0` correcto · `1` FAIL de reglas · `2` rechazo (estructura, hash, estado, plazo, configuración) · `3` infraestructura (red, XDR, io).

## Configuración

| Variable | Para qué |
| --- | --- |
| `CANGUPA_ENGINE_SECRET` | Secret `S...` o seed hex de 64 chars de la cuenta engine |
| `CANGUPA_ENGINE_KEYFILE` | Ruta a esa misma clave, **fuera del repositorio** |
| `CANGUPA_CONTRACT_ID` | Contrato en `C...` |
| `CANGUPA_RPC_URL` | Endpoint RPC de Soroban |
| `CANGUPA_NETWORK` | Passphrase de red |
| `CANGUPA_BUNDLE` | Ruta del bundle de evidencia en JSON |
| `CANGUPA_EXPECTED_AMOUNT` | Importe del escrow en unidades mínimas |
| `CANGUPA_EXPECTED_CURRENCY` | Divisa, por defecto `CPUSD` |

La clave **nunca** se lee del repositorio: si la ruta apunta dentro del repo, el agente
falla con un error explícito. `Debug` del signer no imprime material de clave, solo la
dirección `G...` y el origen.

## Qué garantiza y qué no

Garantizado y cubierto por tests:

- El mismo bundle produce el mismo `report_hash`, y coincide con el motor Python de
  referencia tanto en los fixtures del manifiesto como en bundles externos a él.
- El agente **no firma** si el hash del bundle en disco no es el que el proveedor subió a
  cadena, ni si el estado, el plazo o la cuenta engine no lo permiten.
- Un FAIL nombra el campo, su ruta, el valor observado y el esperado, sin inventar datos.
- La firma se construye sobre el payload de Soroban que la red verifica, y la tx se
  confirma releyendo el contrato: si el estado no quedó en `Attested*`, es un error.

**No** verificado todavía: la invocación real en testnet contra un contrato desplegado
(criterio 5 del issue). Requiere un `CANGUPA_CONTRACT_ID` desplegado y credenciales de la
cuenta engine. Hasta entonces, el camino de envío está implementado y probado contra
dobles, pero sin una ejecución en red que lo respalde.

El bundle se lee de disco. No hay cliente HTTP/IPFS: el hash en cadena se verifica contra
el archivo local que se le pase.

## Estructura

| Módulo | Responsabilidad |
| --- | --- |
| `canonical.rs` | JSON estricto (rechaza claves duplicadas y NaN) y SHA-256 |
| `ruleset.rs` | Reglas deterministas v1.0.0 con estado por campo |
| `report.rs` | Reporte `PASS/FAIL`, `report_hash` y `detail_hash` |
| `golden.rs` | Paridad con `fixtures/manifest.json`; la usan los tests y `selfcheck` |
| `chain.rs` | Lectura del contrato por RPC (rasgo `ChainClient` + impl Soroban) |
| `signer.rs` | Clave del engine fuera del repo |
| `attest.rs` | Plan de atestación y firma/envío de la transacción |
| `keeper.rs` | Bucle de vigilancia: observa, comprueba y atesta |

`app/` es el motor Python de la etapa 1, conservado como oráculo de referencia.
