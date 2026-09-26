# segunda-ventana

> Vía completa (proposal → design → specs → tasks): solo superficie de
> escritorio, pero el estado compartido entre ventanas pasa por el backend
> Rust y merece un design. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, hallazgo C-B del
> crítico). Depende de `panel-dividido` y de `escritorio-que-recuerda`.

## Why

`desktop/tauri.conf.json` declara una sola ventana con `minWidth: 900`. Con
dos monitores —el caso normal del desarrollador que monitorea una carrera
mientras revisa un diff— no hay forma de tener una sesión en una pantalla y
el tablero en la otra. `panel-dividido` cubre el caso de una pantalla; este
cubre el de dos.

## What Changes

- **Desprender una pestaña a una ventana nueva** desde el menú de la
  pestaña o arrastrándola fuera; la ventana nueva es un shell reducido (sin
  barra lateral, con barra de estado) que aloja esa pestaña y sus paneles.
- **Un solo estado**: las dos ventanas comparten la conexión al daemon, las
  suscripciones y los avisos a través del backend Rust; la bandeja de
  permisos aparece en la ventana con foco y jamás en las dos.
- **Persistencia y cierre**: las ventanas y sus pestañas vuelven al
  arrancar (`escritorio-que-recuerda`); cerrar la principal cierra todas con
  la guarda de trabajo sin guardar.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «Pestañas desprendibles a una segunda ventana con
  estado único y bandeja en la ventana con foco».

## Impact

- Archivos: `desktop/src/` (ventanas y estado compartido),
  `desktop/tauri.conf.json`, `desktop/capabilities/`,
  `desktop/ui/src/{App.svelte, lib/ui-state.ts, lib/session-tabs.ts}`,
  `desktop/tests/`.
- Cero dependencias nuevas; cero RPC; sin deber de paridad.
- Riesgo: duplicar suscripciones al daemon por ventana; el backend las
  multiplexa.

## Fuera de alcance

- Ventanas arbitrarias por vista o paletas flotantes.
- Sincronizar ventanas entre máquinas: no hay nube.
