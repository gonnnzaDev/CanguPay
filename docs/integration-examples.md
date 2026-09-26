# Integrar con el contrato

Material para quien integrate el frontend o un cliente. La ABI completa, generada
desde el propio WASM, está en
[`contracts/conditional-payment/abi/conditional_payment.json`](../contracts/conditional-payment/abi/conditional_payment.json).

## Red y despliegue

| | |
| --- | --- |
| RPC | `https://soroban-testnet.stellar.org` |
| Passphrase | `Test SDF Network ; September 2015` |
| Protocolo | 28 |
| Explorer | `https://stellar.expert/testnet/tx/<hash>` |

Un escrow desplegado y verificado:

```
CAXSLOYSXT7DETC26ED52V2LZ4SNCHSM6PI7JZ2Y36N2NZS4ZCPLP3GO
```

## Leer: un solo `snapshot()`

`snapshot()` **no pide `auth`** y devuelve de una vez todo lo que hace falta para
decidir: estado, configuración, hashes y **todos** los plazos, incluido el
timestamp del ledger. No hace falta encadenar varias lecturas ni calcular plazos a
mano.

```bash
stellar contract invoke --id CAXSLOYSXT7DETC26ED52V2LZ4SNCHSM6PI7JZ2Y36N2NZS4ZCPLP3GO \
    --source-account <cualquiera> --network testnet -- snapshot
```

Y lo mismo desde el agente, ya en JSON:

```bash
cargo run -p attestation-agent --bin cangu-attest -- status \
    --rpc https://soroban-testnet.stellar.org \
    --network "Test SDF Network ; September 2015" \
    --contract CAXSLOYSXT7DETC26ED52V2LZ4SNCHSM6PI7JZ2Y36N2NZS4ZCPLP3GO --json
```

```json
{
  "state": "Split",
  "amount": 10000000000,
  "engine": "G…",
  "token": "C…",
  "evidence_bundle_hash": "880df349…",
  "report_hash": "…",
  "attestation_deadline": 1790382512,
  "attestation_remaining": 3530,
  "correction_attempts": 0,
  "ledger": 4871079,
  "ledger_timestamp": 1790378982
}
```

`state` es el enum del contrato: `Created`, `Funded`, `EvidenceSubmitted`,
`AttestedPass`, `AttestedFail`, `Disputed`, `Released`, `Refunded`, `Split`,
`Cancelled`. Un estado desconocido llega como `Unknown`, nunca como `Created`.

## Invocar por rol

Cada operacion exige la firma del rol que le corresponde, y solo esa:

| Llamada | Quien firma | Notas |
| --- | --- | --- |
| `initialize` | buyer | una sola vez, todas las periodos > 0 |
| `fund` | buyer | pull atomico del token al contrato |
| `submit_evidence` | supplier | dos veces como maximo: inicial y una correccion |
| `attest` | **engine** | lo firma el agente, nunca el wallet del usuario |
| `approve` | buyer | solo tras `AttestedPass` y antes del plazo de objecion |
| `raise_dispute` | buyer desde PASS, supplier desde FAIL | dentro del plazo |
| `resolve` | resolver | solo en `Disputed` y antes de `resolution_deadline` |
| `finalize` | cualquiera | no pide `auth`; el contrato decide si vencio |

`attest` es la unica que el agente firma. El frontend **nunca** recibe la clave del
engine: vive en `CANGUPA_ENGINE_SECRET` o `CANGUPA_ENGINE_KEYFILE`, siempre fuera
del repositorio.

## Convenciones de Soroban que hay que respetar

Tres cosas que no son intuitivas y que hacen fallar la invocacion en silencio o con
un error poco descriptivo:

1. **Los `struct` son `Map` por nombre**, no un `Vec` posicional. Al invocar, el
   orden de las claves no importa pero los nombres si, y todos los campos son
   obligatorios.
2. **Los `enum` viajan como `Vec([Symbol("Nombre")])`**, no como el indice
   numerico. Pasar el indice hace que el contrato haga
   `UnreachableCodeReached` y la transaccion se caiga.
3. **El `SignatureHint` son los ultimos 4 bytes de la pubkey.** Con el prefijo, la
   red responde `TxBadAuth` aunque la firma sea valida. Afecta a wallets que
   construyan el sobre a mano; las libreras oficiales ya lo hacen bien.

## Decidir con el snapshot

Las preguntas que un cliente se hace antes de ofrecer una acción, y donde mirar:

| Pregunta | Campo |
| --- | --- |
| ¿Hay algo que hacer? | `state` |
| ¿El engine ya atesto? | `state` es `AttestedPass` o `AttestedFail` |
| ¿Qué atestiguó? | `report_hash` |
| ¿Que evidencia evaluo? | `evidence_bundle_hash` |
| ¿Cuanto tiempo queda? | `attestation_remaining`, `objection_deadline`, `correction_deadline`, `resolution_deadline` |
| ¿El proveedor ya corrigió? | `correction_attempts` (0 o 1) |

El reloj es `ledger_timestamp` del propio contrato, no el reloj local: sin eso un
cliente puede mostrar una accion que ya no esta disponible.

## Multiples wallets

Un rol por perfil de navegador: buyer, supplier, resolver. El engine no esta en
ninguno porque no lo firma ningun humano.

## Limitacion conocida

Los escrows de las pruebas usan un **contrato de token de prueba**, no CPUSD. En la
red publica de Stellar no hay un SAC desplegado para una divisa de prueba y
`fund()` necesita uno. Al integrar contra un token real, la direccion va en
`config.token` y nada mas cambia.
