# codex-al-dia — design

## Context

Verificado el 2026-09-24 contra el código, contra el volcado de esquema del
CLI instalado, contra una transcripción del servidor real y contra el código
fuente de Codex en el tag `rust-v0.156.1`. Revisado después por tres lentes
adversariales (constitución, evidencia, cobertura de escenarios); este texto ya
incorpora sus correcciones.

- **Qué falla hoy.** Con `codex-cli 0.77.0`, la versión a la que
  `adaptadores-propios-acp` ancló el esquema, el handshake pasa y el proveedor
  rechaza cada turno («The 'gpt-5.6-sol' model requires a newer version of
  Codex»). Con `codex-cli 0.156.1`, el adaptador rehúsa el handshake: el user
  agent es `meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64)
  xterm-256color (meltemi-codex-acp; 0.1.1)` y `version::of` buscaba un token
  `codex_cli_rs/` que ya no existe.
- **Por qué cambió el user agent.** `codex-rs/login/src/auth/default_client.rs:150-177`
  arma el user agent como `{originator}/{build_version} ({os} {os_version};
  {arch}) {terminal}` y le añade ` ({name}; {version})`; `build_version` es la
  versión del CLI. Desde `rust-v0.81.0` el manejador de `initialize` fija el
  originator al `clientInfo.name` del cliente
  (`codex-rs/app-server/src/request_processors/initialize_processor.rs:124-165`),
  salvo que el entorno traiga `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`, que gana
  (`default_client.rs:62-81`). El token de versión es siempre el primero. La
  respuesta de `thread/start` trae además `thread.cliVersion`, obligatorio,
  con la versión del CLI que creó el hilo.
- **El volcado nuevo.** `codex app-server generate-json-schema --out <dir>`
  (sin `--experimental`) sobre 0.156.1 produce 310 archivos. De los 21
  vendorizados, 20 difieren en bytes y 16 en contenido (cuatro solo reordenan
  claves); `TurnInterruptResponse.json` queda igual. Todo lo que el adaptador
  **envía** sigue validando, medido con el mismo validador de la suite, y lo
  que **lee** conserva tipo y obligatoriedad.
- **Lo que el servidor envía además.** Sin `experimentalApi`, la respuesta
  real de `thread/start` trae campos que el volcado estable no declara
  (`runtimeWorkspaceRoots`, `activePermissionProfile`, `multiAgentMode`, y en
  `thread`: `environments`, `extra`, `canAcceptDirectInput`,
  `daybreakEnabled`). Validan solo porque ningún esquema entrante declara
  `additionalProperties: false`. El adaptador se apoya únicamente en lo que el
  volcado estable declara.
- **La transcripción real.** Grabada el 2026-09-24 con un cliente mínimo que
  se presenta con el mismo `clientInfo` que el adaptador: un turno de solo
  texto y un turno que pide escribir un archivo, cuya aprobación se declinó.
  Lo que decide: la respuesta a `turn/start` llega **primero**, con
  `status: "inProgress"`, y el turno cierra con `turn/completed`; el servidor
  **devuelve el mensaje del usuario** como ítem `userMessage` en cada turno;
  antes de la respuesta de `thread/start` llegan `remoteControl/status/changed`
  y `account/updated`; después, `thread/started`; durante el turno,
  `mcpServer/startupStatus/updated` (tres servidores MCP de la configuración
  del mantenedor: `codex_apps`, `cua_repl`, `node_repl`),
  `thread/status/changed`, `thread/tokenUsage/updated`,
  `serverRequest/resolved` y `account/rateLimits/updated`. Las notificaciones
  llevan `emittedAtMs` junto a `params`, las respuestas siguen sin
  `"jsonrpc"`, y el CLI etiqueta el hilo con `source: "vscode"` aunque el
  cliente se presentó con su propio nombre.
- **Política efectiva observada** con la configuración del mantenedor y sin
  que el cliente pidiera nada: `approvalPolicy: "on-request"`,
  `approvalsReviewer: "user"`, `sandbox: {"type": "readOnly",
  "networkAccess": false}`, en Windows.
- **Gobierno de aprobaciones.** `approvalsReviewer`
  (`user | auto_review | guardian_subagent`) es nuevo: con `auto_review` un
  subagente del proveedor aprueba o deniega en lugar del cliente. Y la
  política vigente de `own-adapters` («Passthrough de permisos con compuerta
  dura y pérdidas visibles») exige que el adaptador configure la compuerta dura
  nativa del CLI «incluso en los modos permisivos». El adaptador de Codex nunca
  la configuró: dejaba política y sandbox a la configuración del usuario. Con
  `on-request` y `dangerFullAccess`, con `externalSandbox`, con
  `workspaceWrite` y red, con `never`, o con una política `granular` que no
  pregunta, herramientas corren sin pasar por el proxy.
- **MCP del propio CLI.** Los servidores MCP que el usuario configuró en su
  Codex corren fuera del sandbox del CLI, y la aprobación de sus llamadas la
  decide el CLI según política (`codex-rs/codex-mcp/src/elicitation.rs`);
  cuando pide aprobación lo hace por `mcpServer/elicitation/request`, que el
  adaptador hoy rehúsa.
- **Control remoto.** `remoteControl/status/changed`
  (`disabled | connecting | connected | errored`) llega en cada conexión. Con
  el control remoto de Codex activo, otro cliente podría engancharse a los
  hilos y contestar aprobaciones fuera del proxy: un canal remoto que la
  constitución §3 no admite (el acceso remoto es únicamente por túnel SSH).
- **Peticiones del servidor.** `ServerRequest.json` enumera diez. El adaptador
  releva dos y responde `-32601` a las demás (`mod.rs:503-515`); el efecto de
  ese error, leído el servidor, se tabula en D7.
- **Ámbito de lo que llega.** El adaptador no filtraba por `threadId` ni por
  `turnId` y la cola `incoming` se comparte entre turnos: un `error` sin
  reintento termina el turno, y un `turn/completed` rezagado del mismo turno
  hace fallar de inmediato el prompt siguiente.
- **El rango.** `SUPPORTED_FLOOR = (0,77,0)` y `SUPPORTED_CEILING = (1,0,0)`.
  La propuesta afirmó que el techo rehusaría 0.156.1; es falso: `(0,156,1) <
  (1,0,0)`. El rango vigente ya aceptaba 79 minors sin verificar, y lo único
  que lo impidió fue que la versión no se podía leer. Codex publica un minor
  estable unas dos veces por semana.
- **El catálogo.** `fleet/list` reporta nivel verificado y fecha de la última
  corrida persistida (`fleet.rs:1039-1045`), pero no la versión del CLI contra
  la que se midió: con otro CLI instalado, la flota sigue pintando verificado.

## Goals / Non-Goals

**Goals**

- Leer la versión del CLI donde el CLI la escribe, sin adivinarla ni
  presentarse como otro cliente.
- Re-anclar el volcado y el guion a 0.156.1, con el guion siguiendo el orden
  del cable real y validado contra una transcripción vendorizada.
- Configurar la compuerta dura nativa de Codex, verificarla contra lo que el
  CLI hizo efectivo y rehusar cuando no se cumple; declarar a la vista lo que
  Codex sigue corriendo sin el proxy.
- Mapear lo que cambió en el cable y toca a este adaptador: el eco del
  usuario, el contexto de una aprobación (red, terminal, raíz de escritura),
  las peticiones nuevas, el ámbito por hilo y turno, los avisos del proveedor.
- Declarar el rango con honestidad: verificado frente a aceptado, en la
  sesión, en el log y en el catálogo.
- Medir contra el binario real y otorgar el nivel verificado solo si se mide.

**Non-Goals**

- Relevar al proxy las peticiones que hoy se rechazan: escaladas de permisos
  del sandbox (`item/permissions/requestApproval`), elicitaciones MCP
  (`mcpServer/elicitation/request`) y preguntas de herramienta
  (`item/tool/requestUserInput`). Las elicitaciones de aprobación MCP y el
  gobierno de los servidores MCP que Codex arranca van a
  `codex-reanudar-y-mcp`; las preguntas, a `preguntas-del-agente`; las
  escaladas de sandbox, a una change propia que se nombra al archivar esta.
- Reanudar hilos, `turn/steer`, `thread/fork` y `thread/compact/start`, que ya
  existen en `ClientRequest.json` de 0.156.1: `codex-reanudar-y-mcp` y, para
  relevar un turno, `redirigir-turno` si el esquema lo sostiene.
- Tokens y límites (`thread/tokenUsage/updated`, `account/rateLimits/updated`):
  `extension-acp-usage`.
- Diffs y salida de comandos en vivo, subagentes, plan del agente
  (`turn/diff/updated`, `item/commandExecution/outputDelta`, `turn/plan/updated`,
  ítems `plan`, `collabAgentToolCall`, `subAgentActivity`):
  `herramientas-a-la-vista`. El resto de las 82 notificaciones de 0.156.1 que
  el adaptador no mapea se sigue ignorando sin nota.
- Atender peticiones del servidor fuera de un turno en una tarea propia
  (riesgo R3).

## Decisions

### D1 — La versión se lee del primer token, con el producto que el adaptador anunció

- `version::of(user_agent, announced)` lee **solo el primer token**, lo parte
  en el primer `/` y acepta exactamente dos productos: `codex_cli_rs` y
  `announced`, la misma constante `SPEC.name` que viaja en `clientInfo.name`.
  Sufijos de prerelease o de build se ignoran, como antes.
- **Por qué el primer token.** Es la posición que el código del proveedor
  fija; el token de terminal puede traer su propia versión, y un nombre que
  elige el cliente no debe dejar pasar una de esas por la del CLI.
- **Por qué conservar `codex_cli_rs`.** Un CLI anterior a 0.81 no hace eco;
  así recibe el desfase con «actualiza el CLI» y no un «ilegible».
- **Contraste con `thread.cliVersion`.** El desfase se decide en el handshake,
  antes de abrir un hilo, porque la regla es rehusar antes de hablar. Al abrir
  el hilo, el adaptador contrasta la versión leída con `thread.cliVersion` y
  rehúsa si difieren: el CLI que responde no sería el que se leyó. En un hilo
  reanudado ese campo es la versión del creador; lo decide
  `codex-reanudar-y-mcp`.
- **Descartado: presentarse como `codex_cli_rs`.** El nombre es el header
  `originator` de todas las peticiones del CLI a su backend, y `codex_cli_rs`,
  `codex_vscode`, `codex-tui` y `Codex *` cuentan como clientes de primera
  parte (`default_client.rs:139-144`). Sería suplantar a otro cliente (§2). El
  adaptador tampoco envía `threadSource`, `serviceName`, `sessionStartSource`
  ni `capabilities`: la etiqueta `source: "vscode"` que el CLI pone al hilo es
  suya, y la guía de agentes lo dice.
- **`CODEX_INTERNAL_ORIGINATOR_OVERRIDE`.** Si el entorno lo trae, el primer
  token es su valor y se rehúsa, con un detalle que nombra la variable.
  Aceptarla acoplaría el adaptador a un nombre interno; quitarla del entorno
  del hijo sería intervenir en lo que el CLI dice a su backend.

### D2 — Rango: piso verificado, techo en el mayor siguiente, y la distancia dicha

- `VENDORED_SCHEMA_VERSION = "0.156.1"` y `SUPPORTED_FLOOR = (0,156,1)`: el
  piso es la versión de la que viene el volcado. Debajo se rehúsa con
  «actualiza el CLI», y el proveedor le da la razón: 0.77.0 ya no sirve el
  modelo por defecto de una cuenta actual.
- `SUPPORTED_CEILING = (1,0,0)` se conserva. Un techo en el minor siguiente
  dejaría sin agente, en días, a todo usuario que actualice, y el proveedor deja
  de servir sus modelos por defecto a los CLI viejos. Lo que cambia es la
  honestidad: una versión posterior a la verificada **abre declarando la
  distancia** —una fila visible en la sesión antes del primer turno, y
  `providerVerifiedVersion` junto a `providerVersion` en la procedencia— y el
  catálogo dice contra qué versión se midió su nivel (D11).
- **Supuestos que dependen de la versión**, enumerados porque el margen
  descansa en ellos: que un error JSON-RPC a una petición del servidor se trate
  como el efecto tabulado en D7; que una respuesta ilegible a una aprobación
  cuente como denegación; que el revisor y la política pedidos se respeten; y
  que `sandbox` refleje lo efectivo (el volcado lo marca como legado en favor de
  un campo experimental).
- **Por qué el gobierno no queda expuesto a ellos.** La compuerta de D5 falla
  cerrada: un valor de política, revisor o sandbox desconocido o ausente
  rehúsa la sesión, así que un minor que cambie esos valores deja al usuario sin
  sesión, nunca con una sesión sin compuerta. La conformidad pinea los enums de
  `AskForApproval`, `SandboxPolicy` y `ApprovalsReviewer` del volcado: si crecen,
  el próximo re-anclaje falla. Y lo demás falla a la vista: un ítem ilegible se
  anota, un ítem desconocido se muestra, una petición desconocida se rehúsa.
- **Enmienda de spec.** La regla viva decía «nunca asumirse compatible» y el
  rango vigente la incumplía en silencio. Esta change la reemplaza por una que
  se puede cumplir: fuera del rango se rehúsa; dentro, por encima de lo
  verificado, se declara; y un valor de gobierno no reconocido rehúsa. Es un
  MODIFIED sobre `own-adapters`, acotado con `WHERE` a los adaptadores que
  pueden volcar su esquema (hoy solo este: el de Claude no declara rango).

### D3 — Re-anclaje: el volcado estable, byte a byte, repartido por tarea

- Los 20 archivos que difieren se copian byte a byte del volcado (`v1/` para
  el handshake, `v2/` para hilo, turno, ítems y deltas, la raíz para las
  aprobaciones y `JSONRPCMessage.json`), junto con
  `ThreadStatusChangedNotification.json`,
  `ThreadTokenUsageUpdatedNotification.json` y
  `ServerRequestResolvedNotification.json`, que el guion lleva.
- `ClientNotification.json` entra con la conformidad estricta (tarea 1.4) y
  `ServerRequest.json` con la tabla de peticiones (tarea 2.4): cada archivo
  llega con quien lo usa.
- El volcado se genera **sin `--experimental`** y el adaptador no declara
  `capabilities`. Es la superficie en la que el adaptador puede apoyarse; el
  servidor envía además campos que no declara (Context), y el adaptador no los
  lee.
- `PROVENANCE.md` corrige la fecha del binario anterior (publicado el
  2025-12-21, volcado el 2026-07-27) y la frase que decía que la generación de
  métodos v1 seguía existiendo.

### D4 — El guion sigue el cable real, y la transcripción viaja con él

- El guion se reescribe con el orden de la grabación: respuesta de
  `turn/start` primero y en curso, eco `userMessage`, mensaje del agente con
  `phase`, un cambio de archivo cuya aprobación se pide con `startedAtMs`, las
  tres notificaciones intercaladas y el cierre con `turn/completed`. Conserva
  los ids y textos que los e2e ya fijan (`mock-thread-1`, `turn-1`, `item-2`,
  «Working on it.», `NOTES.md`), el user agent lleva el token de terminal
  (`unknown`), y el `serverRequest/resolved` apunta a la petición que el cable
  simulado numera (`1`).
- **Diferencias declaradas en su cabecera**, y solo esas: la aprobación se
  concede y el ítem cierra `completed` (en la grabación se declinó), se omiten
  `emittedAtMs` y los campos que el volcado estable no declara, y se envía
  `"jsonrpc"`. El adaptador tolera las dos formas.
- **La transcripción se vendoriza saneada** como
  `core/mock-provider/scripts/codex-app-server-0.156.1.transcript.ndjson`: ids,
  rutas, nombre de máquina, identificador de instalación, plan y límites
  reemplazados. Un test exige que la secuencia de métodos del guion sea una
  subsecuencia ordenada de la de la transcripción, incluida la respuesta de
  `turn/start` antes de `turn/started`.
- Los guiones retenidos que llevan dentro los tests del daemon
  (`conformance_suite.rs`, `e2e_adaptadores_cancel.rs`) pasan a la forma de
  0.156.1, con los campos obligatorios nuevos.

### D5 — La compuerta dura nativa, pedida, verificada y con sus pérdidas dichas

- El adaptador **pide la compuerta más estricta que el CLI ofrece** en
  `thread/start`: `approvalPolicy: "untrusted"` (pregunta por todo comando que
  no sea de confianza), `sandbox: "read-only"` (toda escritura se pide como
  escalada) y `approvalsReviewer: "user"` (quien contesta es el cliente, es
  decir, el proxy). Es la configuración que el requisito vivo exige («la
  compuerta dura nativa del CLI … incluso en los modos permisivos») y solo
  estrecha: lo que el proxy decide, lo decide por reglas o por el humano, y los
  modos de autonomía del daemon relajan desde ahí, nunca el CLI por su cuenta.
- **Lista blanca que falla cerrada.** La respuesta de `thread/start` trae la
  política efectiva. La sesión abre solo si `approvalPolicy` es `"untrusted"`,
  `approvalsReviewer` es `"user"` y `sandbox` es `{"type": "readOnly"}` sin
  `networkAccess: true`. Cualquier otro valor —una configuración gestionada que
  impone otra cosa, un valor que un minor añada, un campo ausente— rehúsa con
  diagnóstico y remedio. `granular`, `never`, `on-request`, `dangerFullAccess`,
  `externalSandbox` y `workspaceWrite` quedan fuera sin tener que enumerarlos.
- La política efectiva viaja en la procedencia
  (`providerApprovalPolicy`, `providerApprovalsReviewer`, `providerSandbox`).
- **Pérdidas visibles.** Lo que Codex sigue corriendo sin preguntar bajo esa
  compuerta se declara en una fila visible al abrir la sesión: comandos que el
  CLI considera de confianza (lecturas, dentro del sandbox de solo lectura), la
  búsqueda web del proveedor, y las llamadas a servidores MCP de la
  configuración del CLI que el propio CLI no somete a aprobación. Cada servidor
  MCP que el CLI informa arrancando (`mcpServer/startupStatus/updated`) se
  nombra en una fila visible, una vez por sesión: corre fuera del sandbox del
  CLI y fuera del proxy. Gobernarlos es de `codex-reanudar-y-mcp`.
- **Control remoto.** Si una notificación `remoteControl/status/changed` dice
  algo distinto de `disabled` —antes del primer turno o durante uno—, la
  sesión se rehúsa (o el turno se interrumpe y falla) con remedio: otro cliente
  podría contestar aprobaciones fuera del proxy (§3).
- **Descartado: dejar política y sandbox a la configuración del usuario.**
  Era la regla anterior del adaptador, apoyada en §5; pero §5 rige el núcleo,
  no un adaptador que existe para un proveedor, y el requisito de compuerta
  dura la contradecía.
- **Enmienda de spec.** El requisito vivo de compuerta dura dice que «ninguna
  herramienta SHALL ejecutarse sin decisión del proxy», y ni la compuerta más
  estricta de Codex lo cumple: las lecturas de confianza, la búsqueda web y
  parte de MCP no preguntan. El MODIFIED añade lo que ya prometía su nombre —
  pérdidas visibles— para lo que el proveedor corre sin relevar, y exige
  declararlo al abrir la sesión.

### D6 — Un turno solo lo mueve lo que es suyo, y lo que se declina se ve

- El adaptador conoce el hilo de su sesión desde `thread/start`. Una
  notificación con `threadId` de otro hilo no mueve el turno; se anota. Una
  notificación **sin** `threadId` (`configWarning`, `deprecationNotice`,
  `warning`) no se filtra por ese eje, ni una sin `turnId` por el suyo.
- Una vez conocido el turno en curso, lo de otro turno no lo mueve, y los
  turnos terminados se recuerdan para que un `turn/completed` rezagado no
  cierre el siguiente aunque llegue antes de que el siguiente tenga id.
- Una aprobación de otro hilo o de otro turno se **declina sin preguntar y a
  la vista**: una herramienta fallida que dice por qué, como en D7.
- Un hilo que el CLI abre como hijo del de la sesión (`thread/started` con
  `parentThreadId` igual) se declara en una fila visible: su actividad todavía
  no se muestra (`herramientas-a-la-vista`) y sus aprobaciones se declinan.

### D7 — Toda petición del servidor tiene una respuesta declarada

- Una tabla asigna a cada método de `ServerRequest.json` **relevar**
  (aprobación de comando y de cambio de archivo) o **rehusar** (las demás). El
  rehúso es `-32601`, la respuesta del protocolo. Efecto en el servidor, leído
  en el tag:

  | Petición | Efecto de `-32601` |
  | --- | --- |
  | `item/permissions/requestApproval` | concesión vacía para el turno |
  | `mcpServer/elicitation/request` | declinada |
  | `item/tool/requestUserInput` | respuestas vacías; el turno sigue |
  | `item/tool/call` | `success: false`; solo llega con herramientas dinámicas declaradas, que el adaptador no declara |
  | `account/chatgptAuthTokens/refresh` | la llamada al backend falla; solo existe en autenticación externa |
  | `attestation/generate` | nunca llega: exige `capabilities.requestAttestation` |
  | `applyPatchApproval`, `execCommandApproval` | nunca llegan: son de turnos v1, que ya no se pueden iniciar |

- El rehúso **se ve**: una herramienta fallida con `toolCallId`
  `server-request:<id>`, clase «otra», título con el método y el motivo, y sin
  los parámetros. El refresco de credenciales y la atestación quedan pineados
  por test: reciben error, jamás llegan al proxy y sus parámetros no se
  registran (§2).
- La conformidad recorre el `ServerRequest.json` vendorizado y exige política
  para cada método, con un test negativo que prueba que un método nuevo falla
  nombrándolo.
- **Contexto de una aprobación.** Una aprobación de comando puede traer
  `networkApprovalContext` (host y protocolo, con `command` y `cwd` nulos) o
  `kind: "writeStdin"` (escribir en una terminal en curso); una de cambio de
  archivo puede traer `grantRoot` (pide escribir bajo esa raíz por el resto de
  la sesión). El adaptador lleva cada uno al título, al contenido y a la
  ubicación de la petición de permiso, para que el humano y las reglas decidan
  sobre lo que de verdad se concede. Si el `toolCallId` de la aprobación nunca
  se anunció como ítem (el caso de red), el adaptador emite antes una
  herramienta con ese id. Las enmiendas persistentes
  (`acceptWithExecpolicyAmendment`, `applyNetworkPolicyAmendment`) siguen sin
  ofrecerse.

### D8 — Lo que el mapeo muestra, calla y anota

- `userMessage` no se muestra: el daemon ya registró el prompt.
- Un turno `failed` se rehúsa con `turn.error.message` y, si viene,
  `additionalDetails`.
- Los avisos que el proveedor escribe para el humano —`warning`,
  `configWarning`, `deprecationNotice`, `guardianWarning`,
  `windows/worldWritableWarning` y `model/rerouted`— se anotan en el registro
  del adaptador (que el daemon recoge), también cuando llegan durante el
  handshake. No van al transcript como palabra del agente.
- **Privacidad de rutas.** No se tipa, no se registra y no viaja `codexHome`,
  `thread.path`, `instructionSources` ni ruta alguna de `configWarning`: son
  rutas del estado del CLI, credenciales incluidas (§2).

### D9 — Conformidad estricta en lo que sale, e identidad pineada

- Cada clave que el adaptador serializa en `params` y `result` existe en las
  propiedades del esquema volcado, resolviendo `$ref` y la rama de `oneOf` por
  discriminante; en `initialized` se valida el mensaje sin el miembro
  `jsonrpc` del sobre, que ningún esquema declara. Un test negativo con una
  clave mal escrita prueba que el fallo la nombra.
- El `thread/start` que el adaptador envía no lleva más claves que `cwd`,
  `model`, `approvalPolicy`, `approvalsReviewer` y `sandbox`, y
  `clientInfo.name` es el nombre propio del adaptador: ningún nombre de cliente
  de primera parte y ninguna clave de clasificación analítica.

### D10 — La corrida real decide el nivel

Tras el código, la conformidad manual de dos piernas contra `codex-cli
0.156.1` (sesión, streaming, permisos, cancelación) y una sesión real por
Meltemi con una escritura aprobada. Se registra en `docs/conformidad-manual.md`
con fecha, versión, plataforma, la política efectiva observada y la
consecuencia de la configuración MCP del mantenedor; y se registra también la
corrida fallida de 0.77.0 ya medida, con las palabras del proveedor. El nivel 2
se otorga solo si las cuatro comprobaciones pasan (constitución §5: manual y
opt-in).

### D11 — El catálogo dice contra qué versión se midió

La corrida persistida ya guarda la versión del agente. `fleet/list` gana, de
forma aditiva, `verifiedVersion` junto a `verifiedLevel` y `verifiedAt`, y las
dos superficies lo muestran («verificado con 0.156.1»). Es un campo del
contrato consumido por GUI y TUI (§4).

### D12 — Tests y dónde se enlaza cada escenario

- `version.rs` y `codex/mod.rs`: lectura de versión, identidad, contraste con
  `cliVersion`, compuerta de D5, versión posterior, control remoto, ámbito de
  D6, rehúso visible de D7, turno fallido, avisos del handshake.
- `mapping.rs` y `permission.rs`: eco silenciado, avisos, contexto de red,
  terminal y raíz de escritura, servidores MCP y pérdidas declaradas.
- `tests/codex_conformance.rs`: volcado, guion frente a transcripción, claves
  salientes, `initialized`, tabla de peticiones, enums de gobierno, piso
  anclado, guía de agentes.
- `core/mock-provider/tests/codex_wire.rs` y los e2e de `core/meltemid`: la
  forma nueva y el orden real.
- `core/meltemid` y las superficies: `verifiedVersion` en el catálogo.
- Los escenarios de la corrida real se verifican por documentación
  (`sdd/verify-mark`).

## Risks / Trade-offs

- **R1 — Un minor futuro rompe el mapeo.** Declarado (D2). La compuerta falla
  cerrada; el resto falla a la vista.
- **R2 — El backend rechaza un originator que no es de primera parte.** Dos
  turnos corrieron con `meltemi-codex-acp`. Si algún día se rechaza, la
  respuesta no es cambiar el nombre (§2).
- **R3 — Peticiones fuera de un turno.** La cola se lee dentro de un turno; el
  servidor aborta las aprobaciones pendientes en cada transición, y con D6 las
  de un turno que ya no es el actual se declinan a la vista.
- **R4 — La compuerta estricta cuesta preguntas.** Con `untrusted` y
  `read-only`, cada escritura y cada comando que no sea de confianza pasan por
  el proxy. Es la misma postura que el adaptador de Claude; las reglas y los
  modos de autonomía del daemon son el lugar para relajarla.
- **R5 — Que una escritura aprobada bajo `read-only` no se aplique.** La
  grabación declinó; la corrida real de D10 aprueba una. Si Codex no aplica la
  escritura escalada, la alternativa medida es `workspace-write` sin red, con
  las escrituras dentro del worktree declaradas como pérdida visible; se decide
  con la medición, por escrito.
- **R6 — Configuraciones que ahora rehúsan.** Quien configuró `never`,
  `auto_review`, control remoto o un sandbox abierto no abre sesión de Codex
  bajo Meltemi. Es la intención; el remedio dice qué cambiar.
- **R7 — `serverRequest/resolved` durante una espera del proxy.** Con el
  revisor fijado en `user`, el servidor no resuelve por su cuenta; la espera ya
  cede ante la interrupción.

## Migration

No hay datos que migrar. Un Codex por debajo de 0.156.1 recibe el desfase con
«actualiza el CLI»; uno posterior abre con la distancia declarada. La guía de
agentes gana la versión mínima, la postura de la compuerta y el nombre con que
el adaptador se presenta.

## Lo que solo la corrida real confirma

- Que una escritura aprobada bajo `read-only` se aplica (R5), y el streaming,
  la cancelación y los tiempos de un turno.
- Cómo se comporta el sandbox de solo lectura en Windows con una escalada
  aprobada.
- Qué muestra la bandeja ante una aprobación de red.
