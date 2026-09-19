# CanguPay · máquina de estados P0

**Estado:** especificación de trabajo alineada con el alcance congelado; Gonza, Julián y Linder deben ratificar firmas, ABI y precisión del activo en P0-00. Una instancia Soroban representa **una** operación.

## Estados y operación normal

| Origen | Acción y actor | Condición adicional | Destino y efecto |
|---|---|---|---|
| Inicial | `initialize(config)` | configuración válida, solo una vez | `CREATED`; `EscrowCreated` |
| `CREATED` | `cancel()` · buyer | todavía no hay fondos | `CANCELLED`; `Cancelled`; no hay transferencia |
| `CREATED` | `fund()` · buyer | transferencia exacta y atómica | `FUNDED`; `Funded`; `submission_deadline=funded_at+submission_period` |
| `FUNDED` | `submit_evidence(hash)` · supplier | antes de `submission_deadline` | `EVIDENCE_SUBMITTED`; `EvidenceSubmitted(attempt=0)`; nuevo plazo de atestación |
| `EVIDENCE_SUBMITTED` | `attest(PASS, report_hash)` · engine | antes de `attestation_deadline` | `ATTESTED_PASS`; `Attested`; abre plazo de objeción |
| `EVIDENCE_SUBMITTED` | `attest(FAIL, report_hash)` · engine | antes de `attestation_deadline` | `ATTESTED_FAIL`; `Attested`; abre plazo de corrección |
| `ATTESTED_PASS` | `approve()` · buyer | antes de `action_deadline` | `RELEASED`; 100% al supplier; `Approved` |
| `ATTESTED_PASS` | `raise_dispute()` · buyer | antes de `action_deadline`; disputa no usada | `DISPUTED`; `DisputeRaised`; nuevo plazo de resolución |
| `ATTESTED_FAIL` | `submit_evidence(hash nuevo)` · supplier | antes de `action_deadline`; `correction_attempts<1` | `EVIDENCE_SUBMITTED`; intento pasa a 1, hash activo cambia y comienza nuevo plazo de atestación |
| `ATTESTED_FAIL` | `raise_dispute()` · supplier | antes de `action_deadline`; disputa no usada | `DISPUTED`; `DisputeRaised`; nuevo plazo de resolución |
| `DISPUTED` | `resolve(RELEASE/REFUND/SPLIT, bps)` · resolver | antes de `resolution_deadline` | `RELEASED`/`REFUNDED`/`SPLIT`; transfiere una vez; `Resolved` |

El buyer solo disputa un PASS; el supplier solo disputa un FAIL. El segundo FAIL puede disputarse durante su plazo, pero no corregirse de nuevo. `reason_hash` y `dispute_evidence_hash` son obligatorios en `raise_dispute()`. Buyer, supplier y resolver son las tres cuentas humanas de la demo; engine usa una cuenta técnica distinta.

## Vencimientos: únicamente `finalize()`

Todos los plazos comparan `now=env.ledger().timestamp()`. Las acciones ordinarias anteriores requieren `now < deadline`; exactamente en `deadline`, la acción ya no se admite y cualquier cuenta puede llamar `finalize()`. El keeper carece de permisos especiales.

| Estado al invocar `finalize()` | Si `now >= deadline` | Destino y `Finalized(reason)` |
|---|---|---|
| `FUNDED` sin evidencia | `submission_deadline` | `REFUNDED`; buyer recibe 100%; `SUBMISSION_TIMEOUT` |
| `EVIDENCE_SUBMITTED` sin atestación | `attestation_deadline` | `REFUNDED`; buyer recibe 100%; `ATTESTATION_TIMEOUT` |
| `ATTESTED_PASS` sin objeción | `action_deadline=attested_at+objection_period` | `RELEASED`; supplier recibe 100%; `NO_OBJECTION` |
| `ATTESTED_FAIL` sin corrección ni disputa | `action_deadline=attested_at+correction_period` | `REFUNDED`; buyer recibe 100%; `CORRECTION_TIMEOUT` |
| `DISPUTED` sin resolución | `resolution_deadline=disputed_at+resolution_period` | `RELEASED`/`REFUNDED`/`SPLIT` según fallback inmutable; `RESOLUTION_TIMEOUT` |

`finalize()` antes del deadline devuelve `NotFinalizableYet` sin efecto. En `CANCELLED`, `RELEASED`, `REFUNDED` o `SPLIT`, devuelve el estado terminal existente sin reescribir storage, transferir ni emitir otro evento. En `CREATED`, no existe timeout de fondeo en P0: `finalize()` no liquida. No existe una transferencia por cron oculta: sin una transacción `finalize()`, el saldo permanece en el contrato aunque haya vencido el reloj.

## Configuración y conservación

- `max_correction_attempts=1`; `submission_period`, `attestation_period`, `objection_period`, `correction_period` y `resolution_period` positivos y aprobados antes del fondeo.
- `fallback_outcome` es `RELEASE`, `REFUND` o `SPLIT`. Si es `SPLIT`, `fallback_split_bps` debe estar en `1..9999`; `split_bps>10000` se rechaza siempre. Elegir y documentar el manejo de `fallback_split_bps` cuando el outcome no sea SPLIT.
- En un split: `supplier_amount=floor(amount*split_bps/10000)`, `buyer_amount=amount-supplier_amount`. La suma conserva exactamente `amount` en unidades mínimas.
- Fondo y settlement deben ser atómicos. No se permite doble fondeo, segunda disputa, segunda resolución, objeción posplazo, acción post-terminal ni segundo payout.
- Eventos mínimos: `EscrowCreated`, `Funded`, `EvidenceSubmitted`, `Attested`, `Approved`, `DisputeRaised`, `Resolved`, `Finalized(reason)` y `Cancelled`. La corrección emite otro `EvidenceSubmitted` con `attempt=1`; no necesita un evento extra.

## Pruebas P0 que deben guiar a Gonza y la UI

| Escenario | Preparación | Resultado exigido |
|---|---|---|
| Ghost supplier | `FUNDED`, ninguna evidencia, avanzar ledger al `submission_deadline` | `finalize()` reembolsa; evento `SUBMISSION_TIMEOUT` |
| Engine ausente | evidencia presentada, sin atestación al vencimiento | `finalize()` reembolsa; `ATTESTATION_TIMEOUT` |
| Aprobación directa | PASS, buyer aprueba antes del plazo | un payout al supplier; `RELEASED` |
| Silencio del buyer | PASS, avanzar ledger al plazo de objeción | `finalize()` libera; `NO_OBJECTION` |
| Corrección | FAIL, nuevo hash dentro del plazo, segunda atestación | intento 1; otra corrección rechazada |
| Disputa única | PASS disputado por buyer o FAIL disputado por supplier | solo una vez, nunca después de resolución o estado terminal |
| Resolver ausente | `DISPUTED`, avanzar ledger al vencimiento | `finalize()` aplica fallback y `RESOLUTION_TIMEOUT` |
| Split inválido | `split_bps=10001` o extremos en outcome SPLIT | rechazar; saldo sin cambios |
| Idempotencia | llamar `finalize()` dos veces tras un timeout | mismo estado de retorno; una sola transferencia y un solo evento |
| Cancelación | `CREATED`, buyer cancela; luego intenta fondear | `CANCELLED`; fondeo rechazado; ningún pago |

Cada test de plazo debe probar `deadline-1`, `deadline` y `deadline+1` con time-travel de ledger, no con `sleep()` ni reloj del navegador. La demo usa tres instancias separadas para PASS, ghost supplier y disputa con split.
