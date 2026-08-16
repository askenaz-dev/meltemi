# Design — salida-que-se-lee

## Context

Verificado el 2026-08-15:

- `output.rs` es todo el renderizador: `render_outcome(outcome, json, out)` y
  `render_error(error, json, out, err)`, con `json: bool` como única elección.
  Un booleano no admite un tercer formato sin volverse dos booleanos que pueden
  contradecirse.
- **No hay una sola secuencia de color en el CLI**: un barrido de `tui/src/*.rs`
  no encuentra `\x1b`, `NO_COLOR` ni ninguna crate de color. El shell
  interactivo pinta con `ratatui`, que no sirve para una salida de una pasada.
- `main.rs` ya calcula `io::stdout().is_terminal()` y se lo pasa al parser: la
  detección de TTY existe, solo que hoy decide CLI-vs-TUI y nada más.
- El proyecto **no tiene serializador YAML** en el árbol (`Cargo.lock` no
  registra ninguno), y `deny.toml` tiene lista de `[bans]`, así que la
  dependencia nueva se declara aquí y se audita.
- La verdad viva ya obliga a lo que este diseño debe respetar:
  `tui-shell` exige «color solo decorativo, el significado NO depende de él» y
  honrar `NO_COLOR`/`--no-color`/`TERM=dumb`; `cli-contract` exige que bajo
  `--json` salga **exactamente un objeto** y que stdout no mezcle texto humano.

## Goals / Non-Goals

**Goals**: que un listado de treinta y seis filas se lea de un vistazo —resumen,
alineación y color redundante—; que el formato de salida sea una elección
declarada con tres valores; que nada de esto rompa un script existente.

**Non-Goals**: la TUI interactiva; selectores de plantilla; cambiar la forma de
ningún objeto JSON; otras superficies.

## Decisions

### D1 — El formato es una elección de tres, no dos booleanos

`render_outcome(outcome, json: bool, …)` pasa a `Format { Human, Json, Yaml }`.
La alternativa —añadir `yaml: bool` junto a `json: bool`— permite el estado
`--json --yaml` que no significa nada y que alguien tendría que decidir en cada
sitio de llamada. Un enum lo hace imposible por construcción, y el parser
rechaza la combinación una sola vez, con un mensaje.

Que el humano sea un valor del mismo enum es lo que evita el error clásico: hoy
un comando nuevo puede olvidarse de mirar `json` y emitir texto en un flujo que
un script está parseando. Con un enum, no formatear es un caso no cubierto que
el compilador señala.

### D2 — YAML sin dependencia, porque el subconjunto lo permite

**Corrección sobre la primera redacción de esta decisión.** Decía «se añade un
serializador YAML» razonando que escribirlo a mano se rompe con las cadenas
multilínea y las comillas. Al ir a elegir la crate aparecieron dos hechos que
la invierten:

1. **`serde_yaml` está archivado por su autor desde 2024.** Adoptarlo obligaría
   a un `ignore` en `deny.toml`, y los que hay hoy son todos transitivos del
   stack de Tauri —heredados, no elegidos—. Elegir uno a propósito para emitir
   un árbol de cinco tipos es mal negocio.
2. **YAML 1.2 es un superconjunto estricto de JSON**, y ahí está la salida: si
   toda cadena se emite **siempre entre comillas dobles con el escapado de
   JSON**, el resultado es un escalar YAML válido por construcción. Las
   multilíneas y las claves con dos puntos —el riesgo que la versión anterior
   temía— dejan de ser casos de borde porque nunca se intenta el estilo plano.

Así que el emisor vive en el cliente, en unas decenas de líneas: bloques
indentados para objetos y listas, escalares desnudos para números, booleanos y
nulo, y comillas dobles siempre para las cadenas. El árbol de entrada es un
`serde_json::Value` y no hay más tipos que cubrir.

Lo que **no** cambia es la promesa: `--yaml` emite exactamente un documento en
stdout, en éxito y en error, sin texto humano mezclado — palabra por palabra la
de `--json`, porque su razón de ser es idéntica.

Y la change pasa a tener **cero dependencias nuevas**, lo que la deja alineada
con §10 sin necesitar la justificación que §10 pide.

### D3 — El color es decoración; el significado va en símbolo y palabra

Cada distinción que el color marca la marca también otra cosa:

| Distinción | Portador que no es color |
| --- | --- |
| change activa / archivada | la palabra `active` / `archived`, ya presente |
| contador completo / a medias | las cifras `7/7` frente a `0/9` |
| compuerta esperando | el texto `<- gate: … awaits you`, ya presente |
| artefacto ausente | el punto `·` frente a la letra en `PDST` |

Esto no es prudencia: `tui-shell` ya lo exige para la superficie de terminal y
sería incoherente que la salida scriptable —la que más gente lee por SSH, en
CI, o con un lector de pantalla— fuera la excepción. **El color se añade a lo
que ya distingue; no reemplaza nada.** Consecuencia práctica: quitar todo el
color de esta salida no puede quitarle información, y el test lo comprueba
comparando la versión pintada con la monocroma después de retirar los escapes.

Se emiten secuencias ANSI directas —los ocho colores básicos y `bold`— sin
crate: es lo que el shell ya hace, funciona en el terminal de Windows moderno y
evita una dependencia por algo que cabe en una constante.

### D4 — Sin TTY, sin color, y `NO_COLOR` manda

El color se pinta **solo** cuando stdout es un TTY y ninguna señal lo prohíbe.
El orden es: `--no-color` explícito, luego `NO_COLOR` con valor no vacío, luego
`TERM=dumb`, luego la ausencia de TTY. Cualquiera de las cuatro apaga el color.

Que la ausencia de TTY apague por defecto es lo que hace segura la
retrocompatibilidad: un script que hoy hace `meltemi changes | grep active`
sigue viendo exactamente los mismos bytes que antes. Es también la razón por la
que este cambio no necesita MODIFIED: para todo consumidor no interactivo la
salida es idéntica.

`--json` y `--yaml` nunca llevan color, cualquiera que sea el TTY: un documento
para máquinas no se decora.

### D5 — El resumen y la alineación son del listado, no del renderizador

`Outcome.human` lo compone cada comando; el renderizador solo lo escribe. Así
que el resumen y las columnas se construyen donde vive el dato —en las
funciones de `run.rs` que ya arman esos textos— y el ancho de columna sale del
contenido (el nombre más largo), nunca de un número fijo que la primera
capacidad de nombre largo desalinea.

El resumen dice lo que hoy hay que sumar a ojo: en `specs`, capacidades,
requisitos y escenarios; en `changes`, activas, archivadas y **cuántas esperan
una decisión** — que es el dato por el que uno mira la lista y el único que hoy
no está en ninguna parte.

### D6 — Vía rápida, y por qué no colisiona

El delta es solo-ADDED sobre `cli-contract`, la única capability que
`lanzador-conversacional` toca con un MODIFIED sin archivar — pero sobre otro
encabezado («Mapeo comando↔método RPC»). Encabezados nuevos y distintos se
fusionan sin pisar texto ajeno, que es la lección que este repositorio ya pagó.

## Risks / Trade-offs

- **Los anchos calculados cuestan una pasada más** sobre la lista antes de
  imprimir. Con decenas de filas es irrelevante; se anota por si algún listado
  crece a miles, donde convendría un tope.
- **El color en el terminal de Windows** depende de que el modo de secuencias
  virtuales esté activo; lo está por defecto en Windows 10 1809+, que es el
  mínimo soportado. Se comprueba en la terminal real, no se supone.
- **El emisor de YAML es código propio**: menos superficie que una crate sin
  mantenimiento, pero superficie al fin. Se acota emitiendo siempre las cadenas
  entre comillas —donde YAML y JSON coinciden— y se prueba contra un valor con
  saltos de línea, comillas y una clave con dos puntos.
- **Un script que capture stdout con un pseudo-TTY** vería color donde antes no
  lo había. Es el caso que `NO_COLOR` existe para resolver, y la doc lo dice.

## Migration Plan

Aditivo. Para todo consumidor no interactivo la salida no cambia un byte; para
quien mira una terminal, mejora. `--json` conserva su contrato.

## Open Questions

- ¿Merece `--no-color` un `--color=always` simétrico para forzarlo en un pipe?
  Se omite hasta que alguien lo pida: `NO_COLOR` es el estándar y su ausencia no
  es un caso simétrico.
