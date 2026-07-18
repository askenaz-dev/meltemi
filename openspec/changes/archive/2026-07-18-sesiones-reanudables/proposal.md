## Why

Una sesión muere con su turno o con el daemon, y lo único que sobrevive es el log
JSONL. `status` solo enumera sesiones **activas**; la TUI retiene las finalizadas
apenas en memoria (limitación registrada al armar `tui-nucleo-ux`), y tras una
caída no hay recuperación ni forma de inspeccionar qué pasó sin leer JSONL a
mano. Para un plano de control, las sesiones son el activo primario: deben poder
reanudarse, recuperarse y navegarse.

## What Changes

- **Persistencia de metadatos de sesión** en el daemon (más allá del log):
  identidad, agente, proyecto, estado final, marcas temporales.
- **Reanudación ACP** donde el agente la soporte (capacidad `session/load` del
  protocolo, anunciada en el handshake): retomar una sesión con su contexto; si
  el agente no la soporta, degradación honesta ("no reanudable, sí inspeccionable").
- **Recuperación tras caída del daemon**: al rearrancar, las sesiones quedan
  marcadas como interrumpidas (no fantasmas), listables y reanudables si procede.
- **Contrato**: `session/list` (activas + históricas, paginado) y `session/log`
  (lectura del JSONL por RPC) — cierra el acceso post-mortem que `tui-shell`
  acotó a memoria y da al visor de auditoría su fuente real.
- **TUI**: la tabla de Sesiones gana histórico y el drill-in puede abrir una
  sesión finalizada (transcript desde el log); acción "reanudar" cuando aplique.
- **CLI**: `sessions` (listar) con `--json`.

## Capabilities

### New Capabilities
- `session-history`: persistencia, listado e inspección de sesiones pasadas.

### Modified Capabilities
- `acp-session`: reanudación con capacidad negociada y degradación honesta.
- `cli-contract`: gramática gana `sessions` (aditivo).

## Impact

- `core/meltemid` (metadatos + list/log + resume), `proto/` (+2 métodos),
  `tui/` (histórico y visor). El formato del JSONL no cambia (solo se lee).

## Fuera de alcance

- Worktrees y orquestación paralela (#16); checkpoints/rollback (#17).
- Retención/limpieza configurable de históricos (change futura de mantenimiento).
