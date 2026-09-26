# Tasks — senales-de-vida

Las tareas de la interfaz corren además los gates del frontend desde
`desktop/ui` (`npm run check`, `npm test`, `npm run lint:i18n`), con
`node_modules` enlazado desde el checkout principal.

## 1. El vocabulario y su guardia

- [x] 1.1 `docs/ux/design-system.md`: la sección «Motion» gana la tabla de
  las nueve clases (dónde vale, duración, equivalente sin movimiento), los
  tokens, la excepción nombrada de la marca y la lista de superficies que no
  se mueven; `app.css` gana `--dur-1`, `--dur-2`, `--dur-pulse`,
  `--dur-ambient`, `--ease-out` y los `@keyframes` globales `arrive`,
  `skeleton-pulse` y `mark-draw`; el anillo toma `--dur-ambient` en sus dos
  vistas; la guardia de la lista cerrada, de las duraciones por token, de la
  rama de movimiento reducido por archivo y de las superficies de decisión
  quietas (bandeja, avisos, barra lateral, tarjeta de permiso del transcript)
  en `scenarios_shell.rs` (design D1, D2) — escenarios «Una animación fuera de
  la lista rompe la suite», «Toda animación se retira con movimiento
  reducido», «Las duraciones salen de los tokens», «Las superficies de
  decisión no se mueven» — gates: `-p meltemi-desktop --test scenarios_shell`

- [x] 1.2 La llegada y el cambio de estado: `arrive` en los ítems nuevos del
  transcript (turnos, líneas del sistema, turno vacío; nunca la tarjeta de
  permiso) y en la paleta al abrirse, y el fundido del glifo de `StatusBadge`
  al cambiar de estado, cada uno con su retiro bajo movimiento reducido
  (design D1) — gates: la guardia de 1.1, `npm run check`

## 2. Esqueletos

- [x] 2.1 `lib/skeleton.ts` (umbral de 150 ms, anchos fijos, temporizador que
  revela o se cancela) con su prueba ejecutada en
  `desktop/ui/tests/skeleton.test.ts`, y `Skeleton.svelte` (filas de 32 px,
  forma compacta, `aria-busy`, texto accesible, pulso de opacidad, barras
  quietas y texto visible bajo movimiento reducido, sin gradiente); en
  Project, Review, el arnés de Fleet, la configuración de Settings y Usage
  (primera carga en bloque, refresco compacto); ancho estable (`.steady`) en
  todo botón que dice «cargando…» mientras espera (design D3) — escenarios «Una carga rápida no
  parpadea», «Una carga lenta muestra el esqueleto», «El esqueleto se queda
  quieto con movimiento reducido», «Ninguna vista espera con una palabra
  suelta» — gates: `npm test`, `npm run check`, `-p meltemi-desktop --test
  scenarios_shell`

## 3. La señal de vida del turno

- [x] 3.1 `lib/turn-signal.ts` (nada, esperando desde la instrucción o
  fluyendo en un turno; el reloj `m:ss`) con su prueba ejecutada en
  `desktop/ui/tests/turn-signal.test.ts`; en `SessionDetail.svelte` la línea
  «◔ trabajando… m:ss» con el tiempo `aria-hidden`, el cursor ▍ fijo tras el
  último fragmento del turno que fluye, «sin salida» solo para un turno
  cerrado, y el tiempo de la cabecera en cifras tabulares con un intervalo de
  1 s armado mientras la sesión vive y su pestaña está al frente;
  `durationLabel` con `now` explícito; `conv.working` en ES y EN (design D4)
  — escenarios «Antes del primer fragmento se dice que trabaja», «El cursor
  sigue al último fragmento sin parpadear», «Esperar una decisión no es
  trabajar», «El tiempo de la sesión corre mientras vive» — gates: `npm test`,
  `npm run check`, `npm run lint:i18n`, `-p meltemi-desktop --test
  scenarios_shell`

## 4. El pulso de la TUI

- [ ] 4.1 `tui/src/shell/pulse.rs`: los dos ciclos del spinner, el reloj
  `mm:ss`, la lectura de instantes RFC 3339 y la decisión de armar el tic;
  `SessionRow` gana `started_at` y `ended_at`; `LiveData` gana el pulso que el
  bucle fija antes de cada dibujo; la cabecera muestra el spinner con el número
  y la palabra (cede antes que la compuerta); el detalle muestra el tiempo de
  la sesión solo cuando puede ser cierto; el tercer brazo del `select!` con
  `if armed`; unit tests de las funciones puras y del render con el reloj
  fijado (design D5) — escenarios «La cabecera late mientras una sesión
  trabaja», «El spinner ASCII usa su propio ciclo», «El detalle cuenta el
  tiempo de la sesión que trabaja», «En reposo no hay tic» — gates: suite de
  `meltemi`

## 5. La ventana y la marca

- [ ] 5.1 `visible: false` en `desktop/tauri.conf.json`; la compuerta que
  muestra la ventana una sola vez, el comando `first_frame` y el tope de
  500 ms en `desktop/src/lib.rs`, con la geometría restaurada antes de mostrar;
  `main.ts` invoca `first_frame` tras montar; unit test de la compuerta y
  guardia de la configuración y del cableado (design D6) — escenarios «La
  ventana nace oculta y con su geometría», «La interfaz lista muestra la
  ventana», «Una interfaz muda no deja la ventana invisible» — gates: suite de
  `meltemi-desktop`

- [ ] 5.2 El mark en la bienvenida con su contorno dibujado una vez en 600 ms
  (`mark-draw`), dibujado desde el inicio bajo movimiento reducido, degradado
  con id propio y la regla de colores forzados heredada (design D7) — escenario
  «La marca se dibuja una vez en la bienvenida» — gates: `-p meltemi-desktop
  --test scenarios_shell`, `npm run check`

## 6. Smoke y cierre

- [ ] 6.1 Smoke visual sobre el binario release por CDP y del pulso de la TUI
  en la consola de esta máquina, en `docs/qa/<fecha>-senales-de-vida-smoke.md`:
  lo medido, lo capturado y lo declarado (parpadeo en conhost, WebKitGTK y
  WKWebView, maximizar oculto en Windows) — gates: el informe

- [ ] 6.2 Cerrar la change: `validate`, `verify` con todos los escenarios
  enlazados, gate completo del workspace (fmt, clippy, suite) y del frontend,
  `docs/plan-de-cambios.md` al día — gates: los del cierre
