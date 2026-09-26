# Design — aviso-de-espera-opt-in

## Context

La spec `remote-access` ya dejó **una** puerta abierta y sin usar: el usuario
puede configurar un aviso de espera opt-in, emitido por «un cliente conectado o
un proceso del propio usuario —nunca el daemon»— a un endpoint que el usuario
opera. Esta change la atraviesa, y nada más.

Lo que hay hoy alrededor:

- `avisos-de-escritorio` (archivada, 11/11) reclama atención **con la app
  abierta**: bandeja, notificación del sistema, y la campana del terminal bajo
  `MELTEMI_BELL=1`. Nada de eso cruza hasta un teléfono.
- `estado-en-vivo` (archivada en esta tanda) hace que el cliente sepa
  **cuándo** algo empieza a esperar sin sondear, que es la señal que este aviso
  necesita.
- El único push de facto que existe es la campana sobre `ssh -t`, que las apps
  de terminal convierten en notificación local. Útil, indocumentado, y solo con
  la TUI en pantalla.

La restricción que da forma a todo lo demás es §9: **el contenido exacto del
aviso se especifica antes de existir.** Así que este documento no describe una
implementación; fija un contrato de contenido, y el código lo obedece.

## Goals / Non-Goals

**Goals**

1. Que algo llegue al teléfono del usuario cuando una sesión empieza a
   esperarle, usando infraestructura que él opera.
2. Que el contenido esté fijado y sea mínimo, y que esté publicado con la
   change.
3. Que esté apagado por defecto y sea de ámbito usuario.
4. Que el daemon no ejecute nada ni sepa que esto existe.
5. Que un comando que se cuelga no cuelgue el cliente.

**Non-Goals**

- Relé, puerto o push desde el daemon (§3).
- Contenido del proyecto en el aviso, aunque el usuario lo pida (§9).
- Una app móvil que lo reciba: `companero-movil`.

## Decisions

### D1 — El contrato del contenido, y por qué es tan poco

El comando recibe **cinco variables de entorno** y nada más:

| Variable | Contenido |
| --- | --- |
| `MELTEMI_WAITING` | `permission`, `instruction` o `gate` |
| `MELTEMI_SESSION` | El identificador de sesión, tal cual |
| `MELTEMI_MACHINE` | El nombre de host de la máquina del cliente |
| `MELTEMI_AT` | El instante, RFC 3339 en UTC |
| `MELTEMI_COUNT` | Cuántas cosas esperan ahora mismo |

No hay ruta de proyecto, ni nombre de repositorio, ni texto de la petición, ni
nombre de herramienta, ni título de sesión. Y la razón no es timidez: el destino
es un servicio que el usuario opera pero que **no siempre opera solo** —un ntfy
autohospedado tiene un topic, y un topic es una URL que alguien más puede
conocer—. Lo que viaja tiene que ser inútil para quien no sea el usuario y
suficiente para que él decida si vale la pena volver.

«Algo espera en esta máquina» es exactamente eso. El identificador de sesión
viaja porque sin él dos avisos seguidos no se distinguen; es opaco y no dice de
qué proyecto es.

`MELTEMI_COUNT` se añade a lo que la propuesta pedía, y se justifica: sin él,
un usuario que ve un aviso no sabe si vuelve a decidir una cosa o siete, y ese
es precisamente el dato que decide si vuelve ahora o después.

### D2 — Lo ejecuta el cliente, y el daemon no se entera

El daemon no lee la configuración del aviso, no la valida y no ejecuta nada
(§3). El emisor es el cliente conectado, que ya sabe cuándo algo espera porque
`estado-en-vivo` se lo dice y porque la cola de permisos se le empuja.

En la GUI lo lanza el **backend Rust**, no el webview: un webview que pudiera
ejecutar comandos sería una superficie de ataque que no hace falta abrir, y el
backend ya es quien habla con el sistema operativo.

### D3 — Ámbito usuario, y el repositorio no tiene voz

`[notices] on-waiting` se lee solo de la configuración de usuario. **Nunca** del
`.meltemi/config.toml` de un proyecto.

Es la misma frontera que `via-ssh-del-cliente` acabó de escribir para el
endpoint, y por la misma razón exacta: un repositorio capaz de fijar un comando
sería un repositorio que ejecuta algo en tu máquina por el hecho de abrirlo. Se
implementa con el mismo lector de ámbito usuario que esa change añadió, para
que la frontera exista una vez y no dos.

### D4 — Se lanza y se suelta, con tope

El comando se lanza **sin esperarlo**, sin heredar el stdio del cliente, y con
un tope de tiempo tras el cual se le termina.

Un aviso que bloquea la interfaz es peor que no tener aviso: el usuario pierde
el control de su cliente por un `curl` a un servidor que no responde. Lanzar y
soltar convierte el peor caso en «el aviso no llegó», que es recuperable y
visible.

El tope es de diez segundos. No es un número mágico: es más de lo que tarda
cualquier `curl` a un servicio vivo y menos de lo que un usuario tolera sin
notar nada raro. Se termina dentro de un ámbito de proceso, con el mismo crate
que el resto, así que un comando que lance otros no deja restos.

### D5 — Un aviso por espera, no uno por petición

La coalescencia es por **evento de espera**, no por elemento en cola. Si dos
permisos llegan juntos, hay un aviso con `MELTEMI_COUNT=2`, no dos avisos.

Y hay un piso temporal: no se emite otro aviso mientras no hayan pasado treinta
segundos desde el último. Un agente que pide seis permisos en dos segundos
produciría seis notificaciones en el teléfono, y seis notificaciones seguidas
es cómo un usuario aprende a silenciar un canal. El piso convierte una ráfaga en
un aviso con su cuenta.

Cuando la espera se resuelve, **no** se emite nada. El aviso existe para traer
al usuario; decirle que ya no hace falta es ruido que llega cuando ya viene en
camino.

### D6 — Una prueba, porque nadie configura esto a ciegas

«Enviar aviso de prueba» en Ajustes y en la paleta, en las dos superficies. Usa
exactamente el mismo camino que el aviso real, con `MELTEMI_WAITING=test`.

Sin eso, la única forma de saber si la configuración funciona es dejar una
sesión esperando e irse a otra habitación. Y `test` es un sexto valor del
contrato, declarado aquí, para que el destino pueda distinguir una prueba de un
aviso real si quiere.

### D7 — El registro de cuándo disparó vive en el cliente

Cuándo se emitió el último aviso se guarda en el cliente, no en el log de
sesión. El log es del daemon y es la historia de la sesión; un aviso no es un
hecho de la sesión, es un hecho de la máquina del usuario.

## Risks / Trade-offs

- **Un comando mal escrito no avisa y el usuario no se entera.** Mitigado por
  D6: la prueba es el único momento en que un fallo se puede ver. Un fallo del
  aviso real se anota en el cliente, no se grita.
- **El contenido puede parecer demasiado poco.** Es deliberado (D1) y ampliarlo
  exige enmienda (§9). La receta documentada enseña a usar el identificador de
  sesión para abrir la sesión al llegar, que es el caso de uso real.
- **El piso de treinta segundos puede retrasar un aviso.** Aceptado: el segundo
  aviso de una ráfaga no aporta nada que el primero no dijera, y su cuenta se
  actualiza en el siguiente.

## Migration Plan

Apagado por defecto, así que nadie nota nada hasta configurarlo. No hay formato
que migrar ni contrato que mover.

## Open Questions

- Ninguna. El contenido queda fijado aquí; cualquier ampliación es una change
  con su propia enmienda.
