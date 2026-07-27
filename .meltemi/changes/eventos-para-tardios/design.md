# Design — eventos-para-tardios

## Context

`session/event` viaja hoy por `state.peer.notify(...)` contra el `Peer`
capturado al arrancar la sesión. Todo lo demás del daemon que debe alcanzar a
varios clientes ya dejó de hacerlo así: la cola de permisos publica en un
broadcast y cada conexión reenvía su instantánea. Esta change aplica esa misma
forma al transcript.

## Decisions

### D1 — Un solo camino de entrega, sellado con su origen
Tentación descartada: mantener el push directo al iniciador y añadir el hub
solo para los demás. Dos rutas significa que un cliente que suscriba su propia
sesión recibe cada evento dos veces, y el daemon no puede detectarlo. En su
lugar, `forward_update` **solo publica**, y el sobre lleva el identificador de
la conexión que originó la sesión. Cada conexión reenvía si el evento es suyo
(`origen == mi conexión`) o si mira esa sesión. Comportamiento observable
idéntico al de hoy para el iniciador, sin duplicados posibles.

### D2 — Suscripción explícita, no difusión a todos
La cola de permisos difunde a todo el mundo porque es una instantánea corta y
compartida. Un transcript no: son muchos eventos por turno y pertenecen a una
sesión concreta. Difundir todo a todos haría que un teléfono con túnel pagara
el tráfico de sesiones que nadie mira, justo en el escenario que motiva la
change. `session/watch` es opt-in por sesión y por conexión.

### D3 — El identificador de conexión vive en `Peer`, no en un registro aparte
Un contador monótono en proceso, asignado en `Peer::start`. Es la unidad que
ya se clona hacia los handlers, así que el origen viaja sin cablear nada
nuevo. No identifica al usuario ni sobrevive al proceso: solo distingue
conexiones vivas entre sí, que es todo lo que el fan-out necesita.

### D4 — Retraso acotado, no cola infinita
El hub es un `broadcast` con capacidad acotada. Una conexión que se retrase
más que el búfer pierde eventos intermedios y lo sabe (el canal lo señala);
el remedio honesto es `session/log`, que es la fuente completa y paginada. Un
canal ilimitado convertiría un cliente lento en memoria del daemon creciendo
sin techo, que es peor que un hueco declarado y recuperable.

### D5 — Sin replay en el stream
Suscribirse entrega lo que ocurra **desde** ese momento. El histórico ya
tiene su método (`session/log`, con offset y total), y devolverlo también por
el stream crearía dos fuentes del mismo dato con dos ordenaciones que
mantener en sincronía. El cliente que abre un detalle lee el log y se
suscribe; el punto de corte lo marca el `total` que el log le devolvió.

### D6 — Ningún estándar abierto cubre esto (constitución §6)
JSON-RPC 2.0 no define suscripciones; ACP no expone su stream a terceros (es
un protocolo agente↔cliente punto a punto, y aquí el cliente es el daemon);
LSP tiene `$/progress`, atado a una petición y a su token, no a un recurso
observable por otra conexión. La suscripción por recurso es, por tanto,
propia del contrato de Meltemi, y se mantiene mínima: un método, un booleano.

### D7 — La TUI filtra porque ahora puede llegarle más de una sesión
Hoy la TUI pinta cualquier `session/event` que reciba. Era inofensivo cuando
solo llegaban los de su propia sesión; con el hub podría pintar el transcript
de dos sesiones entrelazado. El filtro por la sesión mostrada es parte de
esta change, no una mejora aparte: sin él, la change introduce el defecto.
