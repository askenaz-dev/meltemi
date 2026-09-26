# lectura-por-el-daemon

> Vía completa (proposal → design → specs → tasks): RPC nuevos de lectura
> acotados, con su deber de paridad §4 en el CLI. Nace del comparativo con
> Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brecha R7). Depende de `via-ssh-del-cliente` para tener sentido.

## Why

«Hacer algún cambio desde fuera» tiene hoy la mitad: las escrituras van por
el daemon (`worktree/apply-edit`, con `human_edit` y soft lock), pero el
árbol, la lectura, la búsqueda y «Abrir con…» del Editor leen **el disco del
cliente** (`desktop/src/fsops.rs:54-148`), el LSP también
(`desktop/ui/src/lib/editor/lsp.ts:38-59`), y Ajustes lee el `config.toml`
local (`desktop/ui/src/lib/views/Settings.svelte:36-40`). Con la GUI en el
portátil y el daemon en el PC de casa, el Editor muestra un disco que no es
el del agente, y guardar escribe en otro. No es un fallo de hoy —la GUI
remota no existe todavía— pero es lo primero que `via-ssh-del-cliente` va a
destapar.

La frontera es la de siempre: el daemon puede leer lo que ya gobierna —las
raíces registradas y los worktrees que gestiona— y nada más. No se convierte
en un servidor de archivos.

## What Changes

- **RPC de lectura acotados**: `tree/list`, `tree/read` y `tree/search`,
  válidos solo bajo una raíz registrada (`project/list`) o un worktree
  gestionado; cualquier otra ruta se rehúsa con remedio. Límites de tamaño,
  detección de binario y paginación en el design; nada se escribe por aquí.
- **`config/read` y `config/write` de ámbito usuario** para que Ajustes muestre
  y guarde la configuración del daemon al que está conectado, no la del
  disco local; con la misma traza de quién cambió qué.
- **El Editor y la búsqueda eligen la fuente**: con el daemon local, siguen
  leyendo el disco directo (rápido, sin viaje); con el daemon remoto
  (endpoint `exec:` o socket reenviado), leen por RPC. La barra de estado
  dice de dónde viene lo que se ve.
- **El LSP degrada honesto**: corre donde corre el cliente y necesita el
  disco; con daemon remoto se desactiva con un aviso claro en vez de
  diagnosticar el archivo equivocado. Hacerlo remoto es otra change con su
  análisis (`lsp-superficie-revision`).
- **Paridad en el CLI**: `meltemi tree [ruta]`, `meltemi read <ruta>` y
  `meltemi search <patrón>` sobre los mismos RPC, para que lo que el daemon
  sirve al escritorio lo sirva también al terminal.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `edit-surface`: + requisito «Lectura de árbol, archivo y búsqueda vía
  daemon, acotada a raíces registradas y worktrees; el LSP declara su
  frontera».
- `project-registry`: + escenario de rehúso fuera de raíz.
- `cli-contract`: + los tres verbos de lectura.
- `gui-shell`: + fuente de lectura según el endpoint, dicha en el cromo.

## Impact

- Archivos: `core/meltemid/src/{server.rs, projects.rs, worktrees.rs}` y un
  módulo de lectura acotada, `proto/schemas/v1/{tree, config}.schema.json`
  (nuevos), registros de paleta de TUI y GUI (gate de paridad),
  `desktop/src/fsops.rs`, `desktop/ui/src/lib/editor/`,
  `desktop/ui/src/lib/views/{Editor.svelte, Settings.svelte}`,
  `tui/src/cli.rs`, `tui/src/run.rs`, `docs/paridad-nucleo.md`.
- Cero dependencias nuevas.
- Riesgo real: exponer por RPC un archivo que el usuario no quería que el
  cliente remoto viera (secretos en el árbol). La acotación por raíz es
  necesaria pero no suficiente; el design añade la exclusión de rutas que las
  reglas de permisos ya marcan como sensibles y lo pinea.

## Fuera de alcance

- Escribir por RPC fuera de `worktree/apply-edit`: ya existe y no cambia.
- LSP remoto o un agente de lenguaje en el daemon: `lsp-superficie-revision`.
