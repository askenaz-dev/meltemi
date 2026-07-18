# Propuesta: Fase 0 — Fundación de Meltemi

## Why

Meltemi existe hoy solo como documento fundacional ([meltemi.md](../../../meltemi.md)): no hay ni una línea de código ejecutable. La Fase 0 debe validar las dos apuestas más riesgosas de la arquitectura antes de construir nada encima — (1) que un daemon en Rust puede pilotar un agente de codificación real vía ACP de extremo a extremo, y (2) que el patrón núcleo-headless + clientes finos por JSON-RPC sostiene el flujo spec-driven. Además, un proyecto que predica "ninguna línea sin spec" necesita su propia constitución ratificada desde el primer commit.

## What Changes

- **`constitution.md`**: se ratifican los principios no negociables del proyecto (derivados de meltemi.md §4) como primer artefacto de `.meltemi/`.
- **`proto/`**: esquemas JSON-RPC 2.0 compartidos daemon↔clientes — ciclo de vida de sesión, flujo de propuesta, y eventos de streaming — como contrato versionado.
- **`core/` (nuevo workspace Rust)**: esqueleto de `meltemid` — arranque bajo demanda, instancia única, socket local con permisos exclusivos del usuario, apagado limpio, y gestión de sesiones persistentes.
- **Sesión ACP de extremo a extremo**: `meltemid` lanza el binario oficial de un agente externo como subproceso ACP (JSON-RPC/stdio), completa el handshake, envía un prompt, recibe actualizaciones en streaming y canaliza las peticiones de permiso del agente hacia el cliente.
- **`/propose` mínimo**: un cliente JSON-RPC (CLI de prueba) invoca al daemon, que dirige al agente ACP para generar el andamiaje de una propuesta de cambio en `.meltemi/changes/<name>/`.

## Capabilities

### New Capabilities

- `daemon-lifecycle`: arranque, instancia única, socket local seguro, apagado y estado de `meltemid`; transporte JSON-RPC 2.0 para clientes.
- `acp-session`: lanzar un agente externo oficial como subproceso ACP, negociar capacidades, mantener sesión con streaming de actualizaciones y passthrough de peticiones de permiso.
- `propose-flow`: el flujo `/propose` de extremo a extremo — del comando del cliente, vía daemon y agente ACP, al andamiaje de artefactos en `.meltemi/changes/`.

### Modified Capabilities

<!-- Ninguna: el proyecto es greenfield; no existen specs previas. -->

## Impact

- **Código nuevo**: workspace Rust (`core/`, `proto/`); ningún código existente se ve afectado (el repositorio no tiene código aún).
- **Dependencias**: toolchain Rust estable; crate oficial del Agent Client Protocol; runtime async (tokio); framework JSON-RPC para el socket local.
- **Sistemas**: se crea `.meltemi/` real en el propio repositorio (dogfooding desde el día uno: la constitución del proyecto vive donde el producto la espera).
- **Requisito externo**: al menos un agente CLI con soporte ACP instalado en la máquina de desarrollo para la prueba de extremo a extremo.
- **Fuera de alcance en esta fase** (per meltemi.md §10): TUI completa, motor de specs completo (EARS/deltas), proyección de contexto, orquestación paralela con worktrees, proxy de permisos con reglas persistentes, GUI.
