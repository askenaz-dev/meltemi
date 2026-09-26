# Design — via-ssh-del-cliente

## Context

`meltemi bridge` ya pone el endpoint local del daemon en su propio stdio, y lo
hace incluso con el named pipe de Windows, que ningún reenvío de SSH alcanza.
`ssh pc meltemi bridge` es, hoy, un canal JSON-RPC completo. Lo que no existe
es un cliente que lo hable: `transport::connect`
(`core/meltemi-client/src/transport.rs:41`) solo sabe de sockets y de pipes.

Así que el trabajo no es inventar un transporte remoto. Es **enseñarle al
cliente a hablar por un proceso** en vez de por un descriptor de socket, y
dejar que el `ssh` del usuario sea quien cruce la red. Eso mantiene intactas
las tres cosas que no se tocan: el daemon sigue sin transporte de red (§3),
Meltemi sigue sin material de claves (§2), y no entra ningún crate
(`deny.toml` prohíbe con razón todo cliente TLS del workspace).

Dos piezas del repositorio se usan tal cual:

- `connect_or_start` (`core/meltemi-client/src/bootstrap.rs:44`), que ya sabe
  **no** arrancar daemons en endpoints ajenos y ya tiene escrita la razón.
- `meltemi-process::Scope`, que en Windows ata la vida de lo que se lanza a la
  del lanzador. Un `ssh` huérfano sosteniendo un túnel es exactamente la clase
  de resto que ese crate existe para impedir.

## Goals / Non-Goals

**Goals**

1. Que un endpoint de la forma `exec:<comando>` haga que el cliente lance ese
   comando y hable JSON-RPC por su stdio, con la misma delimitación por líneas.
2. Que las tres superficies lo consuman sin saber que es distinto.
3. Que un endpoint `exec:` **jamás** arranque un daemon.
4. Que el comando sea de ámbito usuario y nunca de proyecto.
5. Que al cerrarse el cliente no quede ningún proceso detrás.

**Non-Goals**

- Perfiles con nombre, presencia, selector de máquinas: `maquinas-remotas`.
- Que Meltemi gestione claves, passphrases o hosts conocidos: son del `ssh`
  del usuario, y §2 lo prohíbe.
- Cualquier transporte de red propio, relé o puerto en el daemon (§3).

## Decisions

### D1 — El endpoint dice cómo se habla, no solo dónde

Un endpoint que empieza por `exec:` significa «lanza esto y habla por su
stdio». El resto de la cadena es el comando, verbatim.

Se elige la forma del endpoint —y no una opción de configuración aparte—
porque el endpoint ya es **el único lugar** por donde las tres superficies
eligen con quién hablan, y ya viaja por `MELTEMI_ENDPOINT`, por la
configuración de usuario y por los argumentos. Una opción paralela habría
creado un segundo sitio donde decidir lo mismo, con la pregunta inevitable de
cuál gana.

El prefijo es legible a simple vista, que importa: cuando algo falle, lo que el
usuario tenga delante dirá `exec:ssh pc meltemi bridge` y no un identificador
opaco.

### D2 — Se lanza el proceso, no un shell

El comando se parte en programa y argumentos aquí, respetando comillas dobles,
y se lanza directamente. **No** se invoca `sh -c` ni `cmd /c`.

Eso no es purismo. Un shell intermedio sería un segundo intérprete entre lo que
el usuario escribió y lo que corre, con sus expansiones, sus comodines y su
manera propia de tratar una cadena; y sería, además, otro proceso entre el
cliente y el `ssh` — exactamente la forma de la que `apagado-entero-y-modos`
aprendió que se cuelan los huérfanos. Lanzar directo hace que el hijo que el
cliente sostiene sea el `ssh`, y no un intérprete que lo lanzó.

El precio, dicho: sin shell no hay tuberías ni redirecciones en la cadena. Quien
las necesite escribe un script y lo nombra, que es lo que un shell habría hecho
con menos control.

### D3 — Ámbito usuario, y el proyecto no tiene voz

El comando se lee de `MELTEMI_ENDPOINT` o de la configuración de usuario.
**Nunca** del `.meltemi/config.toml` de un repositorio.

Esta es la única regla de la change que no admite matices: un repositorio que
pudiera fijar el endpoint del cliente sería un repositorio que ejecuta un
comando en tu máquina por el hecho de abrirlo. Clonar es una acción común y no
debería ser una decisión de seguridad. El daemon tampoco lo lee: esto es cosa
del cliente, y el daemon no sabe que existe.

### D4 — Un endpoint `exec:` jamás arranca un daemon

`connect_or_start` ya rehúsa arrancar donde no corresponde, pero su regla es
comparar con el endpoint propio. Un `exec:` nunca va a coincidir, así que la
regla **ya lo cubre** por accidente. Se escribe explícitamente igual, con su
test, porque una garantía que se sostiene por accidente es una garantía que la
próxima refactorización se lleva sin enterarse.

Y hay una razón concreta detrás: arrancar un daemon local porque un `ssh`
remoto no respondió es servir una máquina en el nombre de otra.

### D5 — Nadie va a poder escribir una passphrase, y se dice en vez de colgarse

El stdio del subproceso **es** el canal JSON-RPC. No es un terminal, y no puede
serlo: si lo fuera, el diálogo del protocolo se mezclaría con el de `ssh`.

De ahí se sigue lo incómodo: un `ssh` que necesite aceptar un host desconocido
o pedir una passphrase no tiene dónde preguntar. Lo que hace `ssh` entonces
depende de la plataforma —buscar un terminal, usar un programa de askpass, o
fallar—, y ninguna de las tres es algo que Meltemi deba adivinar.

La decisión: Meltemi **no** inyecta banderas en el comando del usuario. No
añade `BatchMode=yes`, ni `-o` de ningún tipo. El comando es el del usuario,
entero, y meterle opciones sería decidir por él sobre su `ssh`. Lo que sí hace
es **leer el error**: cuando el proceso muere antes de hablar el protocolo, el
rehúse dice lo que pasó y el remedio —aceptar el host y cargar la clave en el
agente desde un terminal, una vez—, con las primeras líneas de lo que el
proceso escribió en su `stderr`, que es donde `ssh` explica de verdad lo que le
pasó.

Meltemi nunca ve ni reenvía una passphrase, y esa es la propiedad que se
conserva por construcción: no hay ningún camino por el que una llegue aquí.

### D6 — El `stderr` del proceso es diagnóstico, y se guarda acotado

`ssh` escribe sus quejas en `stderr`. Si se descarta, un fallo de conexión es
un «no se pudo» sin causa; si se deja fluir a la consola, ensucia la salida de
la TUI y de la GUI, que no tienen consola.

Se recoge en un buffer acotado (las últimas líneas), se adjunta al error cuando
el proceso muere, y se descarta cuando la conexión prospera. Acotado porque un
proceso ruidoso no debe crecer en memoria del cliente sin techo.

### D7 — El proceso muere con el cliente, y en Windows lo garantiza el kernel

El subproceso se lanza dentro de un `Scope` de `meltemi-process`. En Windows
eso significa que un cliente que se cierra —o que muere de golpe— se lleva el
`ssh` consigo, porque cerrar el handle del ámbito termina lo que tenga dentro.
Fuera de Windows el ámbito es inerte y basta con matar al hijo, que es el
propio `ssh`: no hay intermediario porque D2 quitó el shell.

Sin esto, cada arranque y cierre de la GUI dejaría un `ssh` vivo sosteniendo
una sesión remota. Con esto, no queda ninguno.

### D8 — El cromo dice por dónde habla

La barra de estado de las dos superficies muestra el comando abreviado cuando
el endpoint es `exec:` — «vía `ssh pc`» —, porque un usuario que ve su flota
tiene derecho a saber si la está viendo en su máquina o en otra. Un cliente que
callara la diferencia haría que un borrado en la máquina equivocada pareciera
un borrado en la propia.

### D9 — El endpoint no viaja al entorno del hijo

`MELTEMI_ENDPOINT` se **quita** del entorno del proceso que se lanza.

La razón es un bucle concreto: si el comando ejecuta a su vez un `meltemi`
—un ayudante local, un script envoltorio—, heredaría este mismo endpoint y
marcaría contra sí mismo. Con `ssh` remoto no pasa, porque el shell del otro
lado no hereda nada; con cualquier comando local, sí.

Se quita en vez de sobrescribirse, para que el hijo caiga en el valor por
defecto de **su** máquina, que es lo correcto tanto para el `ssh` remoto como
para un ayudante local. Un script que quiera un endpoint concreto lo fija él,
después de heredar, y eso sigue funcionando.

## Risks / Trade-offs

- **Sin shell, sin tuberías.** Declarado en D2 y con remedio: un script.
- **El primer intento contra un host nuevo va a fallar.** Es inevitable por D5
  y es la razón de que el rehúse lleve remedio y `stderr`. Se acepta: la
  alternativa era gestionar claves ajenas, que §2 prohíbe.
- **Latencia.** Cada petición cruza un `ssh`. Nada de lo que esta change hace
  la cambia, y el sondeo que la habría amplificado ya lo quitó
  `estado-en-vivo`.
- **Un comando mal escrito lanza un proceso.** Acotado por D3: solo lo que el
  usuario puso en su propia configuración o en su propio entorno.

## Migration Plan

Aditivo entero. Un endpoint sin el prefijo se comporta exactamente como hoy, y
esa es la ruta de todos los usuarios actuales. El contrato no se mueve y el
daemon no se entera.

## Open Questions

- Si `maquinas-remotas` acaba guardando comandos por máquina, ¿los guarda como
  endpoints `exec:` o como partes que compone? Se decide allí, con su caso de
  uso delante.
