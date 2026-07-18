## Why

El daemon pilota hoy **un** agente fijado en `.meltemi/config.toml`; el producto
promete una **flota**: los agentes que el usuario ya tiene, descubiertos y
gobernados desde un solo lugar. La vista Flota de la TUI es una casa reservada
vacía ("sin agentes detectados") y no existe forma de saber qué agentes hay en la
máquina, en qué nivel de integración ni con qué estado. Sin catálogo no hay
orquestación: es el corazón del plano de control. (Research previo:
`docs/research/integracion-agentes.md`.)

## What Changes

- **Catálogo de flota en el daemon**: detección local de binarios de agentes
  (sondeo de PATH y rutas conocidas por agente, versión detectada) cruzada con el
  **registro público ACP** legible por máquina.
- Por agente: **nivel de integración** (1 ACP nativo / 2 adaptador / 3 headless
  JSON / 4 artefactos — semántica fina en #10), modelo de permisos, estatus
  (detectado / configurado / no disponible).
- **Contrato**: nuevo método RPC `fleet/list` (aditivo en `proto/`).
- **TUI**: la vista Flota (4) pasa de casa vacía a tabla real; el estado vacío
  de Sesiones enlaza a una Flota con datos.
- **CLI**: subcomando `fleet` (delta a la gramática de `cli-contract`) con
  variante `--json`; registro en la paleta.
- **Selección de agente por proyecto**: `config.toml` puede referirse a un agente
  del catálogo en vez de a un comando literal (compatibilidad conservada).

## Capabilities

### New Capabilities
- `fleet-catalog`: detección, registro, niveles, estatus y consulta de la flota.

### Modified Capabilities
- `cli-contract`: gramática gana `fleet` (aditivo).

## Impact

- `core/meltemid` (módulo de catálogo), `proto/meltemi-proto` (+1 método),
  `tui/` (vista Flota, CLI `fleet`). Config de proyecto extendida, retrocompatible.
- **Decisión abierta para el design**: cómo se obtiene el registro público ACP
  sin violar la postura local-first (¿instantánea empaquetada en el binario con
  refresco manual explícito vs. fetch de red?). La constitución prohíbe puertos
  de escucha y telemetría; una petición saliente explícita es distinta, pero debe
  decidirse y quedar escrita. En CI, siempre fixture local (sin red).

## Fuera de alcance

- La suite de conformidad y la semántica operativa de los niveles 2–4 (#10).
- Reglas de permisos por agente (#9); passthrough MCP (#13).
- Instalación/actualización de agentes (jamás: BYO-agent).
