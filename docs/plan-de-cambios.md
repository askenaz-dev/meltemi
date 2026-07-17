# Plan maestro de cambios — de cero al MVP (hito v0.1)

> Backlog ordenado de propuestas de cambio que llevan a Meltemi desde el repositorio vacío hasta el hito v0.1 de Fase 1 (meltemi.md §10). Este documento es el mapa; **cada change escribe sus cuatro artefactos (proposal, design, specs, tasks) inmediatamente antes de implementarse** — nunca durante, y nunca se implementa nada que no esté aquí o en una change aprobada. Producto de la auditoría de planificación del 2026-07-11 (5 lentes, 64 hallazgos).

## Estado de "listo para programar"

| Prerequisito | Estado |
|---|---|
| Documento fundacional (meltemi.md v1.0) | ✅ Ratificado 2026-07-11 |
| Constitución + rumbo (`.meltemi/`) | ✅ Ratificados 2026-07-11 |
| Change de Fase 0 completa (4 artefactos + enmiendas de auditoría) | ✅ |
| Proyección de contexto raíz (`AGENTS.md`/`CLAUDE.md`) | ✅ |
| Research de integración de agentes persistido | ✅ `docs/research/integracion-agentes.md` |
| Plan maestro de cambios | ✅ Este documento |
| **Ratificación humana (Guillermo)** de meltemi.md, constitución y rumbo | ✅ 2026-07-11 |
| **Registros operativos** (org GitHub, org npm, dominio, crates) | 🟡 org GitHub y npm hechas (`askenaz-dev`); dominio `meltemi.dev` en compra; crates por reservar |

## Fase 0 (en curso)

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 0 | ✅ `fase-0-fundacion` | Constitución ratificada, contrato `proto/`, esqueleto `meltemid`, sesión ACP e2e, `/propose` mínimo | — |

## Fase 1 — backlog ordenado (MVP)

**Ola 1 — método y fundamentos** (arranca al cerrar Fase 0):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 1 | ✅ `enmiendas-fundacionales-v1` | Vía rápida: ratifica el bootstrap en dos etapas (excepción a §9.3), actualiza §0 a la marca V2, asigna fase a §6.12 (métricas), aclara telemetría post-v1, añade plataforma primaria de desarrollo | 0 |
| 2 | ✅ `formato-artefactos-meltemi` | Esquema canónico de `.meltemi/`: nombres de artefactos, cabeceras de delta, canon EARS (política bilingüe: estructura en inglés vs prosa en español — resolver la contradicción §5.1/práctica), front-matter de rumbo | 0 |
| 3 | ✅ `motor-specs-artefactos` | Parseo y validación de la estructura `.meltemi/` completa (constitución, rumbo, specs, changes, archive) | 2 |
| 4 | ✅ `motor-ears-deltas` | Validación EARS en vivo, parseo/aplicación de deltas; la detección semántica de contradicciones/huecos quedó explícitamente diferida a una change futura | 3 |
| 4b | ✅ `enmienda-edicion-movil` | Vía rápida (2026-07-15): cerca de la edición utilitaria in situ (spec `edit-surface`) y alcance del compañero móvil monitorear/aprobar/dirigir vía túnel SSH (spec `mobile-companion`); meltemi.md v1.2 → **v1.3** (ratificación pendiente) | 1 |

**Ola 2 — superficies** (en paralelo con Ola 1 tras el #0):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 5 | ✅ `cli-contrato` | Gramática de subcomandos, regla de despacho CLI↔TUI, códigos de salida, `--json`, disciplina stdout/stderr; mapeo comando↔RPC junto a `proto/` | 0 |
| 6 | ✅ `tui-nucleo-ux` | Arquitectura de información de la TUI (paneles, navegación), estados vacíos y onboarding de primer uso, accesibilidad terminal (no-solo-color, NO_COLOR, fallback ASCII) | 5 |
| 7 | `catalogo-flota` | Registro público ACP + detección local de binarios; nivel de integración, permisos y estatus por agente | 0 |
| 8 | `sesiones-reanudables` | Reanudar sesión ACP, recuperación tras caída, inspección/navegación de sesiones | 0 |

**Ola 3 — capa de agentes completa**:

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 9 | `proxy-permisos` | Motor de reglas permitir/preguntar/denegar + UX de la bandeja (cola de peticiones concurrentes, creación de reglas in situ, mitigación de fatiga de aprobación, auditoría) | 7 |
| 10 | `niveles-integracion-conformidad` | Niveles 2 (adaptadores), 3 (headless JSON) y 4 (artefactos); suite de conformidad por nivel con criterios pasa/no-pasa | 7 |
| 11 | `proyeccion-contexto` | Compilación constitución+rumbo+spec activa → AGENTS.md y variantes; bloques gestionados que nunca sobrescriben contenido del usuario | 3 |
| 12 | `gestion-contexto-repo` | Mapa del repositorio, referencias `@archivo`/`@carpeta` en prompts | 11 |
| 13 | `mcp-passthrough` | Declaración de servidores MCP e inyección a agentes compatibles; higiene §8.7 | 7 |

**Ola 4 — el ciclo SDD completo**:

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 14 | `ciclo-sdd-autoria` | `/constitution`, `/explore`, `/propose` completo (requirements EARS + design + deltas + tasks con gates humanos), `/plan`; modo dual `spec-full`/`fast-forward` con criterio escrito de proporcionalidad | 4, 6 |
| 15 | `revision-specs-ux` | La obsesión (§4.9): render de diff de deltas en terminal, presentación de contradicciones, checklist interactiva de `/review`, comentario→instrucción al agente | 14 |
| 16 | `orquestacion-worktrees` | Worktrees aislados, N agentes × M tareas, carreras, merge asistido lado a lado | 8 |
| 17 | `checkpoints-rollback` | Checkpoint automático pre-tarea, reversión granular, alcance honesto (qué NO se revierte) enganchado a la clasificación del proxy | 16 |
| 18 | `git-commit-por-tarea` | Commit atómico con trailer de trazabilidad hasta el requisito EARS; convención de mensajes | 16 |
| 19 | `comandos-verify-archive` | `/verify` (checklist por requisito + tests vinculados a criterios) y `/archive` (fusión de deltas en la verdad viva) | 4, 18 |
| 20 | `comando-implement` | `/implement`: despliegue de agentes sobre `tasks.md` en modo planificar/actuar con checkpoints | 14, 9, 16, 17, 18 |

**Ola 5 — salida al mundo** (antes de que el repo sea público / del hito v0.1):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 21 | `gobernanza-comunidad` | GOVERNANCE.md, CONTRIBUTING.md (contribución vía specs), CODE_OF_CONDUCT.md, SECURITY.md, texto del CLA acotado (§9.3) + tooling de firma, plantillas `.github/` | — (antes de público) |
| 22 | `documentacion-inicial` | README raíz, quickstart, esqueleto de docs/ y tooling de documentación | 6 |
| 23 | `distribucion-releases` | Versionado, firmado y custodia de claves, empaquetado por plataforma, instalador de una línea | 6 |
| 24 | `migracion-openspec-a-meltemi` | El dogfooding definitivo: las specs vivas del propio proyecto migran a `.meltemi/` | 19 |
| 25 | `hito-v01-aceptacion` | El escenario del hito como spec ejecutable: una feature de idea a código en terminal con dos agentes de proveedores distintos en paralelo | todo lo anterior |

## Documentos transversales (se crean durante Fase 1, referenciados por las changes)

- `docs/plataformas.md` — matriz de soporte: SO mínimos, arquitecturas, webviews (antes de #23).
- `docs/presupuestos-rendimiento.md` — los números de §12 traducidos a criterios EARS y gates de CI (antes de #6).
- `docs/paridad-nucleo.md` — matriz viva capacidad → RPC → TUI → GUI; toda change que añada un RPC la actualiza (desde #5).
- `docs/operaciones/checklist-lanzamiento.md` — registros y activos (ver pendientes del arquitecto).

## Fase 2 (v1.0) — se planifica al cerrar Fase 1

`gui-tauri-paridad` (con `docs/ux/design-system.md` como insumo; **incorpora edición utilitaria in situ con inteligencia LSP y resuelve la política de concurrencia humano↔agente sobre un mismo worktree** — gobernada por la spec `edit-surface`) · `motor-propio-byok` · `sandbox-propio` · `hooks-eventos` · `plugins-skills-sdk` · `i18n-superficies` · `metricas-sdd-locales` · `lsp-superficie-revision`.

> **Gobernanza de alcance** (change `enmienda-edicion-movil`): la edición in situ de Fase 2 está acotada por la cerca de la spec `edit-surface`; el compañero móvil de Fase 3 (`companero-movil`, meltemi.md §10) está acotado a monitorear/aprobar/dirigir y al acceso solo por túnel SSH por la spec `mobile-companion`.

## Fase 3 — se planifica al cerrar Fase 2

`companero-movil` (gobernada por la spec `mobile-companion`: monitorear/aprobar/dirigir, sin edición, túnel SSH exclusivamente; su mecanismo de notificaciones es pregunta abierta declarada) · funciones de equipo y organización (meltemi.md §3).

## Namespaces del proyecto

- **Organización** (identidad paraguas): `askenaz-dev` — [GitHub](https://github.com/askenaz-dev), [npm](https://www.npmjs.com/org/askenaz-dev). Paquetes npm futuros (SDK, fase 2): `@askenaz-dev/<pkg>`.
- **Producto**: `meltemi` — dominio `meltemi.dev`, binarios `meltemid`/`meltemi`, crates `meltemi`/`meltemid`/`meltemi-proto` (crates.io es namespace plano, sin org).
- Repositorio: `github.com/askenaz-dev/meltemi`.

## Pendientes exclusivos del arquitecto (ningún agente debe hacerlos)

1. ✅ **Ratificar** meltemi.md, `constitution.md` y `rumbo/` — hecho 2026-07-11.
1b. ⬜ **Ratificar las enmiendas encadenadas de meltemi.md**: v1.1→v1.2 (`formato-artefactos-meltemi`) y v1.2→v1.3 (`enmienda-edicion-movil`). Las tres versiones están aplicadas al documento; falta la firma del mantenedor fundador (`method-bootstrap`: la herramienta no se auto-ratifica).
2. **Registrar**: ✅ org GitHub `askenaz-dev` · ✅ org npm `askenaz-dev` · 🟡 dominio `meltemi.dev` (en compra) · ⬜ crates `meltemi`, `meltemid`, `meltemi-proto` en crates.io (verificados LIBRES el 2026-07-11 — reservar con un `cargo publish` placeholder o en el primer release para evitar squatting).
3. **Decidir** el mecanismo de firma del CLA cuando llegue #21.
