<!-- SPDX-License-Identifier: Apache-2.0 -->
# Versioning policy

Meltemi versions with **SemVer, pre-1.0**. The workspace carries a **single
version** (`[workspace.package] version` in the root `Cargo.toml`); every crate
inherits it, and every release originates from a git **tag** and its release
pipeline — never a manual upload.

_Resumen en español al final._

## Pre-1.0 rules

While the major version is `0`:

- **A breaking change bumps the minor** (`0.x.0`).
- **A backward-compatible change or fix bumps the patch** (`0.x.y`).
- Every release records its notable changes in the release notes; a change
  classified as breaking is called out explicitly.

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
event kind) are **not** breaking and bump the patch.

## Release provenance

- Releases are cut from an annotated tag `vX.Y.Z`.
- The tag triggers the release pipeline (`.github/workflows/release.yml`), which
  re-runs every gate on the three platforms before any artifact is published.
- A red gate aborts the release; nothing is published.

## Summary (español)

Versionado SemVer pre-1.0 con versión única de workspace; los releases nacen de
un tag. Ruptura (sube minor): contrato `proto/`, gramática CLI o formato de
artefactos `.meltemi/`. Lo aditivo sube patch. Todo release corre sus gates en
las tres plataformas antes de publicar; un gate rojo aborta sin publicar nada.
