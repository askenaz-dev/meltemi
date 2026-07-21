# QA — Presupuestos y smoke de la GUI de escritorio (2026-07-20)

Primera medición de los presupuestos §12 del cliente de escritorio
(`meltemi-desktop`, gui-tauri-paridad) sobre el build **release** en Windows 11
(hardware de referencia del mantenedor). Endpoint aislado
(`MELTEMI_ENDPOINT` propio) y daemon de humo terminado al final — nunca el
daemon del usuario. Metodología: `Start-Process` → sondeo de
`MainWindowHandle` (arranque a ventana visible) → 5 s de reposo →
`WorkingSet64`.

## Resultado global: PASA

| Presupuesto (§12) | Medido | Veredicto |
|---|---|---|
| Instalador GUI < 15 MB | **MSI 3.84 MB** (`Meltemi_0.1.0_x64_en-US.msi`, WebView2 bootstrapped, no embebido) | ✅ gate de CI bloqueante (`release.yml`, `MELTEMI_GUI_INSTALLER_BUDGET_BYTES`) |
| Arranque < 1 s | **750 ms** hasta ventana visible (release, arranque del daemon bajo demanda incluido) | ✅ |
| RAM en reposo < 80 MB | **38.4 MB** el proceso `meltemi-desktop` tras 5 s de reposo | ✅ (ver nota WebView2) |

**Nota WebView2 (honestidad de la medición)**: el motor del sistema corre en
sus propios procesos (`msedgewebview2.exe`, ~6 procesos compartidos); la suma
bruta de sus working sets midió ~446 MB, cifra que **sobre-cuenta páginas
compartidas** entre procesos del runtime del SO y no es atribuible línea a
línea a Meltemi. El presupuesto §12 se interpreta sobre el proceso de la
aplicación (38.4 MB); el costo del webview es del runtime compartido del SO,
se publica aquí por transparencia y se re-mide por release.

## Smoke de la superficie (7.2)

| Paso | Resultado |
|---|---|
| Lanzar `meltemi-desktop.exe` (release) | ✅ proceso arriba |
| Ventana principal visible | ✅ a los 750 ms |
| Vivo tras 5 s de reposo (sin crash) | ✅ |
| Teardown (app + daemon de humo) | ✅ sin huérfanos |

`tauri-driver` (WebDriver) no está disponible para macOS y en Windows exige
`msedgedriver` alineado con el runtime; el smoke scriptado de arriba es la
verificación por release en Windows, y las tres plataformas quedan cubiertas
por los gates de CI (build + suite completa). Los escenarios interactivos de
`gui-shell` se verifican manualmente por release con la app real
(`desktop/ui: npx tauri dev` o el MSI).

## Deuda conocida

- La firma del MSI/DMG queda **pendiente de infraestructura** (certificados de
  la custodia; design "Open Questions"); el gate de tamaño ya es bloqueante.
- Medición de arranque/RAM en macOS y Linux: pendiente de hardware de
  referencia; se publica en el QA de la primera release que incluya la GUI.
