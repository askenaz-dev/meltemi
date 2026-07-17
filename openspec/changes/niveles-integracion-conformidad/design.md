## Context

Solo existe el nivel 1 (ACP stdio). El research interno cataloga agentes con
adapter ACP (nivel 2), headless JSON (nivel 3) y solo-artefactos (nivel 4). El
catálogo (#7) declara niveles; nadie los verifica. La proyección (#11) ya existe
en este punto del orden — es el vehículo del nivel 4.

## Goals / Non-Goals

**Goals:** semántica operativa de los niveles 2–4; suite de conformidad con
criterios pasa/no-pasa por nivel; el catálogo reporta nivel verificado.
**Non-Goals:** passthrough MCP (#13); reglas de permisos (#9, ya hechas — aquí
solo su aplicación a guardarraíles); mantener adaptadores propios (se consumen
los abiertos existentes, declarados en datos).

## Decisions

### D1 — Lanzador por nivel detrás de una sola interfaz
`meltemid` gana un lanzador por nivel tras la interfaz de sesión existente:
**L1** stdio directo (actual); **L2** binario adaptador (declarado en la entrada
del registro) que puentea a ACP — misma sesión, mismos permisos; **L3** ejecución
headless por tarea con salida JSON/JSONL del agente mapeada a un subconjunto de
eventos de sesión (sin canal de permisos: guardarraíles obligatorios); **L4** sin
proceso: proyección de contexto (#11) + traspaso manual declarado (sesión de tipo
externo, trazable pero no pilotada).

### D2 — Guardarraíles del nivel 3, escritos y verificables
Una tarea L3 corre siempre: (a) dentro de un worktree/dir acotado, (b) con los
controles nativos del agente configurados por Meltemi desde la entrada de datos
(modo de aprobación, sandbox del agente), y (c) con las denegaciones del motor de
reglas aplicadas como configuración previa (lo que las reglas denieguen no se
habilita en el agente). Sin (a)–(c) resueltos, el daemon rehúsa lanzar L3.

### D3 — Conformidad como suite ejecutable
Suite `conformance` (tests de integración) con criterios por nivel: streaming,
cancelación, permisos (L1/L2), sesión, salida estructurada (L3), proyección leída
(L4). En CI corre contra **mocks por nivel** (mock-agent + mock-adapter +
mock-headless); contra agentes reales solo manual y opt-in (`MELTEMI_CONFORMANCE_REAL=1`),
jamás en CI (constitución). El resultado por agente se persiste en el directorio
de datos con fecha y versión del agente.

### D4 — Nivel verificado en el catálogo
`fleet/list` gana `verifiedLevel?` + fecha, leído del último resultado de
conformidad persistido; la TUI lo muestra junto al declarado (declarado ≠
verificado es visible, no vergonzante).

## Risks / Trade-offs

- **Salidas headless heterogéneas** → un mapeador por entrada de datos con el
  subconjunto común (inicio, texto, fin, error); lo no mapeable se conserva crudo
  en el log.
- **Adaptadores de terceros evolucionan** → versión mínima declarada en datos;
  la conformidad detecta la deriva.
- **Falsa sensación de seguridad en L3** → la spec exige rehusar sin
  guardarraíles completos; el estado "sin permisos ricos" es visible en la sesión.

## Migration Plan

Aditivo: lanzadores nuevos tras la interfaz existente; L1 intacto. Campos nuevos
opcionales en registro y `fleet/list`.

## Open Questions

- Formato del resultado de conformidad persistido (¿JSONL por corrida?).
- Cuánta configuración nativa por agente es representable en datos vs código.
