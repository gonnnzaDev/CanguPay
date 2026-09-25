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

# Keeper que además llama a finalize() cuando un plazo parece vencido.
cargo run -p attestation-agent --bin cangu-attest -- keeper --bundle evidence.json --amount 1000 --finalize

# Llama a finalize() una vez. El contrato decide si el plazo venció.
cargo run -p attestation-agent --bin cangu-attest -- finalize
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
(criterio 5 del issue). El intento se hizo y está documentado abajo con su diagnóstico.

### Intento de despliegue en testnet (sin completar)

Qué se conseguiu, verificado contra la red y no de forma local:

- Cinco cuentas creadas y fondeadas con friendbot (10 000 XLM cada una), confirmado
  leyendo la entrada de cuenta del ledger, no solo por el `200` de friendbot.
- El cálculo del payload de firma es correcto: reproduce exactamente la firma que la
  propia red ya había aceptado en la transacción de friendbot para esa misma cuenta.

Dónde se bloquea: toda transacción enviada por las herramientas de este repositorio se
rechaza con `TxBadAuth` y `fee_charged: 100`, o sea en la primera comprobación, antes de
llegar a validar nada. Se descartó como causa:

| Hipótesis | Cómo se descartó |
| --- | --- |
| Payload de firma mal calculado | Reproduce la firma válida de friendbot |
| Clave distinta de la cuenta | La dirección derivada del secreto coincide con la cuenta fondeada, sin firmantes ni umbrales adicionales |
| Comisión insuficiente | Probada a 100, 100 000 y 1 000 000, mismo error |
| Secuencia equivocada | Barrido alrededor del valor leído: solo una pasa la comprobación de secuencia, y aun así falla el auth |
| Forma de la operación | Falla igual un pago de 1 stroop a la propia cuenta, sin Soroban ni contrato |

Lo que sí quedó corregido por el camino: al leer `stellar-rpc-client` aparece que desde
el **protocolo 28** el RPC entrega credenciales `SOROBAN_CREDENTIALS_ADDRESS_V2`, que
según [CAP-71-02](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0071-02.md)
**no** se firman con el payload de la transacción sino con el preimage
`ENVELOPE_TYPE_SOROBAN_AUTHORIZATION_WITH_ADDRESS`. El agente firmaba ambas variantes con
el payload de la transacción, lo que la red rechaza con `txBAD_AUTH`. Ahora cada variante
se firma con su payload, con tests que comprueban que producen firmas distintas y que la
v2 verifica contra el preimage correcto.

### Causa del rechazo: XDR 27 contra un nodo en protocolo 28

El repositorio fija `stellar-xdr = "27"` y `soroban-sdk = "27.0.6"`. El nodo de testnet
corre **protocolo 28**. Con `stellar` CLI v28.0.0 (`stellar-xdr 28.0.0`) el mismo
despliegue se completa sin incidencias, lo que confirma que el nodo está bien y que el
problema es el desajuste de versión del toolchain, no la lógica de firma.

Consecuencia práctica: el mismo contrato sí se puede desplegar y ejercitar en testnet
con el CLI — así se verificó P0-04 de punta a punta — pero el **agente** no puede
enviar su propia atestación hasta que se migre a XDR/SDK 28. Migrarlo no es trivial:
cambia el formato de credenciales (CAP-71-02), que este agente ya firma de forma
correcta para ambas variantes.

Por eso **no** se publica ningún contract ID ni tx hash del agente: no existen. El comando de
montaje queda listo y es reproducible:

```bash
# Despliega el token de prueba y el escrow, y deja el contrato en
# EvidenceSubmitted listo para atestuar. Requiere CANGUPA_TESTNET_KEYS con
# un archivo de claves fuera del repo.
cargo build --release --target wasm32v1-none \
    -p conditional-payment -p test-token
cargo run --release -p attestation-agent --example testnet_setup -- \
    <archivo-de-claves> pass
```

El token de `contracts/test-token` es **un token de prueba**, no CPUSD: existe porque en
testnet no hay un SAC desplegado para una divisa de prueba y `fund()` necesita uno.

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
