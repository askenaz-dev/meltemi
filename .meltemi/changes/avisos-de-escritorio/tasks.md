# Tareas — avisos-de-escritorio

Vía completa. Capability nueva `attention-notices`. **Cero dependencias
nuevas** (design D1: el plugin se aplaza con su condición escrita). Un commit
atómico por tarea, con referencia `(avisos-de-escritorio N.M)` y sin trailers
de co-autoría.

## 1. Los disparadores que faltan

- [ ] 1.1 Un módulo puro que decida **si se pide atención y con qué título**
  a partir de la transición, el foco y el recuento — la lógica que hoy vive
  repartida entre `stores.ts` y `App.svelte`, con sus tests (design D2, D3) —
  escenarios «Lo mismo no se pide dos veces» y «El motivo viaja, el contenido
  no»
- [ ] 1.2 La compuerta que espera dispara atención, leyendo el store que
  `barra-de-estado-agentica` elevó (design D2) — escenario «Una compuerta que
  espera pide atención»
- [ ] 1.3 El fin o la interrupción de una sesión dispara atención, en la
  transición y no en el estado (design D2) — escenario «Una sesión que termina
  pide atención»
- [ ] 1.4 Los dos escenarios de foco quedan cubiertos por test sobre el
  mecanismo existente, que ya los implementa — escenarios «Se pide atención
  cuando un permiso queda esperando» y «Con la ventana al frente no se pide
  atención»

## 2. El terminal

- [ ] 2.1 Campana opt-in por configuración ante los mismos momentos, apagada
  por defecto (design D4) — escenarios «Sin activar, el terminal no suena» y
  «Activada, el terminal suena en los mismos momentos»

## 3. Cierre

- [ ] 3.1 `meltemi validate avisos-de-escritorio` limpio y `meltemi verify` con
  los ocho escenarios enlazados; suite, clippy, fmt y gates del frontend verdes
- [ ] 3.2 Comprobación manual documentada de que el sistema honra la petición
  en cada plataforma —parpadeo, rebote, *urgency hint*—, que es lo único que
  CI headless no puede aseverar. Se marca con `verify-mark` y su nota, como
  estableció `conformidad-manual`
