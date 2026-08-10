# Design — compositor-que-trabaja

## Context

Verificado en el código el 2026-08-09:

- **El gancho colgante existe y está muerto**: `Home.svelte:255` declara
  `class:busy={running}` y **no hay una sola regla `.busy`** en toda la
  superficie. `running` se enciende al enviar (`:198`) y se apaga al llegar la
  sesión o al fallar (`:212`, `:236`).
- **Los dos compositores** son `.composer`: `Home.svelte:255` (grid, borde de
  `--border`, `:focus-within` a `--accent`, `:452-463`) y
  `SessionDetail.svelte:750` (`:1054-1074`). El segundo tiene señal real de
  trabajo; el primero solo la ventana entre el envío y `session_started`.
- **Detener ya existe** en la barra de herramientas del encabezado
  (`SessionDetail.svelte:597-601`), abre `ConfirmDialog` (`:803-811`) y notifica
  `session/cancel` (`:492`) — es notificación, no petición.
- **Marca**: `app.css:7` dice literalmente `/* Brand: reserved for marks and
  the shell's primary action. Never chrome. */`, con `--mel-aegean` y
  `--mel-wind` como únicos tokens (`:8-9`); `design-system.md:24-27` añade
  «UI chrome never paints the gradient».
- **Motion** (`design-system.md:177-182`): «Durations 120–160 ms, ease-out,
  opacity/transform only», y `prefers-reduced-motion` «suppresses every
  non-essential animation (spinners become static glyphs with a textual
  "working…" label)».
- **La superficie no tiene una sola animación**: el único `animation-*` del
  repositorio es el kill-switch, y todos los `transition:` existentes son
  `none !important`. Esta sería la primera.

Tres hallazgos que la propuesta no podía conocer y que cambian decisiones:

1. **El kill-switch no apaga: congela.** `app.css:375-383` fuerza
   `animation-duration: 0.01ms` y `iteration-count: 1`, pero **no toca el
   fondo**. Una luz que gira quedaría **detenida y visible** en una posición
   cualquiera — peor que no tenerla, porque un resplandor quieto en el borde no
   dice «no hay movimiento», sugiere un estado que nadie definió. La propuesta
   daba por hecho que «el kill-switch global ya congela la animación» y lo
   trataba como fallback suficiente; no lo es.
2. **`LIVE` incluye `waiting_permission`** (`SessionDetail.svelte:366`), así que
   no puede reutilizarse como condición de la luz sin encenderla justo donde la
   change quiere apagarla.
3. **La acción primaria pinta el degradado con un color suelto**:
   `app.css:256-261` usa `#0891b2` donde el token `--mel-wind` existe. Es la
   única regla que ya viste la marca, y esta change enmienda precisamente esa
   doctrina.

Y un guardián que conviene conocer antes de escribir: hay tests que **prohíben
animaciones** en `Sidebar.svelte`, `Usage.svelte` y `Permissions.svelte`
(`scenarios_multiproyecto.rs:317-331`, `scenarios_analytics.rs:345-354`,
`scenarios_shell.rs:245-263`). Ninguno cubre los dos compositores: el hueco es
legal, no accidental — esas tres superficies son listas y bandejas, donde una
animación desplaza contenido que el usuario está leyendo.

## Goals / Non-Goals

**Goals**: que el compositor diga que el agente trabaja, donde el usuario va a
escribir lo siguiente; que no lo diga cuando el agente está detenido
esperándole; que detener esté al alcance de la caja; y que el sistema de diseño
quede enmendado por escrito en vez de contradicho en silencio.

**Non-Goals**: interrumpir el turno sin terminar la sesión (`redirigir-turno`);
anillos en otras superficies; progreso o porcentaje (ACP no lo transporta).

## Decisions

### D1 — La luz: composición pura, sin nada del presupuesto prohibido

Una capa detrás del marco del compositor con un `conic-gradient` de
`--mel-aegean` → `--mel-wind` sobredimensionado, girando con
`transform: rotate()` en un bucle lento. Sin `@property`, sin
`mask-composite`, sin scroll-driven — las tres cosas que
`design-system.md:184-190` desaconseja por WebView2/WKWebView/WebKitGTK.

Anima **transform y nada más**, que es la mitad de la regla de Motion que sí se
cumple. La otra mitad —la duración— es lo que la enmienda D6 cubre.

### D2 — Encendida solo cuando el agente trabaja de verdad

- **Home**: mientras `running` — la ventana entre enviar y `session_started`.
  El gancho ya está puesto; esta change le da su primera regla.
- **Detalle**: `state === "active" || state === "starting"`. **No** se reutiliza
  `LIVE`, que incluye `waiting_permission` y encendería la luz exactamente
  donde debe apagarse.

Una luz girando mientras el agente está detenido esperando una decisión sería
una luz que miente sobre quién tiene la pelota.

### D3 — Movimiento reducido: se apaga, no se congela

Regla propia bajo `@media (prefers-reduced-motion: reduce)` que **retira la
luz** (no solo su animación) y deja el borde de acento que el compositor ya usa
en `:focus-within`, más el texto de estado que la fila del compositor ya
muestra (`SessionDetail.svelte:764-779`). Es literalmente lo que el design
system prescribe: el indicador se vuelve estático y la palabra lo sostiene.

Confiar en el kill-switch global habría dejado un resplandor detenido en el
borde, que es un estado que nadie definió y que el usuario que pidió menos
movimiento no pidió ver.

### D4 — Por qué la luz y el contador de la barra dicen cosas distintas

La barra de estado cuenta «N en curso» incluyendo las sesiones que esperan
permiso (`StatusBar.svelte:7-8`). La luz se apaga en ese estado. No es
contradicción: responden preguntas distintas — la barra dice **cuántas sesiones
están vivas**, la luz dice **si el agente está trabajando ahora, en esta caja**.
Queda escrito para que no se lea como defecto, y `barra-de-estado-agentica`
—que desglosa ese contador por estado— disuelve la ambigüedad restante cuando
llegue.

### D5 — ■ Detener: el mismo verbo, un segundo acceso

Junto al envío, visible mientras la sesión está viva —aquí **sí** con `LIVE`,
porque detener una sesión que espera una decisión es legítimo y frecuente— y
abriendo el **mismo** `ConfirmDialog` que el encabezado, contra el mismo
`session/cancel`. El botón del encabezado se conserva: dos accesos, un verbo,
una confirmación. La confirmación se mantiene por decisión del mantenedor
registrada en la propuesta: cancelar termina la sesión, no un turno.

### D6 — La enmienda al sistema de diseño, en dos cláusulas

En `docs/ux/design-system.md`, ratificada por el gate de esta change:

1. **Motion gana la clase «indicador ambiental de trabajo»**: un solo
   indicador por vista, bucle lento (2–3 s), anima únicamente transform u
   opacity, **jamás layout**, corre solo mientras un agente trabaja, y bajo
   `prefers-reduced-motion` se retira dejando su equivalente estático con
   palabra. La regla de 120–160 ms sigue rigiendo todo lo demás: es la duración
   de una transición, y esto no es una transición.
2. **La reserva de marca gana su excepción escrita**: la señal de trabajo es el
   único cromo que puede vestir el degradado. El argumento es el que la
   propuesta fija — no es decoración, es **la** señal de estado del producto, y
   el momento de marca de Meltemi: el viento recorriendo el borde mientras las
   velas trabajan.

### D7 — De paso, la acción primaria deja de usar un color suelto

`app.css:256-261` pinta el degradado con `#0891b2` teniendo `--mel-wind`
declarado. Se corrige aquí y no en otra parte porque esta change enmienda la
doctrina de marca: dejar un literal suelto mientras se escribe la excepción
sería enmendar la regla y romperla en la misma tarde. Cambio de una línea, sin
efecto visual (`--mel-wind` es `#22d3ee`, así que el degradado **sí** cambia de
tono: se anota como el único cambio visual colateral y el smoke lo mira).

## Risks / Trade-offs

- **Primera animación de la superficie.** El riesgo no es técnico sino de
  precedente: la enmienda acota la clase para que no se convierta en permiso
  general.
- **Costo del bucle**: una animación permanente repinta. La técnica es
  composición pura sobre su propia capa; el smoke mide el reposo y, si no es
  marginal, el dial es la duración del ciclo, no el concepto.
- **Cambio de tono en la acción primaria** (D7): declarado, no descubierto.

## Migration / Rollout

Solo `desktop/ui` y un documento de diseño. Sin contrato, sin daemon, sin
datos. La TUI no gana deuda: su indicador de actividad es requisito vigente de
`tui-shell` y el anillo es presentación de esta superficie.
