# Design — senales-de-vida

## Context

Leído el código antes de diseñar, estos hechos fijan el diseño:

- **Un solo movimiento.** El anillo `composer-wind` de 2,4 s (`Home.svelte`,
  `SessionDetail.svelte`) es el único `@keyframes` de la interfaz y el único
  sitio autorizado a llevar el gradiente de marca; `scenarios_shell.rs` pinea
  que solo gira (dos `transform:`), que se **retira** bajo
  `prefers-reduced-motion` y que se enciende con el agente trabajando, nunca
  con la sesión esperando al humano. No hay otra `transition` en la interfaz
  que no sea `none`.
- **Las guardias que no se tocan.** La bandeja (`Permissions.svelte`) no
  declara transición ni animación alguna, y la barra lateral
  (`Sidebar.svelte`) tampoco; la pregunta en el compositor aparece «sin mover
  nada». Las tres están pineadas en `scenarios_shell.rs`.
- **Cinco vistas esperan con una palabra.** Project, Review, Fleet (el arnés),
  Settings (la configuración) y Usage muestran `common.loading` mientras
  esperan; tres botones la usan como etiqueta (validar, enviar la acción de la
  carrera, refrescar la flota) y cambian de ancho.
- **Un turno del agente nace con su primer fragmento** (`conversation.ts`):
  entre la instrucción y ese fragmento, el transcript no muestra nada bajo la
  burbuja humana —solo el anillo del compositor dice que algo pasa—, y la rama
  `conv.noOutput` («sin salida») no distingue un turno abierto de uno cerrado.
  Un turno que una petición de permiso interrumpe queda con `closed: false`
  para siempre, así que `closed` no basta para saber qué turno está en vuelo:
  hace falta el estado de la sesión, que el compositor ya lee (`working`).
- **Un cronómetro que no corre.** `durationLabel(startedAt, endedAt)`
  (`lib/time.ts`) usa `Date.now()` cuando la sesión no terminó, pero nada
  vuelve a pintarlo.
- **La TUI no late.** El bucle del shell (`tui/src/shell/mod.rs`) tiene dos
  brazos —teclas y actualizaciones—; la conexión sondea el estado cada 2 s y
  eso ya redibuja. `SessionRow::is_working()` existe; `SessionInfo` trae
  `startedAt` y `endedAt`, pero no el inicio del turno. La spec `tui-shell` ya
  reserva el gemelo ASCII del spinner (`-\|/`) y exige gemelo para todo glifo.
- **La ventana se ve antes que la interfaz.** `desktop/tauri.conf.json` no
  declara `visible: false`: el sistema pinta la ventana vacía mientras el
  webview carga. El `setup` ya restaura la geometría antes de que la interfaz
  exista; lo que falta es que la ventana no se vea hasta entonces.

## Goals / Non-Goals

**Goals**

- Que la superficie diga «estoy trabajando» y «estoy cargando» con algo más que
  una palabra, sin relajar una sola de las reglas de movimiento.
- Un vocabulario con nombre, tokens y guardias, para que el próximo movimiento
  se diseñe contra algo escrito y el build lo impida si no.
- Nada late si nada trabaja.

**Non-Goals**

- Encender el anillo desde el estado del daemon para sesiones de otro cliente
  (`estado-en-vivo`), y las filas de herramienta con resultado
  (`herramientas-a-la-vista`, que usará este vocabulario).
- Animar la bandeja, las tarjetas de permiso, los avisos o la barra lateral.
- Cambiar el sondeo de 2 s de la conexión de la TUI.

## Decisions

### D1 — Un vocabulario de nueve clases, con tokens

`docs/ux/design-system.md` gana la tabla y `app.css` los tokens: `--dur-1`
(120 ms), `--dur-2` (160 ms), `--dur-pulse` (1,2 s), `--dur-ambient` (2,4 s),
`--dur-mark` (600 ms, el trazo de la marca) y `--ease-out`.

| Clase | Dónde vale | Duración | Sin movimiento |
| --- | --- | --- | --- |
| A ambiental | el anillo del compositor; uno por vista | `--dur-ambient` | se retira; el marco de acento dice el estado |
| B llegada | los ítems nuevos del transcript (turnos, líneas del sistema, turno vacío; **nunca** las tarjetas de permiso) y la paleta al abrirse | `--dur-1` | aparece en seco |
| C cambio de estado | el glifo de `StatusBadge` cuando el estado cambia | `--dur-2` | cambia en seco |
| D esqueleto | la espera de datos (D3) | `--dur-pulse` | barras quietas y el texto visible |
| E flujo | el texto llega como llega, sin efecto de tecleo; el borde lo marca un cursor fijo (D4) | — | igual |
| F herramientas | reservada para `herramientas-a-la-vista` | — | — |
| G TUI | el spinner de la cabecera (D5) | ~100 ms por frame | gemelo ASCII |
| H marca | el trazo del mark en la bienvenida (D7) | `--dur-mark`, una vez | dibujado |
| I ventana | oculta hasta que la interfaz existe (D6) | tope 500 ms | igual |

Reglas: solo opacidad o transform, con una excepción nombrada —la marca dibuja
su contorno con `stroke-dashoffset`, pintura y no layout—; jamás altura,
posición ni color de fondo; jamás la bandeja, las tarjetas de permiso, los
avisos ni la barra lateral.

La llegada **no** se aplica al conmutar de vista: las vistas se montan por
rama, varias no tienen una raíz única donde poner la clase sin romper su alto,
y la bandeja es una de ellas.

### D2 — Una lista cerrada, y un test que la cierra

Los `@keyframes` de `desktop/ui/src` son exactamente `composer-wind`,
`arrive`, `skeleton-pulse` y `mark-draw` (los tres últimos, globales en
`app.css`). Un test recorre la fuente y falla si aparece otro nombre, si un
cuerpo anima algo distinto de opacidad o transform (salvo `mark-draw`), si un
archivo que declara `animation:` no tiene su rama de movimiento reducido que la
retira, si una duración de animación es un literal en vez de un token, o si la
bandeja, los avisos, la barra lateral o la tarjeta de permiso del transcript
declaran animación o transición. Es la guardia que convierte «el design lo
dice» en «el build lo impide».

### D3 — Esqueletos, no «cargando…»

Un módulo puro `lib/skeleton.ts` —el umbral de **150 ms**, los anchos fijos de
las barras y el temporizador que revela o se cancela— probado con `node
--test`, y un componente `Skeleton.svelte`: filas de 32 px, `aria-busy="true"`
y el texto de `common.loading` para lectores de pantalla; en forma compacta,
una sola barra en línea. No aparece si los datos llegan antes del umbral. Late
con un pulso de opacidad; bajo movimiento reducido las barras quedan quietas y
el texto se ve. Nunca un gradiente en barrido: el gradiente es del anillo.

Dónde: Project, Review, el arnés de Fleet, la configuración de Settings y la
primera carga de Usage (su refresco, con datos ya en pantalla, usa la forma
compacta en la cabecera en vez de vaciar la tabla). Todo botón que dice
«cargando…» mientras espera conserva su ancho: sus dos etiquetas comparten una
celda de grid y la que no se usa se oculta, así el botón mide lo que la más
larga en cualquier idioma, cosa que un `min-width` fijo no puede saber.

### D4 — La señal de vida del turno

Un módulo puro `lib/turn-signal.ts` decide, a partir de los ítems de la
conversación y de si el agente trabaja: nada; **esperando** desde la
instrucción que abrió el turno; o **fluyendo** en un turno concreto. Con el
agente trabajando, el último ítem que no sea una instrucción encolada decide:
un turno del agente abierto es el que fluye; cualquier otra cosa (la burbuja
humana, una tarjeta ya decidida, una línea del sistema) significa que el
agente todavía no dijo nada.

- **Esperando**: una línea «◔ trabajando… m:ss» tras el último ítem. El tiempo
  va `aria-hidden`, porque el transcript es una región viva y un reloj dentro
  se anunciaría cada segundo; la palabra se anuncia una vez.
- **Fluyendo**: un cursor **▍ fijo** tras el último fragmento de ese turno,
  `aria-hidden` y sin parpadeo, para no ser un segundo ambiental.
- «Sin salida» queda para el turno **cerrado** sin partes.
- El tiempo de la sesión en la cabecera pasa a cifras tabulares y corre: un
  solo intervalo de 1 s por detalle, armado solo mientras la sesión vive y su
  pestaña está al frente; `durationLabel` gana el `now` explícito.

### D5 — La TUI late solo cuando algo trabaja

Un módulo `tui/src/shell/pulse.rs`: el spinner braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` y su gemelo
ASCII `-\|/`, cada uno con **su propio ciclo** (una tabla de diez frames
mapeada a cuatro daría un salto hacia atrás en cada vuelta); el reloj `mm:ss`
(`h:mm:ss` pasada la hora); la lectura de los instantes RFC 3339 del daemon; y
la decisión de armar el tic, que es una función pura del estado vivo.

- **Cabecera**: `⠋ 2 trabajando` mientras alguna sesión trabaja; cede espacio
  antes que la compuerta, y ninguna de las dos antes que la conexión y los
  permisos.
- **Detalle**: el tiempo de la sesión en `mm:ss` mientras trabaja (desde
  `startedAt`), su duración final cuando terminó, y **nada** cuando vive sin
  trabajar: un reloj que el shell no redibuja es un reloj que miente.
- **Bucle**: un tercer brazo, un intervalo de ~100 ms con la precondición
  `if armed`. En reposo el `select!` no tiene nada que tickear y el pulso no
  añade redibujos. El instante que lee el render se fija antes de cada dibujo,
  así el render sigue siendo una función de su entrada y los tests fijan el
  reloj.

`SessionRow` gana `started_at` y `ended_at`; el contrato no se mueve.

### D6 — La ventana se muestra cuando la interfaz existe

`visible: false` en la configuración. El `setup` aplica la geometría restaurada
con la ventana oculta, como ya hacía, y la ventana se muestra cuando la
interfaz invoca `first_frame` —justo después de montar su primer árbol, con
estilos y tema ya aplicados— o a los 500 ms, lo que llegue antes, y una sola
vez (una compuerta atómica). No se espera a un `requestAnimationFrame`: un
webview oculto puede no pintar, y esperar un frame que la ventana oculta nunca
produce terminaría siempre en el tope.

Y desde el `setup`, antes de que nada pueda mostrarla, la ventana lleva el
fondo de la interfaz: el `--bg` del tema elegido en el estado persistido o, sin
elección, el del sistema. Lo que un webview enseña antes de su primer pintado
es su color de fondo; sin esto, blanco, que en el tema oscuro es exactamente el
destello que la decisión existe para evitar. Los dos colores viven también en
Rust y un test los pinea contra `app.css` (añadido tras medir: revisión propia).

### D7 — La marca, una vez

El mark en la cabecera de la bienvenida, con su contorno dibujado una sola vez
en 600 ms ease-out (`mark-draw`); dibujado desde el inicio bajo movimiento
reducido. Hereda la regla de colores forzados de `.mark`, y su degradado lleva
un id propio para no chocar con el de la barra lateral. Es el único movimiento
de marca: sin splash ni brillo.

### D8 — Specs

`gui-shell` gana cuatro requisitos (vocabulario acotado, esqueletos con umbral,
señal de vida del turno, ventana tras el primer frame) y `tui-shell` uno
(spinner y cronómetro con gemelo ASCII, sin costo en reposo). Todo ADDED: la
regla vigente de movimiento reducido no cambia, se detalla.

### D9 — Cómo se verifica

Las guardias de fuente en `desktop/tests/scenarios_shell.rs`, que además exigen
que el caso ejecutado exista en `desktop/ui/tests/*.test.ts` para lo que
deciden los módulos puros (el umbral, la señal); tests unitarios en
`pulse.rs` y en la compuerta de la ventana. Lo visual se mira sobre el binario
release con capturas por CDP y queda en `docs/qa/`, declarando lo que no se
puede medir aquí.

## Risks / Trade-offs

- **R1 — El tic de la TUI en terminales lentos.** ~100 ms por frame es el
  punto de partida; si conhost parpadea, el intervalo sube. La medición es
  manual y queda declarada en el smoke.
- **R2 — La tentación del segundo ambiental.** El cursor del turno es fijo por
  eso, y el test de D2 cierra la lista.
- **R3 — Maximizar una ventana oculta.** En Windows, restaurar una ventana
  maximizada puede mostrarla antes de la señal; el resultado es el
  comportamiento de hoy, no uno peor, y el smoke lo declara.

## Migration

Visual; sin cambios de contrato ni de daemon. Quien tenga el movimiento
reducido activado no ve animación alguna: los esqueletos quedan quietos con su
texto, el cursor ya era fijo y los relojes siguen contando (una cifra que
cambia no es movimiento).

## Revisión propia (2026-09-26)

Lo que el smoke sobre el binario release enseñó, antes del cierre
(`docs/qa/2026-09-26-senales-de-vida-smoke.md`):

- **La ventana se veía en blanco.** Nacía oculta como decía D6, pero el tope de
  500 ms gana en esta máquina —en frío y en tibio— antes de que la interfaz
  monte, y lo visible era el fondo propio del webview. Corregido en la tarea
  5.3: la ventana lleva el fondo del tema desde el `setup`, y D6 lo dice.
- **Montar no es pintar.** El primer pintado llega ~250 ms después de la señal.
  La señal se queda —en una máquina más rápida gana al tope—, pero lo que evita
  el destello es el fondo, no el instante; el informe lo declara así en vez de
  prometer que el primer frame visible sea ya la interfaz.
- **El cursor no se ve con el agente simulado**, que emite su fragmento y cierra
  el turno en el mismo instante. Lo sostienen el caso ejecutado y la guardia de
  fuente; queda declarado hasta mirarlo con un agente que transmita.
- **Tres hallazgos ajenos a esta change**, cada uno con su tarea: la entrada
  «Nueva sesión» de la barra lateral abre Ajustes, `agent_process.binary`
  registra el último argumento, y una sesión libre espera ~6 s antes de su
  punto de restauración.

