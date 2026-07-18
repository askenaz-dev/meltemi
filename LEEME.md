<!-- SPDX-License-Identifier: Apache-2.0 -->
# Meltemi

**El plano de control spec-driven para el desarrollo agéntico.** Open source
(Apache-2.0), gratuito, de la comunidad. Meltemi orquesta los agentes de
codificación que ya usas —mediante estándares abiertos— bajo una disciplina
donde ninguna línea de código se escribe sin una especificación revisada.

> **Un rumbo, muchas velas.** Una spec clara impulsa cualquier número de agentes,
> de cualquier fabricante, sin atarte a ninguno.

_Read me in English: [README.md](README.md)._

## Qué es

Un daemon headless (`meltemid`) más clientes finos —una interfaz de terminal y
CLI (`meltemi`), con una GUI de escritorio planeada—. Habla estándares abiertos:
el Agent Client Protocol para pilotar agentes, MCP para herramientas, JSON-RPC
sobre un **socket local únicamente** (sin puerto de red, jamás). Trae tu agente,
tu clave y tu modelo.

El flujo es spec-first: se propone un cambio, se revisan sus escenarios y solo
entonces se implementa —tarea a tarea, en worktrees de git aislados, con
checkpoints automáticos pre-tarea y un commit atómico por tarea que rastrea cada
línea hasta el requisito que la originó.

## Qué no es

Ni un editor de propósito general (la edición de código es utilitaria, al
servicio del bucle agéntico), ni otro agente, ni un servicio en la nube, ni
CI/CD, ni un marketplace. Sin créditos, sin tarifas, sin lock-in.

## Estado

Fase 1, pre-v0.1. El daemon, el motor de specs, el proxy de permisos, la
orquestación por worktrees, los checkpoints, los commits por tarea y el ciclo SDD
completo (`propose → plan → review → verify → archive → implement`) están
implementados y probados en Windows, macOS y Linux —**Windows es de primera
clase**—. La TUI interactiva y la GUI están en curso. Ver
[`docs/plan-de-cambios.md`](docs/plan-de-cambios.md).

## Instalación y primer paso

```
git clone <este repositorio>
cd meltemi
cargo build --release

meltemi help
meltemi propose "añade un interruptor de modo oscuro a ajustes"
```

Luego sigue el [quickstart](docs/quickstart.md).

## Documentación

- [Quickstart](docs/quickstart.md), [Arquitectura](docs/arquitectura.md),
  [Método SDD](docs/metodo-sdd.md), [Referencia CLI](docs/referencia-cli.md),
  [Accesibilidad](docs/accesibilidad.md), [Plataformas](docs/plataformas.md).
- [Contribuir](CONTRIBUTING.md) · [Gobernanza](GOVERNANCE.md) · [Seguridad](SECURITY.md)

## Licencia

Apache-2.0, para siempre (constitución §12).
