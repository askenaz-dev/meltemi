## Why

Los agentes ganan medio poder por sus herramientas MCP. El usuario ya declara
servidores MCP por proyecto para cada agente por separado, con N formatos.
Meltemi debe ser el punto único: **declarar una vez, inyectar a cada agente
compatible** vía ACP (§6.10), con la higiene de credenciales de §8.7 (las
credenciales de los servidores son del usuario; Meltemi las pasa, no las lee ni
las almacena).

## What Changes

- **Declaración por proyecto** de servidores MCP (`.meltemi/config.toml`):
  stdio y HTTP streamable, con variables de entorno referenciadas — nunca
  valores secretos copiados a artefactos de Meltemi.
- **Inyección ACP**: los servidores declarados se pasan en `session/new` a los
  agentes que anuncian soporte MCP (capacidad negociada); degradación honesta y
  visible cuando el agente no lo soporta.
- **Visibilidad**: la vista de Sesión/Flota muestra qué servidores recibió cada
  agente; el log de sesión registra la inyección (nombres, no secretos).
- **Higiene §8.7**: validación de que la config no incrusta secretos en claro
  (aviso con remedio).

## Capabilities

### New Capabilities
- `mcp-passthrough`: declaración, inyección negociada, visibilidad e higiene.

### Modified Capabilities
- `acp-session`: `session/new` transporta los servidores MCP declarados.
- `fleet-catalog`: soporte MCP como atributo del agente.

## Impact

- `core/meltemid` (config + inyección), `proto/` (campos), `tui/` (visibilidad).
- CI: servidores MCP simulados; sin red (constitución).

## Fuera de alcance

- Cliente MCP nativo del motor propio (fase 2, §6.10).
- Gestión/instalación de servidores MCP o marketplace (jamás: BYO).
