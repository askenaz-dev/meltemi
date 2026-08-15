# Tareas — avisos-de-escritorio

Vía completa. Capability nueva `attention-notices`. **Una dependencia nueva**
—`tauri-plugin-notification`, pineada y confinada al cliente GUI (design D1)—
que obliga a re-medir el presupuesto de instalador. Un commit atómico por
tarea, con referencia `(avisos-de-escritorio N.M)` y sin trailers de
co-autoría.

## 1. Los disparadores que faltan

- [x] 1.1 Un módulo puro que decida **si se pide atención y con qué título**
  a partir de la transición, el foco y el recuento — la lógica que hoy vive
  repartida entre `stores.ts` y `App.svelte`, con sus tests (design D2, D3) —
  escenarios «Lo mismo no se pide dos veces» y «El motivo viaja, el contenido
  no»
  <!-- 2026-08-10: el módulo es puro y no tiene acceso al texto del turno **por
  construcción**, que es más fuerte que prometer no usarlo: lo que puede decir
  es el motivo, el recuento y un sujeto que compone la superficie (nombre de
  change, título de sesión). Ocho tests, incluido el de ráfaga —dos llegando a
  la vez es un momento distinto de uno; el mismo número repintado no lo es—. -->
- [x] 1.2 La compuerta que espera dispara atención, leyendo el store que
  `barra-de-estado-agentica` elevó (design D2) — escenario «Una compuerta que
  espera pide atención»
- [x] 1.3 El fin o la interrupción de una sesión dispara atención, en la
  transición y no en el estado (design D2) — escenario «Una sesión que termina
  pide atención»
- [x] 1.4 Los dos escenarios de foco quedan cubiertos por test sobre el
  mecanismo existente, que ya los implementa — escenarios «Se pide atención
  cuando un permiso queda esperando» y «Con la ventana al frente no se pide
  atención»

## 2. El aviso del sistema

- [x] 2.1 `tauri-plugin-notification` pineado en el workspace y registrado en
  el cliente, con su permiso en las capacidades de Tauri —que hoy son
  deny-by-default y solo conceden `core:default`— y `cargo deny check` verde
  (design D1)
- [x] 2.2 El aviso se emite en los mismos momentos y bajo la misma regla de
  foco que la petición de atención, con el contenido mínimo de D3 (design D1) —
  escenario «El permiso se pide cuando hay algo que decir»
- [x] 2.3 Estados honestos: permiso denegado o servicio ausente se declaran con
  su remedio en Ajustes, y el aviso se puede apagar; **nunca se registra como
  emitido lo que el sistema no entregó** (design D1) — escenarios «Sin permiso,
  se dice y no se finge» y «El aviso se puede apagar»
- [ ] 2.4 Re-medir el presupuesto de tamaño del instalador con la dependencia
  dentro y anotar el número, que es lo que la propuesta pidió no suponer

## 3. El terminal

- [x] 3.1 Campana opt-in por configuración ante los mismos momentos, apagada
  por defecto (design D4) — escenarios «Sin activar, el terminal no suena» y
  «Activada, el terminal suena en los mismos momentos»
  <!-- 2026-08-10: la decisión es un módulo puro (`bell.rs`) y la emisión vive
  en el bucle, **antes** de aplicar el update: así la comparación se hace contra
  la compuerta que el shell todavía sostiene y la misma compuerta reportada dos
  veces es un momento, no dos campanas. El interruptor se lee **una vez** al
  arrancar, siguiendo `MELTEMI_ASCII`: una variable de entorno que cambia a
  mitad de ejecución no es una cosa, y releerla por update sería fingir que sí.
  Cualquier valor que no sea vacío ni `0` la enciende — adivinar ortografías
  solo produce una campana que ignora a la mitad de quienes la pidieron. -->

## 4. Cierre

- [x] 4.1 `meltemi validate avisos-de-escritorio` limpio y `meltemi verify` con
  los once escenarios enlazados; suite, clippy, fmt y gates del frontend verdes
- [ ] 4.2 Comprobación manual documentada de que el sistema honra la petición
  en cada plataforma —parpadeo, rebote, *urgency hint*— **y de que el aviso del
  sistema aparece de verdad** (sobre el bundle en macOS, desde el MSI en
  Windows, con DBus presente en Linux), que es lo único que CI headless no
  puede aseverar. Se marca con `verify-mark` y su nota, como
  estableció `conformidad-manual`
