# apagado-entero-y-modos — design

## Context

Verificado el 2026-09-17, contra el código, los crates pineados y un
experimento en Windows 11 (26200):

- **Cómo matan hoy las dos capas.** Los adaptadores propios:
  `supervisor.rs` lanza con `tokio::process::Command`, `kill_on_drop(true)`,
  cierra la entrada, espera la gracia (5 s) y llama a `Child::kill()`
  (`ProcessControl::kill`, `end()`). El daemon: `acp.rs:178` construye un
  `AcpAgent::from_args(&launch_argv)` y se lo entrega a `connect_with`; el
  crate lanza con `async_process::Command` en `spawn_process()` (público,
  `acp_agent.rs:170`) y mata al soltar `ChildGuard` (`acp_agent.rs:237`).
  Ninguna de las dos ve más allá del proceso que lanzó.
- **Qué mata `kill` en Windows.** `TerminateProcess` sobre el proceso
  nombrado. Un shim `.cmd` es `cmd.exe` ejecutando un script que crea al
  proceso real; matar el shim deja al real. Reproducido: shim con `node -e
  "setTimeout(()=>{},60000)"`, `Stop-Process` sobre el shim, `node` vivo
  después.
- **Qué se lanza como shim.** El catálogo resuelve en Windows por
  `WINDOWS_EXTS = ["exe", "cmd", "bat"]` (`supervisor.rs:195`,
  `fleet.rs`). Todo CLI instalado por `npm i -g` llega como `.cmd`: los dos
  de nivel 2 y, de nivel 1, `gemini`, `copilot`, `kilo`, `opencode` cuando
  vienen de npm. En este equipo `codex.cmd` existe junto a `codex.ps1`.
- **Qué promete la spec.** `acp-session` → «Terminación sin huérfanos»:
  «no queda ningún proceso huérfano». `own-adapters` → «Cancelación que
  llega y turno que dice la verdad»: «el adaptador MUST terminarlo al
  agotarse la gracia».
- **Las dos formas de anunciar modos.** `agent-client-protocol-schema
  1.4.0`, `v1/agent.rs`: `NewSessionResponse` y `LoadSessionResponse` llevan
  `modes: Option<SessionModeState>` (`current_mode_id`, `available_modes:
  Vec<SessionMode{id,name,description}>`) y `config_options:
  Option<Vec<SessionConfigOption>>`. `SessionUpdate` (`v1/client.rs:99`)
  tiene `CurrentModeUpdate`, `ConfigOptionUpdate` y
  `AvailableCommandsUpdate`. `SetSessionModeRequest{session_id, mode_id}` es
  el verbo. El daemon lee `config_options` en `acp.rs:257,283`; no toca
  `modes`, `set_mode` ni actualización alguna: `forward_update`
  (`acp.rs:602`) serializa la actualización al log y nada más.
- **Lo que el crate deja hacer.** `AcpAgent::spawn_process` es público y
  devuelve `(stdin, stdout, stderr, Child)`; `async_process::Child::id()`
  da el pid; `Lines` y `ByteStreams` son bases públicas de `ConnectTo`
  (`lib.rs:97-103`), y `connect_with` acepta cualquier `impl ConnectTo`.
  Nada de lo que esta change necesita del crate es privado.
- **Colisiones.** Sobre «Terminación sin huérfanos» no hay ningún MODIFIED
  sin archivar (los deltas vivos sobre `acp-session` son ADDED, salvo el
  MODIFIED de `sesion-que-espera` sobre «Dirección de una sesión
  existente»). Sobre `own-adapters` solo hay el ADDED de
  `preguntas-del-agente`. Un MODIFIED aquí es seguro.

## Goals / Non-Goals

**Goals**: que terminar un proveedor termine todo lo que ese proveedor
necesitó para existir, en las dos capas que lanzan proveedores; que la vida
de esos procesos quede atada a la de quien los lanzó donde la plataforma lo
permita; que el daemon lea el modo del agente en las dos formas en que la
spec lo anuncia y refleje lo que el agente cambia por su cuenta; cero
dependencias externas nuevas; tests que ejercitan un intermediario real.

**Non-Goals**: huérfanos por crash del daemon fuera de Windows; leer el
formato de los shims de npm; tocar el contrato `proto/`; re-anclar el
adaptador de Codex a una versión nueva; cualquier otra cosa de la revisión
que la originó.

## Decisions

### D1 — Un ámbito de proceso por proveedor, y es el kernel quien lo cierra

En Windows, cada proceso proveedor se asigna a un **Job Object** creado por
quien lo lanza, con `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Dos propiedades
que ninguna otra opción da a la vez:

1. **Terminar el ámbito termina el árbol entero**, a cualquier profundidad:
   `cmd.exe`, el `node` que crea, y el binario nativo que ese `node` cree a
   su vez. No hay que saber qué hay debajo del shim.
2. **Cerrar el handle también lo termina.** El handle lo sostiene el
   proceso que lanzó; si ese proceso muere —crash, `TerminateProcess`,
   cierre de sesión de usuario— el sistema cierra sus handles y el árbol
   muere. Esa es exactamente la promesa de `kill_on_drop`, cumplida por el
   kernel en vez de por un destructor que un crash nunca ejecuta.

Descartadas, por escrito:

- **`taskkill /pid <pid> /t /f`** (lo que hace claudian): cumple (1) pero no
  (2) — un proceso muerto no lanza `taskkill`. Además lanza un proceso más en
  el camino del apagado y hereda el riesgo de reutilización de pid.
- **Resolver el shim al script** (`node <cli.js>`, sin `cmd.exe` en medio):
  no cumple (1) cuando el script lanza un binario nativo, y obliga a leer el
  formato del shim, que genera `cmd-shim` y no nosotros.
- **Un job sobre el propio daemon**, para que todo muera con él: (2) ya está
  cubierta sin eso, porque los handles de cada ámbito viven en el daemon y
  el crash los cierra igual. Un job global además impediría terminar una
  sesión sin terminar las demás.

**Anidamiento.** Windows admite jobs anidados desde Windows 8 y nuestro piso
es Windows 10 1809: el daemon puede estar en un job de su lanzador, el
adaptador en el del daemon, y el CLI en el del adaptador. Cada ámbito
termina solo lo suyo.

**La ventana, aceptada.** Entre que `spawn()` devuelve y
`AssignProcessToJobObject` corre, el intermediario podría haber creado ya a
su hijo, y ese hijo quedaría fuera del ámbito. La ventana mide lo que
`cmd.exe` tarda en mapearse, parsear el script y llamar a `CreateProcess`;
la asignación es la primera llamada al sistema tras `spawn()`. Cerrarla del
todo exige lanzar suspendido (`CREATE_SUSPENDED`), asignar y reanudar el
hilo principal — y el handle del hilo lo devuelve `CreateProcess`, que ni
`std::process` ni `tokio::process` exponen. Se acepta con nombre; si el test
del intermediario real (D5) la observa alguna vez, la escalación es un
lanzamiento a mano con `CreateProcessW`, que es una change y no un parche.

**Fuera de Windows, no-op declarado.** Los shims de npm allí son enlaces a
scripts con shebang; `kill` llega al proceso real. Lo que un agente lance
por su cuenta —sus servidores MCP— es asunto del agente y de su `SIGTERM`,
como hoy. El ámbito existe en todas las plataformas como tipo, para que el
código que lanza no lleve `#[cfg]`; solo su cuerpo es de Windows.

### D2 — Un crate para el ámbito, y ningún nombre nuevo en el workspace

`core/meltemi-process`: biblioteca, sin binarios, un módulo. Expone
`Scope::new()`, `Scope::adopt(&Child)` (tokio) y `Scope::adopt_pid(u32)`
(para un hijo que lanzó otro), `Scope::end()` (`TerminateJobObject`) y
`Drop` (cierra el handle, que es lo que mata). Depende de `windows-sys`
—ya pineado— bajo `cfg(windows)`, y la entrada del workspace gana la feature
`Win32_System_JobObjects` (`CreateJobObjectW`, `SetInformationJobObject`,
`AssignProcessToJobObject`, `TerminateJobObject`); `OpenProcess` ya está en
`Win32_System_Threading` y `CloseHandle` en `Win32_Foundation`. Una feature
sobre una dependencia pineada no es una dependencia nueva, y se dice así en
vez de esconderlo.

Por qué un crate y no un módulo en `meltemi-client`: ese crate es la pila
*cliente* del daemon —transporte, RPC, bootstrap— y los adaptadores no son
clientes del daemon; enlazarlo les metería un `Listener` que no usan y una
pregunta que no deberían tener que responder. Por qué no dos copias: es FFI
`unsafe`, y dos sitios donde equivocarse es uno de más. El precedente de la
casa que parecía ir en contra —D5 de `adaptadores-propios-acp` descartó un
socket local para no meter «código inseguro específico de plataforma» en el
crate de adaptadores— se respeta: el código inseguro no entra en ese crate,
entra en uno cuyo único oficio es exactamente eso, y la alternativa con
biblioteca estándar que allí existía aquí no existe (D1: `taskkill` no cubre
el crash).

La auditoría `tests/dependency_audit.rs` de los adaptadores sigue verde:
`meltemi-process` no enlaza red. Y `deny.toml` sigue mandando.

### D3 — El daemon lanza con ámbito sin dejar de usar el crate oficial

Hoy `connect_with(agent, …)` recibe el `AcpAgent` y el crate lanza por
dentro. Para asignar el hijo a un ámbito hay que verlo, y `spawn_process`
es público: el daemon lo llama él mismo, toma `child.id()`, lo adopta en un
`Scope`, y construye el transporte con `Lines::new(sink, stream)` sobre
`stdin`/`stdout` del hijo — la misma base que `AcpAgent::connect_to` usa
por dentro (`acp_agent.rs:280-372`). Lo que se replica del crate es
pequeño y se cita: el colector de `stderr`, el vigilante del hijo que hace
fallar la conexión si el proceso muere antes de tiempo, y la carrera entre
el protocolo y ese vigilante. Lo que **no** se replica es el parseo del
comando (`from_args` sigue siendo del crate) ni el lanzamiento en sí.

Por qué no un trampolín (un `.exe` nuestro que crea el job, lanza el CLI y
releva stdio): un proceso más por sesión, una bomba bidireccional de bytes
que escribir y probar, y un código de salida que propagar — todo para no
llamar a una función que ya es pública.

Ganancia que no se pidió y se toma: el daemon conoce ahora el pid y el
programa efectivo del agente y los anota en el log de sesión
(`SessionEventKind::AgentResolved` ya existe para el binario; el pid es
información de diagnóstico, no de contrato). Los adaptadores lo hacen desde
`adaptadores-propios-acp`; el nivel 1 se pone a la par.

### D4 — El crash del daemon queda cubierto sin trabajo adicional

Cada `Scope` es un handle en el proceso del daemon (nivel 1) o del adaptador
(nivel 2). Un crash cierra los handles del proceso muerto; cerrar el handle
termina el job. Luego: nivel 1, el daemon muere y los agentes mueren; nivel
2, el adaptador muere y el CLI muere, y si el daemon muere, muere el
adaptador (lo lanza el crate con el mismo `kill`… sobre un `.exe` propio,
sin shim, así que ahí sí llega) y con él su CLI. No hay que hacer nada más,
y se escribe para que nadie añada un job global «por si acaso».

### D5 — Tests con un intermediario de verdad, y lo manual dicho como manual

- **`meltemi-process`**: un test que escribe un `.cmd` cuyo cuerpo lanza un
  nieto que se queda (`ping -n 60 127.0.0.1`, presente en todo Windows),
  lo adopta en un ámbito, termina el ámbito y comprueba que el nieto no
  existe. `#[cfg(windows)]`, con un gemelo en las otras plataformas que
  comprueba que el ámbito es inerte y no rompe nada. Es el test que
  vigila la ventana de D1.
- **Adaptadores**: `tests/process_lifecycle.rs` gana el caso que no tenía:
  un `.cmd` que ignora el fin de entrada, la gracia se agota, el `kill`
  termina el ámbito y el nieto se va. Hasta hoy solo se probaba que cerrar
  la entrada termina un hijo real.
- **Daemon**: el e2e con `mock-agent` sigue igual (es un `.exe`); un test de
  Windows lanza `mock-agent` a través de un `.cmd` y comprueba que cancelar
  la sesión no deja el nieto.
- **Manual y opt-in**: `docs/conformidad-manual.md` gana la comprobación
  contra un CLI real instalado por npm — abrir sesión, cancelar, listar
  procesos — con el mismo doble cerrojo de siempre. Y la tabla de D8.

### D6 — Dos formas de anunciar el modo, una precedencia, ninguna fusión

`session_config::from_acp(config_options, modes)`:

- Si `config_options` trae una opción cuya `category` sea `mode`, se usa
  tal cual y `modes` se ignora entero. No se cruzan valores: una opción de
  configuración con los modos de otro campo sería una opción que nadie
  anunció.
- Si no la trae y `modes` viene poblado, se sintetiza una opción de
  selección: `id = "mode"`, `category = "mode"`, valores =
  `available_modes` (id, name, description), actual = `current_mode_id`.
- Si no viene ninguna de las dos, la lista queda vacía — y sigue siendo la
  respuesta que impide ofrecer el cambio en vivo, como hasta hoy.

El id `mode` no colisiona: si el agente anunciara una opción de
configuración con ese id, estaríamos en el primer caso y la síntesis no
ocurre. El daemon recuerda por sesión que la opción de modo es de forma
heredada; es estado interno, no viaja en el contrato.

`proto/` no se toca. La superficie ve una `SessionConfigOption` más, la
renderiza como cualquier otra, y por eso no nace deber de paridad §4: las
tres superficies ya consumen las opciones de sesión por ese mismo tipo desde
`modelo-y-esfuerzo-por-sesion`.

### D7 — Fijar el modo viaja por su verbo, y se cree al agente

`session/set-config-option` sobre la opción de forma heredada manda
`session/set_mode` con el `mode_id` elegido, y no `set_config_option`
(que el agente no entendería). `SetSessionModeResponse` no devuelve lista;
la aceptación es la respuesta sin error. El daemon fija entonces el valor
actual al pedido —el agente lo aceptó— y deja que `current_mode_update`, si
llega, tenga la última palabra. La regla de `modelo-y-esfuerzo` («lo que el
agente reporta es el registro, no lo que pedimos») se conserva en su
espíritu: se cree al agente, y el agente habló al aceptar.

Las actualizaciones que hoy solo se vuelcan al log pasan a tocar el estado:
`current_mode_update` cambia el valor actual de la opción de modo (de
cualquiera de las dos formas); `config_option_update` reemplaza la lista
anunciada **entera** por la que el agente manda — es lo que el agente dice
que tiene ahora, y fusionar sería inventar. En los dos casos se anota
`ConfigOptionsAnnounced` en el log, que es el evento que ya existe para
«esto es lo anunciado ahora», y las superficies lo ven por el mismo
`session/event` de siempre.

`AvailableCommandsUpdate` se ve en el mismo `match` y **no** se toma:
comandos no son opciones, y darles sitio es otra change.

### D8 — Verificar contra lo instalado, antes de creer nada

`modelo-y-esfuerzo` afirmó que ningún proveedor pineado anuncia opciones
mirando una forma. Esta change no repite el error en dirección contraria:
la tarea 2.3 abre una sesión ACP con cada agente de nivel 1 instalado en
este equipo y anota qué campo pobló cada uno, con versión y fecha, en
`docs/conformidad-manual.md`. Es manual y opt-in porque arranca agentes
reales con la cuenta de quien la corre (§5). Si resultara que todos pueblan
`config_options`, la síntesis de D6 sigue siendo correcta —la spec admite las
dos formas y el crate las expone— y lo que cambia es la nota del backlog:
diría que la conclusión de D9 se sostiene por otra razón.

### D9 — Dos partes, un origen, y se separan sin cortar nada

A y B no comparten archivo: A toca `meltemi-process`, `supervisor.rs`,
`process_lifecycle.rs` y el lanzamiento en `acp.rs`; B toca
`session_config.rs`, `server.rs` y la recepción de actualizaciones en
`acp.rs`. Las tareas están numeradas por parte, cada commit es de una sola,
y cualquiera de las dos se puede revertir por commits sin tocar la otra.
Viajan juntas porque nacieron de la misma revisión y de la misma lección;
si la compuerta prefiere dos changes, el corte es por el número de sección.

## Risks / Trade-offs

- **FFI `unsafe` nuevo en el workspace.** Acotado a un crate de un módulo,
  con un test que lo ejercita contra un proceso real y no contra un doble.
  Es la misma clase de código que el daemon ya tiene para la ACL del pipe.
- **La ventana de D1.** Aceptada con nombre y vigilada por el test. Si se
  observa, la escalación está escrita.
- **Un agente que cambia de modo en mitad de un turno** hace que la fila
  de la barra cambie de valor bajo el cursor. Es información, no layout: la
  regla de que nada se anima sigue en pie y la fila no se mueve de cubeta
  por un modo.
- **Replicar ~60 líneas del crate** (D3) es deuda de mantenimiento si el
  crate cambia su `connect_to`. Se cita la versión (1.2.0) y las líneas, y
  el test de handshake existente detecta un desfase antes que nadie.
