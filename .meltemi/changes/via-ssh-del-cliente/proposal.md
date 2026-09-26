# via-ssh-del-cliente

> Vía completa (proposal → design → specs → tasks): un transporte nuevo en el
> crate compartido de cliente, consumido por GUI, TUI y CLI, sin una sola
> dependencia nueva y sin tocar el daemon. Nace del comparativo con Orca del
> 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brechas MOB-8
> y R1).

## Why

`meltemi bridge` existe y funciona: pone el endpoint local del daemon en su
propio stdio, incluido el named pipe de Windows que ningún reenvío alcanza
(`tui/src/run.rs:1642-1689`, smoke del 2026-08-15). `ssh pc meltemi bridge`
es un canal JSON-RPC completo. Pero **ningún cliente de Meltemi lo
consume**: la GUI y la TUI solo saben conectar a un socket o a un pipe. En
macOS y Linux se salva con `ssh -R` y `MELTEMI_ENDPOINT`; con un daemon en
Windows —la máquina del mantenedor— no hay camino para la GUI ni para la
TUI, y desde una máquina Windows hacia un daemon Unix tampoco hay reconexión
ni vida del túnel.

Orca conecta a un «beefy remote box» con auto-reconexión, reenvío de puertos
y caché de passphrases. El equivalente conforme es más simple y no toca la
frontera: el cliente lanza **el `ssh` del usuario** con un comando fijo y
habla por su stdio. Cero crates (`deny.toml` prohíbe con razón todo cliente
TLS en el workspace), cero red en el daemon (constitución §3), cero material
de claves (§2: las passphrases las gestiona ssh-agent o Pageant, no
Meltemi).

## What Changes

- **Transporte `exec:` en `core/meltemi-client`**: un endpoint de la forma
  `exec:<comando>` (p. ej. `exec:ssh pc meltemi bridge`) hace que el cliente
  lance ese comando como subproceso y hable JSON-RPC por su stdio, con la
  misma delimitación por líneas que el socket. `connect_or_start` **jamás
  arranca un daemon** para un endpoint `exec:` (regla que
  `acceso-remoto-afinado` deja escrita para todo endpoint explícito).
- **Solo de ámbito usuario**: el comando se declara en
  `<config>/meltemi/config.toml` o en `MELTEMI_ENDPOINT`; **nunca** en el
  `.meltemi/config.toml` de un proyecto — un repositorio no puede hacer que
  tu cliente ejecute un comando. El daemon no lo lee: es cosa del cliente.
- **Reconexión y cierre limpios**: si el proceso muere, el cliente lo relanza
  con el backoff que ya tiene el chip ◌/●/▲; al cerrar el cliente, el
  subproceso muere con él (en Windows, dentro del ámbito de proceso del
  crate `meltemi-process`; ningún `ssh` huérfano).
- **Tres superficies**: la GUI conecta por `exec:` como conecta hoy por
  pipe; la TUI igual; el CLI acepta `MELTEMI_ENDPOINT=exec:…` en todos sus
  verbos. La barra de estado dice por dónde habla («vía `ssh pc`»).
- **Documentación**: `docs/acceso-remoto.md` gana «la GUI contra tu PC»,
  con el `~/.ssh/config` mínimo (alias, `ServerAliveInterval`, sin
  `IdentityFile` obligatorio) y la advertencia de que el `ssh` del usuario es
  el que decide claves y hosts conocidos.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `remote-access`: + requisito «Transporte por comando del usuario en el
  cliente» (ámbito usuario, sin arranque de daemon, sin claves, sin
  huérfanos).
- `daemon-lifecycle`: + escenario «un endpoint `exec:` nunca lanza un
  daemon».
- `cli-contract`, `gui-shell`, `tui-shell`: + la forma `exec:` del endpoint
  y su indicación en el cromo.

## Impact

- Archivos: `core/meltemi-client/src/{transport.rs, bootstrap.rs, paths.rs}`,
  `core/meltemi-process` (ámbito del subproceso), `desktop/src/`,
  `desktop/ui/src/lib/components/StatusBar.svelte`, `tui/src/shell/conn.rs`,
  `tui/src/run.rs`, `docs/acceso-remoto.md`, `docs/plataformas.md`.
- Cero dependencias nuevas; el contrato `proto/` no se mueve; el daemon no
  cambia.
- Riesgo real: el prompt de host desconocido o de passphrase de `ssh` no
  tiene terminal en la GUI. El design decide: se exige `BatchMode=yes` y se
  rehúsa con remedio («acepta el host y carga la clave en tu agente desde un
  terminal primero»), sin que Meltemi capture ni reenvíe ninguna passphrase.

## Fuera de alcance

- Perfiles con nombre, presencia y selector: `maquinas-remotas`.
- Que el editor y la búsqueda lean el disco remoto: `lectura-por-el-daemon`.
- Cualquier transporte de red propio, relé o puerto: constitución §3.
