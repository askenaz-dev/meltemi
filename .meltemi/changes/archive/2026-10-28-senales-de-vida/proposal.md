# senales-de-vida

> Vía completa (proposal → design → specs → tasks): enmienda la sección
> «Motion» del design system y toca las dos superficies; el design es
> obligatorio porque cada movimiento nuevo tiene que pasar por las guardias
> pineadas que hoy prohíben animar la bandeja, las tarjetas de permiso, los
> avisos y la barra lateral. Nace del pedido literal del mantenedor
> («visualmente atractivo: animaciones de carga, animaciones de
> procesamiento») y del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas VIS-3, VIS-4,
> VIS-5/6, VIS-7, VIS-9 y VIS-V1). Absorbe las propuestas nombradas allí como
> `estados-de-carga`, `pulso-de-la-tui`, `senal-de-vida-del-turno`,
> `ventana-hasta-el-primer-frame` y `vocabulario-de-movimiento`.

## Why

Meltemi tiene **un** movimiento: el anillo cónico «composer-wind» de 2,4 s
(`desktop/ui/src/lib/views/Home.svelte:723`, `SessionDetail.svelte:1599`),
que además es el único sitio autorizado a llevar el gradiente de marca. Todo
lo demás está quieto o es texto: la carga de Project, Review, Fleet, Settings
y Usage es la palabra «cargando…» (`Project.svelte:94`, `Review.svelte:320`,
`Fleet.svelte:434`, `Settings.svelte:128`, `Usage.svelte:202`); el cronómetro
de la sesión se calcula una vez (`SessionDetail.svelte:753`) y no corre; un
turno en vuelo sin partes se lee como «sin salida» (`:900`); la TUI no tiene
spinner ni cronómetro aunque la spec `tui-shell` ya reserva su gemelo ASCII
(«spinner por `-\|/` o `...`», `.meltemi/specs/tui-shell/spec.md:336-339`) y
el bucle de `tokio::select!` (`tui/src/shell/mod.rs:82-117`) no tiene brazo
de tick; y la ventana puede pintar el fondo del sistema antes del primer
frame porque `desktop/tauri.conf.json` no declara `visible: false`.

Las capturas del 2026-09-24 lo muestran: una superficie densa, limpia y
**muda**. Correcta como plano de control, incapaz de decir «estoy
trabajando» con algo más que una palabra. Orca dice lo mismo con un spinner
braille, un cronómetro por agente y progreso de herramienta en vivo, y su
landing es exactamente eso en movimiento.

Las reglas ya están escritas y esta change las obedece en vez de relajarlas:
`docs/ux/design-system.md:200-226` fija 120–160 ms, ease-out, solo opacidad y
transform, y que bajo `prefers-reduced-motion` la animación **se retira, no
se congela**; las guardias de `desktop/tests/scenarios_shell.rs:258-272` y
`4814-4832` pinean que nada se mueve mientras se decide un permiso ni en la
barra lateral. El movimiento que falta no está prohibido; está sin diseñar.

## What Changes

- **Un vocabulario de movimiento con nombre**, en `docs/ux/design-system.md`
  y como tokens en `desktop/ui/src/app.css`: `--dur-1: 120ms` (llegada),
  `--dur-2: 160ms` (cambio de estado), `--dur-ambient: 2.4s` (el anillo, ya
  existe), `--dur-pulse: 1.2s` (esqueleto), `--ease-out`. Nueve clases, cada
  una con dónde vale, su duración y su fallback sin movimiento:
  ambiental (el anillo; uno por vista), llegada (opacidad 0→1 al conmutar
  vista y en ítems nuevos del transcript y de la paleta), cambio de estado
  (crossfade de glifo ▸→●, ○→◔→✓/✕), esqueleto, flujo, herramientas, TUI,
  marca y ventana. Jamás altura, posición ni color de fondo animados; jamás
  en bandeja, tarjeta de permiso, avisos ni barra lateral, que siguen
  llegando en seco.
- **Esqueletos en vez de «cargando…»**: filas de 32 px con barras de anchos
  fijos y `aria-busy`, solo cuando la espera supera 150 ms; pulso de opacidad
  de 1,2 s; bajo reduced-motion, barras estáticas más el texto de
  `messages.ts`. Nunca un gradiente en barrido: el gradiente es del anillo.
  Los botones que dicen «cargando…» ganan `min-width` para que la etiqueta no
  salte.
- **Señal de vida del turno en el transcript**: cronómetro que corre
  (1 s, cifras tabulares) mientras el turno está abierto, «◔ trabajando…
  m:ss» mientras no hay partes en vez de «sin salida», y un cursor ▍ fijo
  tras el último fragmento — fijo, no parpadeante, para no ser el segundo
  ambiental de la vista. Se alimenta del estado local de streaming que el
  cliente ya tiene; el estado vivo desde el daemon es `estado-en-vivo`.
- **Pulso de la TUI**: frames braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` con gemelo `-\|/` bajo
  `--ascii`/`MELTEMI_ASCII`, cronómetro `mm:ss` en la cabecera y en el
  drill-in, y un tercer brazo de `select!` que solo se arma cuando hay algo
  trabajando: cero CPU en reposo, probado en conhost (constitución §7).
- **Ventana hasta el primer frame**: `visible: false` en la configuración de
  Tauri y `show()` desde `desktop/src/lib.rs` cuando el frontend señala su
  primer frame, con un tope de 500 ms aunque no señale; la geometría se
  aplica antes de mostrar.
- **La marca, una vez**: el trazo del mark en el onboarding (600 ms, ease-out,
  estático bajo reduced-motion); único sitio con animación de marca, sin
  splash ni glow (design system).
- **Las guardias no se relajan: se amplían.** Cada clase nueva tiene su test
  que la limita a donde vale, y el guardián de «nada se mueve mientras se
  decide un permiso» sigue escaneando fuente y comentarios.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisitos «Estados de carga con esqueleto y umbral»,
  «Señal de vida del turno», «Ventana visible tras el primer frame» y
  «Vocabulario de movimiento acotado» (clases, tokens, dónde jamás).
- `tui-shell`: + requisito «Spinner y cronómetro con gemelo ASCII, sin costo
  en reposo».

## Impact

- Archivos: `docs/ux/design-system.md`, `desktop/ui/src/app.css`, las cinco
  vistas con carga, `SessionDetail.svelte`, `Palette.svelte`,
  `Onboarding.svelte`, `desktop/tauri.conf.json`, `desktop/src/lib.rs`,
  `desktop/tests/scenarios_shell.rs`, `tui/src/shell/{render.rs, glyphs.rs,
  state.rs, mod.rs}`, `tui/tests/`, `docs/qa/` (smoke visual).
- Cero dependencias nuevas; cero RPC; ningún cambio de daemon. Es la tanda
  visual que puede avanzar en paralelo con las de daemon.
- Riesgo real: el tick de la TUI en terminales lentos y el parpadeo en
  conhost; se mide antes de fijar los 80–100 ms por frame. Y la tentación de
  un segundo ambiental: el design lo dice y el test lo pinea.

## Fuera de alcance

- Encender el anillo desde el estado del daemon y ver «trabajando» en la
  barra lateral cuando otro cliente arrancó la sesión: `estado-en-vivo`.
- Las filas de herramienta con resultado y diff: `herramientas-a-la-vista`
  (usa las clases B y C de aquí).
- Animar la bandeja, las tarjetas de permiso, los avisos o los contadores de
  la barra lateral: prohibido, y sigue prohibido.
