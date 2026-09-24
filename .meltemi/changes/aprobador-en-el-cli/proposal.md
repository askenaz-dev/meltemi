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
  TTY, cada `permission/request` se presenta en `stderr` con lo que la bandeja
  muestra —clase de herramienta, título, rutas, resumen del diff si lo hay— y
  se lee una tecla: permitir una vez, permitir en esta sesión (misma clase y
  misma raíz), denegar, o escribir una regla persistente. Las opciones
  ofrecidas son **las del agente**, nunca inventadas; la decisión vuelve por
  el mismo RPC que usan GUI y TUI y queda en el log con `decidedBy`.
- **Sin terminal, la postura de hoy**: `stdin` que no es TTY → denegación por
  defecto con la advertencia actual. Los scripts que necesiten conceder algo
  lo dicen donde corresponde: en el archivo de reglas (`permission-rules`) o,
  cuando `modos-de-autonomia` aterrice, con `--mode`. **Esta change no añade
  ninguna bandera de política**: un `--yes` sería el bypass que §3 no admite.
- **La espera respeta la política del daemon**: `[permissions] wait` decide
  cuánto se aguarda al humano; el CLI no la reimplementa, solo responde.
- **Interrupción honesta**: `Ctrl+C` durante una petición deniega esa
  petición y cancela el turno (`session/cancel`), sin dejar la sesión
  colgada ni procesos huérfanos.
- **Documentación**: `docs/referencia-cli.md` y `docs/sesion-libre.md` dejan
  de prometer «una release posterior» y describen las teclas; la matriz de
  paridad anota que `permission/decide` es consumido también por el CLI.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `cli-contract`: + requisito «Aprobador interactivo en terminal» (TTY →
  pregunta con las opciones del agente; no TTY → denegación por defecto; sin
  bandera de política).
- `permission-rules`: + escenario de regla creada desde el CLI con el mismo
  formato y ámbito que desde la bandeja.

## Impact

- Archivos: `tui/src/run.rs` (el manejador de `Incoming::Request`), un módulo
  de lectura de teclas sin pantalla completa (el crate de terminal ya
  presente en `tui/`), `docs/referencia-cli.md`, `docs/sesion-libre.md`,
  `docs/paridad-nucleo.md`.
- Cero dependencias nuevas. Ningún RPC nuevo.
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
