# Design — sesiones-en-la-barra

## Context

Verificado el 2026-09-06, en el código y en las specs vivas:

- El árbol de la barra se deriva en el cliente de `session/list` sin filtro
  unido al registro de proyectos (`Sidebar.svelte:120`, `tree.ts:87`); cada
  proyecto lista hasta ocho sesiones (`Sidebar.svelte:407`) con avatar, agente,
  pastilla de suscripción y un glifo de estado con su palabra solo para el
  lector de pantalla (`:409-418`). El título de la sesión, que el daemon ya
  deriva (`SessionInfo.title`, `stores.ts:66`), no aparece en la fila.
- La tira de pestañas vive contenida en la vista Sesiones; su primera pestaña es
  el centinela `__list__` con `closable: false` (`SessionTabs.svelte:46-69`);
  `activeSession === null` significa «la lista está delante» (`App.svelte:80`).
  `closeTab` cae en la vecina y luego en `null` (`session-tabs.ts:57`);
  `MAX_SESSION_TABS = 8` está atado al corte de ocho del árbol
  (`session-tabs.ts:13`).
- `openComposer` es «la única puerta al compositor» y cambia la vista a `home`
  (`App.svelte:280`); `Home.svelte` monta solo mientras `view === "home"`
  (`App.svelte:471`) y al enviar espera `session_started` y llama
  `onOpenSession`, que es lo que abre la pestaña de la sesión nueva
  (`Home.svelte:295`). El campo toma
  el foco por un efecto al llegar (`:221`).
- Los dos compositores envían **solo** con Ctrl/Cmd+Enter y sin comprobar
  Shift (`Home.svelte:330`, `SessionDetail.svelte:661`); la paleta también
  (`Palette.svelte:128`). En una sesión que trabaja, enviar despacha
  `session/direct` **sin** `interrupt`, que el daemon encola detrás del turno
  en vuelo (`SessionDetail.svelte:578`, `proto lib.rs:1601`); «Interrumpir y
  enviar» es un botón aparte (`:1140`) que `redirigir-turno` construyó. Con una
  pregunta abierta en texto libre, Ctrl+Enter ya releva (`:665`).
- La barra de estado ya parte las sesiones en trabajando / esperando / listas
  para tu instrucción (`StatusBar.svelte:28`); `LIVE_STATE` cubre los seis
  estados del contrato y falla en compilación si aparece uno nuevo
  (`session-state.ts:18`).
- La TUI agrupa por proyecto con cabecera (`tui/src/shell/render.rs:786`); su
  cursor es un índice plano sobre `live.sessions` (`:833`), así que reagrupar
  no mueve el modelo de selección. Su gemelo de «encolar vs relevar» es `Tab`
  (alterna relevo) + `Enter` (`keymap.rs:104`, `messages.rs:211`); la spec viva
  prohíbe depender de combinaciones Ctrl capturadas por el TTY
  (`tui-shell/spec.md:52`).
- **Colisiones**: `lanzador-conversacional` (52/52, archivo bloqueado en la
  firma v1.5) mantiene MODIFIED sin archivar sobre «Árbol Proyecto → Sesiones
  en el sidebar», «La sesión como acción primaria; proponer como herramienta» y
  «Paridad de vistas y modelo de navegación» en `gui-shell`, y sobre
  «Sesiones agrupadas por proyecto con ámbito conmutable» en `tui-shell`. Sus requisitos del compositor viven en
  la capability nueva `conversational-session`, también sin archivar. La casa
  ya resolvió esta situación una vez (`sidebar-ajustable-y-pestanas`,
  `docs/plan-de-cambios.md:1196`): solo ADDED compone sin pisar texto ajeno.
  El requisito «Varias sesiones abiertas a la vez en pestañas» no lo modifica
  nadie.
- Las pruebas de cableado pinean lo que esta change mueve:
  `scenarios_shell.rs` exige `id: LIST` + `closable: false` (:1419) y los
  paneles ocultos con clave (:1341) —el `id="panel-__list__"` es marcado de
  `App.svelte:503`, no un pin—, `openSessionTab(` exactamente cinco veces
  (:1315) y `openComposer(` seis (:4445);
  `scenarios_multiproyecto.rs` prohíbe toda palabra de animación en
  `Sidebar.svelte` (:328).

## Goals / Non-Goals

**Goals**: que la barra responda «quién me necesita» con símbolo, palabra y
cuenta; que las pestañas se gobiernen desde la barra sin que la tira pierda
nada; que una sesión nueva nazca donde se escribe y con el cursor puesto; que
enviar y encolar sean dos gestos del teclado que dicen lo que hacen.

**Non-Goals**: persistir pestañas; reordenar arrastrando; tocar el daemon o el
contrato; modificar requisitos que `lanzador-conversacional` ya modifica sin
archivar; redefinir qué es «trabajar» o «esperar» (lo dice el contrato).

## Decisions

### D1 — Las cubetas viven dentro del proyecto, y son cuatro

El árbol sigue siendo Proyecto → Sesiones: el proyecto es el ámbito (lo
conmuta), la cubeta es cómo se ordenan sus sesiones. Cuatro cubetas, en orden
de señal, con el mismo criterio que la prioridad de señales del design system
—lo que exige una decisión humana va primero—:

| Cubeta | Estados del contrato | Glifo |
|---|---|---|
| Esperan tu decisión | `waiting_permission` | ● |
| Listas para tu instrucción | `waiting_instruction` | ❯ |
| Trabajando | `active`, `starting` | ▸ |
| Detenidas | `ended`, `interrupted` | ■ |

Los nombres siguen el criterio de la barra de estado —trabajando, esperando,
listas— para que las dos superficies partan igual; las palabras canónicas de cada estado (`activa`, `esperando permiso`…) siguen
en la fila como nombre accesible y en el `title`. Una cubeta vacía no se
dibuja: una cabecera «Trabajando 0» es ruido. Las detenidas se acotan a las
cinco más recientes con «Ver las {n}» hacia la vista Sesiones — el corte de
ocho por proyecto de hoy se reparte así: las detenidas se acotan aquí, y las
vivas se muestran todas. El daemon ya acota las que esperan instrucción
(`session.rs:534`); las que trabajan o esperan permiso son las que el usuario
lanzó y piden atención, y su número lo acota la práctica, no una regla.

La partición es una función pura en `tree.ts` (`bucketSessions`) con su test
unitario. La TUI lleva su propia copia en Rust —TypeScript y Rust no comparten
una función—, y lo que impide que diverjan es un pin: un test de cableado lee
las dos tablas y falla si el orden de las cubetas o los estados de cada una no
coinciden.

Las cadenas no se reciclan, solo el criterio: la barra de estado tiene tres
claves con `{n}` incrustado para cuatro cubetas, así que nacen claves nuevas
`nav.bucket.*` (más «Ver las {n}» y «Abiertas») en ES/EN, y sus gemelas en el
catálogo propio del terminal (`tui/src/shell/messages.rs`).

### D2 — Un solo MODIFIED, sobre el requisito que nadie toca

La única frase viva que esta change contradice es «cuya primera pestaña es la
lista y NO SHALL ser cerrable» (`gui-shell/spec.md:857`). Ese requisito —
«Varias sesiones abiertas a la vez en pestañas»— nació en
`sidebar-ajustable-y-pestanas` y ningún delta sin archivar lo modifica, así
que un MODIFIED sobre él es seguro: el bloque entero se restata con sus once
escenarios y dos ediciones declaradas —«La lista es la primera pestaña y nunca
se cierra» pasa a «La lista es la vista, no una pestaña», y el de reinicio deja
de exigir la lista en pantalla porque la llegada es el compositor—. El fallback
«si no queda ninguna SHALL quedar seleccionada la lista» sigue literalmente
cierto: la lista es la vista Sesiones sin pestaña activa.

La guardia de renombres **no sirve aquí**: `migration.rs` recorre los
directorios de `openspec/specs/` y `gui-shell` no está entre ellos —la
capability nació después de la migración—, así que una entrada `SUPERSEDED`
sería código muerto. El renombre se pinea con un test propio en
`desktop/tests/scenarios_shell.rs` que, tras el archivo, lea
`.meltemi/specs/gui-shell/spec.md` y compruebe que el escenario nuevo existe y
el viejo no.

Todo lo demás es ADDED. Las cubetas **no** modifican «Árbol Proyecto →
Sesiones en el sidebar»: el requisito vivo dice que el árbol agrupa por
proyecto y que cada sesión muestra avatar, suscripción y estado; la versión
de `lanzador-conversacional` añade la sección permanente y la acción rápida.
Las cubetas añaden un nivel dentro del proyecto y no quitan nada de ninguna
de las dos versiones, así que un requisito ADDED que dice «dentro de cada nodo
de proyecto…» compone con cualquiera de las dos que acabe archivada.

### D3 — La barra gobierna las pestañas; la tira las sigue mostrando

La sección «Abiertas (n)» es la tira vista de lado: el mismo conjunto, el
mismo orden, la misma pestaña al frente, el mismo cierre con el mismo
fallback (`closeTab` es una función y se llama desde los dos sitios). No es
una segunda verdad: `openSessions`/`activeSession` siguen siendo el estado del
shell, y la barra recibe ambos como props (hoy no recibe ninguno, por eso no
puede marcar cuál está al frente).

Cerrar desde la barra tiene camino de teclado por construcción: el control de
cierre es un botón con nombre accesible, y `Delete` sobre una fila enfocada
cierra, como en la tira. Con la barra plegada a riel la sección se oculta con
el árbol (`aside.folded .tree`) y la tira sigue ahí: nada se vuelve
inalcanzable, que es lo que exige «Plegada no pierde alcance».

Las filas de las cubetas marcan «abierta en pestaña» con un punto y la
palabra en el `title`; seleccionar una fila de cubeta abre o enfoca su
pestaña por el mismo `openSessionTab` de siempre (que rehúsa en el tope
nombrando el remedio).

### D4 — La sesión nueva es una pestaña, y la vista de llegada es esa pestaña

Hoy el compositor es una **vista** (`home`) y la sesión nace en **otra**
pestaña. Con esta change el compositor de sesión nueva es una pestaña
`__new__` dentro de la tira: `openComposer` deja de cambiar la vista a `home`
y pasa a abrir-o-enfocar esa pestaña dentro de la vista Sesiones, con el foco
en el campo. `session/start` conoce la identidad de la sesión antes del primer
token (`free-session`), así que al recibir `session_started` la pestaña
`__new__` **se convierte** en la pestaña de esa sesión: mismo lugar, mismo
panel, el título derivado sustituye a «Nueva sesión». No se abre otra.

La vista de llegada no cambia de experiencia, cambia de forma: un perfil
recién estrenado llega al compositor con el cursor dentro —«Llegar y escribir»
se cumple por construcción—, solo que el compositor es ahora una pestaña y no
una vista. El `ViewId` `home` se conserva como identidad de navegación (la
entrada de la barra, `Ctrl+N`, el registro de la paleta con `view: "home"`, el
último-view recordado) y se resuelve a «la pestaña nueva al frente».

Y aquí hay una fricción que no se disimula: la versión sin archivar de
«Paridad de vistas y modelo de navegación» dice que la vista de aterrizaje
«SHALL ser el compositor conversacional y no una de esas cuatro». El
compositor-pestaña no es una de las cuatro ni es ya una vista; la frase
necesita una enmienda de una línea que **no se escribe aquí**, porque su texto
vive en un delta ajeno sin archivar y solo ADDED compone. La lleva quien
archive segunda, y queda declarada en el proposal como deuda con nombre. Por
eso el escenario de llegada de esta change dice «el compositor al frente» y no
nombra vista alguna.

La pestaña `__new__` cuenta contra el tope (es una pestaña). Cerrarla con
borrador escrito pide decisión, con el mismo `ConfirmDialog` que el editor
usa para una pestaña sucia: un prompt largo es trabajo. Cerrarla vacía no
pregunta. Solo existe una a la vez: pedir «Nueva sesión» con una abierta la
enfoca; cambiar el proyecto desde su chip no abre otra.

### D5 — Dos acordes, y el botón nombra lo que el acorde hará

| Estado de la sesión | `Ctrl+Enter` (despachar ahora) | `Ctrl+Shift+Enter` (encolar) | Botón primario |
|---|---|---|---|
| Pestaña nueva (sin sesión) | inicia la sesión | igual: no hay turno detrás del que esperar | Enviar |
| Espera tu instrucción / reanudable | siguiente turno ahora | igual | Enviar |
| Trabaja, con texto | **releva**: interrumpe y envía | **encola** detrás del turno, nunca interrumpe | Interrumpir y enviar · Encolar |
| Trabaja, sin texto | nada | nada | ninguno ofrecido (regla de `redirigir-turno`) |
| Espera un permiso | encola, como hoy | encola | Encolar |
| Pregunta abierta en texto libre | contesta relevando (como hoy) | encola | como hoy |

El mantenedor pidió dos verbos; para que sean dos, «despachar ahora» mientras
el agente trabaja tiene que ser distinto de «encolar», y lo distinto es lo que
`redirigir-turno` ya hizo posible: relevar. Tres condiciones lo mantienen
seguro. Una: **el compositor lo dice antes** —con un turno en vuelo el botón
primario se llama «Interrumpir y enviar» y «Encolar» está al lado, ambos con su
`kbd`—, así que nadie interrumpe por costumbre sin haber leído la palabra.
Dos: sin texto no se ofrece ninguno de los dos, que es la regla literal de
`redirigir-turno` («Sin texto NO SHALL ofrecerse: no hay nada con lo que
relevar»), y el par ofrecido es exactamente el suyo —«interrumpir y enviar,
junto al envío que la encola»—, con el segundo llamado por lo que hace. Tres:
el texto normativo **nunca** llama «enviar» al acorde que releva, de modo que
el escenario «Enviar no interrumpe» de «Compositor persistente…»
(`conversational-session`, sin archivar) sigue siendo cierto palabra por
palabra: con un turno en vuelo no hay ningún control llamado «Enviar».

Una corrección de hecho sobre el estado «espera un permiso»: hoy la superficie
**no** rehúsa enviar —`canSend` es cierto para todo estado vivo y
`waiting_permission` lo es (`session-state.ts:18`), y el daemon encola sin
mirar el estado (`server.rs:2158`)—. La tabla lo refleja: encola. Deshabilitar
el envío mientras se decide un permiso sería un cambio de comportamiento, y no
es de esta change.

Los tres manejadores existentes de Ctrl+Enter ganan `!event.shiftKey`, y el
nuevo acorde se comprueba primero.

Si el mantenedor prefiere que `Ctrl+Enter` **nunca** interrumpa, el mapeo
alterno es una línea (Ctrl+Enter = encolar, Ctrl+Shift+Enter = interrumpir y
enviar) y esta tabla se reescribe; se deja anotado para que la decisión sea
suya y no del implementador, y es lo primero que la compuerta debería mirar.

Los atajos se muestran solo en la afordancia `kbd` junto a los botones (la
regla viva «El atajo conserva su afordancia»), y `help.keys` los nombra.

### D6 — La TUI con su vocabulario

Dentro de cada proyecto, `render_sessions` ordena las filas por cubeta con una
cabecera de estado por cubeta (glifo ASCII y palabra, como el resto del
terminal), usando la misma tabla de D1 —el cursor plano no se entera—. Los
acordes no tienen gemelo literal: la spec viva del terminal prohíbe depender
de Ctrl y ya tiene el gesto equivalente (`Tab` alterna relevo, `Enter` envía,
el pie de página lo dice). No es un hueco de paridad: es la misma decisión
expresada en el teclado que cada superficie tiene.

### D7 — Lo que la barra no hace

No anima: `scenarios_multiproyecto.rs` prohíbe toda palabra de animación en
`Sidebar.svelte` y esta change lo respeta a rajatabla —una sesión que cambia
de cubeta **salta** de sitio, no se desliza—, porque el árbol contiene la
cubeta «esperan tu decisión» y nada se mueve bajo el cursor mientras se
decide un permiso.

## Risks / Trade-offs

- **Una sesión cambia de cubeta mientras la miras**: el reordenamiento
  instantáneo puede mover una fila bajo el puntero. Es el precio de la regla
  «nada se anima»; se mitiga con la cubeta de decisión siempre primera (el
  destino más frecuente de un salto es el que está más arriba).
- **El acorde que interrumpe**: el riesgo de interrumpir por costumbre queda
  cubierto por el botón que nombra la acción y por la alternativa anotada en
  D5. Si en uso real duele, el mapeo alterno es una línea.
- **Once pines de cableado cambian**: `scenarios_shell.rs` y
  `scenarios_multiproyecto.rs` fijan la anatomía que esta change mueve (la
  pestaña lista, la cuenta de `openComposer`, los paneles con clave). Se
  reescriben con la change, no se debilitan: cada pin nuevo nombra el
  escenario que cubre.
- **Un MODIFIED con once escenarios** es un bloque largo que hay que restatar
  sin errores; el motor rechaza un nombre inexistente, y el renombre lo caza el
  pin propio de D2 (la guardia de migración no llega a `gui-shell`).

## Migration Plan

Aditivo en el daemon (nada cambia) y en la GUI para quien no abra pestañas:
la barra gana secciones y las sesiones aparecen en su cubeta con el mismo
clic de siempre. Quien usaba la pestaña «Lista» encuentra la tabla en la
entrada «Sesiones», que siempre estuvo ahí. Ninguna preferencia persistida
cambia de forma (`desktop-ui.json` no gana campos).

## Open Questions

- ¿Deben las cubetas plegarse? No en esta change: cuatro cabeceras cuestan
  ochenta píxeles y plegar exige recordar el pliegue; si el uso lo pide, es
  una línea en `ui-state`.
- ¿Debe «Abiertas» permitir reordenar? Arrastrar es puntero-solo hasta tener
  su camino de teclado; fuera, con nombre, en el proposal.
- ¿Qué glifo lleva cada cubeta en el vocabulario compartido? La tabla «Status
  vocabulary» del design system no tiene fila para `waiting_instruction` ni
  para `interrupted`, y `Sidebar.svelte` dibuja hoy ‖ y ▲ donde la tabla manda
  ● y reserva ▲ a error. La change añade esas dos filas y alinea el glifo del
  árbol; es enmienda del design system y va con su firma.
