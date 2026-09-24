# abrir-con-lo-tuyo

> Vía completa (proposal → design → specs → tasks), no por el tamaño del
> código —dos plantillas de apertura— sino porque trae una **enmienda de
> rumbo** con ratificación del mantenedor como gate: la terminal integrada
> se declara no-objetivo por escrito. Nace del comparativo con Orca del
> 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brechas D11 y
> D12). Absorbe las propuestas nombradas allí como `abrir-pr-con` y
> `abrir-terminal-aqui`.

## Why

Orca trae GitHub y Linear dentro de la app (PRs, issues, tableros, revisar y
aprobar) y una terminal WebGL con splits infinitos. Meltemi no va a hacer
ninguna de las dos cosas, y conviene decirlo con la razón: el rumbo declara
«ni CI/CD» y «ni un editor de propósito general», y la constitución §3 no
admite red en el daemon — un cliente de la API de GitHub dentro de Meltemi
sería o red en el daemon o una cuenta que Meltemi gestiona (§2).

Lo que sí cabe, y ya tiene precedente, es **abrir con lo del usuario**:
`desktop/src/fsops.rs:148` ejecuta una plantilla `openWith` para mandar un
archivo al editor del usuario en `archivo:línea`. La misma idea vale para
«abrir la PR de esta calle» con el `gh` del usuario y su sesión (§2: la
herramienta oficial se autentica sola), y para «abrir una terminal aquí» en
el worktree o en la raíz. Cuestan un día y cierran la mitad útil de las dos
features de Orca sin cruzar la frontera.

Queda la postura. Hoy el rumbo no dice nada de terminales; mañana alguien
propondrá embeber una porque «Orca la tiene». Mejor que el «no» esté escrito
antes, con la puerta que sí hay.

## What Changes

- **Plantilla «Abrir PR con…»**: de ámbito usuario, con un default por
  plataforma (`gh pr create --web --head {branch}` desde el directorio de la
  calle); disponible en el menú de una calle con commits en su rama y en la
  cabecera de una sesión que corre en un worktree. Meltemi lanza la
  herramienta y no lee su salida: la PR se abre en el navegador del usuario.
- **«Abrir terminal aquí»**: plantilla con default por plataforma (Windows
  Terminal, Terminal de macOS, la terminal por defecto del escritorio en
  Linux) sobre el worktree o la raíz del proyecto; en la GUI como acción y en
  la TUI como tecla que lanza el mismo comando (cortesía de superficie, no
  deber de paridad: el daemon no gana capacidad).
- **Ambas plantillas se editan en Ajustes** junto a la de «Abrir con…», con
  la misma regla: se ejecuta lo que el usuario escribió, con las variables
  `{path}`, `{branch}`, `{file}`, `{line}` sustituidas, y nunca desde el
  `.meltemi/` de un proyecto.
- **Enmienda de rumbo**: `rumbo/product.md`, «Qué NO es», gana «ni una
  terminal integrada: la terminal es la del usuario, y Meltemi la abre donde
  hace falta»; `edit-surface` lo recoge como postura. **Gate**: la
  ratificación del mantenedor; hasta entonces, las plantillas se implementan
  y la enmienda queda aplicada y marcada pendiente, como las anteriores.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `edit-surface`: + requisito «Plantillas de apertura hacia herramientas del
  usuario» (PR y terminal; ámbito usuario; variables; nunca desde el
  proyecto) y la postura sobre terminales.
- `gui-shell` y `tui-shell`: + las dos acciones.

## Impact

- Archivos: `desktop/src/fsops.rs`, `desktop/ui/src/lib/{ui-state.ts,
  views/Settings.svelte, views/Review.svelte, views/SessionDetail.svelte}`,
  `tui/src/shell/`, `.meltemi/rumbo/product.md` (enmienda), `meltemi.md`
  (§1, no-objetivos), `docs/`.
- Cero dependencias nuevas; cero RPC.
- Riesgo: una plantilla que ejecuta algo inesperado. Solo ámbito usuario, se
  muestra el comando antes de ejecutarlo la primera vez, y nunca se hereda
  de un repositorio.

## Fuera de alcance

- Un cliente de GitHub, Linear o de CI dentro de Meltemi: rumbo y §3.
- Una terminal embebida, con o sin WebGL: no-objetivo, ahora por escrito.
- Que un agente abra PRs por su cuenta: es su herramienta y su permiso, por
  el proxy, como cualquier comando.
