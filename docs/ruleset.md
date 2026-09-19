# CanguPay · reglas de verificación P0 (v1.0.0)

**Estado:** propuesta implementable para revisión de Gonza y Julián. Todos los datos de `fixtures/` son inventados. Esta verificación no decide disputas ni libera fondos: el contrato hace los pagos y el resolver humano atiende las disputas.

## Unidad, entrada y salida

El activo de demostración `CPUSD` es sintético y vive únicamente en testnet. En estos fixtures, `amount` es un **entero en unidades mínimas de 10⁻⁷ CPUSD**: `10000000000` representa `1000.0000000 CPUSD`. No admitir decimales JSON, floats ni montos negativos. Gonza debe confirmar que el activo emitido y la interfaz del contrato emplean esa precisión antes de integrar el servicio. El monto de la orden debe coincidir con el `amount` configurado en la operación; la moneda debe ser `CPUSD`.

El bundle presentado por el proveedor es el objeto formado por tres archivos, con las claves exactas `purchase_order`, `invoice` y `delivery`. Ejemplo:

```json
{
  "purchase_order": {"id": "PO-001", "supplier_id": "SUP-001", "amount": 10000000000, "currency": "CPUSD"},
  "invoice": {"id": "INV-001", "purchase_order_id": "PO-001", "supplier_id": "SUP-001", "amount": 10000000000, "currency": "CPUSD"},
  "delivery": {"purchase_order_id": "PO-001", "accepted": true}
}
```

La comparación necesita también `escrow_amount` (entero en unidades mínimas) y `escrow_token_code` (`CPUSD`), leídos del contrato o de la configuración verificada de esa operación. El engine **no** acepta estos dos valores desde un formulario sin contrastarlos con el contrato. El `supplier_id` de los documentos es un identificador comercial sintético; su asociación con la cuenta Stellar `supplier` debe confirmarse fuera del documento y queda como decisión de interfaz en P0-00.

## Normalización y reglas

1. Todas las claves indicadas son obligatorias. `id`, `purchase_order_id`, `supplier_id` y `currency` deben ser cadenas ASCII no vacías; eliminar espacios ASCII solo al principio y al final y pasar las letras `a-z` a mayúsculas. No borrar espacios internos ni inferir IDs ausentes.
2. `amount` debe ser un entero JSON positivo. Un booleano JSON no es un monto válido. Comparar enteros, sin redondeos ni tolerancias. `accepted` debe ser el booleano JSON `true`.
3. `supplier`: `purchase_order.supplier_id == invoice.supplier_id` después de normalizar.
4. `purchase_order`: `purchase_order.id == invoice.purchase_order_id == delivery.purchase_order_id`.
5. `amount`: `purchase_order.amount == invoice.amount == escrow_amount`.
6. `currency`: ambas monedas documentales coinciden con `escrow_token_code == CPUSD`.
7. `delivery`: `delivery.accepted == true`.
8. Si falta un campo obligatorio, el check afectado devuelve `MISSING`; si es de tipo incorrecto, `INVALID`; si está presente pero difiere, `MISMATCH`; si cumple, `MATCH`. No convertir un dato ausente o ambiguo en un dato plausible. El resultado global es `PASS` solo cuando **todos** los cinco checks son `MATCH`; de otro modo, `FAIL`.

Si el JSON no se puede decodificar, hay claves duplicadas, el bundle no es un objeto de tres secciones, o el hash de entrada no coincide con el que presentó el proveedor, rechazar la solicitud **antes** de `attest()`; no crear un reporte que afirme haber examinado una evidencia distinta.

## Hashes y reporte reproducible

La representación canónica de P0 es JSON UTF-8 con claves ordenadas alfabéticamente en cada objeto, sin espacios prescindibles, sin BOM, sin saltos de línea, sin claves duplicadas y sin números decimales. Los identificadores y códigos del bundle son ASCII; el texto libre `reason` de la disputa puede contener Unicode. Esta restricción del bundle evita divergencias entre serializadores; ampliar el esquema exige acordar una canonicalización interoperable como JCS. `SHA-256` produce 32 bytes; intercambiarlos como 64 dígitos hexadecimales minúsculos en documentos y como `BytesN<32>` al llamar al contrato.

- `evidence_bundle_hash = SHA-256(canonical_json({purchase_order,invoice,delivery}))`.
- `report_hash = SHA-256(canonical_json(report))`.
- El reporte contiene exactamente `ruleset_version`, `result`, `checks` y `evidence_bundle_hash`, sin `generated_at`, hora local, nombres de archivos ni datos privados. Si se necesita hora, mostrar la del evento on-chain separadamente.
- Los cinco checks se llaman `supplier`, `purchase_order`, `amount`, `currency` y `delivery`.
- `dispute-evidence.json` se calcula y presenta por separado; nunca se agrega a posteriori al hash del bundle ya atestiguado.
- Según el script actual: `reason_hash = SHA-256(canonical_json({"reason": texto_de_reason}))` y `dispute_evidence_hash = SHA-256(canonical_json(dispute-evidence.json))`. Estos son **dos hashes distintos**; Gonza y Julián deben usar exactamente los mismos bytes al invocar `raise_dispute(reason_hash, dispute_evidence_hash)`.

`fixtures/manifest.json` contiene hashes de referencia calculados con `python3 scripts/verify_fixtures.py`. Es una herramienta de comprobación local: todavía no firma ni envía transacciones.

Reporte esperado PASS (el hash concreto figura en el manifest):

```json
{
  "ruleset_version": "1.0.0",
  "result": "PASS",
  "checks": {"supplier": "MATCH", "purchase_order": "MATCH", "amount": "MATCH", "currency": "MATCH", "delivery": "MATCH"},
  "evidence_bundle_hash": "<64 hex del bundle PASS>"
}
```


### Casos de aceptación

| Caso | Evidencia | Valor contractual supuesto | Resultado esperado | Efecto permitido |
|------|-----------|---------------------------|-------------------|------------------|
| PASS | fixtures/pass/ | 10000000000 | Todos los checks MATCH; PASS | Engine atestigua; buyer aprueba o vence objeción y alguien llama finalize() |
| FAIL | fixtures/fail/ | 10000000000 | Solo amount=MISMATCH; FAIL (factura 9000000000) | Supplier corrige una vez, disputa durante plazo o recibe refund por finalize() |
| DISPUTE | fixtures/dispute/dispute-evidence.json tras PASS | 10000000000 | No reescribe el PASS ni dispara un FAIL | Buyer abre su única disputa y resolver humano valora la evidencia nueva; fallback si no resuelve |

En DISPUTE, el comprador alega que solo 8 de 10 bultos llegaron conformes. `dispute-evidence.json` recoge **esa alegación sintética**, sin adjuntar un acta independiente que la pruebe. El PASS previo no cambia automáticamente: decide el resolver humano. `fixtures/dispute/` contiene `PO-003`, diferente de `fixtures/pass/` (`PO-001`); ambos bundles obtienen PASS. PASS/FAIL indican coincidencia de campos y constancia marcada como aceptada en la muestra, no prueba legal de entrega.

## Pendientes de interfaz para P0-00

- Elegir esquema exacto de autorización del engine y quién custodia su clave fuera del frontend.
- Confirmar dónde se enlaza el `supplier_id` comercial con la cuenta `supplier` on-chain.
- Confirmar el `escrow_id`, precisión real del activo CPUSD y el formato de bytes del hash de disputa.
- Fijar los nombres/códigos del enum de atestación y el ABI antes de cablear cliente y servicio.
