## Why

El daemon `meltemid` ya expone un contrato JSON-RPC v1 (`initialize`, `status`,
`shutdown`, `propose`, más notificaciones de sesión y permisos), pero el único
cliente que lo consume es `meltemi-devclient`, un binario de desarrollo con
gramática ad-hoc, sin disciplina de salida ni códigos de salida definidos. Antes
de construir la TUI (#6) y el ciclo de autoría (#14) hace falta fijar **el
contrato de la superficie de terminal**: la gramática de subcomandos, la regla
que decide entre modo scriptable y modo interactivo, la taxonomía de códigos de
salida, la disciplina stdout/stderr y el mapeo normativo comando↔RPC. Sin este
contrato escrito, cada superficie inventaría el suyo y la paridad de núcleo
(principio constitucional) se volvería inverificable.

## What Changes

- Se introduce el binario único `meltemi` (alias `mel`) en el crate `tui/`, que
  funciona como **CLI scriptable** en esta change; la TUI interactiva llega en
  #6, pero la **regla de despacho** que decide entre ambos modos se fija aquí.
- Se define una **gramática de subcomandos** estable y su mapeo a métodos del
  contrato `proto/`: `status`, `propose`, `stop`, `version`, `help`. Los
  subcomandos del ciclo SDD (`explore`, `review`, `plan`, …) se declaran como
  **reservados** en la gramática, sin implementarse todavía.
- Se establece la **regla de despacho CLI↔TUI**: invocación con subcomando o sin
  TTY → modo scriptable de un disparo; invocación desnuda con TTY → modo
  interactivo (que en esta change es un arranque diferido explícito, no un panel).
- Se fija la **taxonomía de códigos de salida** (éxito, error de uso, error de
  contrato, daemon inalcanzable, cancelado, denegado) — estable y documentada.
- Se fija la **disciplina de flujos**: stdout es solo para la salida útil del
  comando (apta para *pipe*); stderr para diagnósticos, progreso y errores.
- Se añade el flag global `--json`: cada subcomando scriptable emite un objeto
  JSON estable en stdout, apto para consumo por máquina, sin texto humano
  mezclado.
- El binario reutiliza el arranque bajo demanda del daemon (bootstrap existente)
  y traduce los errores de transporte/contrato a códigos de salida del contrato.
- El crate `tui/` se incorpora como miembro del workspace Cargo.

Ningún método de `proto/` cambia: esta change es **aditiva** y consume el
contrato RPC tal cual existe.

## Capabilities

### New Capabilities
- `cli-contract`: el contrato de la superficie de terminal — gramática de
  subcomandos y su reserva, regla de despacho CLI↔TUI, taxonomía de códigos de
  salida, disciplina stdout/stderr, flag `--json` y salida legible por máquina,
  y el mapeo normativo comando↔método RPC.

### Modified Capabilities
_Ninguna._ Los métodos del contrato `proto/` (`daemon-lifecycle`, `propose-flow`,
`acp-session`) se consumen sin modificarse.

## Impact

- **Código nuevo**: crate `tui/` (binario `meltemi`, alias `mel`); miembro del
  workspace en el `Cargo.toml` raíz; cabecera SPDX en cada archivo.
- **Contrato**: se consume `proto/meltemi-proto` (`INITIALIZE`, `STATUS`,
  `SHUTDOWN`, `PROPOSE`); no se altera.
- **Bootstrap**: se reutiliza el arranque bajo demanda del daemon y el transporte
  local (UDS / named pipe) ya implementados en `meltemid`.
- **Relación con `meltemi-devclient`**: el devclient queda como herramienta
  interna de depuración; `meltemi` es la superficie contractual pública. No se
  elimina el devclient en esta change.
- **Desbloquea**: #6 `tui-nucleo-ux` (arquitectura de la TUI sobre esta regla de
  despacho) y, más adelante, #14 `ciclo-sdd-autoria` (que puebla los subcomandos
  reservados).

## Fuera de alcance (de esta change)

- La **TUI interactiva** en sí (paneles, navegación, onboarding): es #6. Aquí solo
  se fija la regla de despacho y un arranque interactivo diferido.
- La **implementación** de los subcomandos del ciclo SDD (`explore`, `propose`
  completo con gates, `review`, `plan`, `implement`, `verify`, `archive`): se
  **reservan** en la gramática, se implementan en changes posteriores. `propose`
  se cablea al método RPC existente, sin la disciplina de gates de #14.
- **Autocompletado de shell**, páginas de manual y empaquetado/distribución: son
  materia de #22/#23.
- Cualquier **transporte de red** o superficie remota que no sea el túnel SSH ya
  contemplado por la arquitectura del daemon.
