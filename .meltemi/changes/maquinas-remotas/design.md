# Design — maquinas-remotas

## Context

Verificado el 2026-10-01, sobre `main` en `df6c75d`:

- **El endpoint se resuelve una vez y no cambia.** `paths::endpoint()` mira
  `MELTEMI_ENDPOINT`, luego `[client] endpoint` de la configuración **de
  usuario** (`paths.rs:50-58`, `configured_endpoint` en `:73`), luego el
  default de la plataforma. La GUI lo resuelve al arrancar y se lo pasa al
  actor del puente (`desktop/src/lib.rs:244`), que lo captura como `String` y
  reconecta siempre al mismo (`bridge.rs:199-231`). La TUI hace lo propio en
  `main.rs`. No hay forma de cambiarlo sin reiniciar el proceso.
- **El canal remoto ya existe entero.** `exec:<comando>` lanza el comando y
  habla JSON-RPC por su stdio, sin shell, dentro de un ámbito de proceso y de
  ámbito usuario (`meltemi-client/src/exec.rs`, `via-ssh-del-cliente`).
  `bootstrap::may_start` devuelve `false` para cualquier `exec:`
  incondicionalmente: un endpoint remoto nunca arranca un daemon local.
  `via()` ya extrae «ssh my-pc» del endpoint para el cromo
  (`desktop/ui/src/lib/via.ts`, `tui/src/shell/render.rs`).
- **La presencia de equipos ya existe**, y es de solo lectura: `identity/status`
  responde `machines[]` con `presence`, `system`, `lastSeen` y `thisMachine`
  leyendo el cliente oficial de la malla del usuario (`identidad-propia`,
  aterrizada en `b359bb9`, archivada en `df6c75d`). El requisito vivo
  «Presencia no es control» prohíbe que una superficie abra nada por el hecho
  de que un equipo esté en línea, y el camino sigue siendo el túnel del
  usuario.
- **El requisito vivo que esta change modifica** reserva hoy «el selector que
  conecta a un equipo» como nota de design de fase 3
  (`.meltemi/specs/remote-access/spec.md:118-121`), junto con los certificados
  SSH del bastión y el aviso de espera —éste último ya implementado y anotado
  como tal—.
- **El cambio de proyecto ya tiene su guarda.** La GUI mantiene `activeProject`
  y `switchProject` (`stores.ts:240`), las pestañas viven en `session-tabs.ts`
  y hay un test de trabajo sin guardar (`desktop/ui/tests/dirty.test.ts`). Es
  el mecanismo que el cambio de máquina reutiliza, no uno nuevo.
- **Nada agrega sesiones de varias máquinas.** `session/list` es del daemon al
  que estás conectado; no existe ni podría existir un agregador sin cruzar §3.

## Goals / Non-Goals

**Goals**: que el usuario tenga sus máquinas con nombre en el cliente, pueda
cambiar de una a otra sin reiniciar, pueda probar una conexión antes de
confiar en ella, y vea la presencia de sus equipos cuando la pida.

**Non-Goals**: material de claves (ni `IdentityFile`, ni passphrase, ni
puerto: eso es del `~/.ssh/config` del usuario); agregar sesiones o buscar
entre máquinas; sincronizar configuración entre equipos; que el daemon sepa
que existen otras máquinas.

## Decisions

### D1 — La lista es configuración de usuario, y de nadie más

```toml
# <config>/meltemi/config.toml
[[client.machines]]
name = "esta"
endpoint = "local"                                  # el default de la plataforma

[[client.machines]]
name = "la-torre"
endpoint = "exec:ssh la-torre meltemi bridge"
```

De **ámbito usuario únicamente**, por la misma razón que el endpoint suelto y
sin ninguna nuance: un endpoint puede nombrar un comando, así que un
repositorio capaz de fijarlo ejecutaría algo en tu máquina por el hecho de
abrirlo (`via-ssh-del-cliente` D3). `configured_endpoint` ya toma el directorio
de configuración y ninguna raíz de proyecto; la lista se lee por el mismo
camino y con la misma firma.

`endpoint = "local"` es azúcar para «el default de esta plataforma», porque
escribir un named pipe con el SID del usuario a mano es una invitación a
equivocarse. Cualquier otro valor se pasa tal cual a `connect`, que ya sabe
distinguir `exec:` de una ruta de socket.

**Sin material de claves, por escrito.** Ni `IdentityFile`, ni `Port`, ni
usuario, ni passphrase: el comando es `ssh <alias> meltemi bridge` y el alias
lo resuelve el `~/.ssh/config` del usuario con su agente de claves. Meltemi no
gestiona claves (§2) y un campo para una ruta de clave sería el primer paso
hacia leerla.

### D2 — Cambiar de máquina es reconectar, no reiniciar

El actor del puente deja de capturar `endpoint` como constante y lo lleva como
variable del bucle; gana un `BridgeCommand::Switch(String)` que lo cambia y
corta la conexión viva para que el bucle reconecte contra el nuevo. Es el
cambio mínimo que sirve: el bucle de reconexión, el backoff y el emisor de
avisos ya están donde tienen que estar y no se tocan.

La TUI hace lo equivalente en su `conn.rs`, que ya tiene el mismo bucle.

**Lo que el cambio arrastra** es lo que ya arrastra cambiar de proyecto: las
pestañas de sesión apuntan a sesiones de **otro** daemon y dejan de tener
sentido. Se cierran con la guarda de trabajo sin guardar que ya existe, porque
inventar una segunda guarda para el mismo peligro es cómo se consiguen dos
comportamientos distintos ante la misma pregunta.

### D3 — Probar una conexión es `initialize` y nada más

«Probar conexión» abre el endpoint, hace `initialize`, anota la versión del
daemon y el tiempo que tardó, y **cierra**. No deja la conexión abierta, no
cambia la máquina activa y no arranca nada: `may_start` ya devuelve `false`
para `exec:`, así que probar una máquina remota apagada **falla**, que es
exactamente la respuesta correcta y no un arranque remoto sorpresa.

El resultado es una de tres cosas, y las tres se dicen: respondió (con versión
y latencia), no respondió (con el diagnóstico del transporte, que
`via-ssh-del-cliente` ya hace llegar entero), o la configuración no se puede
leer.

### D4 — Presencia a demanda, jamás un sondeo

La lista muestra presencia **solo cuando el usuario la abre o la pide**, y
nunca de fondo. Dos razones y las dos pesan: un sondeo periódico hacia máquinas
remotas levantaría conexiones `ssh` que nadie pidió, y la presencia de la malla
ya se lee en una llamada local (`identity/status`) que no toca la red de nadie.

La presencia que se muestra es **la de la malla**, no una prueba de conexión:
que un equipo esté en línea no dice que allí corra `meltemid`. Eso se sabe al
probar o al conectar, nunca se supone — que es el requisito vivo «Presencia no
es control» dicho en esta superficie. La fila cruza la máquina configurada con
la máquina de la malla **por nombre**, y cuando no cruzan no inventa: la fila
existe igual, sin presencia.

### D5 — Un daemon por máquina, siempre

Nada agrega sesiones de varias máquinas. Lo que se ve es lo de la máquina
activa, y el cromo dice cuál. La búsqueda entre máquinas que ofrece Orca es
exactamente lo que §3 no permite sin un agregador, y un agregador es un
servicio.

### D6 — Dónde se ve

- **GUI**: sección «Máquinas» en Ajustes (la lista con su endpoint, el botón de
  probar y el de cambiar) y el nombre de la máquina activa en la barra de
  estado, junto al `vía ssh …` que ya está. El selector no es una vista nueva:
  las cuatro vistas son una promesa documentada.
- **TUI**: `machines` / `maquinas` lista, y `machine <nombre>` / `maquina
  <nombre>` cambia. Sin vista nueva por la misma razón.
- **CLI**: `machines` lista con `--json`/`--yaml`. Cambiar la máquina activa
  desde un verbo de una sola corrida no tendría a quién cambiársela —cada
  invocación resuelve su endpoint y termina—, así que el CLI **lista y no
  cambia**, y lo dice.

### D7 — Cero RPC, cero daemon

Ni un método nuevo ni un campo nuevo en el contrato. El daemon no sabe que
existen otras máquinas y no debe saberlo: cada `meltemid` es de su máquina. Por
eso esta change no toca `proto/` ni `core/meltemid/`, y la paridad de núcleo
(§4) no entra en juego —no hay capacidad de daemon que consumir—; lo que sí
hay es cortesía de superficie, que esta change da en las tres.

## Risks / Trade-offs

- **Cambiar de máquina con trabajo sin guardar.** Mitigado reutilizando la
  guarda del cambio de proyecto, no con una nueva.
- **Un nombre de máquina que no cruza con la malla.** La fila no inventa
  presencia; se muestra sin ella. Peor sería adivinar por sufijo.
- **Una máquina remota apagada tarda en fallar.** El `exec:` hereda el tiempo
  que tarde `ssh`; la prueba de conexión corre con su propio límite y lo dice
  al vencer, en vez de quedarse colgada.
