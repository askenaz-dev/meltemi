# mcp-stdio-passthrough

> Vía rápida (fast-forward): una condición corregida, un mensaje de
> degradación por transporte y una pierna de conformidad. Deltas solo ADDED
> sobre `mcp-passthrough`. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, hallazgo AI-V1 del
> verificador).

## Why

`core/meltemid/src/mcp.rs:17-18` decide si un agente recibe los servidores
MCP declarados con `announces_mcp(caps) = caps.http || caps.sse`. En ACP,
`http` y `sse` son **capacidades opcionales** que un agente anuncia además
del piso; el transporte por stdio —el de la inmensa mayoría de servidores
locales— es el piso mismo. La condición, tal como está, hace que un agente
que no anuncia http ni sse **no reciba ningún servidor**, ni siquiera los
stdio que sí sabe lanzar, y el log lo cuenta como «omitido, honest
degradation». Es una degradación honesta con una premisa falsa.

El passthrough es una de las promesas del núcleo (meltemi.md §6.10, spec
`mcp-passthrough`) y la forma en que «los agentes llegan con sus
herramientas». Un servidor stdio declarado una vez debe llegar a todo agente
ACP.

## What Changes

- **Stdio siempre, http/sse solo si se anuncian**: la entrega separa por
  transporte; los servidores stdio declarados se entregan a todo agente ACP,
  y los http/sse solo cuando `mcpCapabilities` lo anuncia. Lo que no se
  entrega se registra con el transporte y la causa, no con un «omitido»
  genérico.
- **La configuración declara el transporte sin ambigüedad**: si
  `McpServerConfig` no distingue stdio de http hoy, el design lo fija con un
  campo aditivo y un default retrocompatible (comando ⇒ stdio; URL ⇒ http).
- **Conformidad**: pierna «servidor stdio declarado aparece en las
  herramientas del agente» en `docs/conformidad-manual.md`, con OpenCode y
  Claude Code.
- **Docs**: la página de MCP dice qué transporte llega a quién.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `mcp-passthrough`: + requisito «Los servidores stdio se entregan a todo
  agente ACP; http/sse según capacidad anunciada; la omisión nombra el
  transporte y la causa».

## Impact

- Archivos: `core/meltemid/src/{mcp.rs, config.rs, acp.rs}`, tests del
  daemon con el agente simulado anunciando cada combinación,
  `docs/conformidad-manual.md`, la página de MCP en docs.
- Cero dependencias nuevas; el contrato `proto/` no cambia salvo que el
  campo de transporte deba aflorar en `config`, y entonces es aditivo.
- Riesgo: un agente que dice aceptar stdio y falla al lanzar el servidor; es
  su error y su log, no el de Meltemi, pero el registro de sesión debe
  dejarlo ver.

## Fuera de alcance

- Un cliente MCP propio en el daemon: nunca (§3, sin red; el motor propio
  tendrá el suyo en su subproceso).
- Instalar servidores MCP: son del usuario.
