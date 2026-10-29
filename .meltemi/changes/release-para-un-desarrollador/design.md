# Design — release-para-un-desarrollador

## Context

La tanda de la propuesta (0º–7º) aterrizó el 2026-10-28 en la rama de la tanda,
más `pestana-nueva-con-nombre`, que nombró el smoke de 7º. Lo que queda es lo que
ninguna change de la tanda posee: el guion que una persona sigue, la plantilla
de su informe, la regla de que una release así lo enlaza en sus notas y la
recomendación de versión. La versión, la corrida y el informe con filas son del
mantenedor.

`meltemi-release changelog <x.y.z>` escribe la sección de una versión desde el
archivo y la **regenera entera** cada vez que se corre: una línea añadida a
mano en esa sección se pierde en la siguiente escritura. `meltemi-release notes`
imprime la sección tal cual, y es lo que el pipeline publica como notas.

## Decisions

### D1 — El guion es un documento del repositorio, no del sitio

`docs/prueba-de-uno.md`, en español (constitución §11: documentación del
método y del producto), escrito para Windows primero —la máquina del
mantenedor— con las diferencias de macOS y Linux en una sección propia. Cada
paso dice qué hacer, qué se espera ver, dónde queda registrado y qué desviación
conocida puede aparecer. Empieza en los assets de la release, como el
quickstart, nunca en el compilador.

### D2 — La docs CI comprueba los verbos contra el binario

Un test lee los bloques de código del guion, toma cada orden `meltemi <verbo>`
y exige que el verbo exista en la ayuda del binario construido; corre además
los pasos locales que no necesitan agentes (`version`, `help`, `doctor --json`
contra un endpoint que no existe, que tiene que contestar con su informe y
salir con `0` o `14`, nunca colgarse ni arrancar un daemon). Un verbo que el
guion nombra y el binario no tiene rompe el build, igual que con el quickstart.

### D3 — El informe enlazado viaja en la sección del changelog

`changelog <x.y.z> --report <ruta>` escribe, como primera línea de la sección,
`**Tested by one developer**: [<ruta>](<ruta>)`; la ruta tiene que existir en el
repositorio o se rehúsa. Reescribir la sección sin `--report` conserva la línea
que ya tenía: el enlace no depende de recordar la bandera. Como `notes` imprime
la sección, el enlace llega a las notas de la release sin tocar el pipeline.

### D4 — La plantilla del informe vive en el guion

Como en `docs/hito-v01.md`, la plantilla va en el propio guion, en un bloque
que se copia a `docs/qa/<fecha>-prueba-de-uno.md`. No se crea un informe con
filas vacías en `docs/qa/`: un archivo allí es una corrida, y esta change no
corre nada.

### D5 — La versión: recomendación 0.3.0, escrita por el mantenedor

Lo que la tanda trae es aditivo (ningún cambio en `proto/` ni en la gramática
del CLI rompe a un cliente), y marca la primera release que una persona prueba
de punta a punta: **0.3.0**, por el mismo motivo por el que la 0.2.0 marcó la
primera release por push. La escribe el mantenedor con `set-version`; un agente
no la escribe (`AGENTS.md`).

**2026-10-29.** La recomendación contradecía `docs/versionado.md`, que antes de
1.0 sube el patch por lo aditivo: por la política, la tanda era 0.2.1. El
mantenedor decidió escribir la regla que faltaba —un hito probado de punta a
punta sube el minor— y la política la dice desde `entrega-sin-huecos` 1.2. La
0.3.0 queda conforme.

## Risks / Trade-offs

- El guion envejece con el producto. El test de D2 lo ata a los verbos del
  binario; lo que el test no ve (lo que un paso muestra en pantalla) lo corrige
  la propia corrida, que es para lo que existe.
- Una release con `--report` hacia un informe que todavía no se escribió: la
  ruta tiene que existir, así que el orden es corrida → informe → `changelog
  --report` → `set-version`.
