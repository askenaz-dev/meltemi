# regla-precisa-en-la-bandeja

> Vía completa: amplía la entrada de la cola de permisos, un campo del
> contrato que leen las tres superficies. Nace de la revisión adversarial de
> `aprobador-en-el-cli` (2026-09-25).

## Why

El requisito vivo «Bandeja interactiva con creación de reglas in situ»
(`permission-rules`) pide proponer «la regla más específica posible». Hoy:

- la bandeja de la TUI, sin sugerencia del daemon, propone la herramienta sola
  (`tui/src/shell/live.rs`, `selected_rule_proposal`): con Claude o Codex eso
  es `allow execute`, todo comando; y para una petición sin clase ni título,
  una regla con el marcador `permission` de la cola, que nunca gobierna nada;
- la GUI solo ofrece la sugerencia anti-fatiga;
- el aprobador del CLI ya construye la regla precisa —la herramienta con el
  comando o la ruta exactos— y no la ofrece cuando sería más amplia que lo que
  se vio, cuando tocaría `.meltemi/` o cuando la sesión trabaja en un worktree
  de tarea. Las bandejas no pueden hacer lo mismo porque la cola no expone ni
  el comando ni la ruta.

## What Changes

- `PendingPermission` gana `command` y `path` (aditivos): los mismos hechos que
  las reglas leen.
- La bandeja de la TUI y la de la GUI proponen la regla que propone el CLI, con
  los mismos casos en que no se ofrece, y dicen por qué cuando no la ofrecen.
- La construcción y la guarda viven en una función por lenguaje (Rust para CLI y
  TUI; TypeScript para la GUI), con un test de paridad entre ambas.

## Capabilities

### Modified Capabilities

- `permission-rules`: la cola expone comando y ruta; toda superficie propone la
  regla más específica y los mismos casos sin regla.

## Impact

- `proto/` (campos aditivos), `core/meltemid/src/pending.rs`, `tui/src/shell/`,
  `desktop/ui/src/lib/views/Permissions.svelte`.
- Depende de `adaptadores-con-rawinput` para que los agentes reales tengan
  comando y ruta que exponer.
