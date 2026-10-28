# archivo-sin-cabeceras-arrastradas

> Vía rápida: un defecto del motor de archivo con nombre desde el 2026-10-24 y
> sin change, y el archivo de lo terminado que esperaba ese arreglo. 2º de la
> tanda de `release-para-un-desarrollador` (2026-10-28): el `CHANGELOG.md` de
> `antes-de-anunciar` se genera del archivo.

## Why

`meltemi archive` funde cada requisito ADDED o MODIFIED copiando su bloque del
delta, y `split_requirements` (`core/meltemid/src/archive.rs`) hace durar un
bloque hasta el `### Requirement:` siguiente. Si entre los dos hay una cabecera
de sección del delta —`## MODIFIED Requirements`, `## REMOVED Requirements`—,
queda dentro del bloque anterior y se copia a la verdad viva. El archivo de
`release-con-el-push` (2026-10-24) dejó un `## REMOVED Requirements` suelto en
`release-distribution` que hizo leer siete requisitos vivos como eliminados;
lo pescó `dogfood.rs` y se reparó a mano. `fleet-catalog`, `remote-access` y
`session-history` llevan cabeceras toleradas de ese tipo desde julio.

Mientras tanto, 28 changes terminadas siguen activas. El `CHANGELOG.md` que
`antes-de-anunciar` genera sale del archivo, y la release que va a probar un
desarrollador listaría como pendiente lo que ya está en `main`.

## What Changes

- **El bloque de un requisito termina en una cabecera de sección del delta**,
  y esa cabecera no se copia nunca. Una cabecera así que la verdad viva ya
  arrastre desaparece en la siguiente fusión de su capacidad.
- **Las tres cabeceras arrastradas se retiran a mano** de `fleet-catalog`,
  `remote-access` y `session-history`.
- **Lo terminado se archiva con el motor**, en el orden en que aterrizó, con
  dos comprobaciones previas escritas en el design: que ningún requisito lo
  modifiquen dos changes sin archivar, y que ningún MODIFIED se escribiera
  antes de que otra change archivada cambiara ese mismo requisito.
  `lanzador-conversacional` no se archiva: su gate es la firma del mantenedor.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `verify-archive`: + la fusión no arrastra la estructura del delta.

## Impact

- `core/meltemid/src/archive.rs` y sus tests; tres specs vivas a mano; el
  histórico `.meltemi/changes/archive/` y las specs vivas que cada archivo
  funde; el bloque proyectado de `AGENTS.md`, `CLAUDE.md` y `GEMINI.md`, que
  cada archivo regenera; `docs/plan-de-cambios.md`.
- Cero dependencias, cero RPC.

## Fuera de alcance

- Archivar `lanzador-conversacional`, `primer-arranque-del-home` (con su smoke
  3.2 abierto) y las changes de la tanda que aún no terminan.
- La revisión por requisito (`review`), que no es gate de archivo.
