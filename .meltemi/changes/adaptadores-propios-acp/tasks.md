# adaptadores-propios-acp — tasks

## 1. Puente compartido y fixtures de cable de proveedor

- [x] 1.1 Crear el crate `core/meltemi-adapters` (lib + binarios `meltemi-claude-acp` y `meltemi-codex-acp` con esqueleto ACP: initialize, session/new, prompt, cancel), miembro del workspace con cabeceras SPDX y sin dependencias externas nuevas (tokio, serde, agent-client-protocol ya pineados); gate cargo-deny verificando que el crate no enlaza pila HTTP/TLS
- [x] 1.2 Librería de puente compartida: supervisión del subproceso proveedor (lanzamiento, apagado limpio, kill ante cuelgue), framing NDJSON bidireccional y mapeo base sesión ACP ↔ ciclo de vida del subproceso, con tests unitarios sobre transportes en memoria
- [x] 1.3 Crate `core/mock-provider` con el binario `mock-claude-wire`: emite stream-json guionado (evento inicial con `capabilities`, deltas parciales, tool calls, resultado final) y acepta entrada stream-json; guiones por archivo/variable de entorno, patrón mock-agent
- [x] 1.4 Binario `mock-codex-wire` en el mismo crate: servidor JSON-RPC 2.0 NDJSON guionado (handshake con versión, conversación hilo/turno/ítem, petición de aprobación) más volcado de esquema fixture para el test de conformidad

## 2. Adaptador de servidor JSON-RPC (`meltemi-codex-acp`)

- [x] 2.1 Vendorizar como fixture el esquema volcado por versión del CLI oficial (`codex app-server generate-json-schema`) y test de conformidad de los tipos del adaptador contra él, fallando con el campo divergente señalado
- [x] 2.2 Lanzamiento y handshake del CLI oficial en modo app-server, con detección de desfase de versión y rehúso diagnosticado con remedio; binario y versión efectivos al log de sesión
- [x] 2.3 Mapeo de conversación a la sesión ACP: primitivas hilo/turno/ítem como actualizaciones en streaming, cierre de turno y cancelación propagada al servidor
- [x] 2.4 Aprobaciones del servidor relevadas a `session/request_permission` (decide el proxy de meltemid), con denegación por defecto ante ausencia de decisión
- [x] 2.5 E2e de workspace: meltemid pilota el binario real `meltemi-codex-acp` contra `mock-codex-wire` (streaming, permisos, cancelación), sin red ni agentes reales

## 3. Adaptador de sesión stream-json (`meltemi-claude-acp`)

- [ ] 3.1 Lanzamiento del binario oficial con la sesión iniciada (`-p --input-format stream-json --output-format stream-json --include-partial-messages`), detección de features vía el arreglo `capabilities` del evento inicial y guarda contra el modo de clave de API (flip de `--bare`): rehúso diagnosticado, jamás inyección de credenciales
- [ ] 3.2 Mapeo de eventos a la sesión ACP: deltas parciales, tool calls, transcripts de subagentes y resultado final como actualizaciones en streaming
- [ ] 3.3 Shim MCP de permisos por stdio (el mismo binario en modo shim, canal privado con el proceso padre) registrado vía `--mcp-config` y apuntado por `--permission-prompt-tool`; cada petición relevada a `session/request_permission`
- [ ] 3.4 Hooks `PreToolUse` inyectados vía `--settings` como compuerta dura (deniegan incluso en modo permisivo del CLI) y pérdidas visibles: auto-denegaciones de herramientas interactivas mostradas en sesión con motivo
- [ ] 3.5 Passthrough de la proyección MCP de la sesión por `--mcp-config` y mapeo de reanudación (`--resume`/`--fork-session`) a la carga de sesión ACP, acotada al directorio del proyecto y sus worktrees
- [ ] 3.6 E2e de workspace: meltemid pilota el binario real `meltemi-claude-acp` contra `mock-claude-wire` (streaming, prompt-tool, compuerta dura, cancelación), sin red ni agentes reales

## 4. Registro, detección empaquetada y superficies

- [ ] 4.1 Detección genérica de capa empaquetada: sondeo del directorio hermano del daemon en ejecución para capas `bundled = true`, precedencia PATH → candidatas → hermano, fuente del hallazgo reportada; campos aditivos en `proto/` (esquema + tipos + conformidad) y tests con registro sustituido
- [ ] 4.2 Flip de las filas de nivel 2 del registro: capa adaptador a los binarios propios con `bundled = true`, retiro de los `adapter-install` de terceros, capas `cli-*` intactas, notas legales reescritas con verdad (gris se queda gris, tolerado se queda tolerado) y `version` de la instantánea actualizada con verificación documentada (fuente y fecha)
- [ ] 4.3 Remedio de capa empaquetada ausente (reinstalar o reparar Meltemi, sin comando de terceros) compuesto en el daemon y presentado por igual en CLI humana/`--json`, TUI y GUI; rehúso de lanzamiento nombrando la capa según su tipo
- [ ] 4.4 Render de la procedencia empaquetada en la vista Flota de la TUI y el detalle de la GUI; matriz de paridad actualizada
- [ ] 4.5 `docs/agentes.md` reescrita en lockstep con el registro (el test de coherencia registro↔guía se re-ancla): capa empaquetada explicada, remedios nuevos, y receta de adaptador de terceros por configuración con su nota legal, presentada sin recomendarla
- [ ] 4.6 Los instaladores empaquetan los dos binarios adaptadores junto a meltemid en las tres plataformas; el QA de presupuesto de tamaño re-mide sus gates con el costo real a la vista

## 5. Conformidad y cierre

- [ ] 5.1 La suite de conformidad ejerce los criterios de nivel 2 en CI pilotando los adaptadores propios contra los mock wires (streaming, cancelación, permisos, sesión), sin red ni binarios de proveedores reales
- [ ] 5.2 Corrida de conformidad manual contra los CLIs reales documentada (instrucciones opt-in por plataforma), resultado persistido con fecha y versión, y escenarios solo-manuales marcados vía verify-mark con nota
