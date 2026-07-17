## Why

No todos los agentes hablan ACP nativo. La visión (§7.4) define cuatro niveles de
integración; hoy solo existe el nivel 1 (ACP por stdio). Sin los niveles 2–4 y
una **suite de conformidad** con criterios pasa/no-pasa, "compatible con Meltemi"
sería una afirmación sin verificación — y la promesa de orquestar *los agentes
del mercado* quedaría en los que ya hablan ACP.

## What Changes

- **Nivel 2 — adaptadores**: ejecución vía adaptadores ACP abiertos existentes
  para agentes populares sin ACP nativo (siempre binarios oficiales + su propia
  auth; juego limpio §4.7).
- **Nivel 3 — headless estructurado**: modo no interactivo con salida JSON/JSONL
  del agente; sin canal de permisos rico → se ejecuta dentro de guardarraíles
  (worktree + controles nativos del agente configurados por Meltemi desde un
  solo lugar).
- **Nivel 4 — artefactos**: integración por archivos de instrucciones/artefactos
  (proyección de contexto como única vía).
- **Suite de conformidad por nivel**: criterios ejecutables pasa/no-pasa
  (streaming, cancelación, permisos, sesiones…) que fijan qué significa cada
  nivel; corre contra `mock-agent` en CI y contra agentes reales manualmente.
- El catálogo (#7) muestra el nivel **verificado**, no el declarado.

## Capabilities

### New Capabilities
- `integration-levels`: semántica operativa y conformidad de los niveles 1–4.

### Modified Capabilities
- `fleet-catalog`: el nivel por agente pasa a estar respaldado por conformidad.
- `acp-session`: lanzamiento vía adaptador (nivel 2) como variante del arranque.

## Impact

- `core/meltemid` (lanzadores por nivel), suite de conformidad nueva (tests),
  `docs/research/integracion-agentes.md` se convierte en fuente de la matriz.
- CI sigue sin red y sin agentes reales (constitución): la conformidad en CI usa
  mocks por nivel.

## Fuera de alcance

- Passthrough MCP (#13). Reglas de permisos (#9) — aquí solo su *aplicación* a
  los guardarraíles del nivel 3.
- Mantener adaptadores propios: se consumen adaptadores abiertos existentes.
