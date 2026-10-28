# Design — meltemi-doctor

## Context

Lo que un diagnóstico necesita ya existe repartido: `status` da la versión del
daemon, `fleet/list` el estado compuesto de cada agente con su remedio,
`meltemi_client::paths` las carpetas y el endpoint, `machines::probe` una
conexión que saluda sin arrancar nada, y el daemon escribe su log en
`<datos>/logs/meltemid.log.<fecha>`. La GUI tiene un «Copiar diagnóstico» que
solo aparece cuando la conexión ya falló y copia dos líneas.

## Goals / Non-Goals

**Goals**: un informe que responde «¿qué tienes instalado y qué falla?» en una
pantalla, igual en las tres superficies, con remedio en cada fila que lo
necesita.

**Non-Goals**: reparar nada; mandar el informe a ningún sitio; un RPC nuevo.

## Decisions

### D1 — Se compone en el cliente, en `meltemi_client::doctor`

El módulo junta hechos locales (versión del cliente, endpoint y su tipo,
carpetas de datos y de configuración, configuración de usuario, `git`, `ssh`,
en Windows el servidor OpenSSH, las últimas líneas de error del log del daemon)
y las respuestas del daemon (`status` y `fleet/list`), y las compone en filas.
El daemon no gana nada, así que no nace deber de paridad; las tres superficies
lo ofrecen porque sirve.

### D2 — Nunca arranca un daemon

El diagnóstico se conecta sin pasar por el arranque bajo demanda, como la
prueba de una máquina (`maquinas-remotas`): un diagnóstico que levanta un
daemon cambia lo que diagnostica. Un daemon que no responde es una fila que
falla, con el remedio (`meltemi status` lo arranca), no un error del comando.

### D3 — Filas con id estable; cada superficie pone sus palabras

Cada fila lleva un id estable (`client`, `daemon`, `versions`, `endpoint`,
`data-dir`, `config-dir`, `user-config`, `agent:<id>`, `agents-absent`, `git`,
`ssh`, `sshd`, `daemon-log`), un estado (bien, aviso, falla, sin comprobar),
los hechos que encontró y, si hace falta, un remedio. El terminal y el CLI la
dicen en inglés o en español con las palabras del módulo; la GUI con las suyas
(`messages.ts`), por id. El JSON lleva el id, el estado, los hechos y las
palabras en inglés, que es lo que se copia a un issue. El estado se dice con
glifo y palabra, y el glifo tiene gemelo ASCII.

### D4 — Ningún paso sin tope

`git --version` y `ssh -V` tienen tres segundos cada uno, y el daemon cinco en
total. Lo que no responde a tiempo es «sin comprobar» con el motivo, que es un
resultado válido. El log se lee por la cola (los últimos 64 KiB del archivo más
nuevo).

### D5 — No se ejecuta ningún agente

La detección de la flota es inspección del sistema de archivos y sigue
siéndolo: el diagnóstico no corre el CLI de ningún agente para saber su
versión. La versión instalada se comprueba al abrir una sesión, donde ya
existe (`codex-al-dia`); el informe dice contra qué versión se verificó cada
nivel. Los agentes no detectados se resumen en una fila, y se listan uno por
uno los listos y los que tienen una capa a medias, con su remedio.

### D6 — El espacio en disco no se mide

La biblioteca estándar no lo dice y medirlo exige una dependencia nueva o una
llamada al sistema por plataforma. Las carpetas se comprueban por existencia y
escritura; el espacio se deja fuera y se dice aquí.

### D7 — Código de salida

`0` cuando ninguna fila falla; `14` cuando alguna falla —«una validación que
terminó con hallazgos», que es lo que es—. El informe sale entero en los dos
casos.

### D8 — Nada sale de la máquina

El informe se imprime o se copia al portapapeles; Meltemi no lo manda a ningún
sitio. Lleva lo que el usuario ya ve en su pantalla —versiones, rutas de sus
carpetas, nombres de agentes, líneas de su propio log— y la plantilla de error
le pide revisar antes de pegar.
