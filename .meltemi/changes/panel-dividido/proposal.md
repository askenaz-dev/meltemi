# panel-dividido

> Vía completa (proposal → design → specs → tasks): solo superficie de
> escritorio, pero cambia la arquitectura de tres zonas del design system y
> merece un design que diga hasta dónde. Nace del comparativo con Orca del
> 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brecha D4).
> Depende de `escritorio-que-recuerda` (las pestañas persistentes) y de
> `senales-de-vida` (las clases de movimiento).

## Why

«Split Anything: agents, terminals, browsers, diffs, and files into split
panes» es una feature de Orca que el rumbo de Meltemi no quiere entera —la
edición es utilitaria, no un IDE— pero cuya mitad útil es real: ver el
transcript de una sesión **y** el diff que produce, o dos calles de una
carrera lado a lado, sin alternar pestañas. Hoy el shell tiene barra
lateral, barra superior, área principal con pestañas y barra de estado
(`docs/ux/design-system.md`, «Shell architecture»); el área principal
muestra una pestaña a la vez, y el tablero ya pinta calles lado a lado
dentro de su propia vista.

Dos paneles bastan para la mitad útil y no convierten a Meltemi en un
gestor de ventanas.

## What Changes

- **Dos paneles como máximo**: el área principal se parte en dos (vertical
  u horizontal, elección persistida); cada panel aloja una pestaña de
  sesión, revisión o editor; arrastrar una pestaña al otro panel o usar el
  atajo la mueve. Sin anidamiento: un tercer panel no existe.
- **Foco y teclado**: el panel activo recibe las teclas del keymap; el
  atajo de cambio de panel se documenta junto a los existentes; la paleta
  abre en el panel activo.
- **Persistencia**: el estado del split viaja con las pestañas de
  `escritorio-que-recuerda`.
- **Guardias intactas**: la bandeja de permisos y los avisos siguen fuera
  de los paneles y siguen sin animarse; el anillo ambiental es uno por
  vista, es decir, uno por panel visible como máximo, y el design lo dice.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «Área principal divisible en dos paneles, sin
  anidamiento, con foco y persistencia».
- `edit-surface`: + escenario «editor en un panel, sesión en el otro».

## Impact

- Archivos: `desktop/ui/src/{App.svelte, lib/session-tabs.ts,
  lib/tab-groups.ts, lib/ui-state.ts, lib/components/TabStrip.svelte}`,
  `docs/ux/design-system.md`, `desktop/tests/scenarios_shell.rs`.
- Cero dependencias nuevas; cero RPC; no nace deber de paridad (la TUI
  conserva su disposición).
- Riesgo: el presupuesto de ancho mínimo (`minWidth: 900`) con dos
  paneles; el design fija el mínimo por panel y el colapso automático.

## Fuera de alcance

- Splits arbitrarios, paneles flotantes, terminal embebida: rumbo y
  `abrir-con-lo-tuyo`.
- Una segunda ventana: `segunda-ventana`.
