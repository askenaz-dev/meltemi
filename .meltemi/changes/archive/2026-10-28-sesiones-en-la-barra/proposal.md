# sesiones-en-la-barra

> Vía completa (proposal → design → specs → tasks). El delta lleva un
> **MODIFIED** sobre un requisito vivo de `gui-shell` («Varias sesiones abiertas
> a la vez en pestañas»: la lista deja de ser una pestaña), así que la vía
> rápida queda descartada por criterio, no por gusto. Todo lo demás entra como
> ADDED que compone con los deltas sin archivar que hoy tocan la barra lateral
> (design D2). Cero contrato, cero daemon: no nace deber de paridad §4, y la
> paridad de superficies (la TUI) se paga igual porque es la misma lista.

## Why

El mantenedor revisó la maqueta de acabado del shell (2026-09-06) y dijo
cuatro cosas con sus palabras. Primero: «las sesiones tamb deben aparecer a la
izquierda en el menú y diferenciadas por en curso, en pausa, esperando
respuesta del usuario, detenido, etc, los distintos estados». Segundo: «lo que
dice lista, no tiene sentido, no sirve para nada». Tercero: «al hacerle nueva
sesión debe crear el tab y además dejar el cursor en el campo de texto con
foco», y «el campo debe permitir ctrl + enter para enviar o ctrl + shift + enter
para encolar mensaje». Cuarto, mirando la barra de nuevo: «no veo donde la
gestión de tabs a la izquierda».

Lo que hay hoy, verificado en el código: el árbol de la barra agrupa las
sesiones **por proyecto** y las lista de la más reciente a la más antigua, con el agente, la
suscripción y un glifo de estado sin palabra visible
(`desktop/ui/src/lib/components/Sidebar.svelte:407-418`); no muestra el título
de la sesión aunque las pestañas sí lo hacen desde `titulo-de-sesion`. Las
pestañas viven en una tira dentro de la vista Sesiones cuya **primera pestaña
es «Lista»** y no se cierra (`SessionTabs.svelte:69`): esa pestaña duplica lo
que la entrada «Sesiones» de la barra ya hace —volver a la tabla— y por eso no
significa nada para quien la mira. «Nueva sesión» abre el compositor como
**vista** (`App.svelte:280`, `openComposer`), no como pestaña: mientras se
escribe, las pestañas abiertas desaparecen de la pantalla, y al enviar la
sesión nace en otra pestaña distinta del sitio donde se escribió. Y los dos
compositores atienden `Ctrl+Enter` sin comprobar Shift
(`Home.svelte:330`, `SessionDetail.svelte:661`, también `Palette.svelte:128`),
así que `Ctrl+Shift+Enter` hoy dispara lo mismo que `Ctrl+Enter`: no hay
gesto de encolar, solo el que el daemon decide cuando la sesión trabaja.

Nada de esto es cosmético. La barra es «la forma visible del keymap» según
el design system (`docs/ux/design-system.md`, Shell architecture), y hoy no responde a la pregunta que un operador de varios
agentes hace cada minuto: **¿quién me necesita?** Las cuatro respuestas —te
pide una decisión, espera tu instrucción, trabaja, terminó— existen ya en el
contrato (seis estados) y en la barra de estado (`StatusBar.svelte:28` parte
en trabajando/esperando/listas), pero el sitio donde se elige a quién atender
no las usa.

## What Changes

- **Las sesiones de cada proyecto se agrupan por lo que piden.** Dentro de cada
  nodo de proyecto del árbol, cubetas en orden de señal: **● esperan tu
  decisión** (`waiting_permission`), **❯ listas para tu instrucción**
  (`waiting_instruction`), **▸ trabajando** (`active`, `starting`) y
  **■ detenidas** (`ended`, `interrupted`). Cada cubeta lleva glifo, palabra y
  cuenta; una cubeta vacía no ocupa sitio; cada fila conserva el avatar y la
  pastilla de suscripción que el requisito vivo del árbol exige, y gana el
  título de la sesión (la regla de `titulo-de-sesion`) y una marca de «abierta
  en pestaña». Las detenidas se acotan a las más recientes con «Ver las {n}» hacia
  la vista Sesiones. El árbol sigue agrupado por proyecto: la cubeta es un
  nivel **dentro** del proyecto, no un reemplazo (design D1).
- **Las pestañas abiertas se gobiernan desde la barra.** Una sección
  «Abiertas (n)» bajo la navegación lista las pestañas en el orden de la tira:
  seleccionar trae al frente, cerrar cae en la vecina como en la tira, la que
  está al frente se marca por forma y palabra. Cerrar una pestaña no cierra la
  sesión: sigue en su cubeta. Con la barra plegada la sección se va con el
  árbol y la tira sigue siendo el camino (design D3).
- **«Lista» deja de ser una pestaña.** La tabla de sesiones es la vista
  Sesiones de la navegación; la tira contiene solo sesiones abiertas. Cerrar la
  última pestaña deja la tabla en pantalla, y «Sesiones» en la barra vuelve a
  ella. Es el único requisito vivo que se modifica (design D2).
- **Nueva sesión nace como pestaña con el cursor dentro.** Cualquier puerta —
  el botón primario, la entrada de la barra, el «+» de un proyecto, `Ctrl+N` —
  abre una pestaña «Nueva sesión» con el compositor y deja el foco en el campo.
  Pedirla de nuevo enfoca, no duplica. Enviar **convierte** esa pestaña en la
  de la sesión (la identidad se conoce antes del primer token; el título se
  deriva de la primera instrucción): no se abre otra. La vista de llegada es
  esa misma pestaña (design D4).
- **Dos acordes en todo compositor**, con los verbos que el mantenedor pidió:
  `Ctrl+Enter` (Cmd en macOS) **despacha ahora**; `Ctrl+Shift+Enter`
  **encola** detrás del turno y nunca interrumpe; Enter a secas sigue siendo
  salto de línea. Con la sesión esperando, despachar ahora es enviar el
  siguiente turno. Con un turno en vuelo, despachar ahora es **relevar** —la
  puerta que `redirigir-turno` ya construyó— y el compositor lo dice **antes**
  de que se pulse: el botón primario se llama «Interrumpir y enviar», «Encolar»
  está a su lado, y sin texto no se ofrece ninguno de los dos. Los tres
  manejadores que hoy atienden `Ctrl+Enter` comprueban Shift para que los
  acordes no se pisen. Los atajos se muestran en la afordancia `kbd`, nunca
  dentro de una etiqueta (regla viva) (design D5).
- **La TUI hace lo mismo con su vocabulario**: dentro de cada proyecto, las
  filas se ordenan por cubeta con su cabecera de estado; los acordes no tienen
  gemelo literal porque la spec viva prohíbe depender de combinaciones Ctrl
  capturadas por el TTY — el gemelo existente es `Tab` (relevo) + `Enter`, que
  ya está y se declara como tal (design D6).

## Capabilities

### Modified Capabilities

- `gui-shell`: + cuatro requisitos ADDED (las cubetas por estado dentro del
  proyecto, las pestañas gobernadas desde la barra, la sesión nueva como
  pestaña enfocada, los dos acordes del compositor) y **un MODIFIED** sobre
  «Varias sesiones abiertas a la vez en pestañas», que restata el bloque
  entero con dos ediciones declaradas: el escenario «La lista es la primera
  pestaña y nunca se cierra» pasa a llamarse «La lista es la vista, no una
  pestaña» con sus pasos reescritos, y el de reinicio deja de exigir «la lista
  en pantalla» porque la llegada es el compositor. Los otros nueve escenarios
  se restatan palabra por palabra. La guardia `migration.rs::SUPERSEDED` **no**
  cubre `gui-shell` (capability posterior a `openspec/`), así que el renombre
  se pinea con un test propio (design D2). Ningún requisito que
  `lanzador-conversacional` modifique sin archivar se toca.
- `tui-shell`: + un requisito ADDED (las cubetas dentro del proyecto en el
  terminal). Nada MODIFIED: «Sesiones agrupadas por proyecto con ámbito
  conmutable» ya lo modifica `lanzador-conversacional` sin archivar.

### Fricciones declaradas con deltas sin archivar

Dos textos sin archivar de `lanzador-conversacional` hablan de lo mismo que
esta change, y no se tocan aquí porque solo ADDED compone; la reconciliación
se declara y se secuencia, no se da por hecha (design D4, D5):

- «Paridad de vistas y modelo de navegación» dice que la vista de aterrizaje
  «SHALL ser el compositor conversacional y no una de esas cuatro». Con esta
  change el compositor deja de ser una vista y pasa a ser **la pestaña de
  llegada**. Quien archive segunda lleva la enmienda de esa frase; se anota
  como deuda con nombre, no como composición silenciosa.
- «Compositor persistente…» tiene el escenario «Enviar no interrumpe». Sigue
  siendo cierto: con un turno en vuelo esta change no ofrece ningún control
  llamado «Enviar» —ofrece «Encolar» e «Interrumpir y enviar», que es
  exactamente el par que `redirigir-turno` manda ofrecer—, y el acorde que
  releva se nombra *relevar* en todo texto normativo.

### New Capabilities

- Ninguna.

## Impact

- `desktop/ui/src/lib/components/Sidebar.svelte` (cubetas, sección Abiertas,
  título en la fila), `tree.ts` (partición por cubeta), `SessionTabs.svelte`
  y `session-tabs.ts` (sin pestaña lista; la pestaña de sesión nueva),
  `App.svelte` (la sesión nueva como pestaña; `openComposer` deja de cambiar de
  vista), `Home.svelte` y `SessionDetail.svelte` (los acordes, el botón que
  nombra su intención), `Palette.svelte` (guardia de Shift), `messages.ts`
  (ES/EN), `tui/src/shell/render.rs` (cubetas), y las pruebas de cableado que
  pinean lo anterior (`desktop/tests/scenarios_shell.rs`,
  `core/meltemid/tests/scenarios_multiproyecto.rs`, `desktop/ui/tests/*.test.ts`).
- **No hay contrato ni daemon nuevo**: `session/list` sin filtro ya trae los
  seis estados y el título; `session/direct` ya distingue `interrupt` presente
  o ausente. Por tanto **no nace deber de paridad §4**; la TUI entra por
  paridad de superficies, no de métodos.
- Cero dependencias nuevas.
- `docs/ux/design-system.md` gana dos frases en «Shell architecture» (cubetas
  y sección Abiertas) y una en «Focus and keyboard» (los acordes), con la
  change y la fecha, como manda su cabecera.

## Fuera de alcance

- **Persistir las pestañas abiertas entre arranques**: la deuda declarada de
  `sidebar-ajustable-y-pestanas` sigue en pie (medir el arranque con ocho
  pestañas antes). El escenario vivo «reiniciar sí las olvida» se conserva.
- **Reordenar pestañas arrastrando** desde la barra: gesto puntero-solo hasta
  que tenga camino de teclado; change propia si el uso lo pide.
- **Un tope por cubeta configurable**: las detenidas se acotan a un número fijo
  con «Ver las {n}»; ajustar ese número es de `Ajustes`, no de aquí.
- **Cambiar la semántica de `session/direct`**: los acordes mapean a lo que el
  daemon ya ofrece (`interrupt` presente o ausente); nada nuevo en el borde
  del turno.
- **El menú nativo de la aplicación** (`menu-nativo-aplicacion`, nombrada sin
  abrir): esta change es la barra lateral; el nombre se eligió para no
  confundirlas.
