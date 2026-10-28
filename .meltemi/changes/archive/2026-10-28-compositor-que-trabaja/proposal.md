# compositor-que-trabaja

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre `gui-shell`, cero
> dependencias, cero contrato, cero daemon. El gate único carga además la
> **enmienda al sistema de diseño** que esta change necesita: el mantenedor
> firma la excepción de movimiento y la de marca en el mismo acto.

## Why

Hoy, mientras un agente trabaja, el compositor no lo dice. El del Home tiene
un gancho colgante — `class:busy={running}` sin una sola regla CSS que lo use
— y el del detalle de sesión ni eso: la única señal de que hay un turno en
vuelo es el glifo del StatusBadge en el header. El mantenedor pidió la
referencia exacta: GitHub Copilot, donde la caja de texto tiene una luz que
recorre el borde mientras procesa. Es la señal correcta en el sitio correcto:
el compositor es donde el usuario va a escribir lo siguiente, y es ahí donde
debe enterarse de que el agente todavía está en ello.

Y detener una sesión está lejos de donde se mira: un botón fantasma en la
esquina de herramientas del header. Copilot, Claude y Codex ponen el stop en
la caja; el compositor de Meltemi debe ofrecer lo mismo.

Dos reglas vigentes se interponen, y la change las encara por escrito en vez
de esquivarlas:

1. **Motion** (design-system.md): «duraciones 120–160 ms, ease-out,
   opacity/transform only». Un bucle ambiental de 2–3 s no cabe en esa frase.
2. **«Brand: never chrome»** (app.css): los tokens de marca están reservados
   para las marcas y la acción primaria del shell.

La luz que se propone cumple la letra de la segunda mitad de Motion
(literalmente opacity/transform) y contradice la primera; y usa la marca como
señal de estado. El mantenedor ya eligió: colores de la marca. La lectura que
esta change ratifica: la luz no es cromo — es **la** señal de estado del
producto, y es el momento de marca de Meltemi: el viento (`--mel-aegean` →
`--mel-wind`) recorriendo el borde mientras las velas trabajan.

## What Changes

- **El anillo de trabajo**: mientras la sesión trabaja, una luz con el
  degradado de marca recorre el borde del compositor. Técnica dentro del
  presupuesto cross-webview: una capa recortada detrás del marco con un
  `conic-gradient` sobredimensionado que gira por `transform: rotate()` —
  sin `@property`, sin `mask-composite`, sin scroll-driven.
- **Dónde y cuándo, exacto**: en el Home desde que se envía hasta que llega
  `session_started` (el gancho `busy` existente); en el detalle de sesión
  mientras `state ∈ {starting, active}`. **Se apaga en `waiting_permission`**:
  esperándote no es trabajando, y una luz que gira mientras el agente está
  detenido esperando tu decisión sería una luz que miente.
- **Movimiento reducido**: el kill-switch global ya congela la animación; el
  estado se sostiene sin ella — borde estático de acento y el texto de estado
  que la fila del compositor ya lleva. Es exactamente el fallback que el
  design system prescribe («los spinners se vuelven glifos estáticos con
  etiqueta textual»).
- **■ Detener en el compositor**: visible mientras la sesión está viva, junto
  al envío. Abre el **mismo** `ConfirmDialog` que el header (decisión del
  mantenedor: la confirmación se mantiene — cancelar termina la sesión, no un
  turno, y eso merece un segundo gesto). El botón del header se conserva:
  mismo verbo, dos accesos.
- **Enmienda al sistema de diseño**, en `docs/ux/design-system.md` y
  ratificada por el gate: (a) Motion gana la clase «indicador ambiental de
  trabajo» — un solo indicador por vista, bucle lento (2–3 s), jamás anima
  layout, corre únicamente mientras un agente trabaja; (b) la reserva de
  marca gana su excepción escrita: la señal de trabajo es el único cromo que
  puede vestir el degradado de marca.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «El compositor dice que se trabaja» — el anillo en
  los dos compositores con sus condiciones de encendido y apagado, el
  fallback sin movimiento, y el detener con confirmación al alcance de la
  caja. ADDED-only.

## Impact

- Archivos: `desktop/ui/src/lib/views/Home.svelte`,
  `desktop/ui/src/lib/views/SessionDetail.svelte`, `app.css` (tokens del
  anillo), `docs/ux/design-system.md` (enmienda), i18n es/en para la etiqueta
  del detener y el estado textual si el design la pide nueva.
- Cero dependencias, cero contrato, cero daemon. La TUI no gana deuda: su
  indicador de actividad (glifo de streaming) es requisito vigente de
  `tui-shell`; el anillo es presentación de esta superficie.
- Verificación: tests de componente por escenario (encendido por estado,
  apagado en `waiting_permission`, fallback reduced-motion) + smoke visual
  CDP sobre el binario release en los dos temas.
- **Costo del bucle, dicho**: una animación permanente repinta. La técnica
  elegida es composición pura (transform en capa propia); el smoke mide en
  WebView2 que el uso de GPU/CPU en reposo con el anillo activo es marginal,
  y si no lo es, el dial es la duración del ciclo, no el concepto.

## Fuera de alcance

- **Interrumpir el turno sin terminar la sesión**: es `redirigir-turno`, con
  su verbo de daemon y su paridad; este ■ Detener es el `session/cancel` que
  ya existe, acercado a donde se mira.
- **Anillos en otras superficies** (tarjetas de sesión, sidebar, tablero):
  un indicador por vista es la regla que la enmienda fija; multiplicarlo la
  rompería el día uno.
- **Barra de progreso o porcentaje**: ACP no transporta progreso de turno;
  inventarlo sería mentir con precisión.
