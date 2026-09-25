# adaptadores-con-rawinput

> Vía rápida: un cambio acotado en los dos adaptadores propios, sin campo
> nuevo en el contrato (`rawInput` y `locations` ya son de ACP). Nace de la
> revisión adversarial de `aprobador-en-el-cli` (2026-09-25).

## Why

El aprobador del CLI muestra lo que el agente envía en la petición de permiso,
y las reglas del daemon leen de ahí mismo el comando (`rawInput.command`) y la
ruta (`rawInput.path` o la primera ubicación). Los dos adaptadores propios no
los mandan:

- `meltemi-claude-acp` pone la entrada entera de la herramienta como texto en
  `content` y un título con su primera línea (`claude/permission.rs:98-123`),
  sin `rawInput` ni `locations`;
- `meltemi-codex-acp`, en una aprobación de comando simple, no manda ni el
  título ni el comando: el comando viajó antes, en la `tool_call` del mismo
  ítem (`codex/permission.rs:155-199`, `codex/mapping.rs:626-635`).

Consecuencias medidas al cerrar `aprobador-en-el-cli`:

- una regla con `command-prefix` o `path-prefix` nunca iguala una petición de
  Claude ni de Codex; solo sirven las reglas de herramienta sola, que son
  demasiado amplias para escribirlas con tranquilidad;
- el aprobador no ofrece recordar con ninguno de los dos (su design D5): hoy
  recordar una decisión solo funciona con el agente simulado;
- la sugerencia anti-fatiga del daemon queda en herramienta sola.

## What Changes

- `meltemi-claude-acp` lleva en la petición `rawInput` con la entrada que la
  herramienta recibió, y `locations` cuando la herramienta nombra una ruta.
- `meltemi-codex-acp` lleva en la petición el título y `rawInput.command` del
  ítem que pide aprobarse, los mismos que ya mandó en su `tool_call`.
- Casos de conformidad que pinean los hechos en ambos adaptadores, y un e2e con
  el agente simulado que muestra una regla con prefijo de comando gobernando una
  petición con esa forma.

## Capabilities

### Modified Capabilities

- `own-adapters`: + los hechos de la herramienta viajan en la petición de
  permiso (`rawInput`, `locations`), no solo en el texto.

## Impact

- `core/meltemi-adapters/src/{claude,codex}/{permission,mapping}.rs`, sus tests
  de conformidad y `docs/agentes.md`.
- Cero dependencias; el contrato daemon↔clientes no se mueve.

## Fuera de alcance

- Que las bandejas propongan reglas con esos hechos: `regla-precisa-en-la-bandeja`.
