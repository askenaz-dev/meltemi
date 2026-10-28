# sesion-que-espera — design

> Reconocimiento: cinco barridos independientes sobre el código y las specs
> vivas, cada uno con un pase adversarial que intentó refutarlo. Lo que sigue
> son las decisiones, con la evidencia que las obliga. Donde el reconocimiento
> se contradijo a sí mismo, la verificación directa manda y queda dicha.

## Lo que el proposal suponía y el código corrigió

El proposal dijo «el crate ACP mata el subproceso al drop» y pidió pinearlo. El
mecanismo exacto decide **dónde** puede vivir la espera, así que no es un
detalle:

```
ChildGuard(Child)::drop → Child::kill()                  acp_agent.rs:228-241
select(protocol_future, child_monitor)                   acp_agent.rs:369-374
connect_with(agent, async |connection| { … })            acp.rs:212
```

No es `kill_on_drop` de `async_process` (que está en `false` y nunca se
habilita): es el `ChildGuard` propio del crate, dueño del hijo dentro del futuro
que `connect_to` corre. Y el bucle multi-turno de Meltemi vive **dentro** de ese
closure. Retornar de `run_session` *es* matar al agente; no es una consecuencia
de matarlo.

## D1 — La espera vive en el borde del turno, dentro del scope de la conexión

La sesión que espera no se implementa «no llamando a finalize»: se implementa
**no saliendo del bucle**. Con la cola vacía, en vez de que `take_or_close()`
cierre y el bucle rompa (`session.rs:95-108`, `acp.rs:357`), el borde espera a
que llegue trabajo.

La alternativa que parece más limpia —devolver el control y guardar la conexión
en el registro para reusarla— no es implementable contra este crate sin pelearse
con él: `connect_with` es una API *scoped*, y lo que hay que conservar es un
futuro en ejecución, no un valor. Si hay que sostener una tarea viva de todos
modos, la tarea correcta es la que ya existe. Se conserva además la propiedad
que hace legible este código: **un solo lugar decide qué pasa al terminar un
turno**, y `redirigir-turno` acaba de reformarlo.

Precedente que quita el miedo: **una espera indefinida dentro de este mismo
closure ya se envía en producción y es el default interactivo**.
`WaitPolicy::WhileConnected` deja `deadline = None`, lo que convierte el futuro
`bounded` de la espera de permisos en `std::future::pending()` — aparcado sin
plazo (`acp.rs:502-507`). Esto no inventa una forma nueva de esperar; extiende
una que ya corre.

**La cola necesita un primitivo que no tiene.** `QueueInner` son cuatro campos
—`items`, `accepting`, `cancelled`, `interrupted`— y **ni un `Notify`, canal o
waker** (`session.rs:27-46`). La espera se despierta con un `Notify` propio de
la cola, señalado por `enqueue`/`interrupt_with`/`mark_cancelled` **bajo el mismo
lock que muta el estado y después de mutarlo**, exactamente la disciplina que
`redirigir-turno` dejó escrita y probada.

**Corolario que hay que decir en voz alta**: `take_or_close()` deja de ser el
verbo del borde. Su cierre atómico —encontrar vacío y dejar de aceptar bajo un
solo lock— era justamente lo que cerraba la carrera «encolar mientras el bucle
sale». Se sustituye por `take_or_wait()`, que **no suelta el lock entre
comprobar y registrarse en la señal**; esa es la única forma de que no exista
ventana. El `close()` sigue existiendo para los finales de verdad.

## D2 — El desacople del RPC no es comodidad: es la condición de la espera

`session/start` corre la sesión entera dentro de la petición
(`free_session.rs:266-291`). Si el bucle espera indefinidamente, esa petición
**no responde nunca**. No hay D1 sin esto.

Es exactamente la evidencia que `eventos-para-tardios` dejó pedida: no una
preferencia de forma, sino un caso donde la forma bloqueante impide una
capacidad.

**El precedente no hay que inventarlo, y está en el mismo archivo**:
`session/direct` **ya responde temprano** en su rama `queued`/`relayed`, con
`status: null`, difiriendo el desenlace al stream de eventos y a `session/log`
(`server.rs:1948-1960`). La misma función tiene la forma bloqueante en su rama
`resumed`. Se copia la que ya funciona.

El servidor, además, **ya despacha cada petición en su propia tarea**
(`server.rs:225-230`), así que una sesión larga nunca bloqueó el bucle de
conexión — solo su propia respuesta.

**Forma elegida: parámetro aditivo `detach` en `session/start`.** No un verbo
nuevo. Un verbo nuevo obligaría a duplicar toda la superficie de arranque
(perfil, agente, proyecto, checkpoint) y a mantener dos caminos que hacen lo
mismo salvo cuándo contestan; el parámetro deja un solo camino con dos finales.
Ausente u omitido ⇒ comportamiento de hoy, byte a byte, con la conformidad de
tres vías que el repositorio ya exige para banderas aditivas.

## D3 — El default de la CLI **no cambia**, y la razón está medida

`session/log` y `session/watch` tienen `—` en la columna CLI de la matriz de
paridad (`docs/paridad-nucleo.md:27-28`): huecos declarados y preexistentes. Es
decir: **una CLI que arranca desacoplada no tendría por dónde ver el turno**.
Imprimiría un id y saldría.

Por eso `detach` es opt-in y la CLI sigue bloqueando. No es prudencia genérica:
es que la superficie scriptable no tiene hoy el otro extremo del cable.

Y hay un segundo filo, más grave, que el reconocimiento sacó y conviene decir
entero: cuando el conteo de clientes queda en cero durante `no_client_grace`,
**toda petición de permiso pendiente se resuelve como denegación
constitucional** (§3, `acp.rs:611-640`). Una CLI que arrancara desacoplada y
cerrara su conexión dejaría a la sesión sin cliente y, con ella, condenada a
denegar todo permiso que pidiera. El default bloqueante evita eso por
construcción.

## D4 — La política de idle: dos cotas, y una de ellas ya está construida

El proposal escribió «hoy **nada** gobierna esa acumulación». Es cierto para el
daemon: no hay tope de sesiones, ni control de admisión, ni código de error de
recursos en ningún punto del árbol. (La GUI sí tiene un tope **de pestañas** con
su rehúso y su remedio, `gui-shell/spec.md:863-864` — pero eso gobierna la
superficie, no los subprocesos.)

Dos cotas, ambas con el idioma que el repositorio ya usa —`Option<T>` en el
struct crudo y el default en el accesor con `unwrap_or`, como
`no_client_grace`— y ambas conservadoras:

1. **`idle-timeout`** (segundos, default conservador): tiempo máximo que una
   sesión espera sin instrucción antes de terminar.
2. **`max-idle-sessions`** (default conservador): cuántas pueden esperar a la
   vez. Alcanzado el tope, la más antigua se cierra **primero**, y se dice — no
   se rehúsa el arranque, porque rehusar castiga al usuario por sesiones que ya
   no está mirando.

**La tercera cota no hay que escribirla: ya existe como futuro reusable.**
`no_clients_sustained(clients, grace)` (`acp.rs:611`) es exactamente «nadie
mira, sostenidamente», y se compone en el mismo `select!` de la espera. Una
sesión que espera sin ningún cliente conectado no tiene a quién esperar.

## D5 — El final honesto, y el problema de idioma que arrastra

Al vencer, `finalize` con `reason` que dice *idle*, nunca `completed` fingido.
`SessionEnded.reason` es **string libre** en el tipo Rust y en el JSON Schema —
no hay enum que extender ni schema que tocar.

Pero el reconocimiento encontró el filo: la GUI **imprime ese string crudo** en
el transcript (`session_ended` cae a `system(event)` → `flatten(event.payload)`),
y la TUI imprime solo el tipo de evento. Ninguna de las dos tiene clave de
traducción para él. Un `reason: "idle"` aparecería en inglés, sin traducir, en
una superficie que la constitución §11 obliga a internacionalizar.

Decisión: el `reason` del contrato sigue siendo el identificador estable en
inglés (`idle_timeout`), y **cada superficie lo traduce**, con su clave ES/EN.
El lint de i18n de la GUI (`scripts/i18n-lint.mjs`, en CI) es lo que impide que
esto se olvide.

## D6 — El estado nuevo, y el hecho incómodo: casi nada lo obliga

El contrato gana `SessionState::WaitingInstruction`. Y aquí está el riesgo real,
que el reconocimiento midió: **añadir una variante casi no rompe nada**.

- Sitios que son error de compilación: **tres**. Un `match` exhaustivo en Rust
  (`tui/src/shell/render.rs:1166`) y dos en TypeScript (`Record<SessionState, …>`
  en `StatusBadge.svelte` y el catálogo ES/EN).
- Todo lo demás —el glifo del sidebar, los dos contadores de la barra de estado,
  `tree.ts` LIVE, `isLive` de `Sessions.svelte`, `live_sessions` del daemon, el
  `is_historical` de la TUI— son **listas positivas o ramas `default:`** que
  aceptarán el sexto estado en silencio y lo pintarán mal: como *terminada*, o
  invisible.
- Y **nada guarda** el enum `sessionState` duplicado entre los dos JSON Schemas,
  aunque el repositorio ya tiene ese idioma de test escrito dos veces para
  exactamente esta razón.

Por eso las tareas enumeran los sitios uno por uno, y por eso nace un test que
**recorre el enum del contrato y exige que cada estado esté en cada mapa de
superficie** — la clase de guardián que ya existe para los tipos de evento
(`every_declared_event_type_has_a_glyph_and_a_tone`) y que fue lo que atrapó la
omisión en `redirigir-turno`.

**Lo que sí está protegido por construcción**: el anillo del compositor lo
gobierna una comparación literal `state === "active" || state === "starting"`
(`SessionDetail.svelte:440-442`), con su comentario diciendo que reusar `LIVE`
lo encendería donde debe apagarse, y un test que pinea ese texto. El estado
nuevo **queda oscuro sin hacer nada**, que es lo correcto: esperar no es
trabajar.

## D7 — La restitución a `active` hay que ordenarla

`end_waiting` pone `Active` **incondicionalmente** cuando el contador llega a
cero (`session.rs:319-326`), pisando lo que hubiera. Es correcto hoy porque un
permiso solo ocurre dentro de un turno. Con un estado de espera nuevo deja de
ser inocuo: el borde debe declarar `WaitingInstruction` **después** de que la
espera humana haya terminado, nunca antes. Se pinea con un test, porque es la
clase de orden que se rompe en la siguiente refactorización sin que nadie lo
note.

## D8 — Los deltas: menos MODIFIED de los que el proposal temía, y uno que sí

El proposal supuso que la spec viva pinea textualmente el cierre en cola vacía.
**No lo hace**: `.meltemi/specs/acp-session/spec.md` no tiene requisito de ciclo
de vida que lo diga, y `session/start` no aparece en ninguna spec viva.

Lo que sí queda falsificado es más estrecho y más concreto: el requisito
**«Dirección de una sesión existente»** despacha «al concluir el turno en
curso» — y una sesión que espera **no tiene turno que concluir**. La rama (a) se
sigue aplicando (una sesión viva entre turnos es *activa* en el vocabulario de la
spec), pero su disparador deja de ser cierto. Ese requisito entra **MODIFIED**,
con su texto completo.

Los otros dos candidatos que el barrido propuso —la accesibilidad de la TUI y
las pestañas de la GUI, ambos por enumerar estados— **se rechazan**: sus
escenarios **ya** son parciales hoy (ninguno nombra `interrupted`, que es un
estado vivo y visible), así que no son listas cerradas que un estado nuevo
falsifique. Extenderlos sería higiene, no obligación, y esta change no arrastra
higiene ajena.

Nota de método: `free-session`, `conversational-session` y la regla del anillo
**no están en la verdad viva** — las crea `lanzador-conversacional`, sin
archivar (su archivado es gate de la firma del mantenedor). Esta change se
escribe contra lo vivo y declara la interacción, sin adelantar deltas sobre
capabilities que aún no existen.

## D9 — Lo que queda fuera, con su razón

- **Sobrevivir reinicios del daemon**: fuera por el proposal, y además ya
  especificado — una sesión sin fin registrado se lista `interrupted`. El
  apagado ordenado (`cancel_all` + 5 s de gracia, `server.rs:2475-2494`) alcanza
  a una sesión que espera porque su `select!` incluye el `cancel`.
- **El tope de sesiones vivas totales**: la cota de esta change es sobre las
  *ociosas*, que son las que este cambio crea. Gobernar la concurrencia general
  es otra change, con su análisis.

## Defecto preexistente encontrado, y no arreglado aquí

`sdd/implement` y `worktree/dispatch` cierran con `set_state(Ended)` y **nunca
deregistran** (`server.rs:920`, `server.rs:1820`; documentado como intencional en
`session_finalize.rs:9-11`). Pero `handle_shutdown` espera con
`while !state.sessions.is_empty()` (`server.rs:2483`): tras cualquiera de esos
flujos, el apagado consume los 5 segundos completos y avisa «shutdown grace
elapsed with sessions still active» sin que quede sesión alguna. Es un defecto
real y ajeno a esta change; se declara aquí y se arregla en la suya.
