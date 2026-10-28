# Tasks — archivo-sin-cabeceras-arrastradas

Vía rápida. Un commit atómico por tarea, con referencia
`(archivo-sin-cabeceras-arrastradas N.M)` y sin trailers de co-autoría.

## 1. El motor

- [ ] 1.1 `split_requirements` cierra el bloque en una cabecera de sección del
  delta y no la copia (design D1) — escenarios «Un requisito añadido antes de
  otra sección no arrastra su cabecera» y «Una cabecera arrastrada antes se
  retira al fundir»; gates: tests de `archive.rs`
- [ ] 1.2 Retirar a mano las cabeceras arrastradas de `fleet-catalog`,
  `remote-access` y `session-history` — gates: `dogfood.rs`, `migration.rs` y
  `meltemi validate` de la verdad viva

## 2. El archivo

- [ ] 2.1 Reconciliar el tercer escenario de «El punto de encuentro en dos vías
  está documentado» en el delta de `maquinas-remotas` con la aserción de
  `identidad-propia` (design D3) — gate: `meltemi validate maquinas-remotas`
- [ ] 2.2 Archivar con el motor, en orden de aterrizaje, las changes terminadas
  con `verify` completo, salvo `lanzador-conversacional` (design D2, D4) —
  gates: `meltemi archive` limpio en cada una, `dogfood.rs`, `migration.rs` y
  `meltemi validate` de la verdad viva al final
- [ ] 2.3 `meltemi validate` y `meltemi verify` de esta change; entrada en
  `docs/plan-de-cambios.md` con lo archivado y lo que no, y por qué
