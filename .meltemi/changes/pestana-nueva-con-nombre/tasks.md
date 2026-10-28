# Tasks — pestana-nueva-con-nombre

Vía rápida. Un commit atómico por tarea, con referencia
`(pestana-nueva-con-nombre N.M)` y sin trailers de co-autoría.

## 1. El nombre

- [x] 1.1 La miga, el título y la fila de «Abiertas» reconocen la pestaña del
  compositor (design D1, D2) — escenario «El compositor abierto no muestra su
  id»; gates: `npm run check`, `lint:i18n`, test de cableado y una captura
  sobre el binario
  <!-- 2026-10-28: la captura sobre el binario (Linux, WebKitGTK) lee
  «Sesiones / Nueva sesión» en la miga y «Nueva sesión ▸» en «Abiertas», donde
  antes leía «Registro de la sesión · __new__» y «__new__». -->

## 2. Cierre

- [ ] 2.1 `meltemi validate` y `verify`; entrada en `docs/plan-de-cambios.md`
