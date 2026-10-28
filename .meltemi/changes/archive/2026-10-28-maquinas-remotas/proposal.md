# maquinas-remotas

> Vía completa (proposal → design → specs → tasks). Hay un **MODIFIED** sobre
> un requisito vivo de `remote-access` («El punto de encuentro en dos vías
> está documentado», que hoy manda que el selector de máquinas quede como
> nota de design para fase 3): esta change lo trae al escritorio y a la
> TUI, sin que el daemon gane nada. Depende de `via-ssh-del-cliente`; usa
> `identidad-propia` para la presencia cuando exista. Nace del comparativo
> con Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brechas R2, R10, MOB-7 y R8).

## Why

Orca muestra «MacBook Pro · Connected · 40 worktrees · 5 active / M1 Mini ·
home · Disconnected» y cambia de máquina con un toque. En Meltemi el
multi-daemon ya está decidido como concepto de la aplicación —«cada
`meltemid` sigue siendo de su máquina; una lista de endpoints con su
presencia» (`docs/acceso-remoto.md`, notas para fase 3)— pero no existe en
ninguna superficie: la GUI conecta a un endpoint fijado por variable o por
defecto, el chip ◌/●/▲ y el backoff de reconexión existen, y no hay
perfiles, ni prueba de conexión, ni forma de decir «ahora la de casa».

La regla que lo hace constitucional ya está escrita: nadie recibe
conexiones, ambos extremos marcan hacia afuera, y la infraestructura de
encuentro es del usuario. Una lista de máquinas es una lista de comandos
`exec:` con nombre; la presencia, cuando `identidad-propia` aterrice, la lee
el binario oficial de la malla del usuario. Meltemi no gana cuentas ni
servidor.

## What Changes

- **Lista de máquinas de ámbito usuario**: nombre, endpoint (local, socket o
  `exec:ssh <alias> meltemi bridge`) y, opcionalmente, la fuente de presencia.
  Sin `IdentityFile`, sin passphrase, sin puerto: eso vive en el
  `~/.ssh/config` del usuario y en su agente de claves (§2).
- **Selector en las dos superficies**: sección «Máquinas» en Ajustes de la
  GUI y en la paleta de la TUI; cambiar de máquina reconecta con el chip
  diciendo cuál; «Probar conexión» corre `initialize` y muestra versión y
  latencia. Un cambio de máquina cierra las pestañas de la anterior con la
  guarda de trabajo sin guardar que ya existe.
- **Presencia a demanda, nunca en silencio**: la lista muestra ◌/●/▲ solo
  cuando el usuario la abre o lo pide; ningún sondeo de fondo hacia máquinas
  remotas. Con `identidad-propia`, la presencia sale de `identity/status` y
  el nombre de la máquina, de la malla.
- **Un daemon por máquina, siempre**: nada agrega sesiones de varias
  máquinas en una lista; la búsqueda entre máquinas de Orca es exactamente
  lo que §3 no permite por agregador. Lo que se ve es lo de la máquina
  activa.
- **Documentación**: la nota de fase 3 de `docs/acceso-remoto.md` pasa a
  sección presente, y la spec `remote-access` deja de reservarla.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `remote-access`: **MODIFIED** en «El punto de encuentro en dos vías está
  documentado» (el selector deja de ser nota de fase 3) y + requisito
  «Máquinas con nombre en el cliente, sin material de claves, con presencia a
  demanda».
- `gui-shell` y `tui-shell`: + la sección y el selector.

## Impact

- Archivos: `core/meltemi-client/src/paths.rs` (config de usuario),
  `desktop/ui/src/lib/{daemon.ts, ui-state.ts, views/Settings.svelte,
  components/StatusBar.svelte}`, `tui/src/shell/{conn.rs, palette.rs}`,
  `.meltemi/specs/remote-access/spec.md` (vía delta), `docs/acceso-remoto.md`.
- Cero dependencias nuevas; cero RPC. El daemon no cambia.
- Riesgo: la guarda de trabajo sin guardar al cambiar de máquina y las
  pestañas de sesión que apuntan a otro daemon; el design lo cubre con el
  mismo mecanismo que el cambio de proyecto.

## Fuera de alcance

- Identidad y presencia por malla: `identidad-propia` (0/8), que sigue siendo
  change propia.
- Sincronizar configuración o reglas entre máquinas: no hay nube; si se
  quiere, es `git` del usuario.
