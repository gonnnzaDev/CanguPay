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

### Verificado en testnet

El agente firma y envia de verdad. Dos atestaciones suyas, con el `report_hash` que el
motor calculo:

| Caso | Contrato | tx |
| --- | --- | --- |
| PASS | `CCGZQCVPZPJLTTD4NSCL7MZWFH6PKFAAZCSGXCPZH7ZEB7T56L2WJ2OF` | `f7168ae3e3c0f4e7cf59bc66353f7d9f3e24510043144dcf4864b2f003e1976f` |
| FAIL | `CCQ7XTWYMGKMTBPJ5UIJ764GDQX7ZE2HYSZ3EPW4XUELSC7755SPNJ7A` | `bb714f80fe2acc5f6b8192fd5ec106a90c3fada9f4102767a72619a12c9f872d` |

Antes de eso hubo que corregir cuatro cosas que ninguna prueba unitaria detecta, porque
todas pasan en local y fallan contra la red:

- **El enum se manda por nombre.** `AttestationOutcome` viaja como
  `Vec([Symbol("Pass")])`. Mandarlo como `U32` con el indice parece correcto —el
  contrato tiene ese enum— pero el host no lo convierte y el WASM hace
  `UnreachableCodeReached`. El error de la red es un trap sin mensaje util.
- **El `SignatureHint` son los ultimos 4 bytes de la pubkey.** Con el prefijo, que es
  lo que decia la especificacion antigua, la red busca una clave que no existe y
  responde `TxBadAuth` aunque la firma verifique perfectamente. Es el fallo mas caro
  de esta lista: el mensaje no distingue "firma mala" de "no encuentro la clave".
- **Los `struct` vuelven como `Map` con nombre**, no como `Vec` posicional, y los enums
  como `Vec[Symbol]`. El decodificador los leia por indice, lo que ademas era frágil
  ante un reordenamiento.
- **Los errores de simulacion llegan en un campo `error` con HTTP 200.** Sin
  comprobarlo, el sintoma era "no devolvio resultados", que no dice nada. Ese
  enmascaramiento costo una hora de diagnostico .

Y el toolchain paso a `stellar-xdr 28` y `stellar-rpc-client 28` para hablar con un
nodo en protocolo 28.

Todo el toolchain esta en protocolo 28 (`soroban-sdk 28`, `stellar-xdr 28`,
`stellar-rpc-client 28`). SDK 27 tambien funcionaba contra un nodo de protocolo 28, pero
es una mezcla fragile. Con SDK 28 el WASM **exige** `stellar contract build`; `cargo
build --target wasm32v1-none` falla con un error explicito que lo dice.

Los cuatro caminos del agente verificados con el toolchain unificado:

| Camino | Resultado |
| --- | --- |
| `attest` PASS y FAIL | tx `f7168ae3…` y `bb714f80…`, `report_hash` confirmado en cadena |
| `keeper` | detecta la evidencia y atesta solo, tx `abc98174…` |
| `finalize` | fallback split 3333 con reparto 3 333 000 000 / 6 667 000 000 |
| `keeper --finalize` | detecta el vencimiento y liquida solo, estado `Refunded` |

El token de los escrows de prueba es un contrato de prueba, **no CPUSD**: en testnet no
hay un SAC desplegado para una divisa de prueba y `fund()` necesita uno.

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
