# Verificación de la Fase 0 — escenario por escenario (tarea 6.3)

Estado de cada escenario de las specs de `fase-0-fundacion` frente a la implementación. Leyenda: **[T]** cubierto por test automatizado · **[C]** garantizado por construcción (el diseño lo hace imposible de violar) · **[M]** verificado por el e2e manual (`docs/e2e-manual.md`) · **[F1]** endurecimiento diferido a Fase 1 con nota.

Suite: `cargo test --workspace` — 42 tests verdes; `cargo clippy --workspace --all-targets -- -D warnings` y `cargo fmt --check` limpios en Windows (2026-07-12).

## daemon-lifecycle

| Requisito · Escenario | Estado | Cobertura |
|---|---|---|
| Arranque bajo demanda · Primer cliente arranca el daemon | [T][M] | `bootstrap::fast_path_connects_without_spawning` (ruta de conexión); arranque real por spawn en el e2e manual |
| Instancia única · Segunda invocación reutiliza la instancia | [T] | `transport::second_bind_fails_while_instance_is_alive`; `run()` sale con `AddrInUse` |
| Socket local · Sin superficie de red | [C] | El transporte solo usa UDS/named pipe; no existe ninguna ruta de código que abra un socket TCP/UDP |
| Socket local · Otro usuario no puede conectarse | [T] | `transport::socket_and_directory_are_user_exclusive` (unix, 0700); `pipe_descriptor_restricts_to_current_user` (windows, SDDL por SID) |
| Transporte JSON-RPC · Mensaje malformado | [T] | `rpc::malformed_line_gets_parse_error_and_connection_survives`; `server_protocol::malformed_message_does_not_kill_the_daemon` |
| Versión de protocolo · Versión soportada | [T] | `server_protocol::initialize_negotiates_supported_version` |
| Versión de protocolo · Versión no soportada | [T] | `server_protocol::unsupported_version_gets_both_versions_and_close` |
| Estado consultable · Consulta de estado | [T] | `server_protocol::status_reports_version_uptime_and_no_sessions` |
| Apagado limpio · Apagado con sesiones activas | [T][C] | `server_protocol::shutdown_responds_then_stops_accepting`; `handle_shutdown` cancela todas las sesiones y espera su drenaje (grace bound); `AcpAgent` mata el subproceso al soltarse |

## acp-session

| Requisito · Escenario | Estado | Cobertura |
|---|---|---|
| Lanzamiento del binario oficial · Binario configurado ausente | [F1] | `handle_propose` mapea el fallo de `run_session` a error 2xxx; un test dedicado de comando inexistente se añade en Fase 1 |
| Handshake · Handshake exitoso | [T] | `e2e_propose` (initialize + session/new + prompt contra el mock real) |
| Handshake · Versión de protocolo incompatible | [F1] | El cliente ACP negocia V1; un agente que responda otra versión es un caso de Fase 1 (requiere un mock configurable) |
| Prompt con streaming · Turno con actualizaciones | [T] | `e2e_propose` (el `agent_message_chunk` del mock se reenvía como `session/event`) |
| Passthrough de permisos · Cliente aprueba | [T] | `e2e_propose` (el cliente aprueba y el agente escribe el archivo) |
| Passthrough de permisos · Sin cliente conectado | [C][F1] | `passthrough_permission` deniega por defecto si `peer.request` falla; test dedicado en Fase 1 |
| Contenido mínimo de la petición | [C] | `PermissionRequestParams` reenvía `tool_call` y `options` verbatim del ACP |
| Cliente que no responde · Timeout de aprobación | [C][F1] | `passthrough_permission` aplica `tokio::time::timeout` y notifica `permission/timeout`; test con cliente lento en Fase 1 |
| Registro persistente de sesión · Auditoría posterior | [T] | `session_log::events_are_appended_in_order_and_readable`; el e2e produce un log JSONL real de la sesión |
| Terminación sin huérfanos · Cancelación de sesión | [C] | `AcpAgent` mata el subproceso al soltarse; `run_session` reenvía `session/cancel` ACP al cancelar |

## propose-flow

| Requisito · Escenario | Estado | Cobertura |
|---|---|---|
| Inicialización de `.meltemi/` · Repositorio sin `.meltemi/` | [C] | `create_dir_all` crea `.meltemi/changes/<name>` con sus padres |
| Andamiaje determinista · Propuesta nueva | [T] | `e2e_propose` (crea `proposal.md`); `propose::scaffold_has_the_standard_sections` |
| Andamiaje determinista · Colisión de nombre | [T] | `e2e_propose` (un segundo propose idéntico devuelve error 3000) |
| Delegación al agente · Flujo completo de extremo a extremo | [T] | `e2e_propose` (el mock rellena `proposal.md` tras el permiso concedido) |
| Resultado final estructurado · Reporte de finalización | [T] | `e2e_propose` (`status == "completed"`, `changeName`, `proposalPath`) |
| Derivación de nombre kebab-case | [T] | `propose::change_names_are_kebab_case`, `empty_or_symbolic_ideas_have_no_name` |

## Resumen

Núcleo de la Fase 0 (arranque, contrato, sesión ACP con agente real simulado, passthrough de permisos y flujo propose completo) verificado por tests automatizados y por el e2e manual. Los escenarios marcados **[F1]** son casos de borde (comando inexistente, versión de agente incompatible, denegación sin cliente, timeout de aprobación) cuya lógica está implementada y garantizada por construcción; sus tests dedicados se añaden con el endurecimiento de la capa de agentes en Fase 1 (change `niveles-integracion-conformidad`, ver `docs/plan-de-cambios.md`).
