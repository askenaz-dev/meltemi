## Why

Dirigir a un agente exige darle el trozo justo del repositorio sin pegar rutas a
mano. La proyección (#11) entrega el contexto *del método*; falta el contexto
*del código*: un mapa navegable del repo y referencias `@archivo`/`@carpeta` en
los prompts que se expandan de forma determinista y auditable (§6.9).

## What Changes

- **Mapa del repositorio** en el daemon: árbol respetando `.gitignore`, tamaños y
  lenguajes, consultable por RPC (fuente para compositor de la TUI y para la GUI).
- **Referencias `@` en prompts**: `@ruta/archivo` y `@carpeta/` se expanden al
  contenido (con límites de tamaño explícitos y truncado declarado, nunca
  silencioso) antes de enviar el prompt al agente; lo expandido queda registrado
  en el log de sesión.
- **Autocompletado**: el compositor de la TUI ofrece completar `@` contra el mapa.
- **Contrato**: `repo/map` (+ parámetros de profundidad/filtro).

## Capabilities

### New Capabilities
- `repo-context`: mapa del repo y expansión auditable de referencias `@`.

### Modified Capabilities
- `acp-session`: el prompt enviado registra las expansiones realizadas.

## Impact

- `core/meltemid` (mapa + expansión), `proto/` (+1), `tui/` (autocompletado del
  compositor). Sin dependencias nuevas previstas (walk propio + gitignore del
  design).

## Fuera de alcance

- Indexación semántica o embeddings (jamás en el núcleo local sin change propia).
- Selección automática de contexto por relevancia (fase 2+).
