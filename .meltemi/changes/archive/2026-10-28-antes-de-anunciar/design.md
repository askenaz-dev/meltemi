# Design — antes-de-anunciar

## Context

Hoy hay una plantilla de issue (`change-proposal.md`), una de PR, ningún
`CHANGELOG.md`, un `docs/quickstart.md` que empieza en `cargo build` y en un
`config.toml` escrito a mano, y ninguna página de preguntas frecuentes. La
release automática (`release-con-el-push`) ya lee `CHANGELOG.md`:
`meltemi-release notes <versión>` extrae la sección `## <versión>` y, sin ella,
el pipeline cae a las notas que genera GitHub (código 3), que es como salió la
0.2.0. El archivo `.meltemi/changes/archive/` tiene cada change con su fecha.

## Goals / Non-Goals

**Goals**: que quien llega por primera vez instale desde los assets, sepa a
dónde escribir según lo que le pase, y lea en cada release qué changes cerró.

**Non-Goals**: encender Discussions (lo hace el mantenedor), el sitio (es de
meltemi-web), los gestores de paquetes (`distribucion-y-actualizacion`), el
anuncio mismo.

## Decisions

### D1 — Tres plantillas de issue y ninguna en blanco

`bug.md` (error: versión, plataforma, superficie, pasos, esperado y visto, y
la salida de `meltemi doctor` cuando exista), `question.md` (pregunta) y la
`change-proposal.md` que ya existe, corregida para nombrar los artefactos en
el orden de la constitución (proposal → design → specs → tasks).
`config.yml` desactiva el issue en blanco y enlaza `SECURITY.md` para lo que
no debe ir en público. Mientras Discussions esté apagado, las preguntas van
como issue con su plantilla; el día que el mantenedor lo encienda, la
plantilla de pregunta puede cambiar por un enlace. El lint de gobernanza
exige las secciones de cada una.

### D2 — Insignias en los dos README, no en el sitio

CI (la insignia del propio GitHub Actions), licencia y última release. El
sitio no se toca: su spec le prohíbe orígenes externos.

### D3 — El changelog lo escribe la herramienta de release, del archivo

`meltemi-release changelog <x.y.z> [--until <AAAA-MM-DD>]` escribe en
`CHANGELOG.md` la sección `## x.y.z — <fecha>` con **cada change archivada que
ninguna otra sección lista todavía** y, con `--until`, solo las archivadas
hasta esa fecha, que pasa a ser la fecha de la sección. Es la regla de la
propuesta —toda release lista las changes que archiva— dicha en código: una
change entra en la primera release que se escribe después de archivarla. No
necesita git: la fecha está en el nombre de cada carpeta del archivo.

Cada entrada es el nombre de la change, enlazado a su carpeta del archivo, y
la frase que la resume: la que abre en negrita la primera viñeta de «What
Changes» de su propuesta, o, si no hay, la primera frase de «Why». Está en
español porque es la lengua del método, y el encabezado del archivo lo dice.

Las secciones van de la versión más nueva a la más vieja, y escribir una que
ya existe la reemplaza: correr la herramienta dos veces da el mismo archivo.

El formato es el que lee `notes`, y se comprueba con `notes`, no con una copia
de su regla.

*Alternativa descartada*: asignar cada change a la primera etiqueta que
contiene su último commit. Una change que aterriza antes de una release y se
cierra después (como `release-con-el-push`, cerrada tres días después de la
0.2.0) caería en la release equivocada, y el resultado dependería de cuánta
historia tenga el clon.

### D4 — El historial se escribe una vez, con la misma herramienta

`CHANGELOG.md` nace con las dos releases publicadas: `0.1.0 --until
2026-07-26` y `0.2.0 --until 2026-10-23`. La 0.1.1 no tiene sección: su
borrador nunca se publicó. La sección de la release siguiente la escribe el
mantenedor con el mismo comando antes de `set-version`; `docs/release.md` lo
dice, junto con la cadencia: una release al cerrar cada tanda, y nunca más de
un mes sin release mientras `main` se mueva.

### D5 — El quickstart empieza en los assets de la release

Instalar (instalador de escritorio o archivo del núcleo, con la verificación
del checksum y de la atestación), `meltemi version` y `meltemi fleet`, una
sesión libre gobernada con `meltemi session … --agent <id>` y la misma en el
escritorio, y después el ciclo. El `.meltemi/` del proyecto aparece con
`propose`, no antes. Compilar desde la fuente queda en el README («Option B»)
y en `CONTRIBUTING.md`. La prueba de docs ya corre `version` y `help` contra
el binario; gana una que exige que el quickstart no pida `cargo build` ni un
`config.toml` escrito a mano.

### D6 — La FAQ junta lo que costó aprender

`docs/faq.md`: el agente que la flota marca como no lanzable y los shims de
Windows; por qué el CLI deniega todo sin terminal y cómo se concede con una
regla; el named pipe y el túnel; el turno que termina sin decir nada (código
`15`); un agente que edita sin preguntar; el aviso del instalador sin firma de
plataforma; dónde están los datos y los registros. Cada respuesta enlaza la
guía que la desarrolla. Una prueba exige que cada pregunta esté.

## Risks / Trade-offs

- Resúmenes en español en un archivo que también leen anglohablantes. Se
  acepta y se dice; traducirlos es una decisión de `docs-en-el-sitio`.
