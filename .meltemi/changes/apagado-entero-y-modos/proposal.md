# apagado-entero-y-modos

> Vía completa: dos requisitos MODIFIED/ADDED sobre capabilities existentes y
> un crate nuevo en el workspace, así que no cabe en la vía rápida (criterio
> D7 de `artefactos-de-cada-push`). Nace de la revisión de un proyecto ajeno
> —claudian, plugin de Obsidian que pilota los mismos CLIs que nosotros— que
> el mantenedor pidió contrastar con nuestro enfoque de adaptadores el
> 2026-09-17. La revisión concluyó que nuestro enfoque resuelve lo mismo en
> una capa mejor, y dejó **dos cosas medidas** que hacemos peor. Esta change
> son esas dos cosas, y nada más de lo que allí se vio.

## Why

**Primera cosa, medida como defecto.** En Windows, un CLI instalado por npm
se resuelve a su shim `.cmd` —`claude.cmd`, `codex.cmd`, `gemini.cmd`,
`opencode.cmd`— y un `.cmd` corre bajo `cmd.exe`, que a su vez crea el
`node` (o el binario nativo) que hace el trabajo. Terminar el shim con
`TerminateProcess` —que es lo que hace `tokio::process::Child::kill`, lo que
hace `async_process::Child::kill`, y lo que hace `Stop-Process`— termina
solo a `cmd.exe`. Lo reprodujimos el 2026-09-17 con un shim cuyo cuerpo era
`node -e "setTimeout(()=>{},60000)"`: tras matar el shim, el `node` seguía
vivo.

Ese agujero lo tenemos en las dos capas que lanzan proveedores. Los
adaptadores propios (`core/meltemi-adapters/src/supervisor.rs`) cierran la
entrada, esperan la gracia y matan con `kill()` más `kill_on_drop(true)` —
cuyo comentario dice, literalmente, «no orphan holding a worktree open». Y el
daemon, para los agentes de nivel 1, delega el lanzamiento en
`agent_client_protocol::AcpAgent`, que mata con `Child::kill()` al soltar su
`ChildGuard` (`acp_agent.rs:237-240` del crate 1.2.0) — el mismo
`TerminateProcess`, sobre el mismo `cmd.exe`. La spec viva promete lo
contrario: `acp-session` → «Terminación sin huérfanos» dice que «no queda
ningún proceso huérfano». En Windows, con un shim, hoy eso es falso.

**Segunda cosa, medida en el crate.** El esquema ACP que pineamos
(`agent-client-protocol-schema 1.4.0`, `v1/agent.rs:1097-1102`) trae en las
respuestas de `session/new` y `session/load` **dos** campos: `modes`
—la forma original de la spec, «Session Modes», con `current_mode_id` y
`available_modes`— y `config_options`, la forma general que llegó después y
que la absorbe bajo la categoría `mode`. Nuestro daemon lee solo
`config_options` (`core/meltemid/src/acp.rs:257,283`). Un agente que anuncie
sus modos por el campo de modos —que es lo que hace un agente nacido antes
de las opciones de configuración— aparece en Meltemi sin opción alguna, y las
superficies, obedientes a «Sin opción anunciada no se ofrece el cambio en
vivo», no ofrecen nada. `modelo-y-esfuerzo-por-sesion` concluyó en su design
D9 que «ningún proveedor pineado anuncia opciones de sesión»; es posible que
lo que no anunciaban fuera la forma que mirábamos. Y en la misma línea: el
daemon no mira `current_mode_update` ni `config_option_update`, así que un
agente que cambia de modo por su cuenta deja al daemon con una opción que ya
no es cierta.

Las dos cosas son la misma lección: **leer entera la realidad del proveedor,
y terminarla entera.** Viajan juntas porque tienen la misma fecha y el mismo
origen, y el design las mantiene separables: ninguna tarea de una toca un
archivo de la otra.

## What Changes

### A. Apagado entero

- **Crate nuevo `core/meltemi-process`**: el único sitio con código de
  ámbito de proceso. En Windows, un *Job Object* con
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` por proceso proveedor: todo lo que ese
  proceso cree hereda la pertenencia, terminar el ámbito termina el árbol
  entero sea cual sea su profundidad, y **cerrar el handle también lo
  termina** — de modo que si el proceso de Meltemi que lo sostiene muere de
  golpe, el sistema cierra sus handles y el árbol muere con él. Fuera de
  Windows es un no-op declarado. Cero dependencias nuevas: `windows-sys` ya
  está pineado; gana la feature `Win32_System_JobObjects`, que es una
  bandera sobre una dependencia existente y no una dependencia.
- **Los adaptadores lanzan dentro del ámbito** (`supervisor.rs`): el `kill`
  tras la gracia pasa a terminar el ámbito, y el `kill_on_drop` pasa a ser
  verdad por construcción. La auditoría de dependencias del crate sigue
  verde: nada de red entra.
- **El daemon deja de delegar el lanzamiento de nivel 1**: sigue usando
  `AcpAgent` para parsear el comando y para lanzar (`spawn_process` es
  público y devuelve el hijo con su `id()`), pero conecta él mismo con la
  base de streams pública del crate (`Lines`), asigna el hijo a un ámbito y
  lo vigila. Ganancia lateral: el daemon conoce el pid y el binario efectivo
  del agente y los anota en el log, como los adaptadores ya hacen.
- **Spec**: `acp-session` → «Terminación sin huérfanos» se restata para decir
  lo que hoy no dice: terminar al agente termina también a lo que el agente
  necesitó para existir, y donde la plataforma permite atar esas vidas a la
  del daemon, se atan. `own-adapters` gana un requisito equivalente para el
  proveedor pilotado.
- **Tests con un intermediario real**: un `.cmd` que lanza un nieto y una
  aserción de que el nieto ya no está tras el apagado. Solo corre en Windows
  y lo dice. `docs/conformidad-manual.md` gana la comprobación contra un CLI
  real instalado por npm, opt-in como todo lo demás de esa página.

### B. Modos anunciados

- **`session_config::from_acp` recibe las dos formas**: si `config_options`
  no trae una opción de categoría `mode` y `modes` viene poblado, los modos
  se exponen como una opción de selección de categoría `mode`. Si vienen las
  dos, la opción de configuración **gana entera**; nada se fusiona.
- **El cambio viaja por el verbo que corresponde**: fijar esa opción manda
  `session/set_mode`, no `session/set_config_option`. El contrato no cambia
  —la superficie sigue viendo una `SessionConfigOption`— y por tanto no nace
  deber de paridad §4: TUI y GUI ya renderizan opciones.
- **Lo que el agente cambia solo se refleja**: `current_mode_update` y
  `config_option_update` actualizan lo anunciado. Hoy el daemon los vuelca al
  log como cualquier actualización y no toca su estado.
- **Verificación manual contra los agentes instalados**: qué forma anuncia
  cada agente de nivel 1 que este equipo tiene, persistida con fecha y
  versión. Opt-in, porque arranca un agente real.

## Capabilities

### Modified Capabilities

- `acp-session`: MODIFIED «Terminación sin huérfanos» (el bloque entero
  restatado, con dos escenarios nuevos); + ADDED «El modo anunciado se lee en
  cualquiera de sus dos formas»; + ADDED «Lo que el agente cambia por su
  cuenta queda reflejado».
- `own-adapters`: + ADDED «Terminar al proveedor es terminar todo lo que
  lanzó».

### New Capabilities

- Ninguna.

## Impact

- Workspace: crate nuevo `core/meltemi-process` (biblioteca, sin binarios);
  `meltemid` y `meltemi-adapters` dependen de él. Feature
  `Win32_System_JobObjects` en la entrada `windows-sys` del workspace. Cero
  dependencias externas nuevas en la clausura (§10): lo que entra al grafo es
  el crate propio. Sí se **declara** una que ya estaba: `meltemid` pasa a
  depender de `futures` de forma directa, porque el colector de `stderr` que
  el daemon replica del crate ACP usa sus traits de lectura asíncrona. Ya
  viajaba en el lockfile como dependencia del propio crate ACP, y se pinea
  igual que el resto.
- `core/meltemid`: `acp.rs` (lanzamiento con ámbito, pid en el log,
  actualizaciones de modo y de opciones), `session_config.rs` (dos formas),
  `server.rs` (ruteo de `set_mode`). `core/meltemi-adapters`:
  `supervisor.rs` y `tests/process_lifecycle.rs`.
- `proto/`: **un evento de log más, y nada del verbo**. La opción de modo
  viaja como `SessionConfigOption` y `session/set-config-option` sigue siendo
  la puerta, tal como se propuso. Lo que la implementación encontró es que el
  requisito «el identificador de proceso y el programa efectivo del agente
  SHALL constar en el log de sesión» no tenía dónde constar: `agent_resolved`
  y `session_started` se escriben **antes** del lanzamiento, así que ninguno
  puede llevar un pid que todavía no existe. Se añade `agent_process
  {pid, binary}` al catálogo de eventos del log y su entrada en el schema.
  Es aditivo —una variante nueva de una unión etiquetada, que ningún cliente
  anterior leía— y no toca ningún método ni ningún tipo de petición o
  respuesta.
- Docs: `docs/agentes.md` (sección Windows), `docs/conformidad-manual.md`
  (una comprobación más y una tabla de formas anunciadas),
  `docs/plan-de-cambios.md`.
- **Lo que solo Windows puede confirmar**: el test del intermediario corre
  solo ahí. En macOS y Linux los shims de npm son enlaces a scripts con
  shebang y el kernel entrega la señal al proceso real; el ámbito es un
  no-op y los tests lo declaran, no lo fingen.
- **Una ventana aceptada y escrita**: entre que `spawn()` devuelve y el hijo
  se asigna al ámbito, un intermediario podría haber creado ya a su nieto.
  La ventana es el tiempo que `cmd.exe` tarda en mapearse, parsear el script
  y llamar a `CreateProcess`; la asignación es la siguiente llamada al
  sistema. Eliminarla exige `CREATE_SUSPENDED` + `ResumeThread`, que ni
  `std` ni `tokio` exponen sin llamar a `CreateProcess` a mano. Se acepta,
  se nombra en el design, y el test con el `.cmd` real es quien la
  vigila.

## Fuera de alcance

- **Huérfanos en macOS/Linux si el daemon muere de golpe**: `kill_on_drop`
  tampoco corre en un crash allí. Es otra clase, sin medir, y su remedio es
  distinto (`PR_SET_PDEATHSIG` en Linux, nada equivalente en macOS). Se
  nombra; no se arregla aquí.
- **Resolver el shim al script que envuelve** (lanzar `node <cli.js>` sin
  `cmd.exe` en medio): no basta. El paquete npm de un agente nativo lanza
  desde JavaScript un segundo binario, y el formato del shim lo genera una
  herramienta que no es nuestra. El ámbito de proceso mata el árbol a
  cualquier profundidad y no lee formatos ajenos.
- **`turn/steer` y demás superficie nueva de Codex**: volcado el esquema del
  `codex-cli 0.77.0` instalado, declara exactamente cuatro peticiones de
  servidor (`item/commandExecution/requestApproval`,
  `item/fileChange/requestApproval` y sus gemelas de la generación
  anterior), que es lo que el adaptador cubre. `turn/steer`,
  `item/tool/requestUserInput`, `item/permissions/requestApproval`,
  `thread/fork|rollback|compact` **no existen en 0.77.0**. Son el mapa del
  próximo re-anclaje, no de esta change. Una frase de allá vale la pena
  guardar para cuando llegue: *relevar a mitad de turno se resuelve como
  aceptado solo con la aceptación nativa definitiva; ante ambigüedad se
  rechaza, para que quien llamó conserve el texto.*
- Todo lo demás que la revisión descartó por escrito: la vía del SDK, leer
  los almacenes nativos de los agentes, inyectar prompt de sistema en la
  config de un agente, herramientas dinámicas de Codex, alias de nombres de
  método ACP.
