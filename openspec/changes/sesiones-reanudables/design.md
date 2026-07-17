## Context

`status` solo enumera sesiones activas; el JSONL persiste pero no hay RPC para
listarlo ni leerlo; la TUI retiene finalizadas solo en memoria; tras una caída
no queda rastro consultable del estado. ACP define la capacidad de cargar
sesiones (`loadSession`) que algunos agentes anuncian.

## Goals / Non-Goals

**Goals:** metadatos persistentes; `session/list` (activas+históricas) y
`session/log` (lectura paginada del JSONL); marcado de interrumpidas tras caída;
reanudación negociada con degradación honesta; histórico en TUI y CLI.
**Non-Goals:** worktrees (#16); retención/limpieza configurable (futura);
re-ejecución de turnos perdidos (imposible honesto).

## Decisions

### D1 — Índice de metadatos junto a los logs
`sessions/index.jsonl` apend-only por proyecto (id, agente, nivel, estado final,
inicio/fin, ruta del log). El índice se reconstruye del directorio si falta
(los logs son la verdad). Al arrancar, toda sesión del índice sin fin registrado
se marca `interrupted` (la caída no deja fantasmas "activos").

### D2 — Lectura por contrato, no por filesystem del cliente
`session/list` (filtros: proyecto, estado, límite) y `session/log` (paginado por
offset de línea) — los clientes finos jamás leen el disco del daemon (paridad
con acceso remoto por túnel).

### D3 — Reanudación negociada
Si el agente anunció `loadSession` en su handshake, la acción "reanudar" abre una
sesión nueva pidiendo la carga de la anterior (id del agente persistido en
metadatos); sin la capacidad, la acción no se ofrece como posible sino como "no
reanudable — inspeccionable" (degradación honesta, spec viva de la TUI ya exige
reservado-no-error).

### D4 — Superficies
TUI: la tabla de Sesiones gana pestaña/filtro de históricas; drill-in de una
finalizada muestra el transcript desde `session/log` (paginado hacia atrás). CLI:
subcomando `sessions` (delta acumulativo de gramática: asume `fleet` y `project`
ya operativas) con `--json`.

## Risks / Trade-offs

- **Logs grandes** → paginado por líneas + tail por defecto en la TUI.
- **Índice corrupto/perdido** → reconstrucción desde los logs (fuente de verdad).
- **Reanudar ≠ restaurar**: el estado del repo pudo cambiar; la acción lo
  advierte (el agente recibe el contexto que él mismo persista, no magia).

## Migration Plan

Aditivo. Sesiones previas sin índice aparecen al reconstruirlo desde los logs
existentes.

## Open Questions

- Umbral de retención visible por defecto en la TUI (¿últimas 50?).
