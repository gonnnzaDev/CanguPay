# Registro de decisiones · CanguPay

Las entradas **confirmadas** reflejan lo acordado para el prototipo. Las decisiones de interfaz pendientes se identifican explícitamente y requieren revisión de Gonza, Julián y Linder antes de integrar piezas.

## D-001 · Alcance P0 congelado — 19-09-2026

**Estado:** confirmado en el plan de trabajo comunicado por Linder; revisión técnica del equipo pendiente.

**Decisión:** un escrow por instancia Soroban en Stellar testnet, activo CPUSD sintético vía SAC, reglas documentales deterministas, una corrección, una disputa, resolver humano, `finalize()` público/idempotente como única función que ejecuta vencimientos. El supplier fantasma obtiene refund por submission timeout; el engine ausente por attestation timeout; el resolver ausente aplica fallback preacordado. Ledger timestamp define plazos. Demos con tres cuentas humanas Freighter y cuentas técnicas separadas.

**Fuera de P0:** Scale, RWA, factoring, OCR, IA de extracción, USDC mainnet, casos reales, integración ERP. P1 se decide al tener P0 estable. Tres mensajes de validación asincrónica, recapitulación a organizadores y chequeo de nombre son actividades paralelas; ninguna bloquea escribir el prototipo. No se declara ninguna respuesta positiva inexistente.

**Motivo:** mantener una demo comprobable de reglas, custodia y pagos, sin dependencia de entrevistas ni servicios externos que puedan impedir el flujo principal.

## D-002 · Nombre CanguPay — 19-09-2026

**Estado:** confirmado por Linder y reflejado en el nombre del repositorio. `CumplePago` es el nombre anterior y debe sustituirse en piezas del equipo. Esto no equivale a una revisión de marca o dominio disponible: el resultado real del chequeo quedó anotado en D-006 (25-09-2026).

## D-003 · Interfaz web y agente — pendiente

**Observación:** `README.md` describe Next.js/TypeScript + Freighter y FastAPI/Python; el scaffold actual incluye `apps/web/src/main.rs` (comentarios de Actix) y `services/attestation-agent/src/lib.rs` (comentarios Rust). Ninguno implementa aún una aplicación o un servicio operativo. El `Cargo.toml` raíz declara un paquete sin `src/` ni miembros workspace definidos.

**Decisión requerida (P0-00):** Gonza y Julián acuerdan si la web será cliente Next.js/TypeScript y el agente Python, con eliminación/traslado de stubs Rust, o si proponen una alternativa que cubra Freighter, API/keeper y build repetible. Documentar la elección, el ABI y el reparto de ownership antes de integrar.

### D-003a · Stack del agente de atestación — elegido para P0-07

**Estado:** elegido para implementar P0-07. La ratificación formal de P0-00 (Gonza, Julián y Linder) sigue pendiente: esto no cierra D-003 ni decide la web.

**Decisión:** el agente de atestación se implementa en **Rust** dentro del workspace, con `stellar-xdr` y `stellar-rpc-client` v27, el mismo protocolo que el contrato (`soroban-sdk 27.0.6`). La alternativa considerada era FastAPI/Python; queda descartada para el agente porque obligaría a mantener el motor de reglas en dos lenguajes y a reimplementar la firma y la serialización XDR.

**Consecuencias:** `app/` (motor Python) se conserva como oráculo de referencia, no como servicio. `docs/ruleset.md` debe registrar que el servicio operativo es el crate Rust. La web sigue sin decidir.


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

## D-006 · Chequeo preliminar de nombre CanguPay — 19-09-2026 (ejecutado 25-09-2026)
Estado: confirmado por Linder; chequeo ejecutado el 25-09-2026.
Fuentes consultadas: búsqueda web general de "CanguPay" y homógrafos; RDAP de Verisign para `.com`; consultas DNS (A/AAAA/NXDOMAIN) para `cangupay.com`, `cangupay.pe`, `cangupay.com.ar`, `cangupay.app`; registro público de empresas en Suiza vía directorio comercial; GitHub.
Coincidencias encontradas: **ninguna exacta "CanguPay"**. Coincidencia próxima: **Cangopay AG** (Suiza, Alpnach, fundada 08-08-2023, consultoría, UID CHE420456233) — escritura similar, distinto signo y distinto sector; no la consideramos confundible, pero queda anotada. Registros **INDECOPI (Perú) e INPI (Argentina) no se consultaron**: no fueron accesibles desde el entorno del chequeo.
Dominios: `cangupay.com` **no registrado** (RDAP 404, control `google.com` = 200), `cangupay.pe`, `cangupay.com.ar` y `cangupay.app` sin resolución DNS. Disponibilidad verificada de nuevo antes de comprar o publicar.
Riesgo: **bajo en las fuentes consultadas** (sin homógrafos exactos y con dominio `.com` libre); **medio en cuanto a registros oficiales**, por no haberse consultado INDECOPI/INPI.
Decisión: mantener CanguPay como nombre de trabajo para la hackatón.
Aclaración: chequeo preliminar; no equivale a clearance legal de marca en Perú ni Argentina. Si el proyecto sigue más allá de la hackatón: registrar dominio y buscar búsqueda formal de antecedentes en INDECOPI/INPI.

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
Estado: confirmado el 20-09-2026. Rotación ejecutada por Gonza: cuentas nuevas creadas, direcciones públicas compartidas por canales públicos, secret keys distribuidas únicamente por DM a quienes las necesitan para tests locales. Archivo retirado de main en PR #20 (merge confirmado).
Hecho: el commit 92990c4 llevó a main un archivo con la frase de recuperación de una cuenta, en repo público.
Acciones completadas: cuenta tratada como comprometida y dejada de usar; cuentas nuevas para los roles afectados; archivo retirado de main; rotación con buenas prácticas (no compartir claves por chat).
Nota: borrar el archivo no limpia el historial público; la rotación es la mitigación real. Ninguna credencial vuelve al repo: solo placeholders en .env.example.

## D-010 · Licencia MIT para el repositorio — 25-09-2026
Estado: confirmado por decisión de equipo en P0-12; `LICENSE` estaba vacío.
Decisión: adoptar **MIT** (texto completo en `LICENSE`, © 2026 CanguPay team) y declararlo en el README. Motivo: licencia corta y permisiva, estándar para proyectos de hackatón y sin fricción para revisores o forks. Alternativas descartadas: Apache-2.0 (más larga, aporta poco en un prototipo sin patentes), GPL-3.0 (copyleft incompatible con la reutilización que queremos).
Alcance: cubre código, contratos y docs del repo. No cubre la marca CanguPay (ver D-006) ni los fixtures, que son sintéticos y de uso libre.

## D-011 · README, entregables y huecos de P0-12 — 25-09-2026
Estado: confirmado para la revisión de README; la demo y el video siguen pendientes.
Decisión:
- README con nombre CanguPay y sección **Fuente de verdad** que enlaza `docs/issue-backlog.md`, issues #2–#14, `docs/decision-log.md`, `docs/ruleset.md`, `docs/state-machine.md`, `docs/validation-log.md` y `docs/pitch-script.md`.
- Bloque **Demo, testnet y video** creado con contract ID, tx hashes, explorer y enlace de video **vacíos y marcados como pendientes (P0-11)**: no se afirma ninguna corrida on-chain que no exista.
- Declaraciones mantenidas: activo sintético CPUSD, doble competencia (Odyssey Perú + Argentina Builder Challenge), Khipu como antecedente sin reutilización de código, sin validación comercial.
- `docs/validation-log.md`: los tres mensajes de validación asincrónica constan como **no enviados**; plantilla registrada, sin respuestas inventadas.
- `docs/pitch-script.md`: guion de 90 s, 3 min, video y lista de lenguaje prohibido (USDC, RWA, factoring).
Pendiente para cerrar P0-12: enlaces de demo/testnet/explorer/video (depende de P0-11) y `docs/demo-scenarios.md`.

## D-012 · Cierre de D-003: stack effective del proyecto — 26-09-2026
Estado: **propuesto para ratificación de P0-00** (Gonza, Julián y Linder). Registra lo que
el repositorio ya hace, que es la parte que faltaba documentar.

D-003 quedó abierto porque la lectura on-chain y la ABI no estaban decided. Ya están, y la
implementación las fijó. Lo que sigue es la firma, no el diseño.

**Confirmado por el código:**

1. **Contrato:** Rust con `soroban-sdk`. Funciones `initialize`, `fund`, `submit_evidence`,
   `attest`, `approve`, `raise_dispute`, `resolve`, `finalize`, `cancel` y las vistas
   `state`, `config` y `snapshot`.
2. **Agente de atestación:** Rust, dentro del workspace, como crate `attestation-agent`. La
   alternativa FastAPI/Python quedó **descartada**: obligaría a mantener el motor de reglas
   en dos lenguajes y a reimplementar la firma y la serialización XDR.
3. **Motor de reglas en Python:** se conserva `services/attestation-agent/app/` como
   **oráculo ejecutable**, no como servicio. La paridad entre ambos está probada con tests
   contra bundles externos al manifiesto, no solo contra `fixtures/manifest.json`.
4. **Web:** Next.js/TypeScript en `apps/web`, con Freighter. Ya implementada y con
   typecheck y tests en verde.

**Mecanismo de lectura (lo que D-007 dejaba pendiente):** Stellar RPC, no Horizon. Se
resuelve con `getLedgerEntries` para la escritura de la instancia y `simulateTransaction`
para las vistas.

**ABI de lectura:** un unico getter, `snapshot()`, que no pide `auth` y devuelve estado,
configuración, hashes y **todos** los plazos en una llamada. Se añadió porque `state()` y
`config()` no exponen deadlines ni hashes, y leerlos del almacenamiento por posicion
obliga al cliente a conocer el orden de los campos privados del contrato. Ver D-013.

Ratificación pendiente: quién firma esto y qué implica para D-003 y D-004.

## D-013 · Convenciones de serializacion que la red exige — 26-09-2026
Estado: **hecho y verificado en testnet.** Se documenta porque ninguna aparece en la
especificacion de forma utilizable y las tres fallan de forma poco descriptiva.

El agente fallo contra la red de cuatro maneras que ninguna prueba unitaria detecta,
porque todas pasan en local:

1. **Los `struct` se envian y devuelven como `Map` con claves por nombre**, no como un
   `Vec` posicional. Con el `Vec` posicional el decoder lee shifting y produce una lectura
   equivocada en silencio. Es tambien la forma correcta: reordenar el contrato no invalida
   al cliente.
2. **Los `enum` viajan como `Vec([Symbol("Nombre")])`**, no como el indice numerico. Pasar el
   indice parece correcto —el enum tiene ese discriminante— pero el host no lo convierte y el
   WASM entra en `UnreachableCodeReached`, un trap sin mensaje util.
3. **El `SignatureHint` son los ultimos 4 bytes de la pubkey.** Con el prefijo, que es lo
   que decia la especificacion antigua, la red busca una clave que no existe y responde
   `TxBadAuth` aunque la firma verifique perfectamente. Es el fallo mas caro de la lista
   porque el error no distingue "firma mala" de "no encuentro la clave".
4. **Los errores de `simulateTransaction` llegan en un campo `error` con HTTP 200**, sin
   `sorobanData`. Sin comprobarlo, el sintoma es "no devolvio resultados", que no dice
   nada; ese enmascaramiento costo una hora de diagnostico.

Ademas, cuando la cuenta que exige `auth` es la propia fuente de la transaccion, Soroban
devuelve credenciales `SourceAccount` y no `Address`. Buscar solo `Address` hace pensar
que la simulacion no devolvio nada.

**Consecuencia sobre el mecanismo de lectura:** el RPC sirve la entrada de cuenta con una
secuencia anterior a la que espera la red, por lo que el envio se rechaza con `TxBadSeq`
aunque la firma sea correcta. El agente relee la cuenta y reintenta una vez, conservando el
error de la red si tambien asi falla.

## D-014 · Toolchain unificado en protocolo 28 — 26-09-2026
Estado: **hecho.** `soroban-sdk`, `stellar-xdr` y `stellar-rpc-client` suben a 28.

El nodo de testnet corre protocolo 28. Con las herramientas en 27 el contrato funcionaba
contra el, pero es una mezcla fragil. Ahora las tres piezas son de la misma version.

**Consecuencia operativa:** desde `soroban-sdk` 28 el WASM **no** se compila con
`cargo build --target wasm32v1-none`; falla con un error explicito. Hay que usar
`stellar contract build` (CLI 25.2+). Esto afecta al pipeline de **todos** los contratos
del repo, no solo al agente.

**Validación cruzada:** con el CLI se despliega y se ejecuta el flujo completo; con el
agente, tambien. Que el mismo camino funcione por ambas vias es lo que permitió distinguir
los fallos del codigo de los del protocolo.

## D-015 · Token de testnet: contrato de prueba en lugar de CPUSD — 26-09-2026
Estado: **abierto, requiere decisión de P0-01.**

El contrato mueve fondos con `TokenClient`, que es la interfaz del Stellar Asset Contract:
si `config.token` no apunta a un SAC desplegado, `fund()` falla y el escrow nunca llega a
`EvidenceSubmitted`, que es el estado desde el que el agente atesta.

En la red publica de Stellar **no hay un SAC desplegado para una divisa de prueba**. Para
poder ejercitar el flujo completo se escribio `contracts/test-token/`, un contrato minimo
con la interfaz de SAC (`transfer`, `balance`, `mint`, `admin`).

**Consecuencia honesta sobre lo verificado:** todo lo demas esta probado contra la cadena
de verdad —`attest` PASS y FAIL desde el agente, keeper, `finalize` con fallback split,
ghost supplier, ghost engine, idempotencia, correccion unica, disputa y `resolve`— pero
**contra un token que no es CPUSD**. La logica del escrow, los plazos, la correccion y la
firma estan verificadas; la unidad monetaria, no.

Pendiente de decidir: desplegar un SAC nuevo en testnet como CPUSD sintetico, o declarar
que el alcance verificable se limita al token de prueba. Mientras siga asi, ninguna
corrida puede presentarse como una prueba de CPUSD.

**Defecto conocido que esta decision destapa:** el valor por defecto de `--currency` en el
CLI es la cadena `"CPUSD"`, y desde D-016 el keeper contrasta `expected_currency` contra
`snapshot.config.token`, que es una direccion `C...`. Con un SAC real, cualquier
ejecucion sin `--currency` explicito se rechaza. El default quedo mintiendo y con el token
de prueba pasaba solo porque ambos valores coincidian.

## D-016 · Validaciones que completan la atestacion — 26-09-2026
Estado: **hecho y verificado.**

Revision de P0-07 sobre `main` (Gonza, 26-09) dejo cuatro hallazgos. Los tres de codigo
estan cerrados; el cuarto era el token (D-015).

1. **Temporales de test aislados.** Cuatro sitios escribian en directorios fijos de
   `temp_dir()`, y con dos procesos de `cargo test` a la vez se pisan. Verificado con cuatro
   procesos simultaneos: salida 0 y cero fallos en los cuatro.
2. **Importe y token anclados al contrato.** El motor evaluaba con el importe de la linea de
   comandos y solo contrastaba el hash de evidencia, lo que **no alcanza**: un bundle puede
   ser coherente consigo mismo y llevar un importe que no es el del escrow, en cuyo caso el
   hash coincidiria y la atestacion describiria otro importe. `Report` registra ahora el
   importe y la divisa con los que se evaluo, y tanto `plan_attestation` como el keeper
   rechazan que difieran de los del contrato. `report_hash` no cambia, porque se calcula de
   un subconjunto explicito.
3. **JSON estricto en las entradas.** `strict_loads()` existia y no lo usaba ninguna entrada
   de produccion. El CLI y el keeper parseaban con `serde_json::from_str`, que acepta claves
   duplicadas y se queda con la ultima en silencio; como el hash se calcula despues, la
   firma seria legitima sobre un bundle que el proveedor no entrego.

**Consecuencia de proceso:** la verificacion se hizo contra `main` en `4563e00`, que quedo
seis commits atras. Antes de leer un informe como vigente conviene comprobar sobre que
commit se ejecuto.

## D-017 · Fuera el montador de testnet propio — 26-09-2026
Estado: **hecho.**

Existia `examples/testnet_setup.rs`, que montaba escrows copiando la logica de firma en vez
de delegar en la de produccion. Arrastraba los tres fallos de D-013 mas el de
`SourceAccount`, y **compilaba sin errores**, asi que el fallo solo aparecia al usarlo, con
un error de red que invitaba a culpar al protocolo. `examples/keygen.rs` tampoco lo usaba
nadie.

Se borraron ambos. El montaje de testnet se documenta con el CLI de Stellar, que ademas
sirve de control: si una transaccion entra por el CLI y no por el agente, el problema esta
en el agente. Mantener una segunda implementacion de firma junto a la buena es una forma de
reintroducir los fallos que la primera ya corrigio.

## Registro de cambios

| Fecha | Decisión | Responsable | Efecto |
|---|---|---|---|
| 19-09-2026 | D-001 y D-002 | Linder | Congelar P0 y actualizar el nombre |
| 19-09-2026 | D-003 y D-004 abiertos | Equipo | Cerrar stack, monto y ABI antes de integración |
| 19-09-2026 | D-005: backlog P0 publicado (#2–#14) y labels creados | Linder | Trazabilidad códigos internos ↔ issues GitHub |
| 19-09-2026 | D-007: stack Rust confirmado; lectura on-chain pendiente de reunión | Gonza, Julián | Cierra stack de D-003 y precisión de D-004; P0-07 etapa 2 sigue bloqueada |
| 20-09-2026 | D-008: ForLess01 integra ambas competencias | Equipo | 2 de 4 = 50% Argentina, cumple elegibilidad |
| 20-09-2026 | D-009: incidente de credencial reportado (rotación pendiente) | Linder | Trazabilidad del incidente; confirmación pendiente de Gonza |
| 20-09-2026 | D-009: incidente de credencial confirmado y rotado | Gonza, Linder | Cuenta sustituida, archivo retirado de main |
| 25-09-2026 | D-006: chequeo de marca/dominio ejecutado (sin homógrafos exactos; `.com` libre; INDECOPI/INPI no consultados) | Linder | Nombre CanguPay se mantiene como nombre de trabajo |
| 25-09-2026 | D-010: licencia MIT adoptada; `LICENSE` escrito | Equipo | Repo publicable con licencia clara |
| 25-09-2026 | D-011: README con fuente de verdad, guion de pitch/video y log de validación sin enviar | Linder | P0-12 avanzado; demo/video quedan como pendiente de P0-11 |
| 26-09-2026 | D-012: stack efectivo documentado (agente Rust, web Next.js, lectura por Stellar RPC, getter `snapshot()`) | Equipo | Propuesto para ratificar en P0-00; cierra el contenido de D-003, pendiente la firma |
| 26-09-2026 | D-013: convenciones de serializacion que la red exige | Equipo | Explica por que el agente fallo contra la red pese a pasar las pruebas locales |
| 26-09-2026 | D-014: toolchain unificado en protocolo 28 | Equipo | El WASM exige `stellar contract build`; afecta al pipeline de todos los contratos |
| 26-09-2026 | D-015: token de testnet es un contrato de prueba, no CPUSD | Equipo | **Abierto.** Bloquea presentar las corridas como prueba de CPUSD |
| 26-09-2026 | D-016: cierre de los hallazgos de revision de P0-07 | Equipo | Tres de codigo cerrados; el cuarto es D-015 |
| 26-09-2026 | D-017: fuera el montador de testnet que duplicaba la firma | Equipo | El montaje usa el CLI, que sirve de control |