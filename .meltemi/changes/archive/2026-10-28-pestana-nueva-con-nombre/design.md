# Design — pestana-nueva-con-nombre

## Context

`NEW_SESSION_TAB` (`desktop/ui/src/lib/session-tabs.ts`) es el id de la pestaña
del compositor. `SessionTabs.svelte` lo reconoce y la etiqueta con
`nav.open.newSession`. La miga de `App.svelte` y las filas de «Abiertas» de
`Sidebar.svelte` no lo reconocen: caen al id cortado a ocho caracteres, que es
el respaldo para una sesión que el listado todavía no conoce.

## Decisions

### D1 — Una sola palabra para la pestaña del compositor

Las tres superficies usan `nav.open.newSession`, la misma cadena que la tira.
La miga deja de decir «Registro de la sesión» para esa pestaña: no hay registro
todavía, hay un compositor.

### D2 — El respaldo del id cortado se queda para lo que es

Una sesión real que el listado aún no trae sigue mostrando su id corto: es
un id de verdad y es lo único que se sabe de ella.
