# Design — lanzador-conversacional

## Context

El presente, verificado en el árbol y no recordado: `NewSession.svelte` arma los
parámetros de uno de cuatro RPC (`propose`, `sdd/explore`, `worktree/dispatch`,
`session/direct`), **descarta el resultado** —que para `propose` y `sdd/explore`
trae el `sessionId`—, empuja un aviso y se cierra (`NewSession.svelte:118-119`).
`SessionDetail.svelte` siembra el transcript con `session/log` y lo mantiene vivo
con `session/watch`, sin campo de entrada: cada instrucción de seguimiento vuelve
a pasar por el modal. Y el registro de proyectos solo se alimenta de `touch()`
desde seis sitios de uso real (`projects.rs:42-69`): no hay forma de añadir un
directorio que no sea correr algo dentro de él.

Tres hallazgos de la instrumentación cambian el diseño respecto de lo que la
propuesta suponía, y esta es la razón de que este design sea largo:

1. **No existe verbo de arranque libre.** `session/direct` exige `sessionId`
   (`SessionDirectParams`), `propose`/`sdd/explore` son verbos del método y
   `worktree/dispatch` exige change y tarea. La puerta hay que abrirla.
2. **El stream por sesión no lleva la conversación entera.** Solo
   `forward_update` publica al hub (`acp.rs:375-393`). `prompt_sent`,
   `instruction_queued`, `permission_requested`, `permission_decided` y
   `turn_completed` van **únicamente al log**. Un cliente en vivo ve los trozos
   de prosa del agente pero no el prompt que los abrió ni el cierre del turno:
   con eso no se pliega un turno. La propuesta dijo «el stream por sesión ya
   existe»; existe la suscripción, no el contenido. D3 lo resuelve sin RPC
   nuevo.
3. **Worktrees y checkpoints están cableados a la tripleta `(change, task,
   agent)`** — ruta `.meltemi/worktrees/<change>/<task>-<agent>`, rama
   `meltemi/<change>/<task>-<agent>`, ref `refs/meltemi/checkpoints/<change>/
   <task>-<agent>` (`worktrees.rs:36-55`, `checkpoints.rs:39-41`), y ambos
   exigen repo git con al menos un commit. No hay API que no pida la tripleta.
   La frase de la propuesta «aislamiento y checkpoints» no se compone sola: D2
   decide qué se ofrece de verdad y qué no, por escrito.

## Goals / Non-Goals

**Goals:** que iniciar trabajo sea escribir en un compositor y entrar en la
conversación; que la conversación tenga compositor persistente y estados
honestos; que exista un verbo de sesión libre gobernada, con paridad ×3 y sin
relajar una sola pieza del gobierno; que el registro de proyectos gane alta y
olvido explícitos con semántica escrita; que la promesa pública diga lo que el
producto hace.

**Non-Goals:** relajar el proxy de permisos, el log o la resolución de agente;
convertir el método en opcional *para el desarrollo de Meltemi* (§1 intacta);
render conversacional del histórico archivado; cambio de modo/modelo in-sesión;
merge de worktrees de sesión libre (no habrá worktrees de sesión libre);
reescritura de historia por `project/forget`.

## Decisions

### D1 — `session/start`, no un `session/direct` con `sessionId` opcional

La alternativa barata era hacer `sessionId` opcional en `session/direct` y que su
ausencia creara la sesión. Se descarta por cuatro razones, en orden de peso.

**Contrato.** Volver opcional un campo hoy requerido es la única clase de cambio
*sustractiva* que este contrato no ha hecho nunca; todo lo demás ha sido campo
aditivo con `skip_serializing_if`. Y `SessionDirectResult` quedaría con la mitad
de sus campos sin sentido: `queuePosition` y `resumedFrom` no significan nada al
crear, y `disposition` —un enum cerrado `["queued","resumed"]` en el esquema y en
Rust— necesitaría un tercer valor que todo cliente instalado desconoce.

**Semántica.** `direct` significa *dirigir lo que ya corre*; su error propio
`session_not_directable` existe precisamente para distinguir «viva pero no
dirigible» de «no existe». Un método que además crea borra esa distinción justo
donde importa.

**Gramática CLI.** `meltemi direct <session> <instruction>` no puede
desambiguar: `meltemi direct "arregla el build"` es indistinguible de un id de
sesión mal escrito. El parser es un `match` sobre el primer token posicional
(`cli.rs`, `plan_subcommand`), no hay tipos que lo salven.

**Costo de paridad.** Es idéntico en ambas formas —la puerta de la paleta y del
registro tipado cuelga del método, no del handler—, así que la comodidad no
compra nada.

`session/start {projectRoot, instruction, agent?}` → `{sessionId, agentCommand,
status, deniedPermissions, checkpointRef?, checkpointUnavailable?,
checkpointRemedy?}`.

**Enmienda del 2026-07-31 (tarea 1.1), y la razón.** El arco de arriba se
escribió con `checkpointRef?` como único campo del punto de restauración, y eso
no alcanza para lo que D2 exige tres párrafos más abajo: «la sesión también
arranca, y el remedio es el primer commit, jamás `git init`… Las tres
superficies muestran la causa que corresponde». Sin campo que la lleve, la causa
no llega a ninguna superficie, y el requisito de la spec —«el resultado SHALL
declarar que no hay punto de restauración, con el remedio que corresponda a la
causa»— quedaría sin cumplir por el contrato mismo. Así que el resultado gana
dos campos opcionales más, ambos presentes **solo** cuando `checkpointRef` está
ausente: `checkpointUnavailable`, enum cerrado `not_a_git_repo | no_history`
—que es lo que cada superficie traduce—, y `checkpointRemedy`, la prosa inglesa
que corresponde a esa causa, para el cliente scriptable que imprime y no
traduce. Es el patrón que la flota ya practica (`installState` + `remedy`), y es
aditivo: ningún campo existente cambia de forma.

El handler es la receta de
`propose.rs:33-261` sin el andamiaje: validar raíz, resolver agente (por
`resolve_fleet_agent`, no `resolve_launch` — D6 de flota), acuñar UUIDv4,
`sessions.register`, `projects::touch`, `SessionLog::create` +
`SessionStarted`, `enable_direction` (esto es lo que la hace conversación y no
disparo único), `set_state(Active)`, registro START en el índice,
`permissions::load_rules`, expansión de `@`, `edits.enter`, `acp::run_session`,
y cierre por **`session_finalize`**. Ese último punto no es decorativo: saltárselo
es exactamente el bug que `gui-acabado-y-cierre-sdd` D3 arregló en `run_turn`
—toda sesión completada listaba como interrumpida para siempre porque el índice
nunca recibía `ended_at`—. Se reusa `SessionContext` + `finalize_ok`/`finalize_err`
literalmente, no una cola nueva.

### D2 — La sesión libre opera en la raíz, con punto de restauración declarado

**Qué hace hoy cada camino**, medido: `propose` (`propose.rs:192`), los cinco
verbos de `sdd_flow` (`sdd_flow.rs:491`) y la reanudación de `session/direct`
(`server.rs:1944`) corren en la raíz del repositorio. Solo `worktree/dispatch`
(`server.rs:749`) y `sdd/implement` (`server.rs:1597`) usan worktree. El corte no
es caprichoso: worktree es donde corre un agente **desatendido sobre una tarea**,
que es el caso que §3 nombra. La sesión libre es lo contrario —el humano está en
el compositor, ve cada tarjeta de permiso y decide—, y el precedente entero de
los caminos atendidos es la raíz.

**Qué costaría el worktree**, concretamente. No hay API sin tripleta, así que
habría que inventar valores sintéticos (`change = "free"`, `task = <session-id>`)
que se filtran a `worktree/list`, `worktree/diff`, `competitors()` y la UI de
carrera, contaminando el modelo de competencia con entradas que no compiten
contra nadie. Exigiría repo git con al menos un commit (`require_git_root` +
`head_rev`), de modo que la sesión libre **rehusaría donde `propose` hoy
funciona** —una regresión de alcance en la puerta de entrada del producto—. Y
dejaría abierta la pregunta que nadie responde: quién fusiona. El único verbo de
merge que existe es por competidor de una tarea.

**Qué protege al usuario en la raíz**, sin adornos: (a) el proxy de permisos con
deny-by-default y escalado al humano —la misma cola, las mismas reglas, la misma
denegación constitucional sin clientes—; (b) el log JSONL apend-only, que
registra prompt, cada decisión de permiso con quién la tomó, y cada actualización
del agente; (c) el candado blando de `human_edit`: `apply-edit` sobre un árbol
con sesión viva exige `confirm`; (d) el git del usuario.

**Qué se ofrece en lugar del aislamiento: un punto de restauración.**
`checkpoints::create` no necesita worktree —toma el árbol a fotografiar como
argumento y usa un índice de scratch (`GIT_INDEX_FILE`), así que no toca el
índice del usuario ni mueve rama alguna—. La sesión libre crea **un** checkpoint
al arrancar, con la tripleta reservada `(free, <session-id>, <agent>)`, ref
`refs/meltemi/checkpoints/free/<session-id>-<agent>`. Costo declarado:
`checkpoint/list` gana una pseudo-change `free`, y como filtra por change, es
listable y no se mezcla. La reversión guiada de ese checkpoint queda **fuera de
alcance con razón escrita**: revertir el árbol de trabajo del usuario no es lo
mismo que revertir un worktree aislado —ahí hay trabajo humano sin commitear— y
merece su propia change con su propio diseño de confirmación. El punto de
restauración existe, es un ref de git, y el usuario puede volver a él con git.

**Pero «fuera de alcance» no basta: hay que cerrar la puerta.** Hoy *todos* los
`checkpoints::create` del árbol apuntan a un worktree gestionado
(`server.rs:682`, `server.rs:968`, `server.rs:1572`); el de la sesión libre sería
el primero cuyo `worktree` registrado es **el árbol del usuario**. Y
`checkpoints::revert` no mira de dónde vino el registro: hace `git reset --hard`
seguido de `git clean -fd` sobre `record.worktree` (`checkpoints.rs`), y
`checkpoint/revert` ya está cableado en la CLI, en la paleta y en el registro de
la GUI. Es decir: escribir el registro `free` **arma un verbo existente contra el
árbol del usuario** y destruye trabajo humano sin commitear más los archivos no
rastreados; el `confirm` vigente no salva nada, porque el usuario está
confirmando lo que la superficie llama «revertir un worktree». Así que el
guardián entra en esta change, no en la futura: `checkpoint/revert` MUST rehusar
todo checkpoint cuyo `worktree` registrado no sea un worktree gestionado —el
predicado ya existe, `worktrees::is_managed`—, con diagnóstico y con el remedio
de restaurar desde git. Ninguna superficie ofrece el control para esos
checkpoints. La alternativa (no registrar el checkpoint libre) se descarta
porque lo volvería invisible en `checkpoint/list`, que es justamente lo que D2
promete.

**Y el caso sin punto de restauración se declara, no se disimula.** Son dos
causas distintas y el remedio no es el mismo. Si la raíz no es repo git, la
sesión libre **arranca igual** (como `propose` hoy), el resultado trae
`checkpointRef` ausente y el remedio es `git init`. Si la raíz **es** repo git
pero todavía no tiene ningún commit, `checkpoints::create` falla igualmente —
siembra el índice de scratch con `read-tree HEAD` y busca el padre con
`rev-parse HEAD`, y sin historia no hay HEAD—: la sesión también arranca, y el
remedio es el primer commit, jamás `git init` sobre un repositorio que ya
existe. Las tres superficies muestran la causa que corresponde. Es la única
forma honesta: prometer checkpoints y no crearlos sería peor que no
prometerlos.

**Enmienda del 2026-07-31 (tarea 2.4), y su razón.** Este apartado dice que el
daemon «MUST declarar en el resultado **y en el log** si ese punto existe». Se
cumple en el caso positivo —`checkpoint_created` va al log de sesión, y desde
2.5 al stream con él—, y **no** se cumple en el caso negativo: cuando no hay
punto de restauración, la causa y su remedio viajan en el resultado y en el log
del daemon (`tracing`), pero no en el log de la sesión. Las dos formas de
cumplirlo eran peores que el hueco. `SessionEventKind::Error` es el único
portador genérico del enum, y `analytics.rs:266` cuenta esos eventos como
errores de sesión: toda sesión libre en una carpeta sin git reportaría un fallo
que no ocurrió, y el plegado de burbujas de D4 usa `error` como cierre de turno.
Y añadir una variante al enum reabre un contrato que el bloque 1 cerró, además
de dejar obsoleta la cuenta de tipos de la tarea 7.6. Los escenarios de la spec
exigen la declaración **en el resultado**, y ahí está, con la causa cerrada y su
remedio. Si el enum de eventos gana alguna vez un aviso neutro, este es el
primer sitio donde debe usarse.

### D3 — El stream de sesión lleva el evento completo, no solo el del agente

Sin esto no hay conversación en vivo: `prompt_sent` y `turn_completed` —las dos
marcas que abren y cierran un turno— nunca salen del log. La corrección es
mover la publicación al punto de escritura del log (un helper que apenda y
publica), en vez de tenerla colgada solo de `forward_update`.

Por qué esto **no** es un RPC nuevo ni una capacidad nueva: el push
`session/event` ya transporta `SessionEvent` con su `type` discriminado y todas
las variantes del enum; el esquema no cambia; la entrega ya está gobernada por
`delivers()` (origen o mirada declarada), heredada de `eventos-para-tardios`. Y
la prueba de que las superficies ya lo esperaban está escrita en el cliente: el
mapa `EVENT_STYLE` de `SessionDetail.svelte:41-61` tiene entradas para
`prompt_sent`, `permission_requested`, `turn_completed`, `instruction_queued`
—tipos que hoy solo llegan sembrando desde `session/log`—. El daemon
simplemente nunca los envió.

Tres consecuencias que se aceptan por escrito. El volumen del canal crece
(capacidad 1024; `Lagged` sigue advirtiendo y el remedio honesto sigue siendo
releer `session/log`). `agent_update` **no** debe publicarse dos veces: la
publicación queda en un solo lugar. Y `permission/request` sigue siendo un push
propio —lleva opciones y plazo, es decidible—; el evento es la traza de
auditoría, no su reemplazo.

Este requisito vive en `conversational-session` y no en `acp-session` por una
razón que conviene dejar dicha: es la conversación quien lo exige y quien lo
verifica; `acp-session` sigue prometiendo lo suyo —las actualizaciones del
agente, en orden, al origen y a quien mire— sin contradicción. Si el mantenedor
prefiere que la verdad viva del stream quede junta en `acp-session`, mover el
requisito antes de archivar es un cambio de carpeta, no de contenido.

**Un corolario que resuelve la navegación.** La propuesta pide que enviar
navegue hacia adentro, pero `session/start` bloquea hasta el fin del turno,
como todos sus hermanos. No hace falta romper esa forma: `session_started`
—publicado ahora al hub, con `origin` = la conexión que lanzó— llega a esa
conexión antes del primer token, con el `sessionId` dentro. El cliente navega
con el evento; el resultado final del método sigue trayendo el estado del turno
y el conteo de denegaciones, para la CLI scriptable que no escucha pushes.

### D4 — Reglas de plegado: las burbujas leen el log, jamás lo sustituyen

El plegado es cliente puro sobre el mismo conjunto de eventos que el log. La
gramática, completa:

- **Turno humano** abre con `prompt_sent` (`payload.text`). `instruction_queued`
  (`payload.instruction`) se renderiza como turno humano **pendiente** hasta que
  llegue su `prompt_sent`.
- **Turno del agente** abre con el primer `agent_update` posterior y acumula:
  prosa cuando `update.sessionUpdate == "agent_message_chunk"` y
  `content.type == "text"`; bloque de pensamiento plegado —nunca mezclado con la
  prosa— para `agent_thought_chunk`; chip de herramienta para `tool_call` y
  `tool_call_update`, actualizado en su sitio por `toolCallId`; bloque de plan
  para `plan`. El nivel 3 llega mapeado por `map_headless_line`
  (`levels.rs:279-298`) como `{"type":"text"|"message",...}` y se trata como
  prosa; una línea no mapeable llega como `Value::String`.
- **Cierra** con `turn_completed { stopReason }` —el motivo se muestra, no se
  esconde—, o con `session_cancelled`, `session_ended` o `error`.
- **Permisos en línea**: `permission_requested` renderiza una tarjeta en su
  posición, accionable mientras la petición siga pendiente en la cola del proxy,
  decidida por los RPC de permiso que ya existen. `permission_decided` la
  colapsa a resultado + quién decidió + regla. Una tarjeta cuya petición ya no
  está pendiente —vencida, decidida en otra superficie, daemon reiniciado— se
  muestra resuelta, **jamás accionable**: pulsar un botón muerto es simular.
- **Todo lo demás** —`session_started`, `refs_expanded`, `mcp_injected`,
  `mcp_not_delivered`, `checkpoint_created`, `checkpoint_restored`,
  `agent_resolved`, `task_started`, `task_committed`, `human_edit`,
  `usage_reported`, cualquier `type` desconocido y cualquier `agent_update` de
  forma no reconocida— se renderiza **en su lugar** como línea neutra de
  sistema, con el glifo y tono vigentes (desconocido → neutro con su nombre
  crudo, que es el requisito «Tipo desconocido no rompe» que ya está en la
  verdad viva). No se esconde en una bandeja: cae a la vista, en orden.
- **El conmutador** muestra el log de operador completo y crudo, en orden de
  llegada. El invariante que lo hace verificable: **el número de eventos del log
  de operador es igual al número de eventos recibidos**, y conmutar no pierde
  posición ni descarta nada. Las burbujas son una lectura; el log es la verdad.

`usage_reported` no tiene entrada en `EVENT_STYLE` hoy (el proto declara 20
tipos, el mapa cubre 19): cae al neutro por el fallback, que es correcto, pero
se le da su glifo en esta change porque ya se está tocando el renderizador.

### D5 — La enmienda de la promesa, con su texto exacto

La contradicción es real y se enmienda de frente. **Lo que no se toca**: la
constitución §1 gobierna *el desarrollo de Meltemi*, no lo que el usuario hace
con Meltemi. Este repositorio sigue siendo spec-first sin excepción. Lo que se
enmienda es la promesa de producto, que hoy describe como cerrojo lo que debe
ser propuesta de valor.

**`.meltemi/rumbo/product.md`, párrafo «Qué es Meltemi».** Reemplazar:

> Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y
> proyección de contexto), bajo una disciplina donde ninguna línea de código se
> escribe sin una especificación revisada.

por:

> Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y
> proyección de contexto): toda sesión corre gobernada —proxy de permisos con
> deny-by-default, registro apend-only y punto de restauración declarado— y
> sobre ese piso la disciplina spec-first está siempre a un gesto: proponer,
> planificar y verificar viven en el mismo compositor donde se empieza a
> trabajar. La especificación revisada es el estándar que Meltemi hace fácil de
> sostener, no el peaje que impide empezar.

**`meltemi.md`, tesis (línea 42).** Reemplazar:

> - **Meltemi es el plano de control spec-driven para el desarrollo agéntico**:
>   un entorno 100% open source (Apache 2.0) donde ninguna línea de código se
>   escribe sin una especificación revisada primero, y donde esa disciplina
>   gobierna a **los agentes de codificación que el usuario ya tiene y ya paga**
>   — los de los grandes laboratorios y los open source por igual.

por:

> - **Meltemi es el plano de control spec-driven para el desarrollo agéntico**:
>   un entorno 100% open source (Apache 2.0) donde toda sesión de agente corre
>   gobernada —permisos, registro auditable y punto de restauración— y donde
>   especificar antes de escribir es el camino más corto y no un peaje previo;
>   esa disciplina gobierna a **los agentes de codificación que el usuario ya
>   tiene y ya paga** — los de los grandes laboratorios y los open source por
>   igual.

La ratificación del mantenedor es gate: sin ella la change no archiva, igual que
`enmiendas-fundacionales-v1` y `enmienda-agent-boss`. Tras ratificar hay que
correr `meltemi project`, porque el bloque proyectado de `AGENTS.md` /
`CLAUDE.md` / `GEMINI.md` compila el rumbo y quedaría desfasado.

### D6 — `project/register` y `project/forget`: semántica completa

**`project/register {root}` → `{project}`.** Exige que la ruta exista y sea un
directorio; si no, rehúsa con `PROJECT_ROOT_INVALID` (3002) y remedio. No exige
`.meltemi/`: obligarlo convertiría el alta en un acto del método y el registro
existe para apuntar la herramienta a un directorio *antes* de que sea un
proyecto Meltemi —es literalmente el caso «Abrir carpeta…»—. Canonicaliza antes
de guardar, y conviene decir con precisión por qué, porque el motivo obvio es
falso: `project_key` (`core/meltemi-client/src/paths.rs:77`) **ya** canonicaliza
por su cuenta antes de hashear, así que dos formas equivalentes de la misma
carpeta ya pliegan a una sola entrada. Lo que no se canonicaliza solo es el
campo `root` que se persiste y que las superficies muestran: `list` resuelve
last-wins sobre él (`projects.rs:96`), de modo que sin canonicalizar en el alta
el registro acabaría mostrando la forma que se tecleó por última vez, y la
comparación por ruta de `forget` tendría que pelear con ella. Segundo motivo,
igual de concreto: `canonicalize()` falla en una ruta inexistente y
`project_key` cae entonces al literal, que es precisamente el caso que el alta
rehúsa con 3002. Es idempotente: un alta repetida actualiza `lastSeenAt` y
conserva `firstSeenAt`, exactamente como `touch`. **No crea nada en disco** y no
recorre nada: recibe una ruta y la valida. El diálogo que la produjo vive en el
cliente (D8).

**`project/forget {root}` → `{forgotten}`.** Apenda una línea de olvido al mismo
JSONL apend-only; el plegado last-wins la resuelve. El precedente exacto de
lápida está en el módulo hermano: `worktrees::list` ya pliega líneas `REMOVED
<path>` (`worktrees.rs:89-90`). **No exige que la ruta exista** —y esto no es un
detalle: los proyectos ausentes en disco son justo los que uno quiere olvidar; un
`forget` que canonicaliza obligatoriamente los volvería inolvidables—. Resuelve
por clave cuando la ruta canonicaliza, y por comparación normalizada contra las
raíces registradas cuando no.

**Qué ocultan los listados y qué no.** `project/list` deja de listarlo. Nada
más: sus sesiones siguen en `session/list`, sus logs siguen leyéndose por
`session/log`, la analítica sigue contándolas y el árbol en disco no se toca
jamás. Un proyecto olvidado que se usa o se registra otra vez reaparece —una
línea nueva gana al plegado— y esa reaparición es correcta, no un fallo.

**La trampa del plegado, resuelta antes de que muerda.** `handle_project_list`
dispara `rebuild_from_sessions` cuando la lista plegada queda vacía
(`projects.rs:162-165`). Con lápidas, un registro donde *todo* fue olvidado
pliega a vacío y el rebuild resucitaría a todos. Así que el plegado MUST
distinguir «no hay ningún registro parseable» de «todo lo visible fue
olvidado», y el rebuild solo dispara en el primer caso. El corolario honesto:
si el archivo del registro falta o se corrompe por completo, el rebuild
reconstruye desde los registros de sesión —que son la fuente de verdad por D2 de
`multiproyecto-suscripciones`— y con él se van las lápidas. `forget` promete
sobre el listado de hoy, no permanencia; queda escrito.

### D7 — Error de resolución estructurado: 2001 enriquecido, sin código nuevo

Hoy el usuario recibe prosa inglesa cruda: «neither `agent.command` nor
`agent.id` is configured» (`levels.rs:65`). Toda superficie solo puede
transcribirla.

**No se añade código de error.** `error.schema.json` tiene un `enum` **cerrado**
de códigos más un `x-catalog`: un código nuevo obliga a editar los dos y a
versionar el contrato. En cambio `$defs.errorData` declara tres propiedades, no
fija `additionalProperties: false` y solo exige `kind` y `detail` — así que un
campo opcional **valida contra el esquema vigente incluso antes de declararlo**,
y declararlo es puramente aditivo. Se reusan 2000 (`agent_command_not_
configured`) y 2001 (id desconocido / binario no detectado) con `candidates`
añadido a `ErrorData`.

Cada candidato lleva el vocabulario que la flota ya publica: `id`, `detected`,
`installState`, `remedy`, `remedyCommand` —los mismos que `FleetAgent` expone en
`fleet/list`, calculados por `fleet::detect_layers` + `fleet::compose_state`, de
modo que existe **un solo camino de detección** y la respuesta del error no
puede discrepar de la vista Flota. Obligación de §2 escrita en el requisito:
el payload MUST NOT llevar valores de entorno, rutas de credenciales ni nada con
forma de secreto; lleva ids y estado de detección.

Un detalle de plomería que hay que tocar: `RpcError::application` fija
`data` a `ErrorData { kind, detail, remedy }` (`rpc.rs:55-72`), sin hueco para
más. Se añade un constructor hermano que acepta los candidatos; el existente no
cambia de firma, así que ningún sitio de llamada se mueve.

### D8 — El diálogo nativo vive en el cliente, y no toca la superficie del webview

**Frontera §3, explícita:** el daemon no abre ventanas, no enumera discos y no
sondea nada fuera de la ruta que se le entrega; recibe una cadena y la valida
(D6). Elegir la carpeta es cromo, y el cromo es del cliente. Esa es la asimetría
que §4 permite: la **capacidad** `project/register` tiene deber de paridad y lo
cumple en las tres superficies —la TUI teclea la ruta, la CLI la toma como
argumento—; el diálogo del sistema operativo no lo tiene.

**Dependencia (§10):** `tauri-plugin-dialog`, pineada exacta en
`[workspace.dependencies]` junto a `tauri = "=2.11.5"`, consumida solo por el
crate `desktop`. El set de dependencias de `meltemid` no se mueve ni un
milímetro.

**Alcance de permisos: ninguno nuevo para el webview.** El plugin se inicializa
en el builder, pero **no** se expone al front: se añade un comando propio
`pick_project_folder() -> Option<String>` a `generate_handler!`, siguiendo el
patrón que ya usan `fsops::open_with` y `project_root`. Así
`capabilities/default.json` sigue siendo literalmente `["core:default"]` y
`desktop/tests/surface.rs:45-53` —que prohíbe por substring `fs:`, `shell:`,
`http:`, `os:allow`, `process:`— pasa sin enmienda. La CSP no se toca: la
llamada viaja por IPC, ya permitido por `connect-src ipc: http://ipc.localhost`,
y el mismo test prohíbe `https://` en la CSP.

**Lo que se comprueba al implementar, no se asume:** el plugin arrastra `rfd` y
su cola transitiva de GTK en Linux; `deny.toml` ya ignora un conjunto Tauri/GTK3
y hay que verificar contra él antes de dar la dependencia por buena, no después.

### D9 — Nombres de la CLI: `meltemi session`, `meltemi projects register|forget`

La propuesta escribió `start` y `project register|forget` y delegó
explícitamente los nombres finales al design. Los dos se cambian, con razón.

**`start` colisiona con `stop`, que significa el daemon.** `stop` → `shutdown`
está en la verdad viva del contrato CLI. Un `start` al lado se lee como «arranca
el daemon», que es lo contrario de lo que hace. Se elige **`meltemi session
<instruction> [project-root] [--agent <id|perfil>]`**: la gramática ya usa
sustantivos plurales para los listados (`sessions`, `projects`, `worktrees`,
`checkpoints`, `changes`, `specs`), de modo que el singular se lee como «una de
esas, ahora», y el nombre calca el método `session/start`. El riesgo —`session`
y `sessions` adyacentes en la ayuda y en la difusa de la paleta— se nombra en
Riesgos; ninguna de las dos confusiones es destructiva, porque `meltemi session`
sin instrucción es error de uso.

**`project register` es ambiguo en el parser.** `project` ya toma una raíz
opcional (`meltemi project <root>` regenera el contexto proyectado), así que
`meltemi project register` parsearía `register` **como una ruta**. Se cuelga del
verbo de listado, que hoy no toma posicionales: **`meltemi projects register
<path>`** y **`meltemi projects forget <path>`**, calcando la forma de `usage
[day|week|month|total]`, que ya usa un posicional discriminador.

**`--agent`** se añade a la lista de flags que el parser global deja pasar como
posicional para que el subcomando la lea —la misma lista donde ya viven `--all`,
`--project`, `--since`, `--until`—, porque el parser global es estricto con
flags desconocidas y una flag rechazada haría inusable lo que la ayuda anuncia.

### D10 — Plan de paridad: qué toca exactamente cada método nuevo

Tres métodos nuevos (`session/start`, `project/register`, `project/forget`).
`tui/tests/parity.rs` es bidireccional: falta o sobra, falla igual. Por método:

1. `proto/meltemi-proto/src/lib.rs` — `pub const` dentro de `pub mod methods`
   (el test lo parsea textualmente partiendo por `= "`), más los tipos
   `Params`/`Result` con `rename_all = "camelCase"`.
2. `proto/schemas/v1/<nombre>.schema.json` — el `title` **declara qué métodos
   posee** el archivo (el generador lo parte por `·` y `&`). `project/register`
   y `project/forget` pueden compartir archivo, pero entonces el atajo `$defs.
   params` queda deshabilitado —solo aplica cuando el archivo reclama un único
   método— y hay que nombrar `projectRegisterParams` / `projectForgetParams`.
3. `proto/meltemi-proto/tests/conformance.rs` — caso `assert_conforms` y su
   negativo `assert_rejected`.
4. `core/meltemid/src/server.rs` — brazo en `dispatch_request` y handler.
5. `tui/src/shell/palette.rs` — el método declarado por **exactamente una**
   `Entry` (hay un test de unicidad aparte).
6. `desktop/ui/src/lib/registry.ts` — una entrada escrita con el helper literal
   `R("...")`: el test parte por esa cadena, así que otra forma es invisible.
7. `desktop/ui/src/lib/messages.ts` — la clave `palette.m.*` en **ambos**
   catálogos, una clave por línea (el segundo gate, `desktop/tests/surface.rs`,
   extrae claves con una heurística de línea).
8. `docs/paridad-nucleo.md` — una fila con el método entre backticks; el test
   (`the_parity_matrix_documents_every_method`) lo exige para **todos** los
   métodos del contrato, no solo los invocables por el cliente: son 48 hoy y
   pasan a 51.
9. `desktop/ui/src/lib/generated/method-forms.ts` — regenerar con `npm run
   gen:forms --prefix desktop/ui`; CI corre `check:forms` y falla por
   desactualizado. Un método que el generador no resuelve **no falla**: degrada
   a JSON crudo y queda listado en la cabecera del archivo generado. Hay que
   mirarla.
10. `tui/src/cli.rs` (gramática y `--agent`) y regenerar `docs/referencia-cli.md`
    (`cargo run --example gen_cli_ref`), con su propio gate en
    `tui/tests/docs.rs`.

Además, el verbo `direct` de la paleta deja de ser `reserved: true`. Hoy
`reserved` es solo una etiqueta de render (`render.rs:987`); lo que lo hace
inerte de verdad es que `Action::Submit` no tiene brazo para él y cae al
`_ => None`, cerrando el overlay sin hacer nada. Cablearlo son cuatro pasos:
brazo en `state.rs`, variante de `Effect`, despacho en `mod.rs`, operación
async en `conn.rs`. Y un quinto que es una trampa: la paleta hace
`to_ascii_lowercase()` sobre la línea antes de partirla (`state.rs:308`), así que
**una instrucción no puede pasar por la línea de la paleta sin perder
mayúsculas**: el `direct` interactivo necesita un overlay de entrada que preserve
el texto tal cual.

La misma trampa muerde dos veces en el registro de proyectos, y hay que decirlo
antes de implementar. Primero, la ruta sufre exactamente lo mismo que la
instrucción: minusculizada por la línea de la paleta, es una ruta distinta en
Linux y macOS. Segundo, y menos evidente: `projects <lo que sea>` **ya significa
otra cosa** en el shell —`state.rs` tiene un brazo `command.starts_with("projects
")` que fija `project_scope` al texto y salta a Sesiones—, de modo que
`projects register /ruta` se leería hoy como «filtra las sesiones por el proyecto
`register /ruta`»: una orden convertida en filtro, en silencio. El alta y la baja
por la paleta van por su propio overlay de entrada, que preserva el texto y se
resuelve **antes** que ese brazo de filtro; el brazo de filtro conserva su
comportamiento para todo lo demás.

### D11 — Plan de pruebas: qué prueba cada método, y qué solo ve el smoke

- **e2e del daemon contra `mock-agent`**, en repos fixture temporales, nunca
  contra la raíz de este repo, nunca agentes reales ni red: `session/start`
  arranca gobernada y dirigible; la instrucción de seguimiento se encola y se
  despacha como siguiente turno; el índice recibe registro de fin y la sesión
  **no** lista como interrumpida; el checkpoint de arranque existe; un fixture
  **sin git** arranca igual y declara que no hay punto de restauración; un
  fixture **con git y sin commits** arranca igual y da el remedio del primer
  commit, no `git init`; y el guardián: revertir el checkpoint de una sesión
  libre rehúsa y deja el árbol del usuario intacto —incluido un archivo no
  rastreado que `git clean -fd` habría borrado—, mientras revertir el de un
  worktree gestionado sigue funcionando igual que hoy.
- **Plegado del registro**, unitario sobre el JSONL: alta idempotente; alta que
  canonicaliza; alta de ruta inexistente rehusada con 3002; olvido que oculta;
  olvido de una raíz ausente en disco; reaparición por uso; línea corrupta que
  no oculta al resto (precedente vivo); y el caso trampa —registro con todo
  olvidado **no** dispara el rebuild—.
- **Error estructurado**: configuración sin agente devuelve 2000/2001 con
  candidatos; un id de catálogo no detectado rehúsa y no degrada; y una
  aserción de §2: ningún valor de entorno aparece jamás en el payload.
- **TUI por escenario**, en el estilo de los tests de `shell/state.rs`: `direct`
  interactivo desde el drill-in preservando mayúsculas; alta y baja de proyecto
  desde la paleta con la ruta intacta y **sin** caer en el brazo de filtro de
  `projects <texto>`; el verbo de sesión libre alcanzable.
- **GUI por cableado**: `desktop/tests/surface.rs` (capacidades siguen en
  `core:default`, CSP intacta), `npm run check`, `npm run lint:i18n`.
- **Smoke visual conducido por CDP sobre el binario real**, publicado en
  `docs/qa/`: home con el compositor enfocado; enviar navega hacia adentro;
  estados del compositor (ocupada, encolada con posición, terminada ofreciendo
  reanudar); conmutador burbujas↔log conservando el conteo de eventos; tarjeta
  de permiso en línea, accionable y luego resuelta; sidebar con un proyecto
  registrado y uno olvidado; el diálogo nativo abre. La razón está medida, no
  supuesta: los cuatro defectos de `gui-acabado-y-cierre-sdd` eran invisibles a
  los tests de cableado, y esta change toca exactamente la clase de cosas que
  solo se ven conduciendo la app —layout, foco, navegación, estado.
- Una trampa de layout concreta para el compositor: `SessionDetail` es
  `grid-template-rows: auto auto 1fr` con dos hijos en flujo cuando no hay
  banner; un cuarto hijo añadido sin tocar la plantilla cae en la pista `1fr` y
  se come el panel. La plantilla se reescribe con el compositor, no después.

## Risks / Trade-offs

- **`session` junto a `sessions`.** Adyacentes en la ayuda generada y en la
  difusa de la paleta. Se acepta porque ninguna confusión es destructiva y
  porque la alternativa (`start`) colisiona con `stop`, que ya significa otra
  cosa. Si el uso muestra tropiezos, renombrar el verbo CLI es aditivo: el
  método no cambia.
- **La pseudo-change `free` en el registro de checkpoints.** Es una tripleta
  sintética, exactamente lo que D2 rechaza para worktrees. Se acepta porque el
  costo es una entrada listable en `checkpoint/list` filtrable por change, y no
  contamina el modelo de competencia —no hay rama, no hay worktree, no hay
  competidor—. La alternativa limpia es cablear los checkpoints por sesión;
  queda como change futura con su razón escrita aquí.
- **El volumen del hub crece.** Publicar todo el log al canal multiplica el
  tráfico de una sesión ruidosa. Capacidad 1024 y `Lagged` siguen siendo el
  contrato; el remedio honesto sigue siendo releer `session/log`. Si aparece
  presión real, el filtro por tipo en la suscripción es la salida —y sería
  aditivo.
- **`forget` no sobrevive a un rebuild.** Declarado en D6. Es consecuencia
  directa de que los registros de sesión sean la fuente de verdad, así que
  «arreglarlo» sería romper una invariante mayor por una menor.
- **La sesión libre baja la barrera de entrada al trabajo no especificado.** Es
  el punto entero de la change y su riesgo entero. Lo que lo hace aceptable es
  que el gobierno no se toca —proxy, log, punto de restauración— y que el
  método queda a un gesto en el mismo compositor. Lo que lo invalidaría: que
  el modo Libre acabara siendo el 100% del uso y Proponer nunca se pulsara.
  La analítica local ya cuenta sesiones por verbo; es medible sin telemetría.
- **La enmienda es un gate humano.** Sin ratificación la change no archiva, y el
  trabajo de superficie quedaría hecho sobre una promesa que el rumbo contradice.
  Es deliberado: la contradicción se resuelve arriba, no se disimula abajo.

## Sin verificar al escribir este design

No se convierten en afirmación y se comprueban al implementar: la versión exacta
a pinear de `tauri-plugin-dialog` compatible con `tauri = "=2.11.5"`, y su cola
transitiva contra la lista de `deny.toml`; si `checkpoints::create` sobre la raíz
del repositorio se comporta idénticamente a como lo hace sobre un worktree en los
tres sistemas operativos (el índice de scratch dice que sí; hay que verlo); si el
generador de formularios resuelve los `$defs` de un esquema que reclama dos
métodos sin degradar a JSON crudo; el costo real del hub con un agente verboso; y
si algún consumidor actual del stream asume que solo llegan `agent_update` —la
revisión del árbol dice que no, pero es una ausencia, no una prueba.
