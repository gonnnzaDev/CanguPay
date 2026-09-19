# Registro de decisiones · CanguPay

Las entradas **confirmadas** reflejan lo acordado para el prototipo. Las decisiones de interfaz pendientes se identifican explícitamente y requieren revisión de Gonza, Julián y Linder antes de integrar piezas.

## D-001 · Alcance P0 congelado — 19-09-2026

**Estado:** confirmado en el plan de trabajo comunicado por Linder; revisión técnica del equipo pendiente.

**Decisión:** un escrow por instancia Soroban en Stellar testnet, activo CPUSD sintético vía SAC, reglas documentales deterministas, una corrección, una disputa, resolver humano, `finalize()` público/idempotente como única función que ejecuta vencimientos. El supplier fantasma obtiene refund por submission timeout; el engine ausente por attestation timeout; el resolver ausente aplica fallback preacordado. Ledger timestamp define plazos. Demos con tres cuentas humanas Freighter y cuentas técnicas separadas.

**Fuera de P0:** Scale, RWA, factoring, OCR, IA de extracción, USDC mainnet, casos reales, integración ERP. P1 se decide al tener P0 estable. Tres mensajes de validación asincrónica, recapitulación a organizadores y chequeo de nombre son actividades paralelas; ninguna bloquea escribir el prototipo. No se declara ninguna respuesta positiva inexistente.

**Motivo:** mantener una demo comprobable de reglas, custodia y pagos, sin dependencia de entrevistas ni servicios externos que puedan impedir el flujo principal.

## D-002 · Nombre CanguPay — 19-09-2026

**Estado:** confirmado por Linder y reflejado en el nombre del repositorio. `CumplePago` es el nombre anterior y debe sustituirse en piezas del equipo. Esto no equivale a una revisión de marca o dominio disponible: anotar el resultado real del chequeo cuando se realice.

## D-003 · Interfaz web y agente — pendiente

**Observación:** `README.md` describe Next.js/TypeScript + Freighter y FastAPI/Python; el scaffold actual incluye `apps/web/src/main.rs` (comentarios de Actix) y `services/attestation-agent/src/lib.rs` (comentarios Rust). Ninguno implementa aún una aplicación o un servicio operativo. El `Cargo.toml` raíz declara un paquete sin `src/` ni miembros workspace definidos.

**Decisión requerida (P0-00):** Gonza y Julián acuerdan si la web será cliente Next.js/TypeScript y el agente Python, con eliminación/traslado de stubs Rust, o si proponen una alternativa que cubra Freighter, API/keeper y build repetible. Documentar la elección, el ABI y el reparto de ownership antes de integrar.

## D-004 · Formato de evidencia — propuesto, pendiente de ratificación

**Propuesta de Linder:** objetos JSON de `fixtures/`, `amount` entero en unidades mínimas CPUSD (10⁻⁷); canonicalización y hashes en `docs/ruleset.md`. Las tres piezas del bundle se hashean juntas; la disputa se registra por separado. Gonza verifica precisión SAC y representación `BytesN<32>`; Julián confirma cómo la UI construye exactamente el mismo bundle.

**Fixture de la demo 3:** `PO-003` obtiene PASS; luego el buyer cuestiona la entrega parcial. La objeción es una alegación sintética y no modifica automáticamente la atestación. Esta demo sigue el camino `ATTESTED_PASS → DISPUTED → SPLIT`.

## D-005 · Backlog P0 publicado en GitHub — 19-09-2026
Estado: confirmado por Linder tras crear los 13 issues manualmente.
Decisión: publicar el backlog P0 con dueño y label en https://github.com/gonnnzaDev/CanguPay/issues y crear los labels `P0`, `P1`, `bloqueado` y `demo`. Los números `#` los asigna GitHub; los códigos internos se mantienen como referencia de equipo. Mapeo real:
| Código interno | Issue GitHub |
| --- | --- |
| P0-00 | #2 |
| P0-01 | #3 |
| P0-02 | #4 |
| P0-03 | #5 |
| P0-04 | #6 |
| P0-05 | #7 |
| P0-06 | #8 |
| P0-07 | #9 |
| P0-08 | #10 |
| P0-09 | #11 |
| P0-10 | #12 |
| P0-11 | #13 |
| P0-12 | #14 |
Reglas derivadas: todo PR escribe `Closes #N` con el número de esta tabla; si un issue se cierra y se recrea, se actualiza esta tabla el mismo día; el tablero Projects es opcional y no condiciona el desarrollo.

## Registro de cambios

| Fecha | Decisión | Responsable | Efecto |
|---|---|---|---|
| 19-09-2026 | D-001 y D-002 | Linder | Congelar P0 y actualizar el nombre |
| 19-09-2026 | D-003 y D-004 abiertos | Equipo | Cerrar stack, monto y ABI antes de integración |
| 19-09-2026 | D-005: backlog P0 publicado (#2–#14) y labels creados | Linder | Trazabilidad códigos internos ↔ issues GitHub |
