# Design — contrato-publicado

## Context

`proto/schemas/v1/` tiene 34 schemas. `proto/README.md` explica el transporte,
el versionado y lista los métodos en una tabla que nombra 19 de ellos.
`meltemi-proto` tiene un test de conformidad que valida los tipos contra los
schemas. El job `version` de `release.yml` ya hace checkout del árbol, no tiene
secretos y sube artefactos que `publish` recoge por el patrón `meltemi-*`.

## Decisions

### D1 — Un tarball en `version`, sin código nuevo

```
mkdir -p contract/schemas && cp -r proto/schemas/v1 contract/schemas/
cp proto/README.md LICENSE contract/
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner -cf - -C contract . | gzip -n > meltemi-contract.tar.gz
```

Después, `tar -tzf` contra `git ls-files proto/schemas/v1` (más `README.md` y
`LICENSE`) tiene que dar la misma lista; si no, el job falla con la
diferencia. Es determinista: el mismo árbol da los mismos bytes, así que el
checksum de dos releases sin cambios en el contrato coincide, y eso es una
señal útil.

*Alternativa descartada*: un subcomando de `meltemi-release` con crates de
`tar` y `flate2`. Serían dos dependencias para algo que el runner ya hace.

### D2 — La versión del contrato es el directorio

El tarball no lleva un archivo `PROTOCOL_VERSION`: la versión es el directorio
`schemas/v1/`, como en el árbol, y el README dice «Versión actual: **1**». Un
test exige que ese número sea `PROTOCOL_VERSION` y que exista
`schemas/v<PROTOCOL_VERSION>/`, de modo que los tres no puedan divergir.

### D3 — El índice completo, con test

La tabla de `proto/README.md` gana una fila por cada schema que falta, con su
mensaje, dirección y tipo tomados de su `title` y su `description`. El test lee
la tabla y el directorio y falla con la lista de schemas sin fila. No exige el
orden ni la redacción, solo la presencia.

### D4 — Lo que se dice del asset

En `docs/release.md`, en la tabla de assets: «`meltemi-contract.tar.gz` — the
JSON Schemas of the daemon ↔ client contract at this release, with their index
and license. It is the contract, not an SDK: no library, no generated types,
no promise beyond [the versioning policy](versionado.md)». Se verifica como
cualquier otro asset.

## Risks / Trade-offs

- **`--sort=name` es de GNU tar.** El job `version` corre en `ubuntu-latest`,
  que lo trae. Si algún día corre en otra plataforma, el paso lo dirá al fallar.
