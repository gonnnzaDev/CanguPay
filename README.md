# CanguPay

**Pago B2B condicionado sobre Stellar.**

El comprador reserva fondos en Stellar, el proveedor presenta la evidencia acordada, un agente verifica reglas documentales y un contrato Soroban libera, devuelve o divide los fondos según condiciones pactadas de antemano.

> *El agente informa; el contrato ejecuta; el humano resuelve controversias.*

> Nombre de trabajo del equipo. Chequeo preliminar de marca y dominio hecho el 25-09-2026 (búsqueda general, RDAP `.com`, DNS): sin coincidencias confundibles, dominios `cangupay.*` sin registrar. No equivale a clearance legal (ver D-006 en `docs/decision-log.md`).

## ⚠️ Declaraciones

- **CPUSD es un activo sintético de testnet utilizado exclusivamente para demostrar el flujo. No representa USDC ni tiene respaldo en dólares.** USDC en mainnet queda en el roadmap.
- **Doble participación:** una misma solución base compite en Stellar Odyssey Perú (Track 01 — AI Agents & Automated Workflows) y en Argentina Builder Challenge (Genesis — Pagos). Ambas participaciones están declaradas.
- **Khipu — antecedentes:** es aprendizaje previo del equipo. Este repositorio es **código nuevo** y no reutiliza código, fixtures ni archivos de Khipu.
- **Sin validación comercial:** es un prototipo basado en experiencia operativa, investigación secundaria e hipótesis explícitas. Cero respuestas registradas: los tres mensajes de validación asincrónica aún no se enviaron ([validation-log](docs/validation-log.md)).
- Todos los documentos de la demo son **sintéticos**. No es asesoría legal, ni producción, ni factoring, ni RWA.

## Fuente de verdad

| Qué | Dónde |
|---|---|
| Plan P0/P1, dueños y criterios de "terminado" | [`docs/issue-backlog.md`](docs/issue-backlog.md) · [issues en GitHub](https://github.com/gonnnzaDev/CanguPay/issues) |
| Decisiones y su estado (D-001…) | [`docs/decision-log.md`](docs/decision-log.md) |
| Reglas documentales, hashes y precisión | [`docs/ruleset.md`](docs/ruleset.md) |
| Máquina de estados y plazos | [`docs/state-machine.md`](docs/state-machine.md) |
| Validación de mercado (sin respuestas aún) | [`docs/validation-log.md`](docs/validation-log.md) |
| Escenarios de demo | [`docs/demo-scenarios.md`](docs/demo-scenarios.md) · guion en [`docs/pitch-script.md`](docs/pitch-script.md) |

## Problema

En una primera operación entre un proveedor pequeño y un comprador nuevo, el proveedor no quiere entregar sin saber si hay fondos, y el comprador no quiere pagar sin comprobar la entrega. CanguPay se enfoca en esa confianza previa y en ejecutar condiciones pactadas, no en resolver la morosidad en general.

## Cómo funciona

1. **Crear:** el comprador define proveedor, resolver, monto, plazos y fallback. Puede cancelar antes de fondear.
2. **Fondear:** deposita CPUSD en el contrato.
3. **Evidencia:** el proveedor envía el hash del bundle de evidencia.
4. **Atestación:** el agente aplica reglas deterministas y atestigua **PASS** o **FAIL**.
5. **PASS:** el comprador aprueba, disputa, o guarda silencio (se libera al vencer el plazo).
6. **FAIL:** el proveedor corrige (una vez), disputa, o vence el plazo y se reembolsa.
7. **Disputa** (una sola): un *resolver* humano decide `RELEASE`, `REFUND` o `SPLIT`. Si no actúa, se aplica el fallback preacordado.

Todos los vencimientos los ejecuta `finalize()`, una función pública e idempotente que cualquier cuenta puede invocar. Ningún silencio deja los fondos atrapados.

**Estados terminales:** `CANCELLED`, `RELEASED`, `REFUNDED`, `SPLIT`.

## Actores

| Actor | Rol |
|---|---|
| Buyer | Crea, fondea, aprueba o disputa un PASS |
| Supplier | Presenta evidencia, corrige o disputa un FAIL |
| Engine | Atestigua PASS/FAIL; no mueve fondos |
| Resolver | Decide solo si hay disputa |
| Keeper | Cualquier cuenta que llame a `finalize()` |

Los roles deben ser cuentas distintas entre sí.

## Arquitectura

| Componente | Tecnología |
|---|---|
| Contrato | Rust / Soroban (una instancia por operación) |
| Agente + keeper | Rust (D-007); hoy hay stubs y un verificador Python local de reglas |
| Frontend | Next.js 16 (App Router), TypeScript, Tailwind CSS v4, @stellar/freighter-api |
| Activo | CPUSD en testnet vía Stellar Asset Contract (`decimals()=7`) |

El tiempo lo decide siempre `env.ledger().timestamp()`, nunca el reloj del navegador.

## Estructura

```
contracts/conditional-payment/   # contrato Soroban
apps/web/                        # frontend
services/attestation-agent/      # agente y keeper
fixtures/{pass,fail,dispute}/    # evidencia sintética
scripts/{setup-testnet,demo}/
docs/                            # spec, decisiones, demos
```

## Puesta en marcha

### Requisitos

- Rust estable y el target `wasm32v1-none`.
- **`stellar` CLI 25.2 o superior** (probado con 28.0.0). No es opcional: desde
  `soroban-sdk` 28 el WASM **no** se compila con `cargo build --target wasm32v1-none`
  a secas, falla con un error que lo dice. Hay que usar `stellar contract build`.

### Build

```bash
# Contrato. Genera target/wasm32v1-none/release/conditional_payment.wasm
stellar contract build --package conditional-payment

# Token de prueba, solo para testnet (NO es CPUSD, ver Declaraciones)
stellar contract build --package test-token

# Regenerar el ABI commiteado
stellar contract info interface \
    --wasm target/wasm32v1-none/release/conditional_payment.wasm \
    --output json > contracts/conditional-payment/abi/conditional_payment.json
```

### Test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

169 tests. Reparto de los tx hashes en consola: 105 del agente (unitarios), 5 de
paridad con fixtures, 59 del contrato.

Paridad con el motor de reglas de Python, que es el oráculo:

```bash
cargo test -p attestation-agent --test golden
python3 services/attestation-agent/app/engine.py <bundle.json>   # referencia
```

### Deploy en testnet

```bash
export NETWORK=testnet
export PASSPHRASE="Test SDF Network ; September 2015"

# 1. Cuentas. Se generan fuera del repo y NO se commitean.
stellar keys generate deployer
stellar keys generate buyer
stellar keys generate supplier
stellar keys generate engine
stellar keys generate resolver
for r in deployer buyer supplier engine resolver; do
  curl -s "https://friendbot.stellar.org/?addr=$(stellar keys public-key $r)"
done

# 2. Desplegar
stellar contract deploy --source-account deployer --network $NETWORK \
    --wasm target/wasm32v1-none/release/conditional_payment.wasm

# 3. Montar un escrow (el orden de claves del --config es el del ABI, por nombre)
TOKEN=<contract id del token de prueba>
ESCROW=<contract id del escrow>
AMOUNT=10000000000   # unidades minimas: 1000 con 7 decimales

stellar contract invoke --id $TOKEN --source-account buyer --network $NETWORK \
    -- initialize --admin <buyer>

stellar contract invoke --id $TOKEN --source-account buyer --network $NETWORK \
    -- mint --to <buyer> --amount $AMOUNT

CONFIG=$(cat <<JSON
{ "amount": "$AMOUNT", "attestation_period": 3600, "buyer": "<buyer>",
  "correction_period": 3600, "engine": "<engine>", "fallback_outcome": 2,
  "fallback_split_bps": 0, "max_correction_attempts": 1, "objection_period": 3600,
  "resolution_period": 3600, "resolver": "<resolver>", "submission_period": 3600,
  "supplier": "<supplier>", "token": "$TOKEN" }
JSON
)
stellar contract invoke --id $ESCROW --source-account buyer --network $NETWORK \
    -- initialize --config "$CONFIG"
stellar contract invoke --id $ESCROW --source-account buyer --network $NETWORK -- fund
```

#### Por qué el montaje usa el CLI y no un script propio

Existió un `examples/testnet_setup.rs` en el agente que hacia el montaje por su cuenta.
Se borró. Copiaba la lógica de firma en vez de delegar en la de producción, y arrastró
los tres fallos que esta capa ya no tiene:

- la pista de firma con el **prefijo** de la pubkey, que hace que la red responda
  `TxBadAuth` aunque la firma verifique;
- cero manejo de las credenciales `SourceAccount`, que aparecen cuando la cuenta que
  exige `auth` es la propia fuente de la transacción;
- el enum enviado por índice en vez de por nombre, que hace que el contrato entre en
  `UnreachableCodeReached`.

Compilaba sin errores, así que nadie lo habría detectado hasta usarlo. Mantener una
segunda implementación de firma junto a la buena es una forma de reintroducir
los fallos que la primera ya corrigió. Cuando se monte un escrow en testnet, se usa el
CLI de Stellar, que además sirve de control: si una transacción entra por el CLI y no
por el agente, el problema está en el agente y no en la red.

### Verificar en la red

```bash
# 4. El proveedor entrega evidencia. El hash debe ser el que calcula el motor.
cargo run -p attestation-agent --bin cangu-attest -- evaluate \
    --bundle evidence.json --amount 10000000000 --currency CPUSD --json

stellar contract invoke --id $ESCROW --source-account supplier --network $NETWORK \
    -- submit_evidence --evidence-bundle-hash <evidence_bundle_hash>

# 5. El agente lee el estado real del contrato
cargo run -p attestation-agent --bin cangu-attest -- status \
    --rpc https://soroban-testnet.stellar.org --network "$PASSPHRASE" --contract $ESCROW

# 6. El agente atesta. Firma con la clave engine, NUNCA con la del buyer.
export CANGUPA_ENGINE_SECRET=<secreto de la cuenta engine, fuera del repo>
cargo run -p attestation-agent --bin cangu-attest -- attest \
    --rpc https://soroban-testnet.stellar.org --network "$PASSPHRASE" \
    --contract $ESCROW --bundle evidence.json --amount 10000000000

# 7. O bien el keeper, que vigila y atesta solo cuando la evidencia esta lista
cargo run -p attestation-agent --bin cangu-attest -- keeper \
    --rpc https://soroban-testnet.stellar.org --network "$PASSPHRASE" \
    --contract $ESCROW --bundle evidence.json --amount 10000000000 \
    --poll 10 --max-iterations 60

# 8. Vencimientos. finalize() no exige auth y el contrato decide si el plazo vencio.
cargo run -p attestation-agent --bin cangu-attest -- finalize \
    --rpc https://soroban-testnet.stellar.org --network "$PASSPHRASE" --contract $ESCROW
```

### Contratos y transacciones en testnet

Ejecutado contra `soroban-testnet.stellar.org` (protocolo 28) con toolchain
unificado en SDK 28. Todos los tx hashes son publicos y verificables en
`https://stellar.expert/testnet/tx/<hash>`.

| Contrato | ID | Qué demuestra |
| --- | --- | --- |
| Escrow flujo feliz | `CCGZQCVPZPJLTTD4NSCL7MZWFH6PKFAAZCSGXCPZH7ZEB7T56L2WJ2OF` | `attest` PASS desde el agente |
| Escrow fallo | `CCQ7XTWYMGKMTBPJ5UIJ764GDQX7ZE2HYSZ3EPW4XUELSC7755SPNJ7A` | `attest` FAIL desde el agente |
| Escrow keeper | `CDUEGOAJXOG5VD6M4I7SMVEV2S6MNAQKBPOWUY5UZQ7AJIN6U7Q4OHEU` | el keeper atesta solo |
| Escrow fallback split | `CB6SQ2N2IWDS36HVDF7CXB2RPVJVJLCYEKERR5EOCEEJR656F7TH4OJY` | `finalize` con split 3333 |
| Escrow keeper finalize | `CD4GXNR7F5MSFZGJ6SMMNRXTICTUYRRM2UV5UNDKXZXDW3ORKR275YXD` | el keeper liquida solo |
| Escrow corrección | `CAXSLOYSXT7DETC26ED52V2LZ4SNCHSM6PI7JZ2Y36N2NZS4ZCPLP3GO` | correccion, 2a rechazada, disputa, `resolve` split 5000 |
| Escrow ghost supplier | `CAY7JN2LLSYJNDTQNLHGB6ZGSOB3UFSTVINBSC5KRL2BDF5SAGWGZFOD` | `Finalized(reason: 1)` y reembolso |
| Escrow ghost engine | `CC6HQPN27COJTIXBQOP247PGHWIBBYS7CRNEWDI77JO52PH2JBY5JPB6` | `Finalized(reason: 2)` y reembolso |
| Token de prueba | `CDZOKMFYQ55D4IWBGMIZHYV2HYP7L4JJ5ABJ4K5KTQ2OS5WNDZCMTNAB` | `transfer`/`balance` del fondeo |

Transacciones de los caminos del agente:

| Camino | tx hash |
| --- | --- |
| `attest` PASS | `f7168ae3e3c0f4e7cf59bc66353f7d9f3e24510043144dcf4864b2f003e1976f` |
| `attest` FAIL | `bb714f80fe2acc5f6b8192fd5ec106a90c3fada9f4102767a72619a12c9f872d` |
| `keeper` atesta solo | `abc981745d898e23190a8e46758508b1b5bd438fff38b1a8f77ac45613623e52` |

El token de la tabla es un **contrato de prueba**, no CPUSD: en la red publica de
Stellar no hay un SAC desplegado para una divisa de prueba y `fund()` necesita uno.
Declarado tambien en la nota de CPUSD de este README.

Usá **tres perfiles de navegador** con Freighter (buyer, supplier, resolver) para no firmar con el rol equivocado.

## Demos

1. **PASS + vencimiento** (principal): el comprador no objeta, cualquiera ejecuta `finalize()` y el proveedor cobra.
2. **Ghost supplier** (respaldo): el proveedor desaparece y el comprador recupera los fondos.
3. **Disputa + split 70/30:** el resolver decide y el contrato distribuye exactamente.

Las demos usan plazos abreviados. Guion de presentación en [`docs/pitch-script.md`](docs/pitch-script.md); pasos de reproducción en `docs/demo-scenarios.md` (por escribir en P0-11).

## Limitaciones

- Engine centralizado (una sola clave). En producción requeriría quórum o engines redundantes.
- Resolver único; issuer centralizado en testnet.
- Sin fees, KYC/AML ni off-ramp.
- Los vencimientos no son automáticos: alguien debe invocar `finalize()`.

## Alcance

**P0:** flujo completo en testnet (contrato, agente determinista, frontend, tests, video). Detalle, dueños y criterios en [`docs/issue-backlog.md`](docs/issue-backlog.md) y en [los issues #2–#14](https://github.com/gonnnzaDev/CanguPay/issues).
**P1:** IA/OCR, múltiples hitos, engine redundante, segundo resolver, fees, factory multi-escrow.
**Fuera:** factoring, RWA, yield, préstamos, KYC, ERP, mainnet.

## Demo, testnet y video

> ⏳ **Pendiente (P0-11).** Este bloque queda reservado para los enlaces verificables cuando la demo corra en testnet.

- **Contrato (testnet):** `_[contract id pendiente]_`
- **Transacciones de las tres demos:** `_[tx hashes pendientes]_`
- **Explorer:** `_[enlace a explore.stellar.org testnet, pendiente]_`
- **Video:** `_[enlace pendiente]_`
- **Cómo repetir la demo:** `docs/demo-scenarios.md` (contenido pendiente de P0-11)

Mientras esos campos estén vacíos, no se afirma que la demo haya corrido on-chain.

## Equipo

**Gonza** ([@gonnnzaDev](https://github.com/gonnnzaDev)) — contrato y testnet · **Julián** ([@Julianv3534](https://github.com/Julianv3534)) — frontend e integración · **Linder** — producto, agente, evidencia y presentación · **Rendo(ForLess01)** — frontend/integración (pareja de Julián en P0-08)

Equipo de 4 en ambas competencias; elegibilidad registrada en D-008 de [`docs/decision-log.md`](docs/decision-log.md).

## Licencia

MIT — ver [`LICENSE`](LICENSE).
