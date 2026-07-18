## Context

Dirigir agentes exige dar el trozo justo del repo. La proyección (#11) entrega el
contexto del método; falta el del código: mapa del repositorio y referencias
`@archivo`/`@carpeta` en prompts, expandidas de forma determinista y auditable.

## Goals / Non-Goals

**Goals:** `repo/map` respetando gitignore; expansión `@` con límites declarados
y truncado anunciado; registro de expansiones en el log; autocompletado en el
compositor de la TUI.
**Non-Goals:** indexación semántica/embeddings (jamás sin change propia);
selección automática de contexto (fase 2+).

## Decisions

### D1 — Recorrido con `ignore` (dependencia nueva justificada)
Se adopta el crate `ignore` (pineado) para el recorrido honrando `.gitignore`
anidados: reimplementar gitignore a mano es un nido de errores conocido y el
crate es el estándar auditado del ecosistema. Es la única dependencia nueva.

### D2 — Mapa por contrato con presupuesto
`repo/map` (raíz, profundidad, límite de entradas) → árbol con tamaños y
extensión; truncado **declarado** en la respuesta (`truncated: true` + cuántas
entradas quedaron fuera), jamás silencioso.

### D3 — Expansión `@` determinista y auditable
En el prompt, `@ruta` inyecta el archivo (con cerca de código y ruta) y
`@carpeta/` inyecta su listado (no contenidos). Límites explícitos por archivo y
por prompt; exceso → truncado con marca visible en el propio prompt enviado. El
log de sesión registra qué se expandió (rutas y bytes), auditable post-mortem.

### D4 — Autocompletado en el compositor
El compositor de la TUI completa `@` contra `repo/map` (prefijo), reutilizando el
patrón de captura de texto ya especificado en el shell.

## Risks / Trade-offs

- **Prompts gigantes** → límites duros con marca de truncado; el humano ve lo que
  el agente verá.
- **Repos enormes** → mapa con presupuesto + profundidad; sin recursión completa
  por defecto.

## Migration Plan

Aditivo (método nuevo + expansión en el envío de prompt). Prompts sin `@` no
cambian.

## Open Questions

- Sintaxis de escape para un `@` literal (probable: `@@`) — fijar en tasks.
