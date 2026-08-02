# Design — tablero-de-carrera

## Context

El daemon corre carreras completas pero las superficies no las muestran.
Estado real verificado en el código (2026-07-31):

- **GUI**: `Review.svelte` ya agrupa worktrees por cambio/tarea (con el
  campo `competitor`), consulta `worktree/diff` y renderiza el diff
  unificado por archivo y hunk con parser propio (`fileSections`/`hunksOf`,
  privados del componente). No muestra procedencia, ni estado de commit,
  ni acciones de carrera, ni se actualiza sola.
- **TUI**: los diez verbos de worktree/checkpoint/commit existen en la
  paleta como `reserved: true` — se anuncian «(reservado)» y el submit se
  cierra en silencio. Ningún resultado de carrera se renderiza en el shell;
  solo la CLI scriptable los imprime (`render_race` vuelca el diff crudo).
- **Contrato**: `WorktreeCompetitorDiff` trae exactamente
  `{agent, path, changedFiles, diff}`; `WorktreeDispatchResult` no nombra
  la sesión que abrió; `SessionInfo` no nombra cambio/tarea/agente. La
  procedencia del despacho (fuente/perfil/nivel) se emite como evento
  `AgentResolved` al log de sesión y no se persiste por worktree. Las
  sesiones de despacho no escriben en el índice de sesiones (solo
  reconstrucción desde logs, con `level` degradado al default 1).
- **Unión sesión↔worktree existente**: la sesión de despacho registra
  `project_root = <ruta del worktree>` archivada bajo la clave del
  proyecto padre — una unión por igualdad exacta de cadenas contra
  `ManagedWorktree.path`, sensible a la ortografía de rutas (lección de
  los commits de `multiproyecto`: canonicalizar la entrada, jamás
  re-canonicalizar lo almacenado).

## Goals / Non-Goals

**Goals**: que la carrera se vea — calles con procedencia, estado y diff
en GUI y TUI por igual (§4); que el contrato deje de callar lo que el
daemon sabe (campos aditivos, cero ruptura); que las sesiones de despacho
sean de primera clase en el histórico; verificación por escenarios
enlazados y smoke visual publicado.

**Non-Goals**: canal push de worktrees; merge automático o ranking de
competidores; vistas numeradas nuevas en cualquiera de las dos
superficies; método RPC nuevo; dependencias nuevas.

## Decisions

### D1 — El tablero lee lo persistido; el contrato se ensancha, no se multiplica

Tres opciones para alimentar el tablero: (a) un método nuevo
`worktree/board` que agregue todo; (b) composición en cada cliente
(N consultas + unión por superficie); (c) **enriquecer `worktree/diff` con
campos aditivos por calle** — la elegida. (a) arrastra la onda de paridad
completa (paleta TUI + registry GUI + matriz + formularios generados) para
servir datos que el diff ya transporta a medias; (b) duplica la lógica de
unión en dos superficies y exigiría que clientes finos lean logs del
daemon, rompiendo el modelo. (c) sigue el precedente de `navigate.rs`
(agregación server-side de estado ya persistido, solo lectura, cómputo
caro una vez por listado) y no toca la matriz de filas. Los campos nuevos
por calle: `source`, `profile`, `level`, `sessionId`, `committed`, `sha`,
`baseRev` propio. Disciplina aditiva completa: `Option` +
`#[serde(default, skip_serializing_if)]`, propiedad en el esquema sin
entrar a `required`, y conformidad de tres vías (presente, omitido, y
byte-igualdad de la forma omitida) — el patrón ya asentado por
`multiproyecto-suscripciones` y `flota-deteccion-guia`.

La superficie scriptable renderiza los mismos campos (`race` muestra
procedencia, sesión y estado por calle, con la ausencia como «—», nunca
inventada). Su ancla de verificación es el escenario de contrato «La calle
declara procedencia, sesión y estado» — que gobierna los datos que
cualquier cliente recibe — más el gate de la tarea que la implementa; no
se duplica un escenario de render por superficie scriptable, y la frescura
de la referencia CLI ya la vigila la spec vigente de `cli-contract`.

### D2 — La calle conoce su sesión porque el despacho asienta registro

Hoy la procedencia por calle solo puede reconstruirse escaneando logs de
sesión (O(sesiones), frágil). En lugar de escanear, **el despacho escribe
registro de índice al abrir y al cerrar** — con `level` real, `agent_id`,
`profile` y **la fuente de resolución** — como ya hacen propose y la
reanudación (los tres primeros existen en `SessionRecord` desde
`multiproyecto-suscripciones`).

> **Enmienda (2026-08-01, implementación de 2.2).** Este design daba por
> hecho que `level`, `agent_id` y `profile` bastaban. No bastan: la spec de
> `worktree-orchestration` exige que la calle declare la **fuente de
> resolución**, y esa fuente no se deduce de los otros dos campos — un id de
> catálogo y un agente configurado que nombra un id se ven idénticos
> (`profile: None`, `agent_id: Some`). Deducirla sería inventarla. Por eso
> `SessionRecord` gana un campo opcional `source`, lo escriben todos los
> caminos que ya resolvían por la flota (propose, autoría SDD, sesión libre,
> reanudación y despacho) y la reconstrucción desde logs lo recupera del
> evento `AgentResolved`, que siempre lo llevó. El índice es privado del
> daemon, así que el campo es aditivo sin tocar el contrato.

La agregación del
diff une entonces por igualdad exacta `record.project_root ==
ManagedWorktree.path` (ambas cadenas escritas por el mismo daemon, misma
ortografía por construcción) y toma el registro más reciente por calle. La
raíz que entra por parámetro se canonicaliza (semántica de
`projects::canonical`); las rutas almacenadas jamás se re-canonicalizan.
`WorktreeDispatchResult` gana `session_id`, cerrando la correlación
calle→sesión también para quien despacha. Beneficio lateral honesto: las
sesiones de despacho dejan de listarse con nivel mentiroso en
`session/list`.

### D3 — GUI: el tablero es la evolución del drill-in de revisión

No una vista numerada nueva: `KEYED_VIEWS` está clavado en cinco entradas
por tests de cableado y onboarding («Cinco vistas») — ampliarla es puro
costo sin valor de producto. El drill-in de revisión ya es medio tablero;
se completa: cabecera por calle con procedencia (agente + perfil, chips ya
existentes), estado del turno/commit/checkpoint con glifo+palabra, las
acciones de carrera (despachar, revertir, commit, merge por archivo) con
los formularios tipados generados y `ConfirmDialog` para las destructivas
(`checkpoint/revert` ya es `dangerous: true` en el registry — la GUI honra
esa marca). `fileSections`/`hunksOf` se extraen a un módulo compartido
para que calles y revisión rendericen el mismo diff (los tests de cableado
que leen la fuente de `Review.svelte` se actualizan con la extracción).
Cada string nueva entra a `messages.ts` en ES y EN (el lint de i18n no
perdona palabras sueltas).

### D4 — TUI: superficie desde la paleta, sin tocar el contrato 1–4

El contrato de dígitos 1–4 está declarado en la spec y clavado en al menos
cinco puntos del código; una quinta vista numerada exigiría un MODIFIED de
reemplazo total sobre el requisito de navegación con todos sus escenarios.
No hace falta: el tablero entra por la paleta — el verbo `race` deja de
ser `reserved` y abre la superficie (patrón ya probado al des-reservar
`direct`). El estado `drilled: bool` (cableado por convención a Sessions)
se generaliza a un enum de superficie drill para no colisionar con el
scroll del transcript. El despacho desde el tablero copia el patrón
`Command::Direct`: peer clonado + task aparte, porque `worktree/dispatch`
responde tras el turno completo del agente y el bucle del shell debe
seguir respirando. El diff se renderiza con el paneo existente
(`h_scroll`/ventana de cola) con tope de líneas declarado — sin widget de
scrollbar nuevo. Todo estado de calle es glifo+palabra con gemelo ASCII
(el test que prohíbe glifos Unicode bajo presentación ASCII aplica).

### D5 — Vida sin canal nuevo: tick, stream propio y re-consulta

No existe suscripción push de worktrees y esta change no la inventa. Cada
superficie usa lo que ya tiene. **TUI**: el tick de 2 s del shell (status
+ session/list) delata despachos ajenos concluidos y dispara la
re-consulta de `worktree/diff`; los propios llegan además por el stream de
eventos de sesión. **GUI**: no tiene tick de datos y esta change no le
inventa uno — el tablero se actualiza al concluir turnos **propios** (el
stream de eventos de sesión, sellado por origen: una conexión ajena no ve
el stream de otra) y conserva el refresco manual para todo lo demás; un
despacho lanzado desde otra superficie se ve al refrescar o reentrar, y la
superficie lo declara en vez de fingir vida. El canal de eventos de
worktree queda nombrado como delta futuro (`eventos-para-tardios`), no
colado aquí.

### D6 — Verificación: escenarios enlazados, smoke publicado, cero marca manual como meta

Los escenarios del daemon se enlazan por marcador `// Scenario:` en tests
e2e con mock-agent (fixture git temporal, jamás este repo). Los de la GUI
usan el patrón asentado de aserciones de fuente en
`desktop/tests/scenarios_shell.rs` (leer el Svelte, asertar el cableado
exacto; donde exista test unitario TS, asertar que el caso está nombrado).
Los del TUI van por reducer/render/e2e vivo con daemon efímero. La meta es
la de `lanzador`: todo enlazado, cero `verify-mark`. La superficie visual
nueva de la GUI recibe smoke CDP con informe en `docs/qa/` — el método que
ya cazó lo que los tests de cableado no ven. La matriz de paridad
actualiza los punteros de vista de las filas de worktree/checkpoint (GUI
«registry + tablero», TUI «race») **sin ganar filas** — la vigilan los
gates de docs y paridad ya existentes, no un escenario nuevo.
Presupuestos: cero deps nuevas (npm y cargo), gates de huella vigentes.

## Risks / Trade-offs

- **Unión por cadena exacta** (D2): registros escritos antes de esta
  change no existen para despachos viejos — el tablero muestra esas calles
  sin procedencia (campos ausentes, render honesto «—»), no inventa.
- **Diffs grandes en el TUI** (D4): el paneo por salto de caracteres es
  O(chars) por fila; el tope de líneas declarado y la ventana de cola
  acotan el costo. Si duele, el delta futuro es paginación, no color.
- **Sellado por origen** (D5): un tablero abierto en GUI no ve en vivo el
  turno despachado desde el TUI; lo ve concluir por tick. Trade-off
  aceptado y visible; la alternativa (aflojar el sellado) tocaría el
  modelo de privacidad de eventos y no se hace aquí.
- **Deltas pendientes vecinos**: `lanzador-conversacional` (sin archivar)
  modifica requisitos de gui-shell/tui-shell que esta change **no toca** —
  aquí todo es ADDED; el orden de archivo entre ambas es indiferente por
  construcción.

## Migration Plan

Aditivo puro: campos opcionales, registros de índice nuevos, superficies
que evolucionan sin retirar nada. Reversión: retirar los campos y las
superficies; los registros de índice escritos son datos válidos del
esquema vigente y no exigen limpieza.

## Open Questions

- ¿Canal de eventos de worktree cuando el uso real lo pida?
  (`eventos-para-tardios` es el lugar; el tablero de hoy debe bastar para
  la demo del hito.)
- ¿El tablero debe cubrir también las tareas de `implement` (no
  competidas)? Hoy no: una tarea sin carrera se ve en el drill-in de
  revisión como siempre; si la práctica pide unificar, será otra change.
