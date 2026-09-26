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
| Frontend | Next.js/TypeScript + Freighter (D-007); hoy hay scaffold sin flujo implementado |
| Activo | CPUSD en testnet vía Stellar Asset Contract (`decimals()=7`) |

El tiempo lo decide siempre `env.ledger().timestamp()`, nunca el reloj del navegador. El contrato, el agente on-chain y la interfaz todavía no implementan el flujo descrito.

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

> 🚧 Los comandos se documentan a medida que se verifican en testnet.

El equipo debe configurar el workspace y verificar comandos reales de build, test y deploy en P0-00. Para comprobar los fixtures sintéticos localmente: `python3 scripts/verify_fixtures.py`.

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
