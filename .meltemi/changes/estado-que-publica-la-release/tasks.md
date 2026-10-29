# Tasks — estado-que-publica-la-release

Vía completa. Un commit atómico por tarea, con referencia
`(estado-que-publica-la-release N.M)` y sin trailers de co-autoría.

## 1. La herramienta

- [ ] 1.1 `toml` en `meltemi-release` sin tocar `Cargo.lock` ni `deny.toml`
  (si no, el archivo pasa a JSON; design D1); el modelo del archivo con su
  lectura y sus errores con nombre
- [ ] 1.2 `capabilities --check`: las tres reglas de estado, los dos idiomas y
  el id único, con un fixture por regla (design D2) — escenarios «Publicada
  exige su artefacto», «En desarrollo nombra su change abierta», «Todavía no
  con su change archivada falla» y «Un texto en un solo idioma falla»; gate:
  tests de `meltemi-release`
- [ ] 1.3 `capabilities --version <v> --out <archivo> [--signed <a>,…]`: el
  JSON con `platformSigned` en `false` salvo lo que nombra `--signed` (design
  D1, D4) — escenario «Sin firma verificada no se declara firmado»

## 2. El estado de hoy

- [ ] 2.1 `docs/capacidades.toml` con las cuatro capacidades del design D3, con
  las notas en español alineadas con las de `ide-state-as-it-is`
- [ ] 2.2 El test de la suite que corre `--check` sobre el árbol real, para que
  archivar una change o tocar `release.yml` lo ejerza en cada PR (design D2)

## 3. La release

- [ ] 3.1 `release.yml`: el paso en `version` y el artefacto
  `meltemi-capabilities`, recogido por `publish` (design D4) — escenario «La
  release publica el estado atestado»; gate: los controles estructurales de
  workflows (pines y permisos)
- [ ] 3.2 `docs/release.md`: `capabilities.json` en la tabla de assets, qué
  dice, de dónde sale y que `platformSigned` solo es `true` con firma
  verificada; en inglés y en el resumen en español

## 4. Verificación

- [ ] 4.1 *(tras la primera release que lo traiga)* Descargar
  `capabilities.json` por la URL de última release, verificarlo con
  `gh attestation verify` y comparar con `docs/capacidades.toml`; anotarlo aquí
- [ ] 4.2 `meltemi validate` y `verify`; archivar
