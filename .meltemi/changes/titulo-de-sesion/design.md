# Design — titulo-de-sesion

## Context

Verificado en el código el 2026-08-09, y el smoke de `piel-de-pestanas` lo
fotografió el mismo día: seis pestañas abiertas dicen `mock a296f430`,
`mock 929240ed`, `mock 39597c6f`… — la misma palabra y ocho caracteres de hex.

- La instrucción del usuario está disponible desde `free_session.rs:56`, y el
  registro de inicio se escribe en `:164-183`: derivar el título entre medio no
  obliga a mover nada. **La expansión de `@` ocurre después** (`:249-253`), así
  que hay dos textos distintos y el título debe salir del **crudo**.
- `SessionRecord` (`session_index.rs:24-60`) ya tiene el molde de campo
  opcional añadido después: `source` (`:54-59`).
- `SessionInfo` (`proto/meltemi-proto/src/lib.rs:396-427`) tiene el molde de
  campo aditivo del contrato: `agent_id` y `profile` (`:421-426`), con
  `#[serde(default, skip_serializing_if = "Option::is_none")]` y fuera de
  `required` en `session-list.schema.json:93-100`.
- `worktree/dispatch` **no recibe instrucción de usuario**:
  `WorktreeDispatchParams` (`lib.rs:2069-2075`) lleva `projectRoot`, `change`,
  `task`, `agent` y nada más.
- La GUI abre la pestaña al llegar el evento, **antes** de que
  `refreshSessions()` conteste (`Home.svelte:207-215`).

Dos hechos del código que cambian el diseño y que la propuesta no conocía:

1. **`merge_into` copia solo los campos que enumera** (`session_index.rs:135-164`).
   El registro de cierre no lleva título; sin una rama propia, el plegado
   last-wins **borraría el título en cuanto la sesión termina**. Es un defecto
   silencioso que acecha a cualquier campo nuevo del índice.
2. **`record_from_log` reconstruye el índice desde el log** (`:187-261`). Lo
   que no viaje en un evento se pierde cuando el índice no está.

## Goals / Non-Goals

**Goals**: que una pestaña diga de qué trata su sesión; que el título lo
calcule el daemon, una sola vez, para que dos clientes no muestren dos cosas
distintas; que sobreviva al cierre, a la reconstrucción desde el log y al
resume; y que las sesiones históricas no mientan ni exijan migración.

**Non-Goals**: generar el título con un modelo; renombrar a mano; buscar por
título; la piel de la tira (es `piel-de-pestanas`, ya hecha).

## Decisions

### D1 — Derivación local, determinista, del texto crudo y por caracteres

Primera línea no vacía de la instrucción, espacios colapsados, recortada, y
truncada a **64 caracteres** con elipsis. Función pura con sus tests.

Dos precisiones que el repositorio pagó caras antes:

- **Del texto crudo, no del prompt expandido.** `@archivo` se expande después
  (`free_session.rs:249-253`); un título que dijera el contenido de un archivo
  no sería lo que el usuario escribió.
- **Se trunca por `chars()`, jamás por bytes.** Cortar un `&str` por índice de
  byte parte un carácter multibyte y entrega un título roto — el mismo pozo del
  que salió `texto-intacto-al-agente`. En un proyecto que se escribe en español,
  «Corregir la validación…» tiene acentos antes del carácter 64.

### D2 — Solo tiene título quien tiene una primera instrucción

- **Sesión libre** (`free_session.rs`): de `params.instruction`.
- **Propose y SDD** (`propose.rs`, `sdd_flow.rs`): de la idea que las inicia.
- **Dispatch** (`server.rs:677`): **sin título**. Una calle de carrera no nace
  de una frase; su identidad es la change y la tarea, que las superficies ya
  muestran. Inventarle un título compuesto sería fabricar un dato que nadie
  escribió — y `title` es opcional precisamente para poder no tenerlo.

### D3 — El título viaja por el log, además del índice

`session_started` gana `title` opcional (`lib.rs:1647-1654` y su schema). No es
redundancia: paga tres cosas a la vez.

1. `record_from_log` puede reconstruir el título cuando el índice falta
   (`session_index.rs:187-261`), que es la razón por la que `AgentResolved`
   existe en el log.
2. La GUI abre la pestaña al recibir el evento, antes de que la lista conteste
   (`Home.svelte:207-215`): sin el título en el evento, toda pestaña nueva
   nacería con el hash y cambiaría de nombre un segundo después.
3. El log es la verdad del proyecto; un título que solo viviera en el índice
   sería un dato sin acta.

### D4 — `merge_into` gana su rama, y un test la pinea por el lado negativo

`title` se copia con el patrón de `source` (`session_index.rs:159-161`): si el
registro nuevo lo trae, gana; si no, **se conserva el que había**. Sin esto, el
registro de cierre —que no lo lleva— lo borraría, y el síntoma sería el peor
posible: el título se ve mientras la sesión corre y desaparece al terminar.
El test lo comprueba plegando inicio + cierre y exigiendo que el título siga
ahí.

### D5 — Un resume conserva el título, no lo re-deriva

En `server.rs:2067`, junto a `resumed_from`: `title: record.title.clone()`.
Reanudar es continuar **la misma conversación** —así lo lee el usuario y así lo
hacen las referencias—, y el enlace `resumed_from` ya dice que son la misma
historia. Re-derivar del texto de continuación daría dos nombres a un mismo
hilo. Si algún día se quiere renombrar, será un verbo explícito, no un efecto
de reanudar.

### D6 — Las superficies adoptan el título y solo antepone proyecto la ambigüedad

- **Pestañas** (`SessionTabs.svelte:67`): el rótulo pasa a ser el título; el
  avatar del agente queda como identidad, y el hash baja al emergente, que ya
  lleva id completo y proyecto (`:71-73`).
- **Proyecto antepuesto solo ante ambigüedad**: si las pestañas abiertas cruzan
  más de un proyecto, el rótulo antepone su nombre; con uno solo, no gasta
  ancho en repetir lo que el sidebar ya dice.
- **Lista, detalle, árbol y recientes** (`Sessions.svelte:215`,
  `SessionDetail.svelte:501-505`, `Sidebar.svelte:405-409`,
  `Home.svelte:416-417`): título junto al id, nunca en su lugar — el id sigue
  siendo lo que se copia y se pega.
- **TUI** (`live.rs:67-79`, `render.rs:830-837`): `SessionRow` gana el campo y
  la línea lo muestra, con el mismo tope y truncado del shell.

### D7 — Sin título, lo de hoy; sin migración, ninguna

Las sesiones anteriores a esta change devuelven `title: None`
(`#[serde(default)]` ya lo tolera) y las superficies caen al `agente + hash`
actual. No se reescriben logs ni índices: un histórico que se reescribe deja de
ser un histórico.

## Risks / Trade-offs

- **El campo aditivo toca el contrato**, a diferencia de `source`. Arrastra la
  conformidad de tres vías que `tablero-de-carrera` fijó por escrito —presente,
  omitido, y byte-igualdad de la forma omitida— en `conformance.rs`, y obliga a
  `npm run gen:forms` con su gate `check:forms` (`ci.yml:61-65`).
- **Trece sitios construyen `SessionRecord`**; la mayoría son tests. El riesgo
  no es olvidarse de uno (el compilador lo dice), sino que un camino escriba
  `None` donde debía derivar: el test por camino lo cubre.
- Un título de 64 caracteres en una pestaña de 96 px se recorta con elipsis; el
  emergente lleva la historia entera. Es la misma regla que ya rige el rótulo.

## Migration / Rollout

Campo aditivo en contrato e índice, sin verbo nuevo y sin fila nueva en la
matriz de paridad. Se despliega con la change; las sesiones vivas al momento
del despliegue no tienen título y lo muestran como las históricas.
