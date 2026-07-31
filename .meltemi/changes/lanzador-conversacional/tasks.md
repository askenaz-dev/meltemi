# Tareas — lanzador-conversacional

Orden: el contrato y el daemon aterrizan antes que las superficies que los
consumen. Un commit atómico por tarea, con referencia `(lanzador-conversacional
N.M)` y sin trailers de co-autoría. Gates de repo que aplican a toda tarea de
Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`. Gates de la GUI: `npm run check`, `npm run
lint:i18n`, `npm run check:forms`, `npm run build` (todos con `--prefix
desktop/ui`).

## 1. Contrato: métodos, tipos y esquemas

- [x] 1.1 Declarar `session/start`, `project/register` y `project/forget` en `pub mod methods` de `proto/meltemi-proto/src/lib.rs`, con sus tipos `Params`/`Result` en `camelCase` (`SessionStartParams {projectRoot, instruction, agent?}` → `SessionStartResult {sessionId, agentCommand, status, deniedPermissions, checkpointRef?}`; `ProjectRegisterParams {root}` → `{project}`; `ProjectForgetParams {root}` → `{forgotten}`) — gates: `cargo fmt --check`, `cargo clippy -- -D warnings`
- [ ] 1.2 Añadir el campo opcional `agent` a `ProposeParams` y `SddExploreParams` con `#[serde(default, skip_serializing_if = "Option::is_none")]`, sin tocar ningún campo existente — gate: `cargo test -p meltemi-proto`
- [ ] 1.3 Añadir `candidates: Option<Vec<AgentCandidate>>` a `ErrorData` (`skip_serializing_if`), con `AgentCandidate {id, detected, installState, remedy?, remedyCommand?}` reusando el vocabulario de `FleetAgent` — gate: `cargo test -p meltemi-proto`
- [ ] 1.4 Escribir `proto/schemas/v1/session-start.schema.json` y `proto/schemas/v1/project-registry.schema.json` (este último con `title` que reclame los dos métodos y `$defs` nombrados `projectRegisterParams`/`projectForgetParams`, porque el atajo `params` solo aplica a esquemas de un método), y declarar `candidates` en `$defs.errorData` de `error.schema.json` sin añadirlo a `required` — gate: `cargo test -p meltemi-proto`
- [ ] 1.5 Añadir los casos de conformidad en `proto/meltemi-proto/tests/conformance.rs` para los tres métodos, el `agent` aditivo y el error con candidatos, con sus negativos (`assert_rejected`) — gate: `cargo test -p meltemi-proto`
- [ ] 1.6 Añadir el constructor hermano de `RpcError::application` en `core/meltemi-client/src/rpc.rs` que acepta candidatos, dejando la firma existente intacta para no mover ningún sitio de llamada — gate: `cargo clippy -- -D warnings`, `cargo test --workspace`

## 2. Daemon: arranque de sesión libre

- [ ] 2.1 Implementar `handle_session_start` en `core/meltemid/src/server.rs` calcando la secuencia de `propose.rs` sin el andamiaje: validar raíz (3002), resolver agente, acuñar el id, `sessions.register`, `projects::touch`, `SessionLog::create` + `SessionStarted`, `enable_direction`, `set_state(Active)`, registro START en el índice, `load_rules`, expansión de `@`, `edits.enter`, `run_session` — gates: `cargo clippy -- -D warnings`, `cargo test -p meltemid`
- [ ] 2.2 Cerrar la sesión libre por `session_finalize` (`finalize_ok`/`finalize_err` con `SessionContext`), con el test que fija el invariante: una sesión libre completada recibe registro de fin y **no** lista como interrumpida — gate: `cargo test -p meltemid`
- [ ] 2.3 Crear el punto de restauración al arrancar con `checkpoints::create` sobre la raíz y la tripleta reservada `(free, <session-id>, <agent>)`, devolver su ref en el resultado y registrarlo como evento; verificar que no mueve ninguna rama del usuario ni altera su índice — gate: `cargo test -p meltemid`
- [ ] 2.4 Declarar honestamente los dos casos sin punto de restauración: raíz que no es repo git (remedio `git init`) y repo git todavía sin commits (remedio: el primer commit, nunca `git init`); en ambos la sesión arranca y el resultado no trae ref, y el daemon nunca rehúsa el arranque por esta causa — gate: `cargo test -p meltemid`
- [ ] 2.4b Cerrar la puerta que el checkpoint libre abre en un verbo existente: `checkpoint/revert` MUST rehusar todo checkpoint cuyo `worktree` registrado no sea un worktree gestionado (`worktrees::is_managed`), con diagnóstico y remedio de restaurar desde git, y las superficies MUST NOT ofrecer el control para ellos; con el test que fija que revertir un checkpoint de sesión libre deja intacto el árbol del usuario, incluidos sus archivos no rastreados, y que revertir el de un worktree gestionado sigue funcionando — gates: `cargo clippy -- -D warnings`, `cargo test -p meltemid`
- [ ] 2.5 Publicar al hub de eventos todo el conjunto que el log persiste, moviendo la publicación al punto de escritura del log para que no exista más de un lugar que publique y `agent_update` no se duplique — gates: `cargo clippy -- -D warnings`, `cargo test -p meltemid`
- [ ] 2.6 Test e2e de workspace contra `mock-agent` en un repo fixture temporal (nunca la raíz de este repo, nunca red): arranque gobernado, instrucción de seguimiento encolada y despachada como siguiente turno, permiso denegado sin cliente, cierre correcto en el índice, y el iniciador recibiendo `session_started` sin declarar interés — gate: `cargo test --workspace`
- [ ] 2.7 Tests e2e de los fixtures sin punto de restauración (sin git, y con git sin commits): arrancan, declaran que no hay punto de restauración con el remedio que corresponde a cada causa, y no crean worktrees ni competidores — gate: `cargo test --workspace`

## 3. Daemon: registro de proyectos — alta y olvido

- [ ] 3.1 Implementar `handle_project_register` en `core/meltemid/src/projects.rs`: validación de directorio existente (3002 con remedio), canonicalización antes de derivar la clave, alta idempotente que conserva `firstSeenAt`, sin crear nada en disco ni recorrer nada — gate: `cargo test -p meltemid`
- [ ] 3.2 Implementar la línea de olvido y su plegado: lápida en el JSONL apend-only siguiendo el precedente de `worktrees::list`, resolución por clave cuando la ruta canonicaliza y por comparación normalizada cuando no, sin exigir que la raíz exista — gate: `cargo test -p meltemid`
- [ ] 3.3 Cerrar la trampa del rebuild: distinguir «no hay ningún registro parseable» de «todo lo visible fue olvidado», de modo que `rebuild_from_sessions` solo dispare en el primer caso, con el test que lo fija — gate: `cargo test -p meltemid`
- [ ] 3.4 Batería de plegado sobre el JSONL: alta repetida en dos formas equivalentes, olvido de raíz ausente, reaparición por uso, línea corrupta que no oculta al resto, y la invariante de solo-lectura de `project/list` — gate: `cargo test -p meltemid`
- [ ] 3.5 Despachar los dos métodos en `dispatch_request` y documentar en el módulo que el olvido rige sobre el listado y jamás sobre el disco ni sobre la historia — gate: `cargo test --workspace`

## 4. Daemon: agente aditivo y error de resolución estructurado

- [ ] 4.1 Aceptar el parámetro `agent` en `propose` y `sdd/explore`, resolviéndolo por `resolve_fleet_agent` en vez de `resolve_launch`, con el comportamiento sin parámetro idéntico al vigente — gate: `cargo test -p meltemid`
- [ ] 4.2 Apendar `AgentResolved` en `propose`, en los turnos de autoría de `sdd_flow` y en el arranque libre, para que una reconstrucción desde el log recupere agente y perfil (hoy solo lo escriben dispatch e implement) — gate: `cargo test -p meltemid`
- [ ] 4.3 Sustituir la prosa cruda de `levels.rs` por el error estructurado: 2000 y 2001 con `candidates` derivados de `fleet::detect_layers` + `fleet::compose_state`, un solo camino de detección compartido con `fleet/list` — gate: `cargo test -p meltemid`
- [ ] 4.4 Test de higiene §2: ningún valor de entorno, ruta de credencial ni cadena con forma de secreto aparece jamás en el payload del error — gate: `cargo test -p meltemid`

## 5. Superficies de terminal: CLI y TUI

- [ ] 5.1 Añadir a `tui/src/cli.rs` los subcomandos `session <instruction> [project-root]`, `projects register <path>` y `projects forget <path>`, más `--agent` en la lista de flags que el parser global deja pasar al subcomando; mapearlos en `tui/src/run.rs` — gate: `cargo test -p meltemi`
- [ ] 5.2 Regenerar `docs/referencia-cli.md` (`cargo run --example gen_cli_ref`) y verificar el gate de frescura — gate: `cargo test -p meltemi` (`tui/tests/docs.rs`)
- [ ] 5.3 Registrar los tres métodos en `tui/src/shell/palette.rs`, cada uno declarado por exactamente una `Entry`, y quitar `reserved: true` del verbo `direct` — gate: `cargo test -p meltemi` (`parity.rs`, unicidad de métodos)
- [ ] 5.4 Cablear `direct` interactivo: brazo en `Action::Submit` de `state.rs`, variante de `Effect`, despacho en `mod.rs` y operación async en `conn.rs`, con un overlay de entrada que **preserve el texto tal cual** (la paleta hace `to_ascii_lowercase` sobre su línea) — gate: `cargo test -p meltemi`
- [ ] 5.5 Estados honestos en la TUI: encolada con posición, reanudación, y diagnóstico con remedio cuando la sesión no admite dirección — gate: `cargo test -p meltemi`
- [ ] 5.6 Formularios de alta y baja de proyecto en la paleta y render del registro en la vista de proyectos, con la ruta tecleada por un overlay de entrada que la preserva tal cual y que se resuelve **antes** que el brazo `projects <texto>` de `state.rs` (hoy fija el filtro de ámbito, y se tragaría el discriminador); escenarios de shell en el estilo vigente — gate: `cargo test -p meltemi`
- [ ] 5.7 Añadir las filas de los tres métodos a `docs/paridad-nucleo.md` — gate: `cargo test -p meltemi` (`the_parity_matrix_documents_every_method`)

## 6. GUI: home conversacional

- [ ] 6.1 Registrar los tres métodos en `desktop/ui/src/lib/registry.ts` con el helper literal `R("...")` y sus claves `palette.m.*` en **ambos** catálogos de `messages.ts`, una clave por línea; regenerar `method-forms.ts` y comprobar en la cabecera generada que ningún método degradó a JSON crudo — gates: `npm run check`, `npm run check:forms`, `cargo test -p meltemi` (`parity.rs`)
- [ ] 6.2 Construir el compositor conversacional como vista de llegada: chips de proyecto, agente/perfil y modo, con modo libre por defecto y el método declarado de forma visible antes de enviar — gates: `npm run check`, `npm run lint:i18n`
- [ ] 6.3 Enviar navega hacia adentro: consumir el `session_started` del stream para obtener el identificador y enrutar a la conversación sin esperar al fin del turno — gate: `npm run check`
- [ ] 6.4 Rutear al compositor todos los puntos de entrada vigentes (acción primaria del chrome, atajo, estados vacíos de Sesiones y Proyecto, «Propose» de la vista Proyecto) y retirar `NewSession.svelte` con su estado en `App.svelte` — gates: `npm run check`, `npm run build`
- [ ] 6.5 Presentar el error de resolución estructurado como elección de agente entre los candidatos detectados, en vez de transcribir el diagnóstico — gates: `npm run check`, `npm run lint:i18n`

## 7. GUI: vista de conversación

- [ ] 7.1 Reescribir la plantilla de filas de `SessionDetail.svelte` antes de añadir nada: hoy es `grid-template-rows: auto auto 1fr` con dos hijos en flujo, y un cuarto hijo caería en la pista `1fr` comiéndose el panel — gate: `npm run check`
- [ ] 7.2 Compositor persistente que envía por `session/direct` con la sesión fijada, con sus estados honestos: encolada con posición, reanudar cuando terminó y es reanudable, diagnóstico y remedio cuando no admite dirección, y cancelar como control aparte — gates: `npm run check`, `npm run lint:i18n`
- [ ] 7.3 Implementar el plegado de burbujas según D4, incluidas las tres formas de `agent_update` (ACP nivel 1, línea mapeada de nivel 3, cadena cruda), con el pensamiento plegado y separado de la prosa — gate: `npm run check`
- [ ] 7.4 Conmutador a log de operador con el invariante verificable: el conteo de eventos del log iguala al de eventos recibidos, y conmutar no pierde posición ni descarta nada; los eventos no clasificables se renderizan en su lugar como línea neutra — gate: `npm run check`
- [ ] 7.5 Tarjetas de permiso en línea, decididas por los métodos vigentes, no accionables cuando la petición ya no está pendiente; la bandeja sigue siendo la vista completa — gates: `npm run check`, `npm run lint:i18n`
- [ ] 7.6 Dar glifo y tono a `usage_reported` en `EVENT_STYLE` (el proto declara 20 tipos y el mapa cubre 19) — gate: `npm run check`

## 8. GUI: Proyectos en la navegación y diálogo nativo

- [ ] 8.1 Sección «Proyectos» persistente en `Sidebar.svelte` con el árbol de proyectos y sus sesiones siempre visible, sin animación de layout bajo el cursor — gates: `npm run check`, `npm run lint:i18n`
- [ ] 8.2 Acción rápida por proyecto que lleva al compositor con el proyecto prefijado, y conmutación de ámbito desde el nodo — gate: `npm run check`
- [ ] 8.3 Añadir `tauri-plugin-dialog` pineada exacta en `[workspace.dependencies]` y al crate `desktop`, inicializarla en el builder, y exponerla **solo** como comando propio `pick_project_folder` en `generate_handler!` — `capabilities/default.json` sigue siendo `["core:default"]` y la CSP no se toca — gates: `cargo deny check`, `cargo test -p meltemi-desktop` (`surface.rs`), `cargo clippy -- -D warnings`
- [ ] 8.4 «Abrir carpeta…» en el nav y en el chip de proyecto del compositor: el diálogo del cliente devuelve la ruta, la superficie la da de alta por `project/register` antes de lanzar nada — gates: `npm run check`, `npm run lint:i18n`
- [ ] 8.5 Baja de proyecto desde la superficie con su texto honesto —oculta del listado, no borra nada, reaparece al volver a usarse— y estado ausente con remedio — gates: `npm run check`, `npm run lint:i18n`

## 9. Fundacionales, documentación y verificación

- [ ] 9.1 Aplicar la enmienda de `.meltemi/rumbo/product.md` y de la tesis de `meltemi.md` con el texto exacto del design D5, **previa ratificación del mantenedor**; sin ratificación la change no archiva — gate: revisión humana registrada
- [ ] 9.2 Regenerar el contexto proyectado (`meltemi project`) para que el bloque gestionado de `AGENTS.md`, `CLAUDE.md` y `GEMINI.md` refleje el rumbo enmendado — gate: `cargo test --workspace`
- [ ] 9.3 Escribir la guía de la sesión libre en `docs/`: qué gobierna una sesión sin spec, dónde opera, qué es el punto de restauración y qué no es (no hay reversión guiada en esta change), y cómo se pasa al método desde el mismo compositor — gate: revisión de docs
- [ ] 9.4 Cerrar la entrada de `docs/plan-de-cambios.md`: el supersede de `sesion-conversacional` y `gestion-proyectos-en-superficie` **ya está escrito** en la entrada abierta el 2026-07-27, así que esta tarea solo registra el desenlace —nombres finales de los verbos CLI, el guardián de reversión y la ratificación de la enmienda— y mueve la change a su estado de cierre — gate: revisión de docs
- [ ] 9.5 Ejecutar el smoke visual conducido por CDP sobre el binario real y publicar el informe en `docs/qa/<fecha>-lanzador-conversacional-smoke.md`: home enfocado, envío que navega, estados del compositor, conmutador conservando el conteo, tarjeta de permiso en línea, sidebar con proyecto registrado y olvidado, diálogo nativo — gate: informe publicado con medidas
- [ ] 9.6 `meltemi validate lanzador-conversacional` limpio y `meltemi verify lanzador-conversacional` con todos los escenarios `linked` o `manual` con nota — gates: `meltemi validate` (salida 0), `meltemi verify`
