# Tasks — aprobador-en-el-cli

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y el simulado

- [x] 1.1 El push lleva su id: `requestId` en `PermissionRequestParams` y en
  `permission.schema.json` (aditivo), `spawn_live_push` lo llena con el id de la
  cola, caso de conformidad y e2e de `e2e_permisos` que compara el id del push
  con el de la entrada (design D4) — escenario «La petición en vivo nombra su
  id» — gates: `cargo build --workspace`, suites de `meltemi-proto` y
  `meltemid`

- [x] 1.2 `mock-agent --titled-permission`: su petición de permiso lleva
  `kind: edit` y un título; apagado por omisión, con su unit test (design D10)
  — gates: `cargo build -p mock-agent`, suite de `mock-agent`

## 2. El aprobador

- [x] 2.1 Núcleo puro en `tui/src/approver.rs`: la pregunta desde el push,
  completada con la `tool_call` que la sesión transmitió y con su contenido
  (D2); el saneado de todo texto del agente; la traducción tecla → respuesta,
  con la confirmación de «permitir siempre» (D3); la regla más específica y los
  casos en que no se ofrece (D5); el seguimiento, por `requestId`, de si la
  petición sigue en pie (D6); y las elecciones de permiso y de rechazo
  compartidas con el shell (`live.rs` las usa); unit tests — escenarios «Enter
  y las teclas sin asignar no deciden», «El texto del agente no controla el
  terminal», «La pregunta muestra el comando que la sesión transmitió», «Una
  opción con la que el agente deja de preguntar pide confirmación», «Sin
  herramienta no se ofrece regla», «Sin comando ni ruta no se ofrece regla»,
  «Ninguna regla del terminal alcanza la configuración de Meltemi», «En un
  worktree de tarea no se ofrece regla» — gates: suite de `meltemi` (tui)

- [x] 2.2 Fuentes de teclas: la de terminal (modo crudo sin pantalla
  alternativa, sondeo en tramos, descarte de lo pendiente, solo pulsaciones,
  repeticiones de Windows ignoradas, pegado entre corchetes en Unix) y la
  programada para pruebas (teclas tecleadas antes, esperas hasta la retirada,
  recuento de entradas y salidas del modo crudo), con la traducción de eventos
  de crossterm probada por unit test (design D7) — gates: suite de `meltemi`
  (tui), clippy

- [ ] 2.3 El aprobador en el CLI: el bucle que pregunta de a una, confirma con
  `permission/pending`, retira por instantáneas o por `permission/timeout`,
  espera y descarta (tres esperas con teclas → desatendida), persiste por
  `permission/decide`, y ante Ctrl+C cancela primero y responde después (D6,
  D7, D11); cada verbo avisa a su manejador y lo espera en vez de abortarlo, y
  `dispatch_with` asienta el aprobador (D12); `execute_with` y `dispatch_with`
  con el aprobador (la biblioteca deniega por omisión; recordar, solo en
  `session` y `propose`); `main.rs` lo activa con stdin y stderr en terminal y
  el modo crudo probado; sin terminal, el rechazo del agente y el aviso que
  nombra las reglas (D1); el código `13`; unit tests del bucle sobre un flujo
  en memoria, y e2e en `tui/tests/approver.rs` con la fuente programada
  (fixture sin regla general, simulado `--titled-permission` con
  `PROPOSAL_PATH:` fuera de `.meltemi/`, `--honor-cancel` para Ctrl+C,
  `[permissions] wait = 1` para el vencimiento, un segundo cliente que decide
  cuando la pregunta ya está abierta) — escenarios «Una tecla elige una opción
  del agente», «Denegar elige el rechazo del agente», «Lo tecleado antes de
  poder leer la pregunta no la contesta», «Una instantánea anterior a la
  petición no la retira», «Sin terminal se deniega con el
  rechazo del agente», «La salida de máquina queda intacta con el aprobador
  activo», «Ctrl+C deniega y cancela el turno», «Una pregunta que vence se
  retira del terminal», «Una pregunta resuelta por otra vía se retira del
  terminal», «El terminal vuelve a su estado al terminar», «La regla del
  terminal gobierna la siguiente sesión» — gates: `cargo build --workspace`,
  suite de `meltemi` (tui)

## 3. Gramática y documentación

- [ ] 3.1 La prueba de la gramática fija el conjunto exacto de banderas
  globales y rechaza las que pretendan conceder permisos (D8); la ayuda de
  `session` describe el aprobador y `docs/referencia-cli.md` se regenera;
  `docs/sesion-libre.md` (sección Terminal), `docs/quickstart.md` y
  `docs/accesibilidad.md` describen las teclas, la espera, la denegación sin
  terminal y el límite de un pseudoterminal; `docs/paridad-nucleo.md` completa
  las celdas del CLI (`permission/pending`, `permission/decide`), le da columna
  en los mensajes que inicia el daemon, corrige a quién llegan el push y el
  vencimiento, y dice por qué que solo el CLI lea `requestId` no rompe §4 —
  escenario «Ninguna bandera nueva concede permisos» — gates: suite de
  `meltemi` (tui), tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate aprobador-en-el-cli` limpio, `meltemi verify
  aprobador-en-el-cli` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/
  `land` detenido, estado de la change en `docs/plan-de-cambios.md` con las
  propuestas que nacieron de la revisión (la regla más específica en la
  bandeja; auditar la creación de reglas y el fallo al persistirlas; la
  procedencia de un cliente sin humano), y `meltemi land aprobador-en-el-cli`
