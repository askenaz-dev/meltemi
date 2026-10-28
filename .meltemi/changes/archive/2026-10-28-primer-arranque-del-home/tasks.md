# Tareas — primer-arranque-del-home

Vía rápida: gate único al final. Deltas ADDED sobre `gui-shell`; cero contrato,
cero daemon, cero dependencias. Un commit atómico por tarea, con referencia
`(primer-arranque-del-home N.M)` y sin trailers de co-autoría.

## 1. El chip proactivo

- [x] 1.1 La cara del selector advierte con la flota respondida y vacía, y el
  default deja de ofrecerse (design D1) — escenario «La flota vacía se dice
  antes de fallar»
  <!-- 2026-08-10: la condición lleva `$fleet.length > 0` a propósito: sin esa
  mitad, el chip advertiría durante el instante en que la flota todavía no ha
  contestado, y una advertencia que aparece y se va sola es ruido. -->

## 2. El gesto y el saludo

- [x] 2.1 El menú vacío abre la vista de flota, con la salida cableada por el
  shell (design D2) — escenario «El menú vacío abre la flota»
- [x] 2.2 Reconocimiento único del recuento, persistido en el estado de UI
  (design D3) — escenario «El reconocimiento se dice una vez»
  <!-- 2026-08-10: se persiste porque, sin eso, el saludo volvería con cada
  ventana y dejaría de ser un saludo para volverse una insignia. -->

## 3. Cierre

- [x] 3.1 `meltemi validate primer-arranque-del-home` limpio y `meltemi verify`
  con los tres escenarios enlazados; gates del frontend y suite de cableado
- [x] 3.2 Smoke sobre el binario: el chip advirtiendo con flota vacía y el
  saludo apareciendo una sola vez. **Requiere un fixture con flota vacía**, que
  es un registro sin candidatos alcanzables. Nota en `docs/qa/`
  <!-- 2026-10-28: `docs/qa/2026-10-28-primer-arranque-del-home-smoke.md`, en
  Linux con WebKitGTK. Encontró y corrigió tres defectos: el chip no tomaba el
  tono de advertencia con la flota vacía, la advertencia se recortaba a «sin
  agentes …», y el lado Rust del estado tiraba `fleetGreeted` (y
  `noticesEnabled`) en cada guardado, así que el saludo volvía en cada ventana.
  Re-medido tras corregir: advertencia entera y con tono; «2 detectados» en la
  primera ventana y «el del proyecto» en la segunda. -->
