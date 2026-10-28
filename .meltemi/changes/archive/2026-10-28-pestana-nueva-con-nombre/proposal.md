# pestana-nueva-con-nombre

> Vía rápida. Nace del smoke de `primer-arranque-del-home` del 2026-10-28
> (`docs/qa/2026-10-28-primer-arranque-del-home-smoke.md`, hallazgo 4), dentro
> de la tanda de `release-para-un-desarrollador`.

## Why

Con el compositor de una sesión nueva abierto, que es la primera pantalla de
quien abre la app, la miga dice «Registro de la sesión · \_\_new\_\_» y la
sección «Abiertas» de la barra lista «\_\_new\_\_». Las dos cortan el id de la
pestaña a ocho caracteres sin reconocer la pestaña del compositor, cuyo id es
un marcador interno (`NEW_SESSION_TAB`). La tira de pestañas sí la nombra
«Nueva sesión». Es un id interno a la vista y una miga que nombra un registro
que todavía no existe.

## What Changes

- La miga, el título de la vista y la fila de «Abiertas» nombran la pestaña del
  compositor «Nueva sesión», como ya hace la tira, con la cadena que ya existe
  en los dos idiomas.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + la pestaña del compositor se nombra con palabras en toda la
  superficie.

## Impact

- `desktop/ui/src/App.svelte`, `desktop/ui/src/lib/components/Sidebar.svelte`
  y el test de cableado. Cero dependencias, cero contrato, cero daemon; cromo
  exclusivo de la GUI.

## Fuera de alcance

- El recorte de los chips en tono normal («el del proye…»): es diseño de
  `cromo-que-no-estorba`, y se anotó en el mismo smoke.
