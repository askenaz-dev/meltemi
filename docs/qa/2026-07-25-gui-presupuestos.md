# QA — Presupuestos y smoke de la GUI de escritorio (2026-07-25)

Segunda medición de los presupuestos §12 del cliente de escritorio
(`meltemi-desktop`) sobre el build **release** en Windows 11 (hardware de
referencia del mantenedor), tras cerrar `gui-clase-mundial`,
`multiproyecto-suscripciones` y `analitica-consumo-local` — es decir, con el
sidebar de árbol, el panel de consumo, el editor con LSP BYO completo y la
revisión por hunk ya dentro del binario. Metodología idéntica a la de
[2026-07-20](2026-07-20-gui-presupuestos.md), para que las cifras se puedan
comparar: `Start-Process` → sondeo de `MainWindowHandle` (arranque a ventana
visible) → 5 s de reposo → `WorkingSet64`.

## Resultado global: PASA

| Presupuesto (§12) | 2026-07-20 | 2026-07-25 | Veredicto |
|---|---|---|---|
| Instalador GUI < 15 MB | MSI 3.84 MB | **MSI 3.89 MB** (4 083 712 B, `Meltemi_0.1.0_x64_en-US.msi`) | ✅ gate bloqueante en `release.yml` |
| Arranque < 1 s | 750 ms | **584 ms** hasta ventana visible | ✅ |
| RAM en reposo < 80 MB | 38.4 MB | **29.6 MB** el proceso `meltemi-desktop` tras 5 s | ✅ (ver nota WebView2) |

El instalador creció 50 KB con tres vistas nuevas, el panel de consumo y las
acciones de LSP; el arranque y la RAM del proceso propio bajaron respecto de la
medición anterior (el tema se aplica antes del primer pintado, así que el
montaje ya no repinta).

**Nota WebView2 (honestidad de la medición)**: el motor del sistema corre en sus
propios procesos. Midiendo el **árbol de procesos completo** de la app (8
procesos, filtrado por `ParentProcessId`, no por nombre) la suma bruta de working
sets da **472 MB**, cifra que sobre-cuenta páginas compartidas del runtime del SO
y no es atribuible línea a línea a Meltemi: el mismo runtime sirve a cualquier
otra app WebView2 del sistema. El presupuesto §12 se interpreta sobre el proceso
de la aplicación (29.6 MB); el costo del webview es del runtime compartido del
SO, se publica aquí por transparencia y se re-mide por release.

## Verificación del empaquetado

| Paso | Resultado |
|---|---|
| `tauri build` (release, Windows) | ✅ MSI emitido en `target/release/bundle/msi/` |
| Normalización de nombre del pipeline (`meltemi-desktop-Windows.msi`) | ✅ simulada localmente; checksum calculado sobre el nombre estable |
| Gate de tamaño (`MELTEMI_GUI_INSTALLER_BUDGET_BYTES`) | ✅ 4 083 712 B < 15 728 640 B |

## Smoke de la superficie

| Paso | Resultado |
|---|---|
| Lanzar `meltemi-desktop.exe` (release) | ✅ proceso arriba |
| Ventana principal visible | ✅ a los 584 ms |
| Handshake con el daemon | ✅ `client initialized client=meltemi-desktop` en el log del daemon |
| Vivo tras 10 s de reposo (sin crash) | ✅ |
| Teardown (app + daemon) | ✅ sin huérfanos |

## Deuda conocida (sin cambios)

- Firma del MSI/DMG **pendiente de infraestructura** de certificados; el gate de
  tamaño ya es bloqueante.
- Arranque/RAM en macOS y Linux: pendiente de hardware de referencia; se publica
  en el QA de la primera release que incluya la GUI.
- Captura de escritorio del sitio: pendiente de una máquina de release, con el
  procedimiento documentado en [`docs/ux/capturas.md`](../ux/capturas.md).
