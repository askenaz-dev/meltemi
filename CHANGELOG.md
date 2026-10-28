# Changelog

Every release lists the changes it archives: a change enters the first
release written after it was archived, and its entry links its proposal in
`.meltemi/changes/archive/`. Each line is the one its proposal opens its
changes with, in Spanish, the language of the method (constitution §11).
Written by `cargo run -p meltemi-release -- changelog <version>`; the section
of a version is the release's notes (`meltemi-release notes <version>`).

## 0.2.0 — 2026-10-23

- [`pulido-pre-anuncio`](.meltemi/changes/archive/2026-07-27-pulido-pre-anuncio/proposal.md) — `desktop/ui/src/app.css`: la regla global `button` gana `display: inline-flex; align-items: center; gap: var(--sp-2)` — los siete botones rotos sanan en la raíz y todo botón futuro con icono y etiqueta nace correcto.
- [`texto-intacto-al-agente`](.meltemi/changes/archive/2026-08-02-texto-intacto-al-agente/proposal.md) — `expand_refs` deja de caminar el texto carácter a carácter.
- [`adaptadores-propios-acp`](.meltemi/changes/archive/2026-08-06-adaptadores-propios-acp/proposal.md) — Crate nuevo `core/meltemi-adapters`
- [`tablero-de-carrera`](.meltemi/changes/archive/2026-08-08-tablero-de-carrera/proposal.md) — Procedencia por calle en el contrato (aditivo, sin verbo nuevo)
- [`cromo-que-no-estorba`](.meltemi/changes/archive/2026-08-09-cromo-que-no-estorba/proposal.md) — El cajón parte su contenido en vez de desplazarlo de lado
- [`flota-por-suscripcion`](.meltemi/changes/archive/2026-08-09-flota-por-suscripcion/proposal.md) — La tabla se lee por agente y por suscripción.
- [`panel-opaco-y-nav-plegable`](.meltemi/changes/archive/2026-08-09-panel-opaco-y-nav-plegable/proposal.md) — El fondo del panel vuelve a existir
- [`pestanas-como-chrome`](.meltemi/changes/archive/2026-08-09-pestanas-como-chrome/proposal.md) — Una sola fila, siempre.
- [`sidebar-ajustable-y-pestanas`](.meltemi/changes/archive/2026-08-09-sidebar-ajustable-y-pestanas/proposal.md) — La línea se vuelve un control.
- [`vincular-suscripciones`](.meltemi/changes/archive/2026-08-09-vincular-suscripciones/proposal.md) — La variable de contexto como dato del registro
- [`acceso-remoto-en-dos-vias`](.meltemi/changes/archive/2026-08-16-acceso-remoto-en-dos-vias/proposal.md) — Verbo nuevo del cliente: `meltemi bridge`.
- [`salida-que-se-lee`](.meltemi/changes/archive/2026-08-16-salida-que-se-lee/proposal.md) — Un resumen encabeza cada listado
- [`rama-por-change`](.meltemi/changes/archive/2026-08-18-rama-por-change/proposal.md) — `change/workspace`
- [`modelo-y-esfuerzo-por-sesion`](.meltemi/changes/archive/2026-08-19-modelo-y-esfuerzo-por-sesion/proposal.md) — Contrato
- [`familia-por-titulo`](.meltemi/changes/archive/2026-09-16-familia-por-titulo/proposal.md) — `workspace` entra a la lista
- [`avisos-de-escritorio`](.meltemi/changes/archive/2026-09-26-avisos-de-escritorio/proposal.md) — Capability nueva `attention-notices`
- [`identidad-propia`](.meltemi/changes/archive/2026-10-02-identidad-propia/proposal.md) — `identity/status`
- [`artefactos-de-cada-push`](.meltemi/changes/archive/2026-10-23-artefactos-de-cada-push/proposal.md) — Workflow nuevo `.github/workflows/build.yml`
- [`grabador-de-avisos-sin-carrera`](.meltemi/changes/archive/2026-10-23-grabador-de-avisos-sin-carrera/proposal.md) — El `append.cmd` que el test escribe en Windows toma un candado antes de añadir su línea: crea un directorio con `mkdir`, que se crea o falla de forma atómica, y reintenta hasta conseguirlo; añade la línea y borra el directorio.
- [`procedencia-de-release`](.meltemi/changes/archive/2026-10-23-procedencia-de-release/proposal.md) — Atestación de build sobre el `SHA256SUMS` publicado.

## 0.1.0 — 2026-07-26

- [`enmiendas-fundacionales-v1`](.meltemi/changes/archive/2026-07-12-enmiendas-fundacionales-v1/proposal.md) — Bootstrap del método (reconcilia §9.3)
- [`fase-0-fundacion`](.meltemi/changes/archive/2026-07-12-fase-0-fundacion/proposal.md) — `constitution.md`
- [`formato-artefactos-meltemi`](.meltemi/changes/archive/2026-07-13-formato-artefactos-meltemi/proposal.md) — Definir el formato canónico de `.meltemi/`
- [`motor-specs-artefactos`](.meltemi/changes/archive/2026-07-13-motor-specs-artefactos/proposal.md) — Nuevo crate `core/meltemi-spec`
- [`cli-contrato`](.meltemi/changes/archive/2026-07-14-cli-contrato/proposal.md) — Se introduce el binario único `meltemi` (alias `mel`) en el crate `tui/`, que funciona como **CLI scriptable** en esta change; la TUI interactiva llega en #6, pero la **regla de despacho** que decide entre ambos modos se fija aquí.
- [`motor-ears-deltas`](.meltemi/changes/archive/2026-07-14-motor-ears-deltas/proposal.md) — Validación EARS
- [`enmienda-edicion-movil`](.meltemi/changes/archive/2026-07-15-enmienda-edicion-movil/proposal.md) — No-objetivo #1 (`meltemi.md` §3)
- [`tui-nucleo-ux`](.meltemi/changes/archive/2026-07-16-tui-nucleo-ux/proposal.md) — Se implementa el **shell interactivo** de `meltemi` en el crate `tui/` existente: un **chrome persistente** (cabecera + pie siempre visibles) que enmarca **cuatro vistas de primer nivel** (Sesiones, Proyecto, Permisos, Flota), un solo nivel…
- [`catalogo-flota`](.meltemi/changes/archive/2026-07-17-catalogo-flota/proposal.md) — Catálogo de flota en el daemon
- [`checkpoints-rollback`](.meltemi/changes/archive/2026-07-18-checkpoints-rollback/proposal.md) — Checkpoint automático pre-tarea
- [`ciclo-sdd-autoria`](.meltemi/changes/archive/2026-07-18-ciclo-sdd-autoria/proposal.md) — `/constitution`
- [`comando-implement`](.meltemi/changes/archive/2026-07-18-comando-implement/proposal.md) — `/implement`
- [`comandos-verify-archive`](.meltemi/changes/archive/2026-07-18-comandos-verify-archive/proposal.md) — `/verify`
- [`distribucion-releases`](.meltemi/changes/archive/2026-07-18-distribucion-releases/proposal.md) — Versionado
- [`documentacion-inicial`](.meltemi/changes/archive/2026-07-18-documentacion-inicial/proposal.md) — README raíz
- [`gestion-contexto-repo`](.meltemi/changes/archive/2026-07-18-gestion-contexto-repo/proposal.md) — Mapa del repositorio
- [`git-commit-por-tarea`](.meltemi/changes/archive/2026-07-18-git-commit-por-tarea/proposal.md) — Commit por tarea
- [`gobernanza-comunidad`](.meltemi/changes/archive/2026-07-18-gobernanza-comunidad/proposal.md) — GOVERNANCE.md
- [`hito-v01-aceptacion`](.meltemi/changes/archive/2026-07-18-hito-v01-aceptacion/proposal.md) — El escenario del hito como spec EARS
- [`mcp-passthrough`](.meltemi/changes/archive/2026-07-18-mcp-passthrough/proposal.md) — Declaración por proyecto
- [`migracion-openspec-a-meltemi`](.meltemi/changes/archive/2026-07-18-migracion-openspec-a-meltemi/proposal.md) — Migración de la verdad viva
- [`niveles-integracion-conformidad`](.meltemi/changes/archive/2026-07-18-niveles-integracion-conformidad/proposal.md) — Nivel 2 — adaptadores
- [`orquestacion-worktrees`](.meltemi/changes/archive/2026-07-18-orquestacion-worktrees/proposal.md) — Worktrees aislados por tarea/sesión
- [`proxy-permisos`](.meltemi/changes/archive/2026-07-18-proxy-permisos/proposal.md) — Motor de reglas
- [`proyeccion-contexto`](.meltemi/changes/archive/2026-07-18-proyeccion-contexto/proposal.md) — Compilador de proyección
- [`revision-specs-ux`](.meltemi/changes/archive/2026-07-18-revision-specs-ux/proposal.md) — Render de diff de deltas
- [`sesiones-reanudables`](.meltemi/changes/archive/2026-07-18-sesiones-reanudables/proposal.md) — Persistencia de metadatos de sesión
- [`control-remoto-asistido`](.meltemi/changes/archive/2026-07-19-control-remoto-asistido/proposal.md) — `session/direct`
- [`flota-multiproveedor`](.meltemi/changes/archive/2026-07-19-flota-multiproveedor/proposal.md) — Resolución de agente por sesión
- [`navegacion-del-metodo`](.meltemi/changes/archive/2026-07-19-navegacion-del-metodo/proposal.md) — Listado de changes
- [`analitica-consumo-local`](.meltemi/changes/archive/2026-07-25-analitica-consumo-local/proposal.md) — Contabilidad local de actividad
- [`flota-deteccion-guia`](.meltemi/changes/archive/2026-07-25-flota-deteccion-guia/proposal.md) — Detección en dos capas para nivel 2
- [`gui-clase-mundial`](.meltemi/changes/archive/2026-07-25-gui-clase-mundial/proposal.md) — El design system del mantenedor es la fuente visual normativa
- [`gui-tauri-paridad`](.meltemi/changes/archive/2026-07-25-gui-tauri-paridad/proposal.md) — Cliente de escritorio `desktop/`
- [`multiproyecto-suscripciones`](.meltemi/changes/archive/2026-07-25-multiproyecto-suscripciones/proposal.md) — Multiproyecto de primera clase
- [`sitio-web-producto`](.meltemi/changes/archive/2026-07-25-sitio-web-producto/proposal.md) — Sitio estático
- [`enmienda-agent-boss`](.meltemi/changes/archive/2026-07-26-enmienda-agent-boss/proposal.md) — `mobile-companion` (spec)
- [`espera-humana`](.meltemi/changes/archive/2026-07-26-espera-humana/proposal.md) — La cola es la única fuente de resolución.
- [`eventos-para-tardios`](.meltemi/changes/archive/2026-07-26-eventos-para-tardios/proposal.md) — El evento deja de escribirse contra una conexión y pasa a publicarse.
- [`gui-acabado-y-cierre-sdd`](.meltemi/changes/archive/2026-07-26-gui-acabado-y-cierre-sdd/proposal.md) — `desktop/ui/src/App.svelte`: la columna central pasa de rejilla de filas fijas a columna flex — cada barra a su altura natural, la vista enrutada se queda con el resto.
- [`instaladores-linux-sin-webview`](.meltemi/changes/archive/2026-07-26-instaladores-linux-sin-webview/proposal.md) — El AppImage deja de publicarse.
- [`sesion-esperando`](.meltemi/changes/archive/2026-07-26-sesion-esperando/proposal.md) — El daemon fija `waiting_permission`
