## Why

El passthrough de permisos existe y es seguro (deny-by-default sin cliente,
timeout que deniega), pero es todo-o-nada: la CLI scriptable deniega todo, el
devclient aprueba todo, y no hay reglas ni memoria. La QA de la primera prueba
(2026-07-16, H1) mostró el costo: un `propose` con permiso denegado reporta
`Completed` dejando un andamio vacío sin decirlo en el resultado. Además el
contador de pendientes es **por conexión** (no hay RPC para enumerarlos): una
reconexión pierde la bandeja — deuda confirmada. Esta es la pieza de gobernanza
central del producto (§6.5): una sola bandeja, reglas persistentes, auditoría.

## What Changes

- **Motor de reglas** permitir/preguntar/denegar por herramienta, comando y ruta;
  persistentes por proyecto y globales; evaluadas en el daemon antes de escalar
  al cliente.
- **Pendientes de primera clase en el contrato**: los permisos pendientes viven
  en el daemon (`permission/pending` para enumerar + notificación de cambios);
  la bandeja sobrevive reconexiones y multi-cliente (un cliente resuelve → los
  demás se reconcilian).
- **UX de la bandeja (interior de la casa de la TUI)**: cola de peticiones
  concurrentes, aprobación/denegación por ítem, **creación de reglas in situ**
  ("permitir siempre este comando en este proyecto"), mitigación de fatiga.
- **Auditoría**: cada decisión (regla o humano, con qué regla) ya se registra en
  el JSONL; se enriquece con la regla aplicada.
- **Honestidad de resultado (H1/H4/H5)**: un turno que sufrió denegaciones lo
  declara en el resultado de `propose` (y la palabra de estado deja de ser
  `Debug` capitalizado; rutas normalizadas).
- **CLI**: modo de aprobación interactiva mínima para `propose` scriptable
  (o flag explícito `--allow`/`--deny-all` documentado).

## Capabilities

### New Capabilities
- `permission-rules`: motor de reglas, pendientes persistentes, bandeja y fatiga.

### Modified Capabilities
- `acp-session`: pendientes enumerables; decisión con procedencia (regla/humano).
- `propose-flow`: el resultado declara denegaciones ocurridas durante el turno.
- `tui-shell`: la bandeja (vista 3) pasa de esqueleto a cola operativa.

## Impact

- `core/meltemid` (motor + estado pendiente), `proto/` (+métodos y campos),
  `tui/` (bandeja interactiva + reconciliación del contador del chrome).
- Seguridad: las reglas nunca amplían lo que el agente pidió; deny-by-default
  se mantiene como piso.

## Fuera de alcance

- Configuración de controles nativos de agentes niveles 3–4 (#10).
- Políticas de organización/equipo (fase 3).
