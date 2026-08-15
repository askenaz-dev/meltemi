# Smoke conducido — `acceso-remoto-en-dos-vias`

**Fecha**: 2026-08-15 · **Plataforma**: Windows 11 (el caso que la change
existe para desbloquear) · **Binario**: `target/release/meltemi.exe` contra un
`meltemid` de release, en directorios de datos y configuración aislados.

## Qué se verificó

El puente se condujo **a través de una capa de shell** (`cmd /c meltemi
bridge`) con stdio canalizado y sin terminal, que es lo que un `ssh` remoto
añade menos la red: comillas del shell y entrada/salida no interactiva.

| Comprobación | Medido |
| --- | --- |
| El handshake cruza el puente | `initialize` → `{"protocolVersion":1,"daemon":{...}}` |
| El daemon acepta la conexión como cualquier local | sin error; no distingue el origen |
| Una segunda llamada responde por el mismo canal | `status` → `{"daemonVersion":"0.1.1","uptimeSeconds":1368,...}` |
| stdout es el canal y stderr queda limpio | stderr vacío |
| Cerrar el extremo cierra el puente | código de salida 0, sin proceso huérfano |

Los cuatro escenarios del requisito del puente están además cubiertos por
`tui/tests/bridge.rs`, que **enlaza un endpoint real con el mismo `Listener`
que usa el daemon** — en Windows, un named pipe de verdad — y conduce el
binario real por stdio canalizado.

## Lo que NO se verificó, y por qué

**El tramo `sshd → bridge` no se ejercitó.** El servidor OpenSSH de Windows
está `NotPresent` en esta máquina; instalarlo exige privilegios de
administrador y modifica la superficie de seguridad del equipo, así que no se
hizo por iniciativa propia. Queda por confirmar, cuando el mantenedor habilite
`OpenSSH.Server`:

1. Que `ssh <este-host> meltemi bridge` entrega el mismo canal completo.
2. Que el shell por defecto de sshd en Windows no altera el flujo de bytes
   (CRLF, o `cmd` interponiéndose). JSON-RPC delimitado por líneas es texto y
   tolerante, pero **eso es una expectativa, no una medición**.

El design lo declaró antes de implementar (D6): CI no tiene un sshd y no se
finge uno de juguete, porque un fixture más educado que la cosa real esconde
exactamente el fallo que importa — la lección que ya costó dos veces en los
adaptadores propios.

## Hallazgos de la implementación

Dos defectos que solo la medición encontró, ambos en el camino de vuelta
daemon→cliente, y el segundo era el verdadero:

1. **`tokio::io::copy` vacía solo al EOF.** Un protocolo de petición/respuesta
   espera un búfer que solo se llena si la conversación continúa, y no puede
   continuar porque lo retenido es la respuesta. Cada dirección copia ahora con
   vaciado por trozo.
2. **Abrazo mortal del candado de stdout.** `main` retiene
   `io::stdout().lock()` durante todo el `dispatch`; el candado de Rust es
   reentrante **por hilo**, y `tokio::io::stdout()` escribe desde un hilo del
   pool de bloqueo, donde ya no lo es. La instrumentación lo mostró sin
   ambigüedad: `read 120 / wrote 120` en la ida, `read 129` en la vuelta y
   ningún `wrote`. El puente escribe ahora por el escritor que el proceso ya
   bloqueó, y por eso se despacha desde `dispatch` y no desde `execute`.

Y una lección de método: un test que falla dejaba vivo el proceso hijo, y ese
huérfano retenía el binario construido, de modo que el siguiente `cargo build`
fallaba con un bloqueo de archivo que no se parece en nada a su causa. Los
hijos del e2e llevan `kill_on_drop`.
