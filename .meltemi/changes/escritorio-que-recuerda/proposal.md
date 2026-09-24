# escritorio-que-recuerda

> Vía rápida (fast-forward): cuatro faltas de memoria de la superficie de
> escritorio, cada una de uno o dos días, más su paridad mínima en el CLI.
> Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, hallazgos C-C, C-D, D13 y
> la parte de exportación de D-V1). Absorbe las propuestas nombradas allí
> como `pestanas-que-vuelven`, `bandeja-de-avisos` y
> `la-sesion-dice-su-rama`.

## Why

- **Las pestañas no vuelven.** `desktop/ui/src/lib/ui-state.ts:16-40`
  persiste tema, idioma, última vista, geometría de ventana, plantilla de
  «Abrir con», recientes del editor, frecuencia de la paleta, proyecto
  activo, estado del nav, el saludo de flota y los avisos del sistema — y
  ninguna pestaña. Cada arranque llega al Home con cero pestañas aunque el
  daemon tenga cinco sesiones vivas. Orca restaura scrollback y disposición.
- **Los avisos se van sin dejar rastro.** `desktop/ui/src/lib/notices.ts:22-40`
  retira los avisos informativos a los 6 s y los filtra del store; no hay
  historial. Quien vuelve después de veinte minutos no tiene forma de saber
  qué sesiones terminaron, fallaron o pidieron algo mientras no miraba, y el
  log JSONL lo sabe todo. Orca lo llama «Notifications & Inbox».
- **La sesión no dice su rama.** La spec del tablero pide columna de
  worktree; hoy `projectRoot` ya es la ruta del worktree (parcial) pero la
  lista de sesiones no muestra ni el worktree ni la rama, y una sesión de
  carrera y una sobre el árbol del usuario se ven iguales.
- **No se puede llevar una sesión a otro sitio.** El CLI tiene `log`
  (JSONL); la GUI no exporta nada legible para pegar en una PR o en un
  issue.

## What Changes

- **Pestañas que vuelven**: el conjunto y el orden de pestañas se persisten
  por máquina y proyecto en el estado de UI, y al arrancar se restauran las
  sesiones que aún existen (vivas o terminadas, en su estado real); las que
  ya no existen se descartan sin aviso. El compositor sigue siendo la
  llegada si no había pestañas.
- **Bandeja de avisos con historial**: los avisos se archivan (últimos N por
  máquina) en una bandeja accesible desde la barra de estado, con sesión,
  hora y «ir a la sesión»; los informativos siguen desapareciendo del
  cromo a los 6 s, pero ya no del historial. Alimentada por los mismos
  eventos de hoy (y por `estado-en-vivo` cuando exista).
- **La sesión dice su rama**: columna «Worktree · rama» en Sesiones y en la
  cabecera de la sesión, derivada de `worktree/list` y del `projectRoot`;
  las sesiones sobre el árbol del usuario dicen «árbol de trabajo».
- **Exportar sesión**: «Exportar como Markdown» en el menú de la sesión
  (turnos, herramientas resumidas, decisiones de permiso), y su paridad en
  el CLI: `meltemi log <session> --markdown`.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisitos «Pestañas persistentes por proyecto», «Bandeja
  de avisos con historial» y «Worktree y rama visibles por sesión».
- `session-history`: + exportación Markdown de una sesión.
- `cli-contract`: + `log --markdown`.

## Impact

- Archivos: `desktop/ui/src/lib/{ui-state.ts, session-tabs.ts, notices.ts,
  components/Notices.svelte, components/StatusBar.svelte, views/Sessions.svelte,
  views/SessionDetail.svelte}`, `tui/src/run.rs`, `docs/referencia-cli.md`,
  `docs/paridad-nucleo.md`.
- Cero dependencias nuevas; cero RPC nuevos (la rama sale de `worktree/list`).
- Riesgo: restaurar pestañas de sesiones de otra máquina al cambiar de
  daemon (`maquinas-remotas`); la persistencia lleva la identidad del
  endpoint.

## Fuera de alcance

- Historial de avisos en el daemon o compartido entre clientes: el log de
  sesión ya es la verdad; esto es una vista local.
- Menú nativo de la aplicación: `menu-nativo-aplicacion` (nombrada).
