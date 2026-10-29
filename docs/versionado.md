<!-- SPDX-License-Identifier: Apache-2.0 -->
# Versioning policy

Meltemi versions with **SemVer, pre-1.0**. The workspace carries a **single
version** (`[workspace.package] version` in the root `Cargo.toml`); every crate
inherits it, and every release originates from a commit of a new version on
`main` and its release pipeline, which creates the tag — never a manual
upload, and never a tag pushed by hand.

_Resumen en español al final._

## Pre-1.0 rules

While the major version is `0`:

- **A breaking change bumps the minor** (`0.x.0`).
- **A backward-compatible change or fix bumps the patch** (`0.x.y`).
- Every release records its notable changes in the release notes; a change
  classified as breaking is called out explicitly.

## A tested milestone bumps the minor

A release that marks a milestone a person tried end to end — a run of
[the one-developer test](prueba-de-uno.md) whose report says the release goes
out — **bumps the minor even when nothing in it is breaking**, and its notes
open with the link to that report ([releases](release.md)). The maintainer
decided this on 2026-10-29: a milestone somebody tested is worth a number
people can name. The first is **0.3.0**.

## What counts as a breaking change

A change is **breaking** when it alters any of the project's stable contracts:

1. **The `proto/` contract** — the JSON Schemas under `proto/schemas/` and the
   `meltemi-proto` types: a removed or renamed method, a removed field, a
   tightened schema, or a bump of `PROTOCOL_VERSION`.
2. **The CLI grammar** — removing or renaming a subcommand, changing a
   subcommand's arguments, or changing the exit-code taxonomy (`tui::exit`).
3. **The artifact format** — the on-disk shape of `.meltemi/` (specs, changes,
   worktree/checkpoint registries) that another version must read.

Additive changes (a new method, a new subcommand, a new optional field, a new
event kind) are **not** breaking and bump the patch, unless the release is a
tested milestone (above).

## Release provenance

- A release originates from a commit on `main` whose version has no release
  yet. The release pipeline (`.github/workflows/release.yml`) re-runs every
  gate on the three platforms and only then creates the tag `vX.Y.Z` on that
  very commit and publishes the release.
- The tag is the pipeline's output, never its input: a tag pushed by a person
  starts nothing.
- The version is written with `cargo run -p meltemi-release -- set-version
  X.Y.Z`, which keeps every file that carries it in step and refuses a version
  that does not move forward. Since a version commit on `main` is a
  publication, it is the maintainer's gesture; an agent makes it only when the
  maintainer asks in words.
- A red gate aborts the release; nothing is published.

## Summary (español)

Versionado SemVer pre-1.0 con versión única de workspace; los releases nacen de
un commit de versión nueva en `main`, y el tag lo crea el pipeline: un tag
empujado a mano no publica nada. Ruptura (sube minor): contrato `proto/`, gramática CLI o formato de
artefactos `.meltemi/`. Lo aditivo sube patch, salvo que la release marque un
hito probado de punta a punta —una corrida de `docs/prueba-de-uno.md` cuyo
informe dice que sale—: entonces sube el minor y sus notas abren con el
informe (decisión del mantenedor, 2026-10-29; la primera es la 0.3.0). Todo release corre sus gates en
las tres plataformas antes de publicar; un gate rojo aborta sin publicar nada.
