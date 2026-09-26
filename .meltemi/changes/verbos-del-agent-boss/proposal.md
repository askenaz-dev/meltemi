# verbos-del-agent-boss

> Seis verbos CLI sobre cinco métodos que ya existen y ya tienen casa en la
> TUI y la GUI. Salió de la vía rápida al diseñarse: `watch` y `log` son flujos
> —un registro por línea— y eso es una excepción a las dos reglas de formato de
> `cli-contract`, que se escribe como MODIFIED en vez de quedar implícita
> (design D5, D13). Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas MOB-5 y C-F).

## Why

Los cuatro verbos del Agent Boss —monitorear, aprobar, revisar, dirigir— son
la promesa de `mobile-companion`, y el daemon ya los sirve: `session/list`,
`session/watch`, `session/log`, `permission/pending`, `permission/decide`,
`session/direct`, `session/cancel`. Pero `docs/paridad-nucleo.md` tiene cinco
filas con un guion en la columna del CLI (líneas 28-30 y 59-60):
`session/log`, `session/watch`, `session/cancel`, `permission/pending` y
`permission/decide`. Desde un script o desde un `ssh` sin pantalla completa
no se puede aprobar un permiso, seguir una sesión ni cancelarla.

Eso vale hoy por dos cosas. Una: la única forma de aprobar desde el teléfono
es la TUI por `ssh -t pc meltemi`, que exige 80×24 y una app de terminal;
`ssh pc meltemi approve 3` no existe. Dos: Orca tiene un CLI para que los
**agentes** manejen la app (`orca worktree create / snapshot / click / fill`);
en Meltemi ningún verbo del daemon está expuesto a un agente por su CLI para
mirar sus sesiones hermanas o esperar una decisión. No hay que inventar un
servidor MCP propio: `meltemi --json` ya es una herramienta que cualquier
agente sabe llamar, en cuanto tenga los verbos.

## What Changes

- **`meltemi permissions`**: lista las peticiones pendientes (id, sesión,
  herramienta, título, desde cuándo, vencimiento y las opciones del agente),
  `--json` incluido. El comando y las rutas no viajan en la cola: el listado
  remite al registro de la sesión, donde está la llamada completa (design D10).
- **`meltemi approve <id> [--option <id>]`** y **`meltemi deny <id> [--option
  <id>]`**: deciden **una** petición por su id con una de las opciones que el
  agente ofreció (por defecto la aprobación simple cuando es una sola, o el
  rechazo del agente; `--option` nombra otra de la misma clase). Nunca existe
  «approve all»: un script que quiera conceder de antemano lo dice en las
  reglas (`permission-rules`) o en el modo, no aquí.
- **`meltemi watch <session>`**: el stream de `session/watch` como líneas JSON
  en `stdout` hasta que la sesión termine o llegue `Ctrl+C`; es lo que un
  script de aviso o un agente necesitan para «esperar a que el otro
  termine».
- **`meltemi log <session>`**: el registro apend-only tal cual, para leerlo
  o canalizarlo.
- **`meltemi cancel <session>`**: `session/cancel`, con el mismo resultado
  que la `x` de la TUI.
- **Códigos de salida y disciplina `stdout`/`stderr`** según `cli-contract`;
  `docs/referencia-cli.md` y `docs/paridad-nucleo.md` al día; y en
  `docs/acceso-remoto.md` la receta «aprobar desde el teléfono, hoy»:
  `ssh pc meltemi permissions` → `ssh pc meltemi approve <id>`, sin TUI.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `cli-contract`: + los seis verbos, su gramática y sus códigos de salida;
  + regla «una decisión por id, nunca en bloque»; + el texto del agente no
  manda en el terminal; MODIFIED las dos reglas de formato (`--json` y la
  elección declarada) ganan la excepción de los flujos.
- `remote-access`: + escenario «aprobar y seguir por SSH sin terminal
  interactiva».

## Impact

- Archivos: `tui/src/cli.rs`, `tui/src/run.rs`, `tui/src/lib.rs`,
  `tui/src/decide.rs` y `tui/src/follow.rs` (nuevos), `tui/src/exit.rs`,
  `tui/tests/`, `docs/referencia-cli.md`, `docs/paridad-nucleo.md`,
  `docs/acceso-remoto.md`.
- Cero dependencias nuevas; cero RPC nuevos: el contrato no se mueve y no
  nace deber de paridad.
- Riesgo: un verbo `watch` que no cierra cuando el daemon muere; se pinea con
  el agente simulado y `kill_on_drop`.

## Fuera de alcance

- Un servidor MCP de Meltemi: la CLI ya es la herramienta; si la evidencia
  pide MCP, es otra change con su análisis §6.
- Verbos para `sdd/gate`, `sdd/review-decide` o `worktree/merge-file` desde el
  CLI: siguen en la lista de la matriz, con su propia change cuando se pidan.
