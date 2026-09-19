# CanguPay — Issues P0 listos para GitHub

**Fecha:** 19 de septiembre de 2026

**Fuente de verdad:** acuerdo P0 del equipo y `docs/decision-log.md`, `docs/state-machine.md` y `docs/ruleset.md`; la especificación larga previa aún no se incorporó al repositorio.
**Uso:** crear un issue por bloque y asignar a la cuenta GitHub real de su dueño. Las siglas son referencias internas; los números `#` los asigna GitHub. No se asumen handles.

**Estado del repositorio al revisarlo:** los cinco documentos `docs/` originales eran archivos vacíos; `README.md` conservaba el nombre CumplePago; los archivos Rust del contrato, agente y web contenían solo comentarios; `Cargo.toml` raíz declara un paquete sin `src/` ni miembros workspace. Existía un solo issue de prueba **cerrado** (`#1`); ningún issue P0 estaba publicado. Este backlog se adaptó a esos hechos. Los fixtures, ruleset, decision log, máquina de estados, verificador local y nombre corregido están preparados en la rama local `codex/cangupay-p0-linder`, pendientes de revisión/publicación.

## Configuración en cinco minutos

1. En [CanguPay](https://github.com/gonnnzaDev/CanguPay/issues), confirmar **Issues** habilitado (el issue de prueba `#1` ya demuestra que estuvieron habilitados).
2. Crear solo cuatro labels: `P0`, `P1`, `bloqueado`, `demo`.
3. Crear los issues P0 de abajo y asignar un dueño por issue. Los colaboradores deben tener acceso al repositorio para ser asignados.
4. Si desean tablero, crear un **GitHub Project** en vista **Board** con `Todo / In progress / Review / Done` y añadir los issues. El tablero no condiciona empezar a programar.
5. En cada PR, escribir `Closes #N` con el número real del issue correspondiente. Revisar el PR antes de hacer merge.

**Regla de sprint:** máximo un issue de implementación activo por persona; marcar `bloqueado` y comentar cuando falte una interfaz o credencial. No preparar 26 tarjetas granulares en la primera reunión.

## Orden del primer bloque

| Orden | Issue | Dueño | Entregable para el siguiente |
|---:|---|---|---|
| 1 | P0-00 Acuerdo de interfaces | Gonza (con todos) | Tipos, funciones, eventos y red de testnet |
| 1 | P0-06 Fixtures y reglas | Linder | Archivos sintéticos y salidas esperadas |
| 1 | P0-08 App y wallet | Julián | Conexión testnet y tres roles |
| 2 | P0-01 Setup SAC | Gonza | Activo, saldo y contract ID |
| 2 | P0-02 Fondeo y aprobación | Gonza | Flujo feliz en test |
| 2 | P0-07 Agente | Linder | PASS/FAIL reproducibles |
| 2 | P0-09 Flujo buyer/supplier | Julián | Recorrido desde UI |
| 3 | P0-03 Plazos y finalize | Gonza | Ghost refunds y no objection |
| 3 | P0-04 Corrección y disputa | Gonza | Resolución o fallback |
| 3 | P0-10 Resolver y timeline | Julián | Tercera cuenta operativa |
| 4 | P0-05 Pruebas críticas y E2E | Gonza (con todos) | Invariantes y tests |
| 4 | P0-11 Integración/demo | Julián (con todos) | Demostración real desde web |
| 4 | P0-12 README y entregables | Linder | Declaraciones, video y enlaces |

## Contenido de los issues

### P0-00 — Acordar contrato de interfaces antes de implementar

**Dueño:** Gonza. **Participan:** Linder y Julián. **Labels:** `P0`. **Depende de:** ninguno.

**Objetivo:** decidir el stack que se ejecutará realmente: el README antiguo proponía Next.js/Python, mientras los stubs de `apps/web/` y `services/attestation-agent/` están en Rust. Configurar el `Cargo.toml` raíz y los paquetes que se conserven. Definir y documentar en `docs/architecture.md` una instancia de contrato por operación; `EscrowConfig`, `EscrowState`, estados, funciones, firmas, eventos, errores, network passphrase, activo demo y serialización de hashes.

**Terminado cuando:**

- [ ] Gonza, Julián y Linder han leído y aceptado las firmas y tipos.
- [ ] Gonza y Julián revisan `docs/state-machine.md` y registran cambios semánticos en el decision log.
- [ ] Julián confirma cómo implementará Freighter y su app web; Gonza confirma el workspace Rust. Los stubs no elegidos se retiran o se marcan claramente como inactivos.
- [ ] Se definen monto en unidades mínimas CPUSD y vínculo entre `supplier_id` documental y wallet del supplier.
- [ ] Se documenta el formato de `evidence_bundle_hash` y `report_hash` (`SHA-256` de bytes canónicos o alternativa única aprobada por el equipo).
- [ ] `max_correction_attempts=1` y `submission_deadline=funded_at+submission_period` están representados.
- [ ] Se decide cómo publicar bindings/cliente para Julián y cómo invocará el engine.
- [ ] La decisión queda registrada en `docs/decision-log.md`.

### P0-01 — Preparar CPUSD y cuentas testnet

**Dueño:** Gonza. **Labels:** `P0`. **Depende de:** P0-00.

**Objetivo:** crear tres cuentas de roles para Freighter (buyer, supplier, resolver), más engine e issuer/distributor como cuentas técnicas; configurar activo de prueba vía SAC.

**Terminado cuando:**

- [ ] Todas las cuentas necesarias tienen fondos testnet y están documentadas solo con direcciones públicas.
- [ ] Buyer dispone de saldo CPUSD y trustlines requeridas.
- [ ] Se conoce el contract ID del token SAC y se verifica una transferencia de prueba.
- [ ] El activo se etiqueta **sintético, sin respaldo fiat y exclusivo de testnet**.
- [ ] Ninguna secret key está en Git; se revisan flags de congelamiento/clawback.

### P0-02 — Crear, cancelar, fondear, presentar evidencia, atestiguar y aprobar

**Dueño:** Gonza. **Labels:** `P0`. **Depende de:** P0-00, P0-01, P0-06.

**Objetivo:** implementar el camino `CREATED → FUNDED → EVIDENCE_SUBMITTED → ATTESTED_PASS → RELEASED` y `CREATED → CANCELLED`, con auth y eventos.

**Terminado cuando:**

- [ ] Buyer puede crear, cancelar antes del fondeo y fondear exactamente `amount`.
- [ ] Supplier presenta hash antes del deadline calculado desde el fondeo.
- [ ] Solo engine emite `PASS/FAIL` con `report_hash`.
- [ ] Buyer aprueba un `PASS` y supplier recibe una sola transferencia.
- [ ] `EscrowCreated`, `Cancelled`, `Funded`, `EvidenceSubmitted`, `Attested` y `Approved` incluyen campos acordados.
- [ ] Pruebas: autorización, estado inválido, atomicidad del fondeo y doble aprobación.

### P0-03 — Implementar vencimientos en finalize()

**Dueño:** Gonza. **Labels:** `P0`. **Depende de:** P0-02.

**Objetivo:** `finalize()` permissionless y única ejecutora de vencimientos, con `env.ledger().timestamp()`, usando la convención `now < deadline` acción normal; `now >= deadline` finalize.

**Terminado cuando:**

- [ ] FUNDED sin evidencia → refund por `SUBMISSION_TIMEOUT`.
- [ ] EVIDENCE_SUBMITTED sin atestación → refund por `ATTESTATION_TIMEOUT`.
- [ ] ATTESTED_PASS sin objeción → release por `NO_OBJECTION`.
- [ ] ATTESTED_FAIL sin corrección ni disputa → refund por `CORRECTION_TIMEOUT`.
- [ ] `finalize()` antes del plazo devuelve error y en terminal es no-op, sin transferencia ni evento nuevo.
- [ ] Tests con time-travel en borde exacto, antes y después de cada deadline.

### P0-04 — Corrección única, disputa y fallback del resolver

**Dueño:** Gonza. **Labels:** `P0`. **Depende de:** P0-02, P0-03.

**Objetivo:** aceptar una corrección tras `FAIL`; una disputa como máximo (buyer desde `PASS`, supplier desde `FAIL`); resolución manual o fallback vencido.

**Terminado cuando:**

- [ ] Corrección incrementa attempts a 1, registra nuevo hash y reinicia `attestation_period`.
- [ ] Una segunda corrección es rechazada, con opción de disputar un nuevo FAIL vigente.
- [ ] Disputa exige hashes de motivo/evidencia y solo nace desde ATTESTED_PASS/FAIL dentro del plazo.
- [ ] Resolver puede decidir release/refund/split antes del deadline.
- [ ] En `DISPUTED`, `finalize()` aplica `fallback_outcome`/`fallback_split_bps` al vencer `resolution_period`.
- [ ] `split_bps>10000` y extremos para outcome SPLIT son rechazados; se conserva monto.
- [ ] `DisputeRaised`, `Resolved` y `Finalized(reason)` tienen datos correctos.
- [ ] Disputa o resolución post-terminal y doble settlement son rechazados.

### P0-05 — Ejecutar pruebas críticas y desplegar el contrato

**Dueño:** Gonza. **Colaboran:** Julián y Linder. **Labels:** `P0`. **Depende de:** P0-02, P0-03, P0-04.

**Objetivo:** ejecutar matriz de pruebas del documento v1.0 y desplegar WASM reproducible en Stellar testnet.

**Terminado cuando:**

- [ ] Pasa el flujo feliz, ghost supplier, ghost engine, corrección, disputa, fallback, split, idempotencia y conservación de saldos.
- [ ] El README contiene comandos reales de build/test/deploy, contract IDs y tx hashes públicos.
- [ ] No hay claves ni documentos reales en repo o logs.
- [ ] Julián recibe ABI/bindings y ejemplos de lectura/invocación.

### P0-06 — Producir fixtures sintéticos y reglas de negocio

**Dueño:** Linder. **Labels:** `P0`. **Depende de:** ninguno.

**Objetivo:** revisar y publicar los fixtures `pass`, `fail`, `dispute`, `fixtures/manifest.json`, `docs/ruleset.md` y el verificador local `scripts/verify_fixtures.py` ya preparados en la rama local; ratificar la precisión del activo y los hashes con Gonza y Julián.

**Terminado cuando:**

- [ ] PASS: proveedor, OC, importe, moneda y entrega coinciden.
- [ ] FAIL: existe una discrepancia inequívoca y está explicada.
- [ ] DISPUTE: la objeción posterior contiene evidencia adicional que el agente original no conocía.
- [ ] Los datos son inventados y marcados como tales.
- [ ] Gonza y Julián pueden calcular qué hash y qué estado esperan sin pedir interpretación adicional.

### P0-07 — Construir verificador determinista y atestación

**Dueño:** Linder. **Revisión de integración:** Gonza. **Labels:** `P0`. **Depende de:** P0-00, P0-06, P0-02.

**Objetivo:** convertir el verificador local de fixtures en servicio operativo según el stack decidido en P0-00, leer el estado real del contrato, producir reporte reproducible `PASS/FAIL`, firmar/enviar `attest()` desde la cuenta engine y preparar keeper. Si el envío bloquea, Gonza toma la integración de firma y Linder conserva el motor de reglas.

**Terminado cuando:**

- [ ] El mismo bundle produce el mismo reporte/hash.
- [ ] El hash del bundle presentado coincide con el procesado por el engine.
- [ ] Un FAIL señala qué campo falló sin inventar datos.
- [ ] La clave del engine vive fuera del repo y no llega al frontend.
- [ ] Una invocación exitosa en testnet produce `Attested` y cambia el estado esperado.

### P0-08 — Inicializar web, red y wallet

**Dueño:** Julián. **Labels:** `P0`. **Depende de:** P0-00 para ABI; el scaffold empieza de inmediato.

**Objetivo:** cerrar la elección de implementación para `apps/web/`, desarrollar cliente web con Freighter y configuración explícita de testnet. El stub actual `apps/web/src/main.rs` no constituye un frontend ni una integración de wallet; actualizar el README con el stack elegido.

**Terminado cuando:**

- [ ] Se detecta la red y se evita firmar en mainnet.
- [ ] Se conecta buyer/supplier/resolver desde perfiles separados.
- [ ] Se muestran identidad conectada, rol, activo demo y estado consultado al contrato.
- [ ] `.env.example` contiene solo variables públicas y placeholders.

### P0-09 — Implementar flujo buyer y supplier desde web

**Dueño:** Julián. **Labels:** `P0`. **Depende de:** P0-02, P0-06, P0-08.

**Objetivo:** crear/fondear/cancelar/aprobar para buyer; enviar/corregir evidencia para supplier; visualización de PASS/FAIL.

**Terminado cuando:**

- [ ] Buyer ve fallback y condiciones antes de fondear.
- [ ] Supplier ve fondos depositados y presenta el hash correcto.
- [ ] Las acciones inválidas por estado o rol se ocultan o explican.
- [ ] El estado se refresca desde chain tras cada transacción, con link de explorer.

### P0-10 — Implementar disputa, resolvedor y timeline

**Dueño:** Julián. **Labels:** `P0`. **Depende de:** P0-04, P0-08.

**Objetivo:** formulario de disputa permitida; panel de resolver y timeline de eventos con plazos del ledger.

**Terminado cuando:**

- [ ] Buyer disputa PASS, supplier disputa FAIL; las otras combinaciones se impiden.
- [ ] Resolver conectado con su cuenta decide RELEASE/REFUND/SPLIT con `split_bps` válido.
- [ ] El usuario ve reason/evidence hashes y transacciones, sin archivos privados on-chain.
- [ ] El contador visible se indica como orientativo; el contrato decide los deadlines.

### P0-11 — Integrar y grabar tres flujos verificables

**Dueño:** Julián. **Colaboran:** Gonza y Linder. **Labels:** `P0`, `demo`. **Depende de:** P0-03, P0-04, P0-07, P0-09, P0-10.

**Objetivo:** ejecutar demo 1 PASS por vencimiento; demo 2 ghost supplier; demo 3 disputa y split.

**Terminado cuando:**

- [ ] Los tres flujos usan cuentas testnet y estados reales del contrato.
- [ ] El video muestra demo 1 y un corte de demo 3 sin presentar simulaciones como operaciones on-chain.
- [ ] Se guardan contract IDs, tx hashes, saldos iniciales/finales y grabaciones de respaldo.
- [ ] Una persona puede repetir la demo siguiendo `docs/demo-scenarios.md`.

### P0-12 — Preparar README, evidencia de trabajo y entrega

**Dueño:** Linder. **Labels:** `P0`, `demo`. **Depende de:** P0-00 para empezar; enlaces de P0-11 para terminar.

**Objetivo:** revisar README con el nombre CanguPay, mantener `docs/decision-log.md`, declaraciones Genesis/Khipu/doble competencia, log de tres mensajes asincrónicos, chequeo de marca/dominio del nuevo nombre, guion de pitch y video. Elegir una licencia: `LICENSE` estaba vacío.

**Terminado cuando:**

- [ ] README distingue código nuevo, antecedentes, activo testnet y limitaciones.
- [ ] P0/P1 y fuente de verdad están enlazados.
- [ ] Se declaran solo resultados de validación efectivamente obtenidos.
- [ ] Demo, testnet, explorer, equipo y pasos de reproducción son verificables.
- [ ] El guion evita llamar RWA, factoring o USDC al activo de prueba.

## Mapeo publicado (19-09-2026)
Fuente: https://github.com/gonnnzaDev/CanguPay/issues · Registro de decisión: D-005 en `docs/decision-log.md`.
| Código | Issue |
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

## Primeros entregables de Linder después de registrar los issues

1. `fixtures/pass/{purchase-order,invoice,delivery}.json`.
2. `fixtures/fail/{purchase-order,invoice,delivery}.json`.
3. `fixtures/dispute/dispute-evidence.json`.
4. `docs/ruleset.md` con normalización y criterios PASS/FAIL.
5. `docs/decision-log.md` con D-001/D-002 confirmadas y decisiones de interfaz abiertas.
6. `fixtures/manifest.json` y `scripts/verify_fixtures.py` para verificar hashes y resultados de ambas muestras.

## Estado actual (19-09-2026)

Los 13 issues P0 fueron creados manualmente en GitHub con dueños y labels asignados:

- **Issues:** #2 (P0-00) a #14 (P0-12)
- **Labels creados:** P0, P1, bloqueado, demo
- **Repositorio:** https://github.com/gonnnzaDev/CanguPay/issues

Ver mapeo detallado en la sección "Mapeo publicado" arriba.
