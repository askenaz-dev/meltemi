# codex-reanudar-y-mcp

> Vía completa (proposal → design → specs → tasks): depende de `codex-al-dia`
> (el esquema re-anclado decide qué métodos existen) y toca el adaptador de
> Codex en dos capacidades que el de Claude Code ya tiene. Nace del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha AI-8).

## Why

Los dos adaptadores propios no son gemelos. `meltemi-claude-acp` reanuda
sesiones y entrega los servidores MCP declarados
(`docs/agentes.md`: «MCP passthrough: yes»); `meltemi-codex-acp` no hace
ninguna de las dos: el registro lo declara `mcp-support = false` y el
adaptador no implementa `load_session`. Dos consecuencias visibles: `direct`
sobre una sesión de Codex terminada no puede reanudarla —`sesion-que-espera`
y `redirigir-turno` cuentan con la reanudación—, y las herramientas MCP que
el usuario declaró una vez (`mcp-passthrough`) llegan a Claude Code y a
OpenCode pero no a Codex.

El proveedor documenta el hilo reanudable y la configuración por hilo en su
modo app-server; lo que falta es mapearlo. La regla de la casa para hacerlo:
por el cable, con la configuración que el app-server acepta por petición;
**jamás** escribiendo en los archivos del CLI del usuario (`~/.codex/…`),
que son suyos (constitución §2 y la lista «descartado por escrito» de
`apagado-entero-y-modos`).

## What Changes

- **`load_session` en el adaptador de Codex** mapeado a la reanudación de
  hilo que el esquema re-anclado exponga; si la versión anclada no la trae,
  el adaptador rehúsa con `provider_capability_missing` y su remedio, en vez
  de fingir una sesión nueva.
- **MCP por el cable**: los servidores declarados en la configuración de
  Meltemi se entregan en la configuración por hilo del app-server (stdio
  siempre; http/sse si el proveedor los acepta), con las referencias `$VAR`
  resueltas solo en el subproceso; el adaptador anuncia `mcpCapabilities`
  según lo que de verdad pueda entregar y el registro pasa a
  `mcp-support = true` cuando se mida.
- **Conformidad**: dos piernas nuevas en `docs/conformidad-manual.md` —
  reanudar un hilo y ver una herramienta MCP declarada en la lista del
  agente— medidas contra el binario real.
- **Docs**: `docs/agentes.md` deja de decir que Codex no lleva MCP.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: + requisitos «Reanudación de hilo en Codex» y «MCP
  entregado por el cable, nunca por archivos del CLI».
- `mcp-passthrough`: + escenario «Codex recibe los servidores declarados».
- `session-history`: + escenario «una sesión de Codex terminada es
  reanudable por `direct`».

## Impact

- Archivos: `core/meltemi-adapters/src/codex/`, `core/mock-provider`
  (fixtures de reanudación y de configuración por hilo),
  `core/meltemid/data/fleet-registry.toml`, `docs/agentes.md`,
  `docs/conformidad-manual.md`.
- Cero dependencias nuevas; ningún RPC nuevo.
- Riesgo real: la semántica de «reanudar» del proveedor (¿el hilo conserva
  aprobaciones? ¿el cwd?) frente a la de ACP; el design lo mide y, ante
  ambigüedad, rehúsa (la lección de `apagado-entero-y-modos`).

## Fuera de alcance

- `turn/steer`, bifurcar o compactar hilos: se estudian con el esquema en la
  mano, como changes propias.
- Leer o modificar la configuración del CLI del usuario: nunca.
