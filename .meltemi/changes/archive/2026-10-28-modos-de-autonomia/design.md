# modos-de-autonomia — design

> El proposal predijo que inyectar reglas no bastaría y que habría que extender
> `evaluate`. Leído el motor, la predicción es correcta y además se puede decir
> exactamente **por qué**, y en qué se convierte.

## D1 — Por qué un bundle de reglas no puede expresar los modos

`evaluate` decide en dos pasos (`permissions.rs:144-180`):

1. **El ámbito gana entero**: si alguna regla que casa es de proyecto, las
   globales dejan de contar por completo.
2. Dentro del ámbito ganador, **`deny` gana el empate**.

Con eso, no hay dónde poner el bundle:

| bundle en… | «el modo da el piso» | «el deny del usuario sobrevive» |
|---|---|---|
| **proyecto** | sí | **no** — un deny *global* del usuario queda descartado por el ámbito |
| **global** | **no** — cualquier regla de proyecto que case descarta el modo | sí |

Y hay un caso que **ninguna** colocación resuelve: **Manual**. Manual significa
«pregúntame todo en esta sesión». Si el usuario tiene `allow Read` configurado,
un bundle no puede hacer que ese allow deje de conceder — un bundle solo añade
reglas, y añadir no quita.

Así que los modos no son reglas. Son una **postura sobre el resultado** de las
reglas.

## D2 — Los modos como postura, en tres frases

`evaluate` sigue siendo el único evaluador de reglas del usuario y no cambia. Lo
que se añade es una función de composición con tres reglas, en este orden:

1. **Un `deny` del usuario gana siempre.** Ningún modo lo levanta. Es lo único
   que el usuario escribió para no tener que volver a decidirlo, y un modo que lo
   pisara convertiría «autónomo» en «ignora lo que dijiste».
2. **Lo irreversible y lo que sale del árbol escalan en todo modo.** Constitución
   §3, literal: «las acciones con efectos externos irreversibles requieren
   aprobación explícita **incluso en modo autónomo**». El clasificador ya existe
   y clasifica por tipo de operación —comando o red— no por ruta
   (`is_out_of_tree`, `permissions.rs:73-82`).
3. **Lo demás lo decide el modo**, que puede mover el resultado en una dirección
   y solo en una:
   - **Manual** baja `Allow` a `Ask`. No concede nada y **retira** lo concedido.
   - **Semi** sube `Ask` a `Allow` **solo para ediciones contenidas** en el árbol
     de la sesión.
   - **Autónomo** sube `Ask` a `Allow` para todo lo que sobrevivió a (2).

Ninguno de los tres inventa un `Allow` donde el usuario puso `Deny`, y ninguno
concede lo irreversible. La tabla completa se pinea con tests del motor.

## D3 — Sin modo, nada cambia: el modo ausente no es un modo

La compatibilidad no se logra eligiendo un default «neutro» que se parezca a
hoy: se logra **no componiendo nada**. Sin campo `mode`, la resolución es
literalmente `evaluate` y ni una línea más — la misma función, el mismo
resultado, byte a byte.

Esto importa porque «Manual» **no es** el comportamiento de hoy: hoy los allow
del usuario conceden, y Manual los retira. Presentar Manual como el default
habría cambiado en silencio lo que hace una sesión existente.

## D4 — Contención: la que el motor sabe expresar, no la que suena bien

Semi concede «ediciones dentro del árbol de la sesión». `is_out_of_tree` **no
sirve** para eso —clasifica mando y red, no rutas— y el proposal ya lo dice.

La contención se decide con la ruta que los hechos ya llevan (`RequestFacts.path`,
el mismo campo que alimenta `path_prefix`, `permissions.rs:213-217`) comparada
con la raíz del árbol de la sesión. Con tres esquinas dichas:

- **Ruta ausente**: no se puede afirmar contención ⇒ **escala**. No conceder por
  no saber es la única dirección segura.
- **Ruta absoluta fuera del árbol** en una sesión con worktree ⇒ escala, aunque
  sea una edición.
- **Sesión libre sin worktree**, que corre sobre el árbol del usuario: ahí «dentro
  del árbol» es el proyecto entero, y **Semi deja de ser una contención
  significativa**. Se dice en la superficie en vez de fingir que protege algo:
  el chip nombra el ámbito real.

## D5 — La deuda del allow se paga aquí, porque montar sobre ella sería heredarla

`allow_meltemi_writes()` promete acotarse a `.meltemi/` y devuelve `allow_all()`
(`sdd_flow.rs:609-611`). Montar los modos encima dejaría el modo Autónomo
apoyado en un allow universal que dice ser acotado. Se acota de verdad, con el
`path_prefix` que el motor ya tiene, y su test.

## D6 — Bypass no existe, y se escribe para que no lo «arreglen»

No hay modo que salte el proxy. Lo irreversible escala en los tres, el deny sin
clientes no se toca, y una regla jamás concede opciones que el agente no ofreció.
El comparativo dejó registrado el anti-patrón vecino («Yolo / Dangerously skip
permissions» por defecto) — que es exactamente lo que el deny-by-default de
Meltemi existe para no ser.

Queda como requisito con lenguaje normativo, no como comentario, para que un
design futuro tenga que **derogarlo por escrito** en vez de añadir un modo más.

## D7 — Lo que se ve y lo que queda escrito

- **Chip en el compositor y en el lanzador** (GUI), selector en la TUI, flag en
  el CLI: el mismo modo, tres accesos.
- **El log de sesión registra el modo activo**, y la decisión de cada permiso
  registra bajo qué modo se tomó. Sin eso, un histórico con modos es un histórico
  que no explica sus propias decisiones.
- El modo es **de la sesión**, no del proyecto ni global: es una palanca de
  confianza para *esta* tarea, y persistirlo sería convertirlo en una preferencia
  que se olvida encendida.

## D8 — La cobertura de verbos, acotada a los que arrancan una sesión

`session/start` y `worktree/dispatch`. **No** `session/direct`: dirigir una
sesión existente no la re-arranca, y cambiar su modo a mitad de camino haría que
el log tuviera que explicar cuál regía en cada turno. Cambiar de modo es empezar
una sesión con otro modo — y eso ya se puede.
