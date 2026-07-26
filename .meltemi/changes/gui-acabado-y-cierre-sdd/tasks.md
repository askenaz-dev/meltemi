# Tasks — gui-acabado-y-cierre-sdd

## 1. Lienzo del shell (GUI)

- [x] 1.1 Sustituir la rejilla de filas fijas de la columna central de
  `App.svelte` por columna flex (barras a altura natural, vista con el resto)
  y cubrir «La vista ocupa el alto disponible» en
  `desktop/tests/scenarios_shell.rs`

## 2. Árbol del editor (GUI)

- [x] 2.1 Declarar `flex: 0 0 auto` en las filas del árbol y de resultados de
  `Editor.svelte` y cubrir «Filas del árbol sin recorte» en
  `desktop/tests/scenarios_shell.rs`

## 3. Mapa del repositorio (daemon)

- [x] 3.1 Filtrar `.git` en el walker de `build_map` (`repo_map.rs`) y cubrir
  «Metadirectorio de git fuera del mapa» con un test unitario junto a los
  existentes

## 4. Cierre de sesión de autoría (daemon)

- [x] 4.1 Registrar el inicio en el índice y finalizar `run_turn`
  (`sdd_flow.rs`) por `session_finalize`, y cubrir «Turno de autoría
  finalizado queda cerrado» y «Fallo del turno también cierra» en
  `core/meltemid/tests/e2e_sdd.rs`

## 5. Verificación

- [x] 5.1 Gates locales (fmt, clippy, tests del workspace, checks de la UI) y
  smoke visual conducido por CDP sobre el binario reconstruido, publicado en
  `docs/qa/2026-07-26-gui-acabado-smoke.md`
