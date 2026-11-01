<!-- SPDX-License-Identifier: Apache-2.0 -->
# Versioning policy

Meltemi uses **SemVer, pre-1.0** and a single workspace version. Since the
maintainer's decision of 2026-11-01, every successful CI run for a push to main
publishes that exact source commit automatically. No release PR or manual patch
bump is required. Tags pushed by hand do not trigger publication.

## Source version and release version

The manifests declare the source base. Every crate inherits
`[workspace.package] version`; Tauri and npm carriers agree with it. Release CI
stamps its assigned version into an isolated checkout and Cargo.lock before
building. It never commits the stamp to main.

A release number is deterministic along the source's **first-parent history**:

1. Compute the declared base plus the number of first-parent commits since that
   base was introduced. The introducing commit receives the exact base.
2. Find the closest first-parent release tag. If it is newer than the declared
   base introduction, use tag plus distance: commits before that historical
   release are already accounted for. Otherwise use the greater of base plus
   distance and tag plus distance. A stale declared patch cannot regress.
3. Reserve that number as an annotated, immutable tag on the approved SHA.
   A collision with another SHA fails; no tag is moved or deleted.

Descendant tags never affect an older source's number. Out-of-order CI completion
may leave gaps while earlier builds fail; recovery fills the same number.
A retry reuses the reservation, version, CI evidence and notes base.

## Breaking changes and milestones

A breaking change bumps the **minor** before 1.0; afterwards it bumps major.
Set the declared base deliberately for that change using
`cargo run -p meltemi-release -- set-version X.Y.Z`. Ordinary additive changes
and fixes advance patch automatically. The contracts that define breaking are:

- `proto/`: removed methods or fields, tightened schemas or protocol version.
- CLI grammar: removed commands or flags, changed arguments or exit codes.
- artifact format: incompatible on-disk `.meltemi/` specs, changes or registries.

A milestone tried end to end may introduce a new minor base (the next planned
milestone is 0.3.0). Its report is checked in under docs/qa and may be linked by
`meltemi-release changelog X.Y.Z --report <path>`. Automatic releases do not
claim that an unperformed manual acceptance test passed.

## Source identity, recovery and latest

The source tag retains the declared base; release-manifest.json, included in
SHA256SUMS and the publication attestation, records sourceSha, sourceVersion,
version, CI run, tag and allocation anchor. Reproduce by checking out sourceSha
and running `RELEASE_VERSION=X.Y.Z node scripts/automatic-release.mjs stamp`
before the normal build. The legacy `meltemi-release gate` remains a local
inspection command; the workflow no longer uses its manual-bump policy.

A recovery dispatch requires a successful ci.yml push-to-main run id, checked
against GitHub's API and current main ancestry. Source commits from before this
automation are refused with an explicit diagnosis. Historical release tags
without a reservation are never rebuilt or renumbered. Only a greater version
whose source descends from current latest can replace latest.

## Summary (español)

Cada push a main con CI verde libera su SHA exacto. La versión sale del tag ancestro first-parent más cercano más distancia; una
base introducida después puede elevarla, nunca bajarla; no depende del orden en que terminen las ejecuciones. Un cambio
de minor/major se declara en la base. El estampado vive solo en el checkout de
build. Tag y manifiesto atan la versión a la fuente; reintentar conserva ambos.
La recuperación exige un CI válido y nunca retrocede latest.
