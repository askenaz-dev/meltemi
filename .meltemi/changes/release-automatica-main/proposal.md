# release-automatica-main

## Why

El mantenedor pide el 2026-11-01 que cada push a main con CI verde publique
una release sin bump ni PR de release manual. Sustituye la decisión del
2026-10-29 de `version-que-propone-ci`: el push aprobado es la autorización.

## What Changes

- CI conserva cada push; Release consume exclusivamente el SHA de una ejecución
  CI satisfactoria de push a main en este repositorio.
- Reserva una versión patch mediante tag anotado inmutable en ese SHA. Un
  reintento conserva versión y tag; un fallo no consume otra versión.
- Estampa los portadores solo en los checkouts de construcción, sin commits
  automáticos a main. Publica el manifiesto fuente/versión y notas del rango.
- Conserva gates, artefactos, atestación y aviso a Web. Un reintento antiguo
  nunca retrocede latest. La recuperación exige un CI run id validado.

## Capabilities

### Modified Capabilities

- `release-distribution`: publicación automática después de CI, reserva e
  identidad reproducible del empaquetado.

## Impact

Workflows, scripts Node sin dependencias, pruebas y documentación de releases.
No cambia código de producto, firmas de plataforma, SDK ni Android. El tag
identifica código fuente; sus manifests conservan la versión base y el asset
release-manifest.json declara el estampado que produjo los binarios.
