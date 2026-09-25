# Tareas — codex-al-dia

Vía completa. Un commit atómico por tarea, con referencia `(codex-al-dia N.M)`
y sin trailers de co-autoría. **Se desarrolla en su taller (`meltemi workspace
codex-al-dia`) y aterriza en `main` con `meltemi land`.**

Gates de cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check` y la suite de los crates tocados. Toda tarea que cambie el
binario del adaptador o el cable simulado corre además `cargo build --workspace`
y las suites de `meltemid` que lo lanzan: `e2e_adaptadores`,
`e2e_adaptadores_cancel` y `conformance_suite` (el gate «e2e del adaptador»).

El orden es el que deja cada commit en verde: la lectura de versión y el eco
`userMessage` antes que el re-anclaje (el guion nuevo trae los dos), y cada
archivo del volcado llega con la tarea que lo usa.

## 1. Versión y re-anclaje

- [x] 1.1 `version::of(user_agent, announced)`: solo el primer token,
  producto `codex_cli_rs` o `announced`; el rehúso `provider_version_unreadable`
  dice lo visto y las dos formas esperadas y nombra
  `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`; el handshake pasa `SPEC.name`. Tests en
  `version.rs` (user agent medido de 0.156.1, token de terminal con versión,
  override, nombre sin versión, producto ajeno en primer lugar) y en
  `codex/mod.rs` (handshake con la forma de eco) (design D1) — escenarios «El
  CLI escribe el nombre del cliente en su user agent», «Un token posterior con
  versión no se confunde con el CLI», «Una forma desconocida se rehúsa sin
  culpar al binario» — gates: suite de `meltemi-adapters`
- [x] 1.2 `ThreadItem::UserMessage` en `wire.rs` y el mapeo lo calla; test en
  `mapping.rs` con el ítem tal como lo emite 0.156.1 (design D8) — escenario
  «El eco del mensaje del usuario no se muestra» — gates: suite de
  `meltemi-adapters`
- [x] 1.3 Re-anclaje a 0.156.1: los 20 archivos que difieren, byte a byte, y
  `ThreadStatusChangedNotification.json`,
  `ThreadTokenUsageUpdatedNotification.json` y
  `ServerRequestResolvedNotification.json`; `PROVENANCE.md`;
  `VENDORED_SCHEMA_VERSION = "0.156.1"` y `SUPPORTED_FLOOR = (0,156,1)` con el
  comentario del techo reescrito; guion `codex-app-server.ndjson` reescrito con
  el orden de la grabación, los ids y textos que los e2e fijan, la aprobación
  concedida, el user agent con token de terminal, `requestId` 1, sin campos que
  el volcado no declare y las diferencias en su cabecera; transcripción saneada
  `codex-app-server-0.156.1.transcript.ndjson` y el test de subsecuencia
  ordenada; lista `SCHEMAS` de `mock-codex-wire`; mapas de
  `codex_conformance.rs` y su aserción del user agent vía `version::of`; tests de
  `core/mock-provider`; guiones retenidos de `conformance_suite.rs` y
  `e2e_adaptadores_cancel.rs` en la forma de 0.156.1 con sus campos
  obligatorios; tests del adaptador que anclaban 0.77.0, y uno que fija que
  `codex_cli_rs/0.77.0` da desfase con «actualiza el CLI» (design D2, D3, D4) —
  escenarios «Tipos validados contra el esquema volcado», «El guion simulado
  sigue al cable real», «Desfase de versión rehusado con remedio» — gates:
  suites de `meltemi-adapters` y `mock-provider`, e2e del adaptador
- [x] 1.4 Conformidad estricta e identidad: comprobación recursiva de las
  claves de `params` y `result` contra las propiedades del esquema (con `$ref`
  y rama de `oneOf` por discriminante); `initialized` sin el miembro `jsonrpc`
  del sobre, validado contra `ClientNotification.json` (vendorizado aquí, con su
  entrada en `SCHEMAS`); test negativo con una clave mal escrita; `thread/start`
  sin más claves que las cinco declaradas en D9; `clientInfo.name` propio y
  ningún nombre de cliente de primera parte (design D9, D1) — escenarios «Un
  campo que el esquema no declara no viaja», «El adaptador se presenta con su
  propio nombre» — gates: suite de `meltemi-adapters`

## 2. Compuerta y gobierno del turno

- [x] 2.1 Compuerta: `approvalPolicy: "untrusted"`, `sandbox: "read-only"` y
  `approvalsReviewer: "user"` en `ThreadStartParams`; lectura de los valores
  efectivos y lista blanca que falla cerrada (distinto, desconocido o ausente
  rehúsa; red en el sandbox rehúsa); valores efectivos en la procedencia;
  respuesta de `thread/start` del guion y de los guiones retenidos del daemon
  con los valores que el servidor devuelve cuando se le piden; test de
  conformidad que pinea los enums de `AskForApproval`, `SandboxPolicy` y
  `ApprovalsReviewer` del volcado (design D5, D2) — escenarios «El adaptador
  pide la compuerta más estricta», «Un valor efectivo distinto del pedido
  rehúsa la sesión», «Un valor desconocido o ausente rehúsa la sesión», «La
  compuerta efectiva consta en el log», «Los valores de gobierno del esquema
  están pineados», «Compuerta dura sobre modo permisivo del CLI» — gates: suite
  de `meltemi-adapters`, e2e del adaptador
  <!-- 2026-09-24: **R5 medido antes de implementar**, contra codex-cli 0.156.1
  en Windows: pedida la compuerta, el servidor devolvió exactamente
  `untrusted`, `user` y `{"type":"readOnly","networkAccess":false}`; un cambio
  de archivo pidió aprobación y, aprobado, se aplicó bajo el sandbox de solo
  lectura (`GATE.txt` con «gated»); y `git status --short`, que no es lectura
  de confianza, pidió aprobación de comando (`kind: "command"`) y corrió al
  concederla. La alternativa `workspace-write` que R5 dejaba abierta no hace
  falta. -->
- [x] 2.2 Pérdidas visibles: una fila al abrir la sesión que nombra lo que la
  compuerta de Codex no cubre, y una fila por servidor MCP que el CLI informa
  arrancando (`McpServerStatusUpdatedNotification.json` vendorizado, en
  el guion y en `SCHEMAS`) (design D5) — escenarios «Lo que la compuerta del CLI
  no cubre se declara al abrir», «Un servidor de herramientas que el CLI arranca
  se nombra» — gates: suite de `meltemi-adapters`, e2e del adaptador
- [x] 2.3 Control remoto: `remoteControl/status/changed` distinto de
  `disabled` rehúsa la sesión si llega antes del primer turno e interrumpe y
  rehúsa el turno si llega durante uno
  (`RemoteControlStatusChangedNotification.json` vendorizado, en el guion con
  `disabled` y en `SCHEMAS`) (design D5) — escenario «Un control remoto activo
  rehúsa la sesión» — gates: suite de `meltemi-adapters`, e2e del adaptador
- [ ] 2.4 Versión: `providerVerifiedVersion` en la procedencia y una fila
  visible cuando la versión efectiva supera la verificada; contraste con
  `thread.cliVersion` que rehúsa si difiere (design D2, D1) — escenarios
  «Versión posterior a la verificada declarada, no rehusada», «El hilo
  contradice la versión leída» — gates: suite de `meltemi-adapters`, e2e del
  adaptador
- [ ] 2.5 Ámbito: el hilo de la sesión filtra lo que mueve el turno; una
  notificación sin hilo o sin turno no se filtra por ese eje; los turnos
  terminados se recuerdan; la aprobación de otro hilo o turno se declina a la
  vista; un hilo hijo se declara en una fila (design D6) — escenarios «Lo que
  dice otro hilo no cierra el turno en curso», «El cierre rezagado de un turno
  anterior no cierra el siguiente», «Una aprobación de otro hilo o turno se
  declina a la vista», «Un hilo hijo se declara en la sesión» — gates: suite de
  `meltemi-adapters`, e2e del adaptador
- [ ] 2.6 Peticiones del servidor: `ServerRequest.json` vendorizado (y en
  `SCHEMAS`); tabla relevar/rehusar; rehúso visible con `toolCallId`
  `server-request:<id>` y sin parámetros; refresco de credenciales y atestación
  pineados (error, sin proxy, sin parámetros registrados); test de conformidad
  que exige política para cada método y test negativo con un método sintético
  (design D7) — escenarios «Una petición nueva del proveedor falla la
  conformidad», «Una petición no relevada se rehúsa a la vista», «Credenciales y
  atestación jamás llegan al proxy» — gates: suite de `meltemi-adapters`, e2e
  del adaptador
- [ ] 2.7 Contexto de aprobación: `networkApprovalContext` (host y protocolo,
  con una herramienta emitida antes si el id nunca se anunció), `kind:
  "writeStdin"` y `grantRoot` (raíz y alcance en título, contenido y
  ubicación) llevados a la petición de permiso; tests en `permission.rs` y en el
  bucle del turno (design D7) — escenarios «Una aprobación de red muestra a qué
  se conecta», «Una escritura en una terminal se distingue de un comando», «Una
  raíz de escritura para toda la sesión se muestra» — gates: suite de
  `meltemi-adapters`, e2e del adaptador
- [ ] 2.8 Turno fallido con `turn.error.message` y `additionalDetails` en el
  rehúso (design D8) — escenario «Un turno fallido se explica con las palabras
  del proveedor» — gates: suite de `meltemi-adapters`, e2e del adaptador
- [ ] 2.9 Avisos del proveedor (`warning`, `configWarning`,
  `deprecationNotice`, `guardianWarning`, `windows/worldWritableWarning`,
  `model/rerouted`) anotados sin rutas, en el turno y en `await_answer` durante
  el handshake (design D8) — escenario «Los avisos del proveedor se anotan y no
  se dicen como agente» — gates: suite de `meltemi-adapters`, e2e del adaptador

## 3. Catálogo, evidencia y documentación

- [ ] 3.1 `verifiedVersion` en `fleet/list` desde la corrida persistida
  (schema de `proto/` aditivo, tipos de `meltemi-proto`, daemon), mostrada en la
  vista Flota de la GUI y de la TUI; `docs/paridad-nucleo.md` al día (design
  D11) — escenario «El catálogo nombra la versión medida» — gates: suites de
  `meltemid`, `meltemi-proto`, `tui` y `desktop`
- [ ] 3.2 `docs/agentes.md`: versión mínima de Codex, remedio cuando el CLI es
  más viejo o más nuevo que la verificada, la compuerta que el adaptador
  configura y lo que no cubre, el nombre con que se presenta al CLI (que el CLI
  informa a su backend como `originator`) y la etiqueta `source` que el CLI pone
  por su cuenta; test que compara la versión mínima de la guía con
  `SUPPORTED_FLOOR` (design D1, D2, D5) — escenario «La guía y el adaptador
  declaran la misma versión mínima» — gates: suite de `meltemi-adapters`
- [ ] 3.3 Corrida real contra `codex-cli 0.156.1`: `conformance_real` con
  `MELTEMI_CONFORMANCE_AGENT=codex-cli` y una sesión por Meltemi con una
  escritura aprobada; resultado, plataforma, compuerta efectiva observada,
  consecuencia de la configuración MCP y tiempos en
  `docs/conformidad-manual.md`, junto con la corrida fallida de 0.77.0 ya medida
  y sus palabras del proveedor; nivel verificado solo si sus comprobaciones
  pasan (design D10, R5) — escenarios «El re-anclaje exige la corrida real»,
  «Una comprobación fallida se registra con las palabras del proveedor» —
  verificación documentada (`sdd/verify-mark`)

## 4. Cierre

- [ ] 4.1 `meltemi validate codex-al-dia` limpio, `meltemi verify
  codex-al-dia` completo, gate del workspace
  (`cargo test --workspace --no-fail-fast`, clippy, fmt), estado de la change
  en `docs/plan-de-cambios.md`, y `meltemi land codex-al-dia`
