# aprobador-en-el-cli

> Vía completa: toca la política de permisos desde una superficie nueva —el
> terminal no interactivo y el interactivo son dos casos con dos respuestas—
> y el design debe fijar qué decide cada uno sin abrir una rendija al
> «bypass» que la constitución rechaza. Nace de la prueba integrada del
> 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brecha AI-12).

## Why

`meltemi session "Read README.md and src/lib.rs, then answer…" --agent
claude-code` terminó «completed» con **doce permisos denegados**. Claude
Code pidió permiso para leer, para `cat`, para `Get-Content`, para `Grep`,
para `Glob`, incluso para lanzar un subagente que leyera por él; el CLI los
canceló todos con la misma frase —«agent requested a permission; denied by
default (interactive approval arrives in a later release)»— y el agente
contestó que no podía saber qué había en el repositorio. Con una instrucción
de escritura, tres denegados y ningún archivo creado. El proxy funciona; el
cliente no tiene con qué responder.

El stub está escrito con honestidad en `tui/src/run.rs:221-232`: «no
interactive approver yet (#6/#9)». Se dejó para «una release posterior», y esa
release nunca llegó porque la GUI y la TUI ya tenían bandeja. Pero el CLI es
la superficie del usuario que trabaja en terminal —el primero de los tres
perfiles del rumbo— y la que llega por SSH desde el teléfono. Con un
adaptador de nivel 2 que enruta **cada** herramienta por el proxy (así
funciona `meltemi-claude-acp`: hook `PreToolUse` → petición ACP), un CLI sin
aprobador solo sirve para conversar.

La frontera ya está escrita y no se mueve: sin cliente conectado, denegación
(constitución §3); lo irreversible exige aprobación explícita incluso en modo
autónomo; una regla jamás concede opciones que el agente no ofreció
(`permission-rules`). El CLI **es** un cliente conectado: puede preguntar al
humano que lo lanzó.

## What Changes

- **Aprobador interactivo cuando hay terminal**: si `stdin` y `stderr` son un
  TTY y el terminal admite el modo crudo, cada `permission/request` se presenta
  en `stderr` con lo que el agente envió —herramienta, título, comando, ruta y
  contenido, completados con la llamada a herramienta que la sesión ya
  transmitió; nada se inventa y nada del agente llega crudo al terminal
  (design D2)— y se lee una tecla, solo cuando la pregunta pudo leerse (D7):
  una de las opciones **del agente**, denegar, o aprobar y recordar como regla.
  «Permitir en esta sesión» aparece exactamente cuando el agente la ofrece, y
  pide confirmación porque el agente deja de preguntar (design D3). La decisión
  queda en el log con `decidedBy: client`, como la de la bandeja.
- **Recordar escribe la regla más específica posible** —herramienta con el
  comando o la ruta exactos— y no se ofrece cuando sería más amplia que lo que
  se vio, cuando tocaría `.meltemi/` o cuando la sesión trabaja en un worktree
  (design D5).
- **Sin terminal, un rechazo y un aviso honesto**: `stdin` o `stderr` que no
  son TTY → denegación con la opción de rechazo del agente, con un aviso que
  deja de prometer una release futura y nombra cómo conceder sin humano: las
  reglas de permisos (`permission-rules`). **Esta change no añade ninguna
  bandera de política**: un `--yes` sería el bypass que §3 no admite, y una
  prueba fija el conjunto de banderas.
- **La espera respeta la política del daemon**: `[permissions] wait` decide
  cuánto se aguarda al humano; el CLI no la reimplementa, solo responde.
- **El push lleva su id en la cola** (`requestId`, aditivo): sin él, quien
  responde el push no puede recordar la regla por `permission/decide` ni
  reconocer que su petición se decidió en otra superficie (design D4).
- **Interrupción honesta**: `Ctrl+C` durante una petición cancela la sesión
  (`session/cancel`) antes de responder, de modo que el log anota la petición
  como interrumpida, y el comando termina con `13`, sin dejar la sesión
  colgada ni procesos huérfanos (design D11).
- **El terminal vuelve siempre a su estado**: ningún verbo aborta el aprobador
  a media pregunta; lo espera, y el aprobador retira la pregunta diciendo por
  qué y sale del modo crudo antes de que el binario termine (design D12).
- **Documentación**: la ayuda de `session` (y con ella
  `docs/referencia-cli.md`), `docs/sesion-libre.md`, `docs/quickstart.md` y
  `docs/accesibilidad.md` describen las teclas; la promesa de «una release
  posterior» vivía solo en el código (`run.rs`), que la retira; la matriz de
  paridad anota que el CLI responde `permission/request` y usa
  `permission/decide`.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `cli-contract`: + requisito «Aprobador interactivo en terminal» (TTY →
  pregunta con las opciones del agente; no TTY → denegación por defecto; sin
  bandera de política).
- `permission-rules`: + «La petición en vivo lleva su id en la cola» y «La
  regla creada desde el terminal es la más específica posible» (herramienta
  con comando o ruta exactos, por la vía de la bandeja, rige desde la
  siguiente sesión sin `--mode`, y no se ofrece cuando sería más amplia que lo
  que se vio).

## Impact

- Archivos: `tui/src/{run.rs, lib.rs, main.rs, approver.rs, approver/}` (el
  manejador de `Incoming::Request` y un módulo de lectura de teclas sin
  pantalla completa, con el crossterm que `ratatui` ya reexporta),
  `tui/src/shell/live.rs` (elegir permiso y rechazo pasa a funciones
  compartidas), `core/meltemid/src/acp.rs`
  y `proto/` (el `requestId` del push), `core/mock-agent/src/main.rs`
  (`--titled-permission`), y la documentación nombrada arriba.
- Cero dependencias nuevas. Ningún RPC nuevo: un campo aditivo en el push.
- Riesgo real: el aprobador lee teclas mientras `stdout` sigue siendo el
  canal de `--json`. El design separa los canales (preguntas y respuestas por
  `stderr`/TTY; `stdout` intacto) y lo pinea con un test que consume la salida
  JSON con el aprobador activo.
- Tests: el aprobador se prueba con un lector de teclas inyectable y el
  agente simulado pidiendo permisos; el caso «no TTY deniega» queda pineado.

## Fuera de alcance

- Cualquier bandera que conceda permisos sin humano ni regla.
- El aprobador remoto sin terminal (aviso de espera, móvil): `aviso-de-espera-opt-in`
  y `companero-movil`.
- Las preguntas del agente (`AskUserQuestion`): `preguntas-del-agente`, que
  podrá reutilizar el mismo lector de teclas.
