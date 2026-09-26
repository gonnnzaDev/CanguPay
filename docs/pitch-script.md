# Guion de pitch y video · CanguPay

Uso: pitch oral (90 s y 3 min) y grabación de demo. Dueño: Linder (P0-12). El video final se graba cuando P0-11 deje contract IDs y tx hashes reales.

## Reglas de lenguaje (obligatorias)

**Nunca decir:**

- "USDC" para el activo de prueba → decir **"CPUSD, activo sintético de testnet"**.
- "RWA", "factoring", "tokenización de facturas", "crédito", "préstamo".
- "producción", "regulado", "auditoría", "validado por clientes".
- "funciona en mainnet" o "ya está operativo".

**Decir siempre:**

- "prototipo en testnet", "evidencia sintética", "sin dinero real".
- "código nuevo; Khipu es aprendizaje previo del equipo".
- "sin validación comercial: cero respuestas de mercado hasta la fecha".

## Pitch de 90 segundos

**0:00 — Problema (15 s)**
"Un proveedor pequeño entrega a un comprador que acaba de conocer. El proveedor no quiere entregar sin saber si hay fondos; el comprador no quiere pagar sin comprobar la entrega. Ese primer trato se cae por desconfianza, no por falta de producto."

**0:15 — Solución (15 s)**
"CanguPay condiciona el pago: el comprador reserva los fondos en un contrato en Stellar, el proveedor presenta la evidencia acordada, un agente verifica reglas documentales y el contrato libera, devuelve o divide el dinero según lo que ambos pactaron antes."

**0:30 — Frase central (5 s)**
"El agente informa; el contrato ejecuta; el humano resuelve controversias."

**0:35 — Cómo funciona (25 s)**
"Se crea el acuerdo con proveedor, resolver, monto, plazos y fallback. Se fondea con CPUSD en testnet. El proveedor envía el hash de su paquete de evidencia. El agente atestigua PASS o FAIL con reglas deterministas. Con PASS, el comprador aprueba, disputa o guarda silencio: al vencer el plazo, cualquiera puede llamar a `finalize()` y el proveedor cobra. Con FAIL, hay una corrección, una disputa o reembolso por vencimiento. Si hay disputa, un resolver humano decide RELEASE, REFUND o SPLIT; si no actúa, se aplica el fallback pactado. Ningún silencio deja el dinero atrapado."

**1:00 — Demo (15 s)**
"Acá está corriendo en testnet" *(se muestra la pantalla con el explorer)*. "Estos son el ID del contrato y los hashes reales de las tres transacciones."

**1:15 — Declaraciones y equipo (15 s)**
"Prototipo con evidencia sintética, sin validación comercial. CPUSD es un activo de prueba sin respaldo en dólares; USDC mainnet está en el roadmap. Una misma solución compite en Stellar Odyssey Perú y en Argentina Builder Challenge; Khipu fue aprendizaje previo y este repo no reutiliza su código. El equipo es Gonza, Julián, Linder y ForLess01."

## Pitch de 3 minutos (mismo orden, más detalle)

1. **Problema (30 s):** primera operación B2B entre desconocidos; el riesgo no es la tecnología sino la confianza previa. Explícitamente: no atacamos la morosidad en general.
2. **Solución (30 s):** pago condicionado; el contrato es el que retiene, no un intermediario. Un escrow por instancia, activo CPUSD vía Stellar Asset Contract con 7 decimales.
3. **Flujo (45 s):** crear → fondear → evidencia → atestación → PASS/FAIL → acción humana o vencimiento → `finalize()` público e idempotente. Estados terminales: `CANCELLED`, `RELEASED`, `REFUNDED`, `SPLIT`.
4. **Demo en vivo o video (45 s):** demo 1 PASS por vencimiento; corte de demo 3 disputa + split 70/30. Se muestran contract ID, tx hashes y saldos antes/después.
5. **Limitaciones (15 s):** engine centralizado, resolver único, issuer de testnet, sin fees ni KYC, vencimientos no automáticos.
6. **Equipo, competencias y roadmap (15 s):** P0 completo en testnet; P1 con OCR/IA, hitos múltiples y engine redundante; fuera de alcance factoring, RWA, yield y mainnet.

## Guion del video (P0-11)

| Tiempo | Plano | Qué se muestra | Qué NO se muestra |
|---|---|---|---|
| 0:00–0:10 | Pantalla de bienvenida | Nombre CanguPay, red testnet visible | Ninguna cifra de dinero real |
| 0:10–0:40 | Demo 1 | Crear → fondear → evidencia → PASS → `finalize()` → cobro | Estados forzados a mano |
| 0:40–1:00 | Demo 3 (corte) | Disputa del buyer → resolver decide `SPLIT` 70/30 → balances exactos | Simulaciones presentadas como on-chain |
| 1:00–1:20 | Explorer | Contract ID, hashes de tx, saldos | Archivos privados o claves |
| 1:20–1:35 | Declaraciones | Locución de activo sintético, sin validación comercial, doble competencia | — |

Reglas del video:

- Toda operación mostrada usa cuentas testnet y estados reales del contrato.
- Los plazos están abreviados y se declara como tal.
- Se guardan contract IDs, tx hashes, saldos iniciales/finales y grabación de respaldo.
- Si algo falla en grabación, se repite; no se parchea en edición.

## Preguntas esperadas

- **¿Es USDC?** No. Es CPUSD, un activo sintético de testnet sin respaldo; existe para probar el flujo.
- **¿Es factoring o RWA?** No. No compramos cartera ni tokenizamos activos; solo condicionamos un pago.
- **¿Ya lo validaron con clientes?** No. Cero respuestas de mercado hasta la fecha; el log está en `docs/validation-log.md`.
- **¿Puedo repetirlo?** Con los enlaces de testnet del README y `docs/demo-scenarios.md` (P0-11). Verificación local de evidencia: `python3 scripts/verify_fixtures.py`.
- **¿Qué pasa si nadie ejecuta `finalize()`?** Los fondos no se pierden: cualquier cuenta puede invocarla; el vencimiento es ejecutable por cualquiera, no automático.
