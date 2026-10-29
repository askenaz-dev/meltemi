# contrato-publicado

> Vía rápida (proposal, design, specs y tasks de una vez): solo agrega un
> requisito, sin MODIFIED, y no toca código de producto. Es la 7º de la tanda
> `entrega-sin-huecos`. Los JSON Schemas del contrato daemon ↔ clientes viajan
> en cada release como un asset atestado. Es el primer paso del SDK de fase 2,
> sin prometerlo.

## Why

El sitio dice del SDK público «todavía no», y es verdad: no hay paquete, y
`meltemi-proto` y `meltemi-client` llevan `publish = false`. Pero lo que un
SDK envolvería ya existe y ya es estable por política: los JSON Schemas de
`proto/schemas/v1/`. El rumbo los llama la fuente de verdad del protocolo, y
`docs/versionado.md` define su ruptura. Hoy quien quiere escribir un cliente
en otro lenguaje tiene que clonar el repositorio y saber dónde mirar. Nada le
dice qué versión del contrato corresponde a qué release, ni le garantiza que
lo que leyó es lo que la release publicó.

Revisándolo apareció además un hueco. El `README.md` de `proto/` es la puerta
del contrato, pero su tabla nombra 19 de los 34 schemas. Faltan, entre otros,
`session-start`, `session-direct`, `session-watch`, `identity` y `harness`.
Publicarlo así sería publicar un índice que miente.

## What Changes

- **`meltemi-contract.tar.gz` en cada release**, con nombre estable. Contiene
  `schemas/v1/*.schema.json`, el `README.md` de `proto/` y la `LICENSE`. Se
  arma en el job `version` con `tar` determinista (orden por nombre, fecha y
  dueño fijos, `gzip -n`) y se sube como artefacto `meltemi-contract`. Así
  `publish` lo recoge bajo el mismo `SHA256SUMS` y la misma atestación. Un paso
  comprueba que su lista de archivos es exactamente la del árbol.
- **El índice completo**: la tabla de `proto/README.md` nombra cada schema, y
  un test de conformidad falla nombrando el que falte. Otro test ata «Versión
  actual» del README a `PROTOCOL_VERSION`.
- **`docs/release.md`** suma el asset: qué es, cómo se verifica (`gh
  attestation verify`, igual que el resto) y que publicarlo no es un SDK.
- **Los crates siguen sin publicarse.** Cuándo publicar `meltemi-proto` (con
  trusted publishing de crates.io) se decide cuando el contrato se asiente, en
  la change del SDK.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisito «El contrato viaja con cada release».

## Impact

- Archivos: `.github/workflows/release.yml` (un paso y un artefacto en
  `version`), `proto/README.md` (la tabla completa),
  `proto/meltemi-proto/tests/conformance.rs` (los dos tests),
  `docs/release.md`, `docs/capacidades.toml` (la nota del SDK nombra el asset,
  si `estado-que-publica-la-release` ya está).
- Cero dependencias nuevas: `tar` y `gzip` del runner.
- Riesgo: que alguien lea el asset como un SDK con garantías. La nota del SDK
  y `docs/release.md` dicen qué es y qué no.

## Fuera de alcance

- Publicar crates o paquetes de npm, PyPI u otros: SDK, fase 2.
- Generar tipos para otros lenguajes desde los schemas.
- Traducir el `README.md` de `proto/` al inglés: se propone si alguien lo pide.
