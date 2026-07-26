# Propuesta: Enmienda — el compañero móvil es el puesto del Agent Boss

> Enmienda fundacional (meltemi.md v1.3 → v1.4, specs `mobile-companion` y
> `remote-access`). Requiere aprobación del mantenedor fundador antes de
> aplicarse. Solo esta proposal está redactada; design/specs/tasks se
> escriben tras el gate (vía rápida una vez aprobada la dirección).

## Why

El mantenedor puso nombre al caso de uso que la fase 3 debe servir: **Agent
Boss** — estar fuera de la oficina revisando qué hacen los agentes, dirigiendo
el trabajo, revisándolo y respondiendo sus preguntas para que no se detengan,
desde el celular. La auditoría del 2026-07-26 contra el daemon real mostró dos
cosas a la vez. La primera: el contrato ya sirve la mayor parte del caso — la
cola de permisos vive en el daemon, sobrevive reconexiones y es multi-cliente
por diseño (`pending.rs`); las sesiones sobreviven la desconexión del cliente
(el subproceso lo posee el daemon, todo cae al log JSONL); y observar, dirigir
y aprobar existen por RPC (`session/list`, `session/log`, `session/direct`,
`sdd/gate`, `permission/pending`/`decide`). La segunda: cinco fricciones
verificadas convierten hoy al jefe remoto en un jefe a ciegas, y una de ellas
lo sabotea activamente:

1. **Denegación instantánea al caer la conexión dueña**: si el túnel parpadea,
   el push fallido resuelve la petición como denegada de inmediato
   (`acp.rs:508-512`) y first-wins hace que esa denegación gane a cualquier
   otro cliente conectado. Cada blip del túnel deniega lo que el agente estaba
   preguntando.
2. **Timeouts hostiles y no configurables**: 120 s interactivo, 30 s en
   implement, hard-coded (`propose.rs:35`, `sdd_flow.rs:31`, `server.rs:1323`,
   `server.rs:1676`; `config.rs` no expone nada). Nadie contesta desde un
   almuerzo en 120 segundos.
3. **`waiting_permission` es superficie muerta**: está en el contrato y las
   superficies lo renderizan, pero el daemon jamás lo setea — un teléfono no
   distingue una sesión bloqueada esperándote de una trabajando.
4. **Los gates SDD pendientes son indescubribles**: `gate_pending` vive en
   `.cycle-state.json`; `change/list` no lo expone; solo la conexión que lanzó
   el verbo lo supo. El jefe remoto no puede ver "hay dos gates esperándote".
5. **Sin tail vivo para clientes tardíos**: `session/event` se emite solo a la
   conexión iniciadora (`acp.rs:342-356`); quien se conecta a mitad de sesión
   solo puede sondear `session/log`. Además los RPC de gate/review/dispatch
   bloquean la request durante el turno entero del agente — insostenible sobre
   un túnel móvil.

Y una frontera que la enmienda debe decidir en vez de heredar como pregunta
abierta: `remote-access` declara "no existe notificación ni control alguno sin
conexión establecida" y exige enmienda para todo push sin túnel — pero el
Agent Boss necesita *enterarse* de que un agente lo espera. La pregunta de
notificaciones quedó explícitamente abierta en `enmienda-edicion-movil`; este
es el momento de cerrarla con postura, no con accidente.

## What Changes

- **`mobile-companion` (spec)** — la misión pasa de "compañero reducido" a
  **puesto remoto del Agent Boss**, con cuatro verbos: monitorear, aprobar,
  **revisar** y dirigir. Revisar = leer diffs de carreras y decidir sobre el
  trabajo (gates, checklist de review, adopción de archivos de un competidor
  vía `worktree/merge-file` con confirmación — decisiones gobernadas y
  trazables, no autoría). La exclusión de **autoría** queda intacta y más
  precisa: `worktree/apply-edit` (edición libre de contenido) sigue fuera del
  móvil para siempre; adoptar o revertir con confirmación no es editar, es
  decidir.
- **`remote-access` (spec)** — la postura de notificaciones se decide:
  - Base intacta: sin túnel no hay control, y el daemon jamás abre red.
  - Puerta nueva, opt-in y autohospedada: el usuario MAY configurar un
    **aviso mínimo** ("un agente espera tu decisión" — sin payload del
    proyecto, sin contenido de la petición) hacia un endpoint que él mismo
    opera. Desactivado por defecto, contenido exacto especificado antes de
    existir (constitución §9), y jamás desde el daemon: lo emite el cliente
    conectado o un proceso del usuario. El design de la change de fase 3
    decide el mecanismo concreto; la enmienda fija el techo de contenido.
- **Prerrequisitos de daemon nombrados** (changes de implementación propias,
  paridad ×3 — sirven a TUI y GUI hoy, no solo al móvil):
  1. `espera-humana`: política de espera de permisos configurable (esperar al
     humano en vez de default-deny a los 120 s cuando hay una cola viva), y
     la caída de la conexión dueña NO resuelve la petición — queda en la cola
     global para quien conecte.
  2. `sesion-esperando`: el daemon setea `waiting_permission` de verdad y
     `change/list` expone `gatePending` — "qué me espera" en una llamada.
  3. `eventos-para-tardios`: suscripción al stream de eventos de una sesión
     para clientes que no la iniciaron; formas asíncronas (ack + eventos) de
     los RPC que hoy bloquean el turno entero.
- **Frontera Windows nombrada**: el helper de túnel rehúsa en Windows (named
  pipe no reenviable por OpenSSH) — hoy el Agent Boss no puede tunelizar
  contra un daemon Windows, la plataforma primaria del mantenedor. La change
  de fase 3 MUST resolverlo (candidatos del design: AF_UNIX en Windows 10+,
  forwarder local user-run; jamás un puerto de red del daemon).
- **Web fuera de este alcance, con puerta declarada**: un cliente navegador es
  territorio sin gobernar (el transporte es JSONL sobre socket local — un
  navegador no lo habla ni con túnel; exigiría un bridge WS↔socket user-run,
  y el sitio estático no puede ser PWA porque su propia spec prohíbe
  JavaScript). Si la demanda existe, entra por enmienda propia; esta deja
  constancia de que el bridge jamás será el daemon.
- **`meltemi.md` §10 fase 3**: el compañero móvil se renombra a su misión
  ("puesto remoto del Agent Boss"), cuatro verbos, y referencia a los
  prerrequisitos de daemon.

## Impact

- **Documentos**: `meltemi.md` (v1.4, nota de enmienda), specs
  `mobile-companion` y `remote-access` (deltas MODIFIED), plan de cambios
  (tres changes de daemon nuevas en fase 2/3, antes de `companero-movil`).
- **Código**: ninguno en esta enmienda; las tres changes de daemon nombradas
  llevan el suyo con sus specs y tests.
- **Constitución**: intacta. §3 (solo socket local, túnel SSH) y §9 (sin
  telemetría oculta; opt-in especificado antes de existir) son exactamente
  los rieles sobre los que esta enmienda corre.
- **Gobernanza**: enmienda a documentos ratificados — gate del mantenedor
  fundador. La ratificación de v1.2/v1.3 sigue pendiente y se encadena.

## Fuera de alcance

- La app móvil misma (`companero-movil`, fase 3) y su stack (la evaluación
  técnica apunta a Tauri Mobile reutilizando `meltemi-client`; decisión en el
  design de esa change, no aquí).
- El cliente web y su bridge (enmienda propia si se pide).
- Push de terceros (APNs/FCM directo del daemon): jamás — cualquier aviso
  nace de un proceso del usuario, con techo de contenido especificado.
- Edición de código o specs desde el móvil: sigue excluida (autoría); esta
  enmienda solo precisa que decidir (adoptar/revertir con confirmación) no
  es editar.
