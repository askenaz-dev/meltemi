# Tasks — registro-de-la-sesion-de-calle

Vía rápida. Un commit atómico por tarea, con referencia
`(registro-de-la-sesion-de-calle N.M)` y sin trailers de co-autoría.

## 1. La calle y su proyecto

- [x] 1.1 `owning_project` y `log_key` en `worktrees.rs`, usados por
  `session/log`, `find_session_record` y la reanudación; `is_running` para que
  una calle terminada se reanude (design D1, D2, D4) —
  escenarios «El registro de un despacho se lee por su calle» y «Reanudar una
  calle deja el registro con su proyecto»; gates: tests de `worktrees.rs` y un
  e2e con el agente simulado
  <!-- 2026-10-28: e2e `e2e_registro_de_calle.rs`; contra el daemon sin el
  arreglo, `session/log` por la calle devolvió `[]` —el hallazgo 3 tal cual— y
  la reanudación se rehusó como «activa pero no dirigible», que es D4. Tests
  unitarios de la forma de la ruta y de `is_running`. -->
- [ ] 1.2 `create` excluye la raíz gestionada (design D3) — escenario «Un
  despacho no ensucia el árbol principal»; gate: el mismo e2e

## 2. Cierre

- [ ] 2.1 `meltemi validate` y `verify` con los tres escenarios; entrada en
  `docs/plan-de-cambios.md`
