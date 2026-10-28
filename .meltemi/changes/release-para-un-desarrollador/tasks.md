# Tasks — release-para-un-desarrollador

Vía completa. Un commit atómico por tarea, con referencia
`(release-para-un-desarrollador N.M)` y sin trailers de co-autoría. Esta change
no implementa nada que otra change de la tanda posea.

## 1. La tanda

- [x] 1.1 La tanda declarada en `docs/plan-de-cambios.md`, y las cuatro changes
  nuevas nombradas
  <!-- 2026-10-28: 0º–7º cerradas en la rama de la tanda, cada una con su
  entrada en el avance; `pestana-nueva-con-nombre` nombrada por el smoke de 7º
  y hecha en la misma tanda. -->

## 2. El informe en las notas

- [x] 2.1 `meltemi-release changelog --report <ruta>`: la línea del informe abre
  la sección, se conserva al reescribir y se rehúsa una ruta que no existe
  (design D3) — escenarios «Las notas enlazan el informe» y «Un informe que no
  existe no se enlaza»; gate: tests de `meltemi-release`
  <!-- 2026-10-28: `with_report` y `REPORT_MARK` en la biblioteca;
  `write_changelog` conserva la línea al regenerar la sección. Los dos tests
  corren el binario: enlazar, reescribir sin la bandera (mismo archivo), las
  notas con un solo enlace, reemplazar, y la ruta que no existe (sale con 1 y
  no escribe nada). -->
- [x] 2.2 La regla en `docs/release.md` (inglés y resumen en español): una
  release que marca un hito probado enlaza su informe; el orden corrida →
  informe → `changelog --report` → `set-version`; la recomendación 0.3.0
  (design D5)
  <!-- 2026-10-28: en «Making a release (maintainer)» y en el resumen en
  español. -->

## 3. El guion

- [x] 3.1 `docs/prueba-de-uno.md` con sus pasos, lo esperado, dónde queda
  registrado, las desviaciones conocidas, las diferencias de macOS y Linux y la
  plantilla del informe (design D1, D4) — escenario «El guion se sigue desde los
  assets»
  <!-- 2026-10-28: once pasos, de la release a desinstalar; cada uno con qué
  haces, qué esperas ver, dónde queda y su desviación conocida. Comprobado
  contra la ayuda del binario y las guías: la regla recordada es por ruta
  exacta, y la decisión de revisión vive en la vista Proyecto o en la paleta de
  la TUI, no en la CLI. -->
- [x] 3.2 El test de la docs CI sobre el guion (design D2) — escenario «Los
  verbos del guion existen en el binario»; gate: `tui/tests/docs.rs`
  <!-- 2026-10-28: dos tests. Uno exige que el primer paso instale desde la
  release y que cada paso numerado diga qué haces, qué esperas ver, dónde queda
  y su desviación; encontró que el paso 9 no decía dónde queda, y se completó.
  El otro cruza cada `meltemi <verbo>` de los bloques de código con la ayuda
  del binario y corre `version` y `doctor --json` contra un endpoint vacío:
  sale con 14, la fila del daemon falla y no se crea nada. El chequeo de
  enlaces internos gana el guion, `hito-v01.md` y `release.md`. -->
- [x] 3.3 Enlaces: las filas del índice en `README.md` y `LEEME.md`,
  `docs/hito-v01.md` y el chequeo de enlaces internos
  <!-- 2026-10-28: de paso, la fila de la FAQ de `LEEME.md` partía en dos la
  viñeta de la documentación (la puso ahí `antes-de-anunciar`); vuelve a su
  sitio. -->

## 4. Cierre del agente

- [x] 4.1 `meltemi validate` y `verify` con los cuatro escenarios; el pendiente
  del mantenedor en `docs/plan-de-cambios.md`
  <!-- 2026-10-28: validate limpio, verify 4/4 enlazados; el pendiente 8 del
  arquitecto lleva el orden (a)–(e). La change no se archiva: las tareas 5.x
  son su gate. -->

## 5. Del mantenedor (gate de archivo)

- [ ] 5.1 Correr `docs/prueba-de-uno.md` en su máquina con sus agentes y
  escribir `docs/qa/<fecha>-prueba-de-uno.md` desde la plantilla
- [ ] 5.2 `changelog <versión> --report docs/qa/<fecha>-prueba-de-uno.md` y
  `set-version <versión>` (recomendación: 0.3.0); el push a `main` publica
- [ ] 5.3 Tras publicar: la fila «Cliente de escritorio» del estado de
  meltemi-web pasa a decir por qué sigue en desarrollo (instaladores sin firma
  de plataforma), como change de meltemi-web
