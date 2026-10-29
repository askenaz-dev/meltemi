# Tasks — contrato-publicado

Vía rápida. Un commit atómico por tarea, con referencia
`(contrato-publicado N.M)` y sin trailers de co-autoría.

## 1. El índice

- [ ] 1.1 La tabla de `proto/README.md` con una fila por cada uno de los 15
  schemas que faltan, y el test de conformidad que la exige (design D3) —
  escenario «Un schema sin fila en el índice falla»; gate: suite de
  `meltemi-proto`
- [ ] 1.2 El test de la versión: «Versión actual» del README igual a
  `PROTOCOL_VERSION` y su directorio presente (design D2) — escenario «La
  versión del índice es la del protocolo»

## 2. El asset

- [ ] 2.1 `release.yml`: el tarball determinista en `version`, la comprobación
  de su lista contra el árbol y el artefacto `meltemi-contract` (design D1) —
  escenarios «El contrato publicado con la release» y «El asset es el árbol»;
  gate: los controles estructurales de workflows
- [ ] 2.2 `docs/release.md`: el asset en la tabla y lo que no es (design D4),
  en inglés y en el resumen en español; la nota del SDK en
  `docs/capacidades.toml`, si ya existe

## 3. Cierre

- [ ] 3.1 *(tras la primera release que lo traiga)* Descargar
  `meltemi-contract.tar.gz` por la URL de última release, verificarlo con
  `gh attestation verify` y comparar su contenido con el tag; anotarlo aquí
- [ ] 3.2 `meltemi validate` y `verify`; archivar
