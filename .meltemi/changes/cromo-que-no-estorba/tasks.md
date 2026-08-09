# Tareas — cromo-que-no-estorba

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(cromo-que-no-estorba N.M)` y sin trailers de co-autoría. Gates del repo en
cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite del
crate tocado.

## 1. Los avisos

- [x] 1.1 Caducidad por tono en `stores.ts`: los informativos se retiran solos
  tras un plazo breve; advertencia y error no tienen plazo alguno; el
  temporizador vive en el store y se cancela al descartar. Pausa y reinicio al
  apuntar o enfocar en `Notices.svelte` (design D2) — escenarios «La
  confirmación se retira sola», «El error se queda hasta que alguien lo retira»
  y «Nada desaparece bajo la mano que iba a leerlo» — gates: `npm test` y suite
  de cableado

## 2. El cajón y el velo

- [x] 2.1 `Drawer.svelte`: desplazamiento vertical únicamente y contenido que se
  parte (design D1) — escenario «El cajón parte la ruta larga en vez de
  desplazarla» — gates: suite de cableado
- [x] 2.2 El velo de la paleta cierra al hacer clic, y un barrido de la
  superficie exige lo mismo de todo velo, nombrando el componente que falte
  (design D3) — escenarios «Hacer clic fuera cierra la paleta» y «Ningún velo
  queda sin cierre» — gates: suite de cableado

## 3. Cierre

- [x] 3.1 `meltemi validate cromo-que-no-estorba` limpio y `meltemi verify` con
  los seis escenarios enlazados (meta: cero marcas manuales); suite completa,
  clippy y fmt verdes; comprobación sobre el binario de release con captura —el
  cajón sin barra horizontal, un aviso informativo retirándose y uno de error
  quedándose, y el clic fuera cerrando la paleta
