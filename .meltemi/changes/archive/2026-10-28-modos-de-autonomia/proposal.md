# modos-de-autonomia

> Vía completa (proposal → design → specs → tasks). El contrato gana el modo
> por sesión y el proxy un vocabulario de posturas; el design es obligatorio
> porque fija la semántica exacta de cada modo sobre el motor de reglas, la
> composición con las reglas del usuario, y salda una deuda interna del
> allow. Referencia del mantenedor (2026-08-09): el selector
> Manual / Accept edits / Plan / Auto / Bypass de los productos vecinos — con
> la frontera constitucional dicha de entrada: **Bypass no existirá**.

## Why

El mantenedor lo pidió directo: «debemos tener las opciones para trabajo
manual, autónomo, semi autónomo… así sabremos si pide más permisos para cada
acción o actúa de forma independiente». Es la palanca de confianza por
sesión: tarea exploratoria en un repo delicado → que pregunte todo; tarea
mecánica en un worktree desechable → que fluya.

El hallazgo que hace esta change barata de concebir y honesta de proponer:
**el daemon ya compone posturas de permiso por sesión** — solo que ninguna
superficie lo expone. `explore` corre con `deny_all()`
(`core/meltemid/src/sdd_flow.rs:55`), los turnos de autoría SDD con
`allow_all()` (`sdd_flow.rs:610-611`), y los flujos interactivos cargan las
reglas del disco por sesión (`free_session.rs:187`, `propose.rs:185`). Cada
sesión ya recibe su propio `RuleSet` (`acp.rs:348-357`) y todo se evalúa en
un punto único (`acp.rs:397-410`). Hasta hay precedente de contrato:
`sdd/implement` lleva `autonomous: bool` y **degrada a supervisado con aviso
si no hay reglas aplicables** — «never autonomy by accident»
(`server.rs:1574-1585`).

Las piezas también existen, cada una con su naturaleza dicha con precisión:
`is_out_of_tree` clasifica **por tipo de operación** — comandos y red — y no
mira rutas (`permissions.rs:74-82`); la contención de rutas la expresa el
matcher `path_prefix` del motor de reglas (`permissions.rs:213-217`); y las
acciones irreversibles ya se registran (`irreversible.jsonl`,
`checkpoints.rs:177-210`). Y la frontera
está escrita donde no se negocia: constitución §3 — «las acciones con efectos
externos irreversibles requieren aprobación explícita **incluso en modo
autónomo**»; y sin cliente conectado, denegación constitucional
(`default_deny`, `acp.rs:509-515`). El comparativo de mercado ya dejó
registrado el anti-patrón: Orca embarca «Yolo / Dangerously skip permissions»
por defecto — exactamente lo que el deny-by-default de Meltemi existe para no
ser.

## What Changes

- **Campo `mode` aditivo por sesión** en `session/start` y
  `worktree/dispatch` (cobertura exacta de verbos en el design), que el
  daemon traduce a un **bundle sintético de reglas con ámbito propio**,
  compuesto con las del usuario y evaluado en el punto único existente. Ojo
  verificado y dicho aquí: la precedencia del motor se decide en la
  evaluación por ámbito, no por posición (`permissions.rs:115-116, 146-182`),
  y con los dos ámbitos actuales —proyecto pisa global por completo— no hay
  colocación del bundle que logre a la vez «el modo da el piso» y «el deny
  del usuario sobrevive»: el design deberá extender `evaluate` (p. ej. un
  tercer ámbito de menor precedencia), no solo inyectar reglas. Sin campo →
  el comportamiento de hoy, byte a byte: compatibilidad por ausencia.
- **Tres modos, semántica anclada en lo que el motor ya clasifica**
  (nombres finales es/en en el design):
  - **Manual**: todo pregunta — el bundle no concede nada (`Ask` universal).
  - **Semi**: las ediciones dentro del worktree de la sesión pasan; todo lo
    demás pregunta. La contención de ruta se expresa con el matcher
    `path_prefix` existente o un clasificador de contención nuevo (design);
    `is_out_of_tree` no basta solo — clasifica mando y red, no rutas.
  - **Autónomo**: allow amplio, **excepto** lo fuera del árbol y lo
    irreversible (comandos, red, efectos externos), que siempre escalan. §3
    literal: autónomo ≠ sin gobierno.
- **Composición declarada, no accidental**: el design fija quién gana entre
  el bundle del modo y las reglas del usuario (la dirección esperada: el modo
  da el piso, las reglas del usuario lo refinan; un deny explícito del
  usuario sobrevive a cualquier modo) — y lo pinea con tests del motor.
- **El modo se ve y queda escrito**: chip en el compositor/lanzador de la
  GUI (la referencia del mantenedor muestra el rótulo del modo junto a la
  caja), selector en la TUI, flag en el CLI; el log de sesión registra el
  modo activo y la bandeja dice bajo qué modo se decidió cada petición — la
  auditoría no gana agujeros.
- **Deuda saldada de paso**: `allow_meltemi_writes()` promete acotarse a
  `.meltemi/` y hoy devuelve `allow_all()` (`sdd_flow.rs:609-611`); montar
  modos sobre bundles obliga a pagarla, y se paga aquí.
- **Bypass, rechazado por escrito**: no hay modo que salte el proxy. Lo
  irreversible escala en todos los modos (§3), el deny sin clientes no se
  toca, y una regla jamás concede opciones que el agente no ofreció
  (`permission-rules`). La propuesta lo dice para que ningún design futuro lo
  «arregle».

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `permission-rules`: + requisito «Modos como posturas por sesión» — el
  bundle sintético, la composición con las reglas del usuario, la
  supervivencia del deny explícito, y la escalada innegociable de lo
  irreversible en todo modo.
- `acp-session`: + el campo de sesión y su registro en el log.
- `cli-contract`: + flag de modo en los verbos que arrancan sesión.
- `gui-shell` / `tui-shell`: + el selector y el chip con el modo visible.

## Impact

- Archivos: `core/meltemid/src/{permissions.rs, acp.rs, server.rs,
  free_session.rs, sdd_flow.rs}`, `proto/` (campo aditivo en los schemas de
  arranque), `tui/`, `desktop/ui` (lanzador y sesión), matriz de paridad,
  docs de permisos.
- Cero dependencias nuevas.
- Riesgo real: la **matriz de composición** modo×reglas×clasificador tiene
  esquinas (deny de usuario vs allow de modo, herramienta sin clasificar,
  edición con ruta absoluta fuera del worktree en sesión con worktree, y
  worktree ausente en sesión libre sobre el árbol del usuario — ahí Semi no
  puede significar «edita lo que sea»); el design las enumera y cada una
  queda pineada por un test del motor. Es la clase de sutileza por la que
  esto es vía completa.
- La política de espera (`[permissions] wait`) no se toca: modos deciden
  *qué* se pregunta; la espera decide *cómo* se aguarda la respuesta.

## Fuera de alcance

- **Modo Bypass o equivalente**: inconstitucional (§3); no es deuda, es
  frontera.
- **Modos custom nombrados por el usuario**: los TOML de reglas ya componen
  posturas arbitrarias; un registro de presets propios es futuro con
  evidencia.
- **Modo por defecto configurable** (global o por proyecto): candidato
  natural de seguimiento, pero v1 arranca sin campo = hoy, para que el
  default nunca cambie por accidente.
- **Cambiar de modo a mitad de sesión**: exigiría re-evaluar peticiones en
  cola; se estudia con evidencia, no se cuela.
