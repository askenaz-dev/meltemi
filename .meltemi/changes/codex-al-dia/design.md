# codex-al-dia — design

## Context

Verificado el 2026-09-24 contra el código, contra el volcado de esquema del
CLI instalado y contra una transcripción del servidor real:

- **Qué falla hoy.** Con `codex-cli 0.77.0`, la versión a la que
  `adaptadores-propios-acp` ancló el esquema, el handshake pasa y el proveedor
  rechaza cada turno («The 'gpt-5.6-sol' model requires a newer version of
  Codex»). Con `codex-cli 0.156.1`, el adaptador rehúsa el handshake: el user
  agent es `meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64)
  xterm-256color (meltemi-codex-acp; 0.1.1)` y `version::of`
  (`core/meltemi-adapters/src/codex/version.rs:26-41`) busca un token
  `codex_cli_rs/` que ya no existe.
- **Por qué cambió el user agent.** Código fuente de Codex, tag
  `rust-v0.156.1`: `codex-rs/login/src/auth/default_client.rs:150-177` arma el
  user agent como `{originator}/{build_version} ({os} {os_version}; {arch})
  {terminal}` y le añade ` ({name}; {version})`; `build_version` es la versión
  del workspace, la del CLI. Desde `rust-v0.81.0` el manejador de `initialize`
  fija el originator al `clientInfo.name` del cliente
  (`codex-rs/app-server/src/request_processors/initialize_processor.rs:124-165`),
  salvo que el entorno traiga `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`, que gana
  (`default_client.rs:62-81`). El token de versión es siempre el primero.
- **El volcado nuevo.** `codex app-server generate-json-schema --out <dir>`
  sobre 0.156.1 produce 310 archivos (raíz, `v1/` con solo el handshake, y
  `v2/`). De los 21 vendorizados cambian 20; `TurnInterruptResponse.json` queda
  igual. Lo que el adaptador **envía** sigue siendo válido contra el volcado
  nuevo, medido con el mismo validador de la suite: ninguna petición ganó un
  campo obligatorio ni perdió uno de los que el adaptador usa. Lo que el
  adaptador **lee** (`userAgent`, `thread.id`, `turn.id`, `turn.status`, los
  campos de aprobación) conserva tipo y obligatoriedad.
- **Lo que se rompe al re-anclar** son los fixtures: el guion del cable
  (`core/mock-provider/scripts/codex-app-server.ndjson`) no trae los campos
  que ahora son obligatorios (`codexHome`, `platformFamily`, `platformOs`,
  `approvalsReviewer`, `startedAtMs`, `completedAtMs`, varios del hilo), y
  varios tests fijan el prefijo `codex_cli_rs/` o `required == ["userAgent"]`.
- **La transcripción real.** Grabada el 2026-09-24 con un cliente mínimo que
  se presenta con el mismo `clientInfo` que el adaptador: un turno de solo
  texto y un turno que pide escribir un archivo, cuya aprobación se declinó.
  Tres cosas que el guion de 0.77.0 tenía que adivinar y que la grabación
  decide: la respuesta a `turn/start` llega **primero**, con
  `status: "inProgress"`, y el turno cierra con `turn/completed`; el servidor
  **devuelve el mensaje del usuario** como ítem `userMessage` en cada turno; y
  entre los ítems del turno llegan `thread/status/changed`,
  `thread/tokenUsage/updated`, `serverRequest/resolved` y
  `account/rateLimits/updated`. Las notificaciones llevan además un miembro
  `emittedAtMs` junto a `params`, y las respuestas siguen sin `"jsonrpc"`.
- **Gobierno de aprobaciones.** 0.156.1 añade `approvalsReviewer`
  (`user | auto_review | guardian_subagent`) a `thread/start` y lo devuelve,
  obligatorio, en su respuesta. `auto_review` hace que un subagente del
  proveedor apruebe o deniegue en lugar del cliente. En la transcripción, con
  la configuración del mantenedor: `approvalPolicy: "on-request"`,
  `approvalsReviewer: "user"`, `sandbox: {"type": "readOnly"}`.
- **Peticiones del servidor.** `ServerRequest.json` enumera diez. El adaptador
  releva dos (aprobación de comando y de cambio de archivo) y responde
  `-32601` a toda otra (`mod.rs:503-515`). Leído el código del tag: el
  servidor trata una respuesta de error como rechazo en las diez, así que
  ninguna cuelga un turno y el efecto es siempre «no».
- **Ámbito de lo que llega.** El adaptador no filtra por `threadId` ni por
  `turnId` (`mapping.rs:102-121`, `mod.rs:429-446`), y la cola `incoming` se
  comparte entre turnos (`mod.rs:146,166`). Un `error` sin reintento termina el
  turno; un `turn/completed` rezagado del mismo turno queda en cola y hace
  fallar de inmediato el prompt siguiente. 0.156.1 añade ítems de subagente
  que referencian otros hilos.
- **Mapeo de ítems.** Un tipo desconocido se muestra como herramienta con su
  tipo crudo (`mapping.rs:306-319`). Con 0.156.1 eso incluye `userMessage`:
  cada turno mostraría el propio prompt del usuario como una herramienta.
- **El rango.** `SUPPORTED_FLOOR = (0,77,0)` y `SUPPORTED_CEILING = (1,0,0)`
  (`wire.rs:38,48`). La propuesta afirmó que el techo rehusaría 0.156.1; es
  falso: `(0,156,1) < (1,0,0)`. El rango vigente ya aceptaba 79 minors sin
  verificar, y lo único que lo impidió fue que la versión no se podía leer.
  Codex publica un minor estable unas dos veces por semana.

## Goals / Non-Goals

**Goals**

- Que el adaptador lea la versión del CLI donde el CLI la escribe, sin
  adivinarla ni suplantar a nadie para recuperarla.
- Re-anclar el volcado vendorizado y el guion simulado a 0.156.1, con el guion
  siguiendo la forma y el orden del cable real.
- Mapear lo que cambió en el cable y toca a este adaptador: el revisor de
  aprobaciones, el contexto de red de una aprobación, el eco del usuario, las
  peticiones nuevas del servidor, y el ámbito por hilo y turno.
- Declarar el rango con honestidad: verificado frente a aceptado.
- Medir contra el binario real y otorgar el nivel verificado solo si se mide.

**Non-Goals**

- Relevar al proxy las peticiones nuevas que hoy se rechazan: escalada de
  permisos del sandbox (`item/permissions/requestApproval`), elicitaciones
  MCP (`mcpServer/elicitation/request`) y preguntas de herramienta
  (`item/tool/requestUserInput`). Van a `codex-reanudar-y-mcp` (las
  elicitaciones de aprobación MCP) y a una change propia de escaladas de
  sandbox; las preguntas, a `preguntas-del-agente`.
- Reanudar hilos y entregar MCP a Codex: `codex-reanudar-y-mcp`.
- Tokens y límites de uso (`thread/tokenUsage/updated`,
  `account/rateLimits/updated`): `extension-acp-usage`.
- Diffs y salida de comandos en vivo, subagentes, plan del agente:
  `herramientas-a-la-vista`.
- Atender peticiones del servidor fuera de un turno, en una tarea propia:
  anotado como riesgo (R3).
- Cambiar la política de aprobación o el sandbox que el usuario configuró en
  su Codex, salvo la única combinación que D5 rehúsa.

## Decisions

### D1 — La versión se lee del primer token, con el producto que el adaptador anunció

`version::of(user_agent, announced)` lee **solo el primer token** del user
agent (`split_whitespace().next()`), lo parte en el primer `/` y acepta
exactamente dos productos: `codex_cli_rs` y `announced`, que es la misma
constante `SPEC.name` que viaja en `clientInfo.name`. La versión es la parte
`major.minor.patch` de lo que sigue, con cualquier sufijo de prerelease o de
build ignorado, como hoy.

- **Por qué el primer token.** Es la posición que el código del proveedor
  fija. Hoy se busca en todos los tokens, y el token de terminal puede traer su
  propia versión (`vscode/1.95.3`, `WezTerm/…`); si el producto aceptado
  incluye un nombre que elige el cliente, cualquier token posterior podría
  leerse como la versión del CLI.
- **Por qué conservar `codex_cli_rs`.** Un CLI entre 0.77 y 0.80 no hace eco;
  aceptarlo hace que su usuario reciba el desfase con «actualiza el CLI» en
  vez de un «ilegible» engañoso.
- **Descartado: volver a la forma vieja enviando `codex_cli_rs` como
  `clientInfo.name`.** El nombre es el header `originator` de todas las
  peticiones del CLI a su backend, y `codex_cli_rs`, `codex_vscode`,
  `codex-tui` y `Codex *` cuentan como clientes de primera parte
  (`default_client.rs:139-144`). Presentarse así sería suplantar a otro
  cliente (constitución §2).
- **`CODEX_INTERNAL_ORIGINATOR_OVERRIDE`.** Si el entorno del CLI trae esa
  variable, el primer token es su valor y la regla de dos productos rehúsa.
  Se rehúsa, y el detalle nombra la variable. Aceptarla como tercer producto
  acoplaría el adaptador a un nombre que el proveedor declara interno; quitarla
  del entorno del hijo sería intervenir en lo que el CLI dice a su backend.
- **El rehúso deja de culpar al CLI.** El detalle de
  `provider_version_unreadable` dice lo que se vio y las dos formas esperadas,
  sin sugerir que el binario no sea el oficial.

### D2 — Rango: piso verificado, techo en el mayor siguiente, y la distancia dicha

- `VENDORED_SCHEMA_VERSION = "0.156.1"` y `SUPPORTED_FLOOR = (0,156,1)`: el
  piso sigue siendo la versión de la que viene el volcado, y el test que lo
  exige se conserva. Debajo del piso se rehúsa con «actualiza el CLI», y la
  medición de hoy le da la razón: 0.77.0 no sirve el modelo por defecto de una
  cuenta actual.
- `SUPPORTED_CEILING = (1,0,0)` se conserva. En `0.x` un minor puede romper,
  y esta change lo vio: el eco del originator llegó en 0.81. Pero un techo en
  el minor siguiente rehusaría a todo usuario que actualice Codex en la semana
  siguiente a cada re-anclaje, y el proveedor, a su vez, deja de servir los
  modelos por defecto a los CLI viejos. Entre las dos rigideces, esta change
  elige la que no deja al usuario sin agente y hace visible lo que no está
  verificado.
- **Lo que cambia es la honestidad del rango.** Una versión por encima de la
  verificada y por debajo del techo **abre la sesión declarando la distancia**:
  la procedencia gana `providerVerifiedVersion` junto a `providerVersion`, y
  la sesión recibe una nota visible («Codex 0.160.0 es más nuevo que 0.156.1,
  la versión contra la que se verificó este adaptador») antes del primer
  turno. Nunca se presenta como verificado lo que no lo está.
- **Por qué esto no abre un agujero de gobierno.** Nada de lo que protege el
  camino de permisos depende del número de versión: el revisor y la política
  se leen de la respuesta de `thread/start` y se exigen en cada sesión (D5), las
  peticiones desconocidas se rechazan (D7), y los ítems desconocidos se
  muestran. Lo que el número protege es que el mapeo funcione, y cuando no
  funciona, falla a la vista y en la dirección segura: un ítem ilegible se
  anota, una respuesta de aprobación que el servidor no entienda cuenta como
  rechazo.
- **Enmienda de spec.** La regla viva dice «nunca asumirse compatible». El
  rango vigente ya la incumplía en silencio; esta change la reemplaza por una
  que se puede cumplir: fuera del rango se rehúsa; dentro, por encima de lo
  verificado, se declara. Es un MODIFIED sobre `own-adapters` («Conformidad
  por versión y desfase rehusado»), y por eso esta change es de vía completa.

### D3 — Re-anclaje: el volcado estable, byte a byte, y cuatro archivos más

- Se copian byte a byte desde el volcado de 0.156.1 los 20 archivos que
  cambiaron (`v1/` para el handshake, `v2/` para hilo, turno, ítems y deltas,
  la raíz para las aprobaciones y `JSONRPCMessage.json`).
- Se añaden: `ServerRequest.json` (para que la conformidad exija una política
  por cada petición posible, D7), `ClientNotification.json` (para validar el
  `initialized` que el adaptador envía y que hoy nadie valida), y
  `ThreadStatusChangedNotification.json`,
  `ThreadTokenUsageUpdatedNotification.json` y
  `ServerRequestResolvedNotification.json` (las tres notificaciones que el
  guion lleva para probar que ignorarlas no cuesta nada, D4).
- El volcado se genera **sin `--experimental`** y el adaptador no declara
  `capabilities`: la superficie anclada es la estable, que es exactamente la
  que el servidor sirve a una conexión que no pidió lo experimental. Queda
  escrito en `PROVENANCE.md` y junto a `InitializeParams`.
- `PROVENANCE.md` corrige además dos cosas: la fecha del binario anterior y
  la frase que decía que la generación de métodos v1 seguía existiendo (en
  0.156.1 ya no hay `newConversation` ni `sendUserTurn` en `ClientRequest.json`).
- Las claves del volcado nuevo vienen en orden alfabético; el diff textual
  contra el anterior es ruidoso y no se lee así: lo que cuenta es la
  conformidad.

### D4 — El guion simulado sigue la transcripción real

El guion se reescribe desde la grabación del 2026-09-24, con ids, rutas y
modelo simulados: handshake con el user agent en forma de eco, respuesta de
`thread/start` con la forma real, respuesta de `turn/start` **primero** y en
curso, el eco `userMessage`, un mensaje del agente en dos fragmentos con
`phase`, un cambio de archivo cuya aprobación se pide con `startedAtMs`, las
tres notificaciones intercaladas, y el cierre con `turn/completed`. El orden
adverso (la respuesta al final) sigue cubierto por los tests unitarios del
bucle del turno, que es donde se prueban los dos órdenes. El guion no lleva
`emittedAtMs` y sí `"jsonrpc"`; el adaptador tolera las dos formas y el
comentario de cabecera lo dice.

### D5 — Las aprobaciones llegan al proxy, o la sesión no abre

- El adaptador envía `approvalsReviewer: "user"` en `thread/start`. No elige
  **qué** se pregunta —eso es la política y el sandbox del usuario, que siguen
  siendo suyos—, elige **quién** contesta lo que el CLI ya decidió preguntar,
  y en este adaptador quien contesta es el proxy. Es el contrato del propio
  adaptador («las aprobaciones que el servidor solicite SHALL relevarse»), y
  solo puede estrechar lo que corre sin decisión, nunca ensancharlo.
- La respuesta de `thread/start` trae la política efectiva. El adaptador la
  lee con tipos permisivos —`approvalPolicy` como valor opaco,
  `approvalsReviewer` como texto, el `type` del sandbox— y **rehúsa la
  sesión** en dos casos, con diagnóstico y remedio:
  1. El revisor efectivo no es `user` aunque se pidió (una configuración
     gestionada que lo impone): las aprobaciones no llegarían al proxy.
  2. La política efectiva es `never` y el sandbox no es `readOnly`: el CLI
     declara que no preguntará nunca y puede escribir o ejecutar sin límite de
     sandbox, así que ninguna herramienta pasaría por el proxy.
  Toda otra combinación abre, porque en ella el sandbox del CLI y las
  preguntas que el CLI hace son la compuerta. Es la lectura literal de los
  valores que el proveedor documenta, no una semántica que el adaptador
  invente.
- La política efectiva viaja en la procedencia (`providerApprovalPolicy`,
  `providerApprovalsReviewer`, `providerSandbox`), así que el log de la sesión
  dice bajo qué reglas corrió el CLI.
- **Descartado: fijar `approvalPolicy` o `sandbox`.** Forzar `on-request`
  ensancharía a quien eligió `untrusted`; forzar un sandbox sería asumir su
  semántica de riesgo (§5). Rehusar la única combinación que anula el proxy
  cumple el requisito vivo de compuerta dura sin tocar lo demás.

### D6 — Un turno solo lo mueve lo que es suyo

- El adaptador conoce el hilo de su sesión desde `thread/start`. Toda
  notificación o petición cuyo `threadId` no sea ese se ignora con una nota en
  el registro del adaptador; si es una aprobación, se declina sin preguntar al
  proxy.
- Dentro del turno: una vez conocido el id del turno en curso, lo que trae un
  `turnId` distinto se ignora (o se declina, si es aprobación). Los turnos ya
  terminados se recuerdan, de modo que un `turn/completed` rezagado de un turno
  anterior no cierra el siguiente aunque llegue antes de que el siguiente tenga
  id.
- Cierra el defecto latente del `error` seguido de `turn/completed(failed)`, y
  deja preparado el terreno para los hilos de subagente de 0.156.1, que el
  mapeo todavía no muestra.

### D7 — Toda petición del servidor tiene una respuesta declarada

- Una tabla en el adaptador asigna a cada método de `ServerRequest.json` una
  política: **relevar** (aprobación de comando y de cambio de archivo) o
  **rehusar** (todas las demás). El rehúso sigue siendo `-32601`, la respuesta
  del protocolo y no una inventada; leído el servidor, equivale a «no» en las
  diez.
- El rehúso **se ve en la sesión**: se emite una herramienta fallida titulada
  con el método y el motivo («rehusada: el adaptador no releva esta petición»),
  igual que un ítem desconocido se muestra por su tipo. No se presenta como
  palabra del agente.
- La conformidad recorre el `ServerRequest.json` vendorizado y exige que cada
  método tenga política; el día que el proveedor añada una petición, el
  re-anclaje falla nombrándola.
- `account/chatgptAuthTokens/refresh` y `attestation/generate` quedan
  pineadas por test: reciben error y jamás llegan al proxy. Responderlas con un
  token sería manejar credenciales ajenas o atestiguar ser otro cliente (§2).
- **Contexto de red y escritura a una terminal.** Una aprobación de comando
  puede traer `networkApprovalContext` (host y protocolo, con `command` y `cwd`
  nulos) o `kind: "writeStdin"`. El adaptador lee ambos y los lleva al título y
  al contenido de la petición de permiso, para que el humano vea qué decide.
  Las enmiendas persistentes (`acceptWithExecpolicyAmendment`,
  `applyNetworkPolicyAmendment`) siguen sin ofrecerse: son reglas del
  proveedor que el adaptador no puede prometer en nombre del humano.

### D8 — Lo que el mapeo muestra, calla y anota

- `userMessage` no se muestra: el daemon ya registró el prompt. Mostrarlo
  sería una herramienta fantasma por turno.
- Un turno `failed` se explica con las palabras del proveedor: el rehúso
  lleva `turn.error.message` (y `additionalDetails` cuando viene) en vez de
  «the server reported the turn failed».
- Los avisos que el proveedor escribe para el humano —`warning`,
  `configWarning`, `deprecationNotice` y `model/rerouted`— se anotan en el
  registro del adaptador (que el daemon recoge) con su texto; `configWarning`
  sin su ruta. No van al transcript como si los dijera el agente.
- Nada de esto lee `codexHome`: la ruta del estado del CLI, credenciales
  incluidas, no se tipa, no se registra y no viaja (§2).

### D9 — Conformidad estricta en lo que sale

Los esquemas salientes no declaran `additionalProperties: false`, así que la
validación actual detecta faltantes y tipos pero no un nombre mal escrito o
renombrado. La conformidad gana una comprobación recursiva: cada clave que el
adaptador serializa existe en `properties` del esquema volcado. Se aplica a
todo lo que el adaptador envía, `initialized` incluido.

### D10 — La corrida real decide el nivel

Tras el código, se corre la conformidad manual de dos piernas contra
`codex-cli 0.156.1` (sesión, streaming, permisos y cancelación) y una sesión
real por Meltemi con aprobación. Se registra en `docs/conformidad-manual.md`
con fecha, versión y la política efectiva observada. El nivel 2 se otorga solo
si las cuatro comprobaciones pasan; si alguna falla, la página lo dice con las
palabras del proveedor. Manual y opt-in: CI no ejecuta agentes reales
(constitución §5).

### D11 — Tests y dónde se enlaza cada escenario

- `version.rs`: la lectura (dos productos, primer token, token de terminal con
  versión, variable de override, nombre sin versión, CLI viejo que da desfase).
- `tests/codex_conformance.rs`: volcado, guion, claves salientes,
  `initialized`, tabla de peticiones, piso anclado, página de agentes.
- `codex/mod.rs`: handshake con la forma de eco, procedencia con versión
  verificada y política efectiva, rehúsos de D5, ámbito de D6, rehúso visible
  y pineado de D7, turno fallido con palabras del proveedor.
- `codex/mapping.rs` y `codex/permission.rs`: eco silenciado, avisos
  anotados, contexto de red y `writeStdin` en la petición de permiso.
- `core/mock-provider/tests/codex_wire.rs` y los e2e de `core/meltemid` que
  reproducen el guion: la forma nueva del handshake y el orden real.
- El escenario de nivel re-otorgado se verifica por documentación
  (`sdd/verify-mark`) con la corrida registrada.

## Risks / Trade-offs

- **R1 — Un minor futuro rompe el mapeo.** Aceptado y declarado (D2). Las
  roturas posibles fallan a la vista; `meltemi-doctor` y `corrida-integrada`
  harán barato detectarlas y re-anclar.
- **R2 — El backend rechaza turnos con un originator que no es de primera
  parte.** La grabación de hoy corrió dos turnos con `meltemi-codex-acp` y el
  backend los atendió. Si algún día los rechaza, la respuesta no puede ser
  cambiar el nombre (§2); sería decir al usuario, con las palabras del
  proveedor, que Codex no admite este cliente.
- **R3 — Peticiones fuera de un turno.** La cola se lee solo dentro de un
  turno. Una elicitación MCP entre turnos espera al siguiente prompt, y el
  servidor aborta las aprobaciones pendientes en cada transición de turno.
  Con D6, una aprobación de un turno que ya no es el actual se declina sin
  preguntar, así que no se pregunta al humano en vano; atenderlas en una tarea
  propia queda para quien releve elicitaciones (`codex-reanudar-y-mcp`).
- **R4 — `serverRequest/resolved` durante una espera del proxy.** Mientras
  el humano decide, el adaptador no lee otras notificaciones. Con el revisor
  fijado en `user` el servidor no resuelve por su cuenta; lo haría al abortar
  por transición de turno, que solo ocurre tras una interrupción, y la espera
  ya cede ante la interrupción (`mod.rs:533-536`).
- **R5 — El rehúso de D5 sorprende a quien configuró `never`.** Es la
  intención: bajo Meltemi, una configuración que anula el proxy no abre. El
  remedio dice qué valor usar y dónde.

## Migration

No hay datos que migrar. Un usuario con Codex por debajo de 0.156.1 recibe el
desfase con «actualiza el CLI»; uno con una versión posterior abre con la
distancia declarada. `docs/agentes.md` gana la versión mínima.

## Lo que solo la corrida real confirma

- Que el turno con aprobación, la cancelación y el streaming funcionan de
  punta a punta con 0.156.1 bajo Meltemi, y cuánto tarda un turno.
- Si el Codex del mantenedor tiene servidores MCP o conectores, que harían
  aparecer `mcpServer/elicitation/request` sin opt-in del adaptador.
- Qué muestra la bandeja ante una aprobación de red, cuyo `toolCallId` nunca
  se anunció como ítem.
