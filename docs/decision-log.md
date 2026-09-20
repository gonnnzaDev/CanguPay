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

## D-006 · Chequeo preliminar de nombre CanguPay — 19-09-2026
Estado: confirmado por Linder.
Fuentes consultadas: [lista real: búsqueda general, GitHub, dominios, redes, INDECOPI/INPI si aplicaste]
Coincidencias encontradas: ninguna confundible en las fuentes consultadas.
Riesgo: [bajo/medio/alto según lo observado]
Decisión: mantener CanguPay como nombre de trabajo para la hackatón.
Aclaración: chequeo preliminar; no equivale a clearance legal de marca en Perú ni Argentina.

## D-007 · Stack del agente y cierre parcial de D-003/D-004 — 19-09-2026
Estado: stack confirmado por Gonza y Julián; mecanismo de lectura on-chain PENDIENTE de su reunión (20 o 21-09).

Confirmado:
1. Stack: Rust para el contrato Soroban y Rust para el agente/orquestador; Next.js/TypeScript para el frontend. Los stubs de `services/attestation-agent/src/` se implementan, no se eliminan. Cierra la parte de stack de D-003.
2. Precisión del activo: el SAC devuelve `decimals()` fijo en 7, coherente con `amount` en unidades mínimas 10⁻⁷ de `docs/ruleset.md`. Ratifica la precisión pendiente de D-004.
3. Las firmas del contrato están implementadas; revisión de enums a cargo de Linder contra la spec §10.3.

Propuesta registrada, NO confirmada (se define en la reunión Gonza-Julián):
- Lectura on-chain vía Stellar RPC (`getLedgerEntries`/`getContractData`), no Horizon.
- Flujo híbrido de datos: el supplier envía el bundle al agente; el agente calcula su hash y lo contrasta con `evidence_bundle_hash`, `amount` y `token` leídos del contrato; si coinciden, evalúa y firma `attest()`.
- Función `get_escrow()` read-only que exponga esos campos.

Impacto:
- El motor Python de P0-07 etapa 1 queda como referencia ejecutable de reglas y tests (29 tests verdes, oráculo contra `fixtures/manifest.json`); la implementación operativa del agente será Rust (P0-07 etapa 2).
- P0-07 etapa 2 permanece bloqueada hasta que Gonza cierre P0-00 con el mecanismo de lectura y la ABI.

Registro: la decisión final de lectura/ABI se anotará en una próxima entrada tras la reunión
Gonza-Julián.

## D-008 · Integración de ForLess01 al equipo completo — 20-09-2026
Estado: confirmado por decisión de equipo (Linder, Gonza, Julián).
Decisión: ForLess01 integra el equipo de CanguPay en ambas competencias: Stellar Odyssey Perú y Argentina Builder Challenge.
Elegibilidad Argentina: equipo de 4 con 2 residentes en Argentina (gonnnzaDev y Julianv3534) = 50%, cumple el mínimo; tamaño 2–4 cumple.
Elegibilidad Perú: Linder y ForLess01 residen en Perú.
Rol: frontend/integración; esta semana pareja de Julián en P0-08; dueño de P0-10 cuando aterrice; una issue activa a la vez.
Atribución: sus commits y PRs en el repo compartido se atribuyen al mismo equipo en ambas presentaciones.

## D-009 · Incidente de credencial en docs/test.md — 20-09-2026
Estado: reportado por revisión externa el 20-09-2026; verificación del commit y rotación EN CURSO. Esta entrada se marcará como confirmada únicamente cuando Gonza confirme la rotación y el archivo esté retirado del ref público correspondiente.
Hecho: el commit 92990c4 llevó a main un archivo con la frase de recuperación de una cuenta, en repo público.
Acciones pendientes: cuenta tratada como comprometida y dejada de usar; cuentas nuevas para los roles afectados; archivo retirado de main; si la frase se usó en otra red, esa cuenta también rotada; secret scanning activado en P0-05.
Nota: borrar el archivo no limpia el historial público; la rotación es la mitigación real. Ninguna credencial vuelve al repo: solo placeholders en .env.example.

## Registro de cambios

| Fecha | Decisión | Responsable | Efecto |
|---|---|---|---|
| 19-09-2026 | D-001 y D-002 | Linder | Congelar P0 y actualizar el nombre |
| 19-09-2026 | D-003 y D-004 abiertos | Equipo | Cerrar stack, monto y ABI antes de integración |
| 19-09-2026 | D-005: backlog P0 publicado (#2–#14) y labels creados | Linder | Trazabilidad códigos internos ↔ issues GitHub |
| 19-09-2026 | D-007: stack Rust confirmado; lectura on-chain pendiente de reunión | Gonza, Julián | Cierra stack de D-003 y precisión de D-004; P0-07 etapa 2 sigue bloqueada |
| 20-09-2026 | D-008: ForLess01 integra ambas competencias | Equipo | 2 de 4 = 50% Argentina, cumple elegibilidad |
| 20-09-2026 | D-009: incidente de credencial reportado (rotación pendiente) | Linder | Trazabilidad del incidente; confirmación pendiente de Gonza |
