## Why

La constitución y el rumbo deben llegar a **cada agente** en el formato de
instrucciones que ese agente lee (§2.8): hoy esa proyección es manual —
`AGENTS.md` lo admite en su primera línea y ya se desincronizó una vez (quedó en
"v1.0" tras dos enmiendas). El motor de specs (`meltemi-spec`) ya parsea todos
los artefactos; falta el compilador que los proyecte. Es, además, la única vía de
integración para agentes de nivel 4.

## What Changes

- **Compilador de proyección** en el daemon: constitución + rumbo (con su regla
  de inclusión `siempre`/`por-patrón`/`manual`) + spec activa de la change en
  curso → documento de contexto compilado.
- **Bloques gestionados**: la proyección escribe dentro de marcadores propios y
  **jamás pisa contenido del usuario** fuera de ellos; idempotente y con huella
  de origen (qué artefactos, qué versiones).
- **Variantes por formato**: `AGENTS.md` como base y variantes por agente según
  la matriz del research (nombres/ubicaciones que cada agente lee).
- **Contrato**: `context/project` (o verbo CLI `project`) para regenerar bajo
  demanda; regeneración automática al archivar una change (hook interno).
- **Dogfooding inmediato**: este repositorio reemplaza su proyección manual por
  la automática (se retira la advertencia de `AGENTS.md`).

## Capabilities

### New Capabilities
- `context-projection`: compilación, bloques gestionados, variantes, regeneración.

### Modified Capabilities
- `cli-contract`: gramática gana el subcomando de proyección (aditivo).

## Impact

- `core/meltemid` + `core/meltemi-spec` (lectura ya resuelta), `proto/` (+1),
  `tui/` (acción en la vista Proyecto y paleta). `AGENTS.md` de este repo pasa a
  generado-gestionado.

## Fuera de alcance

- Mapa del repositorio y referencias `@archivo` (#12).
- Inyección MCP (#13). Selección dinámica de contexto por tarea (fase 2+).
