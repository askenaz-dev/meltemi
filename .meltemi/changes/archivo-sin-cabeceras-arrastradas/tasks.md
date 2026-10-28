# Tasks — archivo-sin-cabeceras-arrastradas

Vía rápida. Un commit atómico por tarea, con referencia
`(archivo-sin-cabeceras-arrastradas N.M)` y sin trailers de co-autoría.

## 1. El motor

- [x] 1.1 `split_requirements` cierra el bloque en una cabecera de sección del
  delta y no la copia (design D1) — escenarios «Un requisito añadido antes de
  otra sección no arrastra su cabecera» y «Una cabecera arrastrada antes se
  retira al fundir»; gates: tests de `archive.rs`
  <!-- 2026-10-28: `is_delta_section` reconoce las cuatro cabeceras; cierran el
  bloque y lo que sigue hasta el siguiente requisito no es de nadie. Dos tests:
  un delta ADDED → MODIFIED → REMOVED funde sin ninguna cabecera, y el motor
  lee la spec resultante sin secciones de delta; una cabecera ya arrastrada en
  la viva desaparece al fundir otra cosa, sin perder el texto del requisito que
  la sigue. Con `is_delta_section` devolviendo siempre `false`, solo en local,
  los dos fallan. Tests de `archive.rs` 5/5. -->
- [x] 1.2 Retirar a mano las cabeceras arrastradas de `fleet-catalog`,
  `remote-access` y `session-history` — gates: `dogfood.rs`, `migration.rs` y
  `meltemi validate` de la verdad viva
  <!-- 2026-10-28: cuatro cabeceras `## MODIFIED Requirements` retiradas: dos en
  `fleet-catalog`, una en `remote-access` (la última línea del archivo) y una
  en `session-history`; ningún texto de requisito se tocó. `meltemi validate`
  de la verdad viva limpio; `dogfood.rs` 2/2 y `migration.rs` 3/3. -->

## 2. El archivo

- [x] 2.1 Reconciliar el tercer escenario de «El punto de encuentro en dos vías
  está documentado» en el delta de `maquinas-remotas` con la aserción de
  `identidad-propia` (design D3) — gate: `meltemi validate maquinas-remotas`
  <!-- 2026-10-28: el `AND` recuperado es el de `identidad-propia`, palabra por
  palabra: «la identidad y la lista de equipos de la variante de malla SHALL
  estar descritas como capacidad presente y leída, no creada, por Meltemi».
  `validate` limpio; `verify` sigue 16/16, porque el escenario conserva su
  título y su test. -->
- [x] 2.2 Archivar con el motor, en orden de aterrizaje, las changes terminadas
  con `verify` completo, salvo `lanzador-conversacional` (design D2, D4) —
  gates: `meltemi archive` limpio en cada una, `dogfood.rs`, `migration.rs` y
  `meltemi validate` de la verdad viva al final
  <!-- 2026-10-28: 27 archivadas, un commit cada una, en orden de aterrizaje
  (último commit de su `tasks.md`), con un script de lote que se detiene si
  `verify` no está completo, si una cabecera de sección llega a la verdad viva
  o si una change sin MODIFIED quita texto, y valida la verdad viva tras cada
  una. Antes de archivar se enlazaron los dos escenarios que faltaban
  (`enfasis-con-color`, `modos-de-autonomia`). La guarda paró dos veces: por una
  línea en blanco normalizada entre bloques (falsa alarma; se cuentan solo
  líneas con texto) y por una cabecera arrastrada en `sesion-que-espera`,
  porque el daemon aislado se había compilado antes de 1.1; se deshizo ese
  archivo, se reinició el daemon con el arreglo y se rehízo. Al final:
  `validate` de la verdad viva limpio, ninguna cabecera de sección en las 37
  specs, `meltemi-spec` 63/63 (dogfood y migración incluidos). El test de
  `desktop/tests/scenarios_shell.rs` que leía el delta de
  `sesiones-en-la-barra` tiene rama de archivada, y su condición se cumple. -->
- [x] 2.3 `meltemi validate` y `meltemi verify` de esta change; entrada en
  `docs/plan-de-cambios.md` con lo archivado y lo que no, y por qué
  <!-- 2026-10-28: `validate` limpio y `verify` 2/2 enlazados; el avance en la
  tanda de `docs/plan-de-cambios.md`. -->
