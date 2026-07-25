# Plan maestro de cambios — de cero al MVP (hito v0.1)

> Backlog ordenado de propuestas de cambio que llevan a Meltemi desde el repositorio vacío hasta el hito v0.1 de Fase 1 (meltemi.md §10). Este documento es el mapa; **cada change escribe sus cuatro artefactos (proposal, design, specs, tasks) inmediatamente antes de implementarse** — nunca durante, y nunca se implementa nada que no esté aquí o en una change aprobada. Producto de la auditoría de planificación del 2026-07-11 (5 lentes, 64 hallazgos).
>
> **Cierre del bootstrap (2026-07-18)**: la etapa OpenSpec queda **cerrada**. El motor de specs de fase 1 hospeda la verdad viva del proyecto en `.meltemi/specs/` y el histórico en `.meltemi/changes/archive/` (migrados con verificación del motor, `migracion-openspec-a-meltemi`); el método del proyecto son los comandos de Meltemi sobre `.meltemi/`. El árbol `openspec/` se conserva como histórico consultable hasta el retiro físico confirmado por el mantenedor (D3 de la migración).
>
> **Desviación aprobada por el mantenedor (2026-07-16, ampliada el mismo día)**: el backlog está **completamente materializado** — las 19 changes restantes existen en `openspec/changes/` con sus **cuatro artefactos** (proposal, design, specs EARS, tasks), listas para `/opsx:apply` en el orden declarado. Disciplina que lo hace sostenible: (1) los **deltas de `cli-contract` son acumulativos** — cada uno asume archivadas las changes anteriores del orden; alterar el orden exige revisar esos deltas; (2) los designs de las changes profundas marcan sus decisiones como **provisionales** y se **revalidan obligatoriamente al llegar al frente de la cola** (en especial `ciclo-sdd-autoria`, `orquestacion-worktrees` y `comando-implement`, la change de integración); (3) los hallazgos de QA y la deuda registrada están absorbidos en las specs correspondientes (H1/H4/H5 → `proxy-permisos`; H6 → `documentacion-inicial`).

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
| **Registros operativos** (org GitHub, org npm, dominio, crates) | 🟡 org GitHub y npm hechas (`askenaz-dev`); dominio `meltemi.dev` comprado, DNS por apuntar a Pages; crates por reservar |

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

## Orden de abordaje declarado (2026-07-16)

Secuencia canónica de las 19 changes restantes (todas con proposal pre-redactada). Satisface el grafo de dependencias y prioriza: completar la capa de agentes con gobierno de permisos, luego el contexto, luego el ciclo SDD, luego el paralelismo, y al final la salida al mundo. Nada queda por fuera.

| Orden | Change | Por qué aquí |
|---|---|---|
| 1º | `catalogo-flota` | Desbloquea #9/#10/#13; sin flota no hay orquesta |
| 2º | `proxy-permisos` | Agentes reales exigen gobierno real; corrige H1 (QA) y la deuda del contador de pendientes |
| 3º | `proyeccion-contexto` | Cierra la proyección manual de AGENTS.md; vehículo del nivel 4; independiente |
| 4º | `niveles-integracion-conformidad` | Cubre el mercado completo (niveles 2/3/4), con la proyección ya disponible |
| 5º | `mcp-passthrough` | Los agentes llegan con sus herramientas; capa de agentes completa |
| 6º | `sesiones-reanudables` | Robustez e histórico; puerta del paralelismo (#16) |
| 7º | `gestion-contexto-repo` | Mapa y `@refs` para dirigir agentes antes de la autoría pesada |
| 8º | `ciclo-sdd-autoria` | El corazón del método — llega con flota, permisos y contexto ya operativos |
| 9º | `revision-specs-ux` | La obsesión (§4.9): sin revisión placentera, el método muere |
| 10º | `orquestacion-worktrees` | N agentes × M tareas sin pisarse; carreras |
| 11º | `checkpoints-rollback` | Autonomía solo con deshacer barato y honesto |
| 12º | `git-commit-por-tarea` | Trazabilidad línea→requisito (constitución §8) |
| 13º | `comandos-verify-archive` | Cierra los verbos del ciclo; el motor ya demostró la fusión |
| 14º | `comando-implement` | La composición de todo (requiere 2º, 8º, 10º, 11º, 12º ya hechas) |
| 15º | `gobernanza-comunidad` | Antes de abrir el repositorio al mundo |
| 16º | `documentacion-inicial` | El quickstart ya describe un producto real, no una promesa |
| 17º | `distribucion-releases` | Empaquetado, firmado, instalador; materializa reserva de crates y alias `mel` |
| 18º | `migracion-openspec-a-meltemi` | El dogfooding definitivo: se retira la herramienta prestada |
| 19º | `hito-v01-aceptacion` | El hito como spec ejecutable; la meta de v0.1 |

> Paralelizable sin romper el orden: 15º–16º (documentos, sin código) pueden avanzar en cualquier momento por una sesión aparte; 3º puede adelantarse en paralelo tras 1º. El orden canónico es el de la tabla; toda desviación se anota aquí.

## Documentos transversales (se crean durante Fase 1, referenciados por las changes)

- `docs/plataformas.md` — matriz de soporte: SO mínimos, arquitecturas, webviews (antes de #23).
- `docs/presupuestos-rendimiento.md` — los números de §12 traducidos a criterios EARS y gates de CI (antes de #6).
- `docs/paridad-nucleo.md` — matriz viva capacidad → RPC → TUI → GUI; toda change que añada un RPC la actualiza (desde #5).
- `docs/operaciones/checklist-lanzamiento.md` — registros y activos (ver pendientes del arquitecto).

## Fase 2 (v1.0) — abierta el 2026-07-20

**Archivadas el 2026-07-25** (183 escenarios verificados, 64 requisitos
revisados uno por uno, deltas plegados a la verdad viva):

| Change | Qué dejó en la verdad viva |
|---|---|
| `gui-tauri-paridad` | `desktop/` Tauri 2 + Svelte 5, crate compartido `core/meltemi-client`, matriz de paridad como gate de CI, política de concurrencia humano↔agente (`worktree/apply-edit` + `human_edit` + nota al siguiente turno), LSP BYO, instaladores con gate < 15 MB; creó `docs/ux/design-system.md` y `docs/paridad-nucleo.md` |
| `gui-clase-mundial` | Shell de tres zonas con la densidad del design system, identidad de entidades, drawer, superficie de Ajustes, paleta con difusa/grupos/formularios tipados, la sesión como acción primaria, guarda de trabajo sin guardar |
| `flota-deteccion-guia` | Detección en dos capas (CLI oficial + adaptador) con estado compuesto, remedio por capa con su comando exacto, estatus legal sin maquillaje y `docs/agentes.md` verificada contra el registro |
| `multiproyecto-suscripciones` | Registro de proyectos (`project/list`), agente y suscripción en los metadatos de sesión, árbol Proyecto→Sesiones en la GUI, agrupación y ámbito conmutable en la TUI |
| `analitica-consumo-local` | `analytics/usage`: actividad plegada de los registros locales, tokens solo donde la salida oficial los reporta, frontera medido/no-reportado con motivo estable y declaración de honestidad junto a las cifras |
| `sitio-web-producto` | `site/` estático (sin JS, sin orígenes externos), descargas a la última release firmada con nombres estables, tokens derivados del cliente y lint del sitio como gate |

Pendientes de Fase 2: `motor-propio-byok` (propuesta redactada, activa) ·
`sandbox-propio` · `hooks-eventos` · `plugins-skills-sdk` ·
`i18n-superficies` · `metricas-sdd-locales` · `lsp-superficie-revision`.

### `motor-propio-byok` — propuesta activa desde el 2026-07-25

Nace de tres peticiones del mantenedor: una vía para **modelos autohospedados**
(ollama y cualquier endpoint OpenAI-compatible), el **harness como concepto de
primera clase con un default**, y una decisión sobre su proyecto **Forge
Harnesses**. La forma la decidió meltemi.md D6: el motor propio entra a la flota
como un agente ACP de nivel 1 (`core/meltemi-engine`), pilotado por stdio igual
que cualquier agente externo — misma detección, mismo proxy de permisos, mismos
worktrees y checkpoints, **jamás un canal privilegiado**. Todo el tráfico de red
vive en el subproceso: `meltemid` no enlaza pila HTTP/TLS alguna, propiedad
auditable que sostiene §3.

Un harness es un manifiesto TOML v1 (dialecto, `base-url`, modelo, prompt,
política de herramientas, límites) que el daemon **valida y lista, nunca
interpreta**; el default va embebido apuntando a `http://localhost:11434/v1`,
el único que no privilegia a proveedor comercial alguno. Sin modelo alcanzable
rehusa con remedio; nunca degrada en silencio. Las claves BYOK entran solo por
referencia `${VAR}`, con el lint de higiene existente.

**Decisión sobre Forge Harnesses: se mantiene como proyecto separado**,
conectado por contrato — este repositorio publica el esquema versionado del
manifiesto (JSON Schema + fixtures de conformidad) y Forge produce manifiestos
que se prueban contra ellos. Absorberlo sometería un laboratorio de iteración de
prompts a la constitución entera (spec-first por cada ajuste, clippy en 3 SO,
Apache-2.0 + CLA desde el día uno), no compraría ni un test de integración en CI
(que por regla no ejecuta agentes reales ni red) y arrastraría a Meltemi hacia el
marketplace que su rumbo explícitamente no es. Si alguna vez se absorbe, entra
Apache-2.0 bajo CLA, sin excepciones.

Mientras la change no se implemente, correr un modelo local bajo Meltemi ya es
posible **hoy y sin código nuevo**: OpenCode (nivel 1) y Aider (nivel 3) de la
flota actual hablan con endpoints ollama/OpenAI-compatible por su propia
configuración. La guía de docs que lo explique entra con la change, verificada
contra las versiones vigentes y no citada de memoria.

El design system del mantenedor vive en `design-system/` y es la fuente visual
normativa (gui-clase-mundial D11).

### `instaladores-linux-sin-webview` — activa desde el 2026-07-25

El gate de tamaño disparó en Linux la primera vez que llegó a apuntar ahí: el
AppImage pesa 78 678 520 B contra 15 MB. El número no se ensancha porque no es un
número: el design D7 de `gui-tauri-paridad` dejó escrito que codifica «no
empaquetamos motor de navegador», y meltemi.md §7 rechaza esa vía por su nombre.
La change retira el AppImage, declara `libwebkit2gtk-4.1-0` y `libgtk-3-0` en el
`.deb` —que hoy instala limpio y no arranca— y corrige la prosa pública en los
dos idiomas. El hueco queda nombrado: fuera de la familia Debian no hay
instalador gráfico hasta que exista un `.rpm`, que es change propia porque exige
verificar los nombres de paquete en Fedora/RHEL en vez de adivinarlos.

**Deuda declarada al archivar** (no se archivó nada fingiendo que estaba
hecho): ✅ la captura de escritorio del sitio está publicada y su procedimiento
es un script (`scripts/capture-desktop.ps1`, `docs/ux/capturas.md`); la firma de MSI/DMG sigue pendiente
de infraestructura de certificados; el arranque y la RAM en macOS y Linux se
publican en el QA de la primera release que incluya la GUI.

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
2. **Registrar**: ✅ org GitHub `askenaz-dev` · ✅ org npm `askenaz-dev` · 🟡 dominio `meltemi.dev` (comprado el 2026-07-25; falta apuntar el DNS a Pages) · ⬜ crates `meltemi`, `meltemid`, `meltemi-proto` en crates.io (verificados LIBRES el 2026-07-11 — reservar con un `cargo publish` placeholder o en el primer release para evitar squatting).
3. **Decidir** el mecanismo de firma del CLA cuando llegue #21.
