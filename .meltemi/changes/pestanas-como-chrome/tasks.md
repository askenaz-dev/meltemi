# Tareas — pestanas-como-chrome

Vía completa. Un commit atómico por tarea, con referencia
`(pestanas-como-chrome N.M)` y sin trailers de co-autoría. Gates del repo en
cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite del
crate tocado.

## 1. Una sola fila

- [ ] 1.1 `TabStrip.svelte`: la tira deja de envolver; las pestañas encogen
  hasta un mínimo declarado en un módulo y luego la tira se desplaza; la forma
  se acerca a la de Chrome —contiguas, esquinas superiores redondeadas, la
  activa unida a su panel— (design D1) — escenario «Muchas pestañas no producen
  un segundo renglón» — gates: suite de cableado
- [ ] 1.2 Controles `<` y `>` que solo existen mientras hay desbordamiento,
  deshabilitados en su extremo, medidos con `ResizeObserver`; y el efecto que
  trae la pestaña activa a la vista moviendo lo mínimo (design D2, D3) —
  escenarios «Los controles aparecen solo cuando sobran pestañas» y «La pestaña
  activa nunca queda fuera de vista» — gates: suite de cableado

## 2. Los grupos

- [ ] 2.1 `desktop/ui/src/lib/tab-groups.ts` (módulo puro, cabecera SPDX):
  crear, unir, sacar, renombrar, plegar; una pestaña en a lo sumo un grupo; el
  grupo vacío se destruye; plegar mueve la actividad si hacía falta;
  `desktop/ui/tests/tab-groups.test.ts` con `node --test` cubriendo las cuatro
  reglas (design D4) — escenarios «Salir del grupo y el grupo que se queda
  vacío» y «Plegar el grupo de la pestaña activa mueve la actividad» —
  gates: `npm test`
- [ ] 2.2 La tira dibuja los grupos: franja de color, etiqueta plegable con su
  recuento, y el nombre del grupo dentro del nombre accesible de cada pestaña;
  menú por pestaña para crear, unirse y salir; strings ES/EN (design D5, D6) —
  escenarios «Una pestaña pertenece a un grupo y lo dice» y «Plegar guarda
  espacio, no trabajo» — gates: suite de cableado

## 3. Cierre

- [ ] 3.1 `meltemi validate pestanas-como-chrome` limpio y `meltemi verify` con
  los siete escenarios enlazados (meta: cero marcas manuales); suite completa,
  clippy y fmt verdes; smoke conducido sobre el binario de release con captura
  —ocho pestañas en una sola fila, los controles apareciendo y deshabilitándose,
  un grupo plegado con su recuento y el borrador intacto al desplegarlo—
  y nota de QA
