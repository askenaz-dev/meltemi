## Why

El binario `meltemi` ya despacha al modo interactivo cuando se invoca desnudo
con un TTY (regla de `cli-contract`), pero ese modo es hoy un aviso diferido: no
hay TUI. La TUI es una de las dos superficies con **paridad de núcleo** — todo el
poder del daemon debe alcanzarse desde la terminal igual que desde la futura GUI.
Antes de construir la UX rica de cada feature (bandeja de permisos #9, revisión
de specs #15, catálogo de flota #7, worktrees #16) hace falta el **shell**: la
arquitectura de información, el modelo de navegación por teclado, los estados
vacíos, el onboarding de primer uso y la línea base de accesibilidad que **toda**
vista deberá honrar. Sin ese esqueleto, cada feature inventaría su propia
navegación y accesibilidad, y la paridad se volvería inverificable.

Esta change es deliberadamente **comprensiva** (el shell completo especificado de
una vez, spec-first): el contrato de navegación, accesibilidad y prioridad de
señales es un todo coherente cuyos invariantes se verifican mejor juntos. La
implementación se entrega por olas (design D9); si se prefiere, la ola de
endurecimiento puede separarse a un follow-up sin tocar el contrato.

## What Changes

- Se implementa el **shell interactivo** de `meltemi` en el crate `tui/`
  existente: un **chrome persistente** (cabecera + pie siempre visibles) que
  enmarca **cuatro vistas de primer nivel** (Sesiones, Proyecto, Permisos,
  Flota), un solo nivel de *drill-in* y una capa de **overlays** (paleta `:`,
  ayuda `?`).
- Se fija un **contrato de navegación por teclado**: un único keymap honrado por
  toda vista (misma tecla = misma categoría de acción), split dígitos-global /
  letras-local, un conjunto de teclas robusto (sin Alt/Meta, sin F1–F12, sin
  Ctrl del TTY) y sin dependencia del ratón.
- Se cubren los **estados vacíos** con honestidad: sin daemon (arranque asíncrono
  distinguiendo transitorio de inalcanzable), sin sesiones (launchpad), sin
  proyecto `.meltemi/` (desacople de ámbito), pérdida de daemon a mitad de
  sesión, y suelo duro de tamaño de terminal.
- Se añade el **onboarding de primer uso**: un overlay ligero, saltable y
  re-invocable que enseña el modelo de navegación (y cómo salir), sin cuenta, sin
  red, sin telemetría.
- Se establece la **línea base de accesibilidad de terminal** que toda vista debe
  honrar: nunca solo color, soporte de `NO_COLOR`, fallback ASCII con gemelo para
  cada glifo, reflow en SIGWINCH y una ruta scriptable garantizada.
- Se da **casa e indicador** a las features futuras sin construir su interior: la
  bandeja de permisos (#9) tiene un indicador siempre visible y una tecla de
  salto; la flota (#7), la revisión de specs (#15) y los worktrees (#16) tienen
  su vista o su acción reservada.
- Se cierra un **hueco de paridad de núcleo**: cancelar una sesión activa
  (`session/cancel`) y apagar el daemon (`shutdown`) —capacidades que el daemon ya
  soporta— obtienen casa y tecla en la TUI, con confirmación por ser irreversibles.
- **BREAKING (interno)**: la rama `Interactive` de `cli-contract` deja de emitir
  el aviso diferido y ahora lanza el shell. No cambia la regla de despacho.
- Se incorpora un framework de TUI (pineado y justificado en el design).

## Capabilities

### New Capabilities
- `tui-shell`: el shell interactivo de la terminal — modelo de vistas y chrome
  persistente, contrato de navegación por teclado, estados vacíos y onboarding,
  línea base de accesibilidad (no-solo-color / `NO_COLOR` / ASCII / reflow),
  indicador y prioridad de señales de la bandeja de permisos, control de ciclo de
  vida de sesión y daemon, y paridad de núcleo vía la paleta de comandos.

### Modified Capabilities
- `cli-contract`: la rama `Interactive` de la regla de despacho CLI↔TUI pasa de
  un aviso diferido a lanzar el shell interactivo (un escenario modificado).

## Impact

- **Código**: extiende el crate `tui/` (binario `meltemi`) con los módulos del
  shell; la rama `Interactive` de `dispatch` deja de ser un stub. Cabecera SPDX
  en cada archivo nuevo.
- **Dependencias**: un framework de TUI y su backend de terminal (pineados,
  justificados en el design; auditados por cargo-deny). Primera dependencia de
  UI del proyecto.
- **Contrato**: consume `status`, `session/event`, `session/cancel`,
  `permission/request`, `permission/timeout`, `shutdown` de `proto/` — sin
  alterarlos.
- **Desbloquea**: #7 `catalogo-flota`, #9 `proxy-permisos`, #15 `revision-specs-ux`
  y #16 `orquestacion-worktrees`, que rellenan el interior de las casas que este
  shell reserva.

## Fuera de alcance (de esta change)

- El **interior** de cada feature: la cola concurrente y las reglas de la bandeja
  de permisos (#9), el render de diffs y la checklist de `/review` (#15), la
  detección de binarios y el registro ACP del catálogo de flota (#7), el merge
  asistido de worktrees (#16). Aquí solo se reservan sus casas, indicadores y
  teclas.
- El **ciclo de autoría SDD completo** (`/explore`, `/propose` con gates,
  `/plan`…): sus verbos se anclan como reservados en la vista Proyecto y la
  paleta; su implementación es #14.
- El **editor de specs enriquecido** y la revisión de diffs línea a línea, que
  brillan en la GUI de escritorio (fase 2).
- La **reanudación e inspección profunda de sesiones finalizadas** (#8): el shell
  solo reserva el punto de entrada; el visor de auditoría es su interior.
- Temas de color, configuración persistente avanzada y autocompletado de shell.
