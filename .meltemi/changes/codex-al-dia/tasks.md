# Tareas — codex-al-dia

Vía completa. Un commit atómico por tarea, con referencia `(codex-al-dia N.M)`
y sin trailers de co-autoría. Gates del repo en cada tarea:
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`
y la suite de los crates tocados; `cargo build --workspace` antes de correr
suites que lanzan binarios (los e2e buscan `mock-codex-wire` y
`meltemi-codex-acp` junto al test). **Se desarrolla en su taller
(`meltemi workspace codex-al-dia`) y aterriza en `main` con `meltemi land`.**
El orden de las tareas es el que deja cada commit en verde: la lectura de
versión antes que el re-anclaje (el guion nuevo trae el user agent en forma de
eco) y el eco `userMessage` silenciado antes también (el guion nuevo lo trae).

## 1. Versión y re-anclaje

- [ ] 1.1 `version::of(user_agent, announced)`: solo el primer token,
  producto `codex_cli_rs` o `announced`; el rehúso `provider_version_unreadable`
  dice lo visto y las dos formas esperadas y nombra
  `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`; el handshake pasa `SPEC.name`. Tests en
  `version.rs` (user agent medido de 0.156.1, token de terminal con versión,
  override, nombre sin versión, producto ajeno en primer lugar) y en
  `codex/mod.rs` (handshake con la forma de eco) (design D1) — escenarios «El
  CLI escribe el nombre del cliente en su user agent», «Un token posterior con
  versión no se confunde con el CLI», «Una forma desconocida se rehúsa sin
  culpar al binario» — gates: suite de `meltemi-adapters`
- [ ] 1.2 `ThreadItem::UserMessage` en `wire.rs` y el mapeo lo calla; test en
  `mapping.rs` con el ítem tal como lo emite 0.156.1 (design D8) — escenario
  «El eco del mensaje del usuario no se muestra» — gates: suite de
  `meltemi-adapters`
- [ ] 1.3 Re-anclaje a 0.156.1: los 20 archivos que cambiaron, byte a byte;
  `ServerRequest.json`, `ClientNotification.json`,
  `ThreadStatusChangedNotification.json`,
  `ThreadTokenUsageUpdatedNotification.json` y
  `ServerRequestResolvedNotification.json` añadidos; `PROVENANCE.md` (versión,
  fecha, handshake en forma de eco, volcado sin `--experimental`, la
  generación v1 ya sin métodos de cliente, fecha corregida del binario
  anterior); `VENDORED_SCHEMA_VERSION = "0.156.1"` y
  `SUPPORTED_FLOOR = (0,156,1)` con el comentario del techo reescrito; guion
  `codex-app-server.ndjson` reescrito desde la transcripción real; lista
  `SCHEMAS` de `mock-codex-wire`; mapas de `codex_conformance.rs` (tres
  notificaciones) y su aserción del user agent vía `version::of`; tests de
  `core/mock-provider` (`codex_wire.rs`, `jsonrpc_wire.rs`); tests del
  adaptador y e2e de `core/meltemid` que anclan 0.77.0, y uno que fija que
  `codex_cli_rs/0.77.0` da ahora desfase con «actualiza el CLI» (design D2, D3,
  D4) — escenarios «Tipos validados contra el esquema volcado», «El guion
  simulado sigue al cable real», «Desfase de versión rehusado con remedio» —
  gates: `cargo build --workspace`, suites de `meltemi-adapters`,
  `mock-provider` y `meltemid`
- [ ] 1.4 Conformidad estricta en lo que sale: comprobación recursiva de que
  cada clave serializada existe en `properties` del esquema, aplicada a
  `initialize`, `initialized`, `thread/start`, `turn/start`, `turn/interrupt`
  y las respuestas de aprobación; `initialized` validado contra
  `ClientNotification.json` (design D9) — escenario «Un campo que el esquema
  no declara no viaja» — gates: suite de `meltemi-adapters`

## 2. Gobierno del turno y de las aprobaciones

- [ ] 2.1 `approvalsReviewer: "user"` en `ThreadStartParams`; lectura
  permisiva de `approvalPolicy`, `approvalsReviewer` y el `type` del sandbox en
  `ThreadStartResult`; rehúso con diagnóstico y remedio si el revisor efectivo
  no es `user` o si la política es `never` con un sandbox que no es
  `readOnly`; la política efectiva en la procedencia (design D5) — escenarios
  «El adaptador pide que las aprobaciones las revise el cliente», «Un revisor
  distinto del cliente rehúsa la sesión», «Una política que nunca pregunta con
  sandbox que escribe rehúsa la sesión», «La política efectiva consta en el
  log» — gates: suite de `meltemi-adapters`
- [ ] 2.2 Versión posterior a la verificada: `providerVerifiedVersion` en la
  procedencia y una fila visible al abrir la sesión cuando la versión efectiva
  supera la verificada; tests con 0.156.1 (sin fila) y con una posterior dentro
  del rango (con fila) (design D2) — escenarios «Versión posterior a la
  verificada declarada, no rehusada», «Versión efectiva registrada en el log»
  — gates: suite de `meltemi-adapters`
- [ ] 2.3 Ámbito por hilo y turno: el hilo de la sesión filtra notificaciones
  y peticiones; los turnos terminados se recuerdan; lo de otro turno se ignora
  y la aprobación de otro turno se declina sin preguntar; tests del bucle del
  turno con un cierre de otro hilo, un cierre rezagado y una aprobación de otro
  turno (design D6) — escenarios «Lo que dice otro hilo no cierra el turno en
  curso», «El cierre rezagado de un turno anterior no cierra el siguiente»,
  «Una aprobación de otro turno se declina sin preguntar» — gates: suite de
  `meltemi-adapters`
- [ ] 2.4 Política por petición del servidor: tabla relevar/rehusar; el
  rehúso se muestra como herramienta fallida con el método; test de
  conformidad que recorre `ServerRequest.json` y exige política para cada
  método; tests que pinean que el refresco de credenciales y la atestación
  reciben error y no llegan al proxy (design D7) — escenarios «Una petición
  nueva del proveedor falla la conformidad», «Una petición no relevada se
  rehúsa a la vista», «Credenciales y atestación jamás llegan al proxy» —
  gates: suite de `meltemi-adapters`
- [ ] 2.5 Aprobación de comando: `networkApprovalContext` (host y protocolo)
  y `kind: "writeStdin"` leídos y llevados al título y contenido de la
  petición de permiso; tests en `permission.rs` (design D7) — escenario «Una
  aprobación de red muestra a qué se conecta» — gates: suite de
  `meltemi-adapters`
- [ ] 2.6 Turno fallido con `turn.error.message` y `additionalDetails` en el
  rehúso; `warning`, `configWarning` (sin ruta), `deprecationNotice` y
  `model/rerouted` anotados en el registro del adaptador; tests en `mapping.rs`
  y en el bucle del turno (design D8) — escenario «Un turno fallido se explica
  con las palabras del proveedor» — gates: suite de `meltemi-adapters`

## 3. Evidencia y documentación

- [ ] 3.1 `docs/agentes.md`: versión mínima de Codex, remedio cuando el CLI
  es más viejo o más nuevo que la verificada, y el nombre con que el adaptador
  se presenta al CLI (que el CLI informa a su backend como `originator`); test
  que compara la versión mínima de la guía con `SUPPORTED_FLOOR` (design D2) —
  escenario «La guía y el adaptador declaran la misma versión mínima» — gates:
  suite de `meltemi-adapters`
- [ ] 3.2 Corregir la frase de la propuesta y de
  `docs/research/comparativo-orca-2026-09-24.md` que decía que el techo
  rehusaría 0.156.1 (design, Context) — sin escenario: corrección de
  artefactos — gates: `meltemi validate codex-al-dia`
- [ ] 3.3 Corrida real contra `codex-cli 0.156.1`: `conformance_real` con
  `MELTEMI_CONFORMANCE_AGENT=codex-cli` y una sesión por Meltemi con
  aprobación; resultado, política efectiva observada y tiempos en
  `docs/conformidad-manual.md`; nivel verificado solo si sus comprobaciones
  pasan (design D10) — escenarios «El re-anclaje exige la corrida real», «Una
  comprobación fallida se registra con las palabras del proveedor» —
  verificación documentada (`sdd/verify-mark`)

## 4. Cierre

- [ ] 4.1 `meltemi validate codex-al-dia` limpio, `meltemi verify
  codex-al-dia` completo, gate del workspace
  (`cargo test --workspace --no-fail-fast`, clippy, fmt), estado de la change
  en `docs/plan-de-cambios.md`, y `meltemi land codex-al-dia`
