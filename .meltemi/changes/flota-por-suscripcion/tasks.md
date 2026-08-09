# Tareas — flota-por-suscripcion

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(flota-por-suscripcion N.M)` y sin trailers de co-autoría. Gates del repo en
cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite del
crate tocado.

## 1. El agrupamiento

- [x] 1.1 `desktop/ui/src/lib/fleet-groups.ts` (módulo puro, cabecera SPDX): de
  la lista plana a filas con profundidad —agente, sus suscripciones por nombre,
  siguiente agente— y el grupo final de las huérfanas; `desktop/ui/tests/
  fleet-groups.test.ts` con `node --test` cubriendo el orden, el recuento y la
  huérfana (design D1, D2) — escenarios «Varias suscripciones del mismo agente
  se leen juntas» y «La suscripción sin agente conocido no desaparece» —
  gates: `npm test`

## 2. La tabla

- [x] 2.1 `Fleet.svelte`: la tabla consume el agrupamiento; cada suscripción
  declara su agente como texto y la sangría solo acompaña; el agente lleva el
  recuento; strings ES/EN (design D3) — escenario «La relación no depende de la
  sangría» — gates: suite de cableado
- [x] 2.2 El nivel se dice con palabras —«declarado» / «verificado»— conservando
  el glifo junto a ellas, cumpliendo el requisito vivo de `integration-levels`
  que la superficie incumplía, con su test marcado con el escenario vivo
  (design D4) — gates: suite de cableado

## 3. Cierre

- [ ] 3.1 `meltemi validate flota-por-suscripcion` limpio y `meltemi verify` con
  los tres escenarios enlazados (meta: cero marcas manuales); suite completa,
  clippy y fmt verdes; comprobación sobre el binario de release con captura de
  un agente con varias suscripciones y de una huérfana
