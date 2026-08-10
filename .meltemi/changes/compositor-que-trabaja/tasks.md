# Tareas — compositor-que-trabaja

Vía rápida: gate único al final, que carga **dos firmas en una** — la change y
la enmienda al sistema de diseño, que es normativo. Un commit atómico por
tarea, con referencia `(compositor-que-trabaja N.M)` y sin trailers de
co-autoría. Gates en cada tarea de `desktop/ui`: `npm run check`,
`npm run lint:i18n`, `npm test`, `npm run build`, y la suite de cableado.

## 1. La enmienda, antes que el código

- [x] 1.1 `docs/ux/design-system.md`: Motion gana la clase «indicador ambiental
  de trabajo» (uno por vista, bucle lento de 2–3 s, solo transform u opacidad,
  jamás layout, únicamente mientras un agente trabaja, y retirado —no
  congelado— bajo movimiento reducido); y la reserva de marca gana su excepción
  escrita para la señal de trabajo (design D6). Se escribe primero para que el
  código que viene detrás no contradiga un documento vigente ni un minuto
  <!-- 2026-08-09: la excepción de marca se escribe **acotada a una sola cosa**
  —«nothing else in the surface may claim this exception; a second thing
  wearing the gradient makes both of them decoration»—, porque una excepción sin
  tope deja de ser excepción. Y la cláusula de movimiento reducido dice
  *retirado, no congelado* con su razón técnica escrita: el kill-switch global
  acorta duraciones y no limpia un fondo pintado. -->

## 2. La luz

- [x] 2.1 El anillo en los dos compositores: capa propia con `conic-gradient`
  de marca girando por `transform`, encendida con `running` en el Home y con
  `state ∈ {starting, active}` en el detalle — **nunca con `LIVE`**, que
  incluye la espera de permiso (design D1, D2) — escenarios «El compositor se
  enciende mientras el agente trabaja» y «La luz se apaga cuando la sesión
  espera una decisión»
  <!-- 2026-08-09: la técnica es una capa recortada DETRÁS de un compositor
  opaco, así que solo se ven los dos píxeles que desbordan su marco — eso es la
  luz del borde, sin `mask-composite`. El gradiente cónico va sobredimensionado
  y cuadrado (200%, `aspect-ratio: 1`) para que la rotación no descubra una
  esquina. El detalle enciende con un `working` propio y no con `LIVE`, y el
  test lo exige por el lado negativo. -->
- [x] 2.2 Movimiento reducido: regla propia que **retira** la luz en vez de
  dejar que el kill-switch la congele visible, con el borde de acento y el
  texto de estado sosteniendo la señal (design D3) — escenario «Sin
  movimiento, el estado se sigue diciendo»

## 3. Detener

- [x] 3.1 ■ Detener junto al envío en el detalle, visible mientras la sesión
  está viva (aquí sí `LIVE`), abriendo el **mismo** `ConfirmDialog` contra el
  mismo `session/cancel`; el acceso del encabezado se conserva (design D5) —
  escenarios «Detener desde el compositor» y «Un verbo, dos accesos»

## 4. La deuda de marca que la enmienda deja a la vista

- [x] 4.1 `app.css`: la acción primaria deja de pintar `#0891b2` suelto y usa
  `--mel-wind`, que existe y es lo que la doctrina nombra (design D7). Cambia
  el tono del degradado: es el único cambio visual colateral y se declara aquí

## 5. Cierre

- [x] 5.1 `meltemi validate compositor-que-trabaja` limpio y `meltemi verify`
  con los cinco escenarios enlazados (meta: cero marcas manuales); gates del
  frontend y de Rust verdes
- [x] 5.2 Smoke conducido sobre el binario de release (receta de
  `docs/qa/2026-08-09-piel-de-pestanas-smoke.md`): la luz encendida con una
  sesión activa, **apagada** con una esperando permiso, **retirada** con
  movimiento reducido forzado, el detener del compositor abriendo la misma
  confirmación, y el tono nuevo de la acción primaria. Medir además el reposo:
  si el bucle no es marginal, el dial es la duración del ciclo (design D1).
  Nota en `docs/qa/`
  <!-- 2026-08-09: confirmado en `docs/qa/2026-08-09-compositor-que-trabaja-smoke.md`.
  La medida que vale por todo el argumento: bajo movimiento reducido la luz
  queda en `display: none` **y** la animación en `1e-05s` — o sea, el
  kill-switch global hizo lo único que sabe hacer (acortar la duración) y
  habría dejado el degradado pintado y detenido si la change se hubiera apoyado
  en él. La regla propia era necesaria, y ahora está probada y no argumentada.
  **Trampa de método anotada**: una versión del driver inyectaba un `<span
  class="wind">` para leer la rama fuera de la ventana de trabajo; ese nodo, sin
  la clase de ámbito de Svelte, sobrevivió a las mediciones siguientes y estuvo
  a punto de reportarse como defecto. Recargar antes de medir y no inyectar DOM
  propio. -->
