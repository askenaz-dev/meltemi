<!-- SPDX-License-Identifier: Apache-2.0 -->
# Releases: packaging, signing, and installation

This document describes how Meltemi is packaged, signed, verified, and installed.
The promise is a **single self-contained binary, installable with one command**
(foundational document §4.5). See [versioning](versionado.md) for how versions
are decided.

_Resumen en español al final._

## Canonical download base

Every download resolves against **one** base, declared here once and nowhere
else. The site, the installer scripts and this document must all name it, and
`core/meltemid/tests/site.rs` fails the build if they diverge:

```
CANONICAL_DOWNLOAD_BASE=https://github.com/askenaz-dev/meltemi/releases
```

Downloads use the version-free `…/releases/latest/download/<stable-name>` form,
so a consumer never has to know a version and the site never has to be
republished for a patch.

## Artifacts

Artifact names are **stable and version-free** per platform, so the
latest-release URL always resolves to the right file. Where the packaging tool
emits a version inside a name — the desktop installers do — the pipeline renames
it to the stable scheme before publishing, and the published checksum is the one
of the stable name.

| Platform | Core archive | Desktop installer |
|---|---|---|
| Windows | `meltemi-Windows.zip` | `meltemi-desktop-Windows.msi` |
| macOS | `meltemi-macOS.tar.gz` | `meltemi-desktop-macOS.dmg` |
| Linux | `meltemi-Linux.tar.gz` | `meltemi-desktop-Linux.AppImage`, `meltemi-desktop-Linux.deb` |

The installer scripts (`install.sh`, `install.ps1`) are published as release
assets too, with their checksums inside the signed `SHA256SUMS`: the site links
a script whose hash travels signed, instead of hosting a copy of it.

Each release publishes, per supported platform (see
[platform notes](plataformas.md)), an archive containing the `meltemi` client and
the `meltemid` daemon, plus:

- a `SHA256SUMS` file with the checksum of every archive;
- a detached signature for the checksums file;
- the **desktop client installer** (gui-tauri-paridad): MSI on Windows, DMG on
  macOS, AppImage and deb on Linux, with its own `SHA256SUMS` under the same
  signing custody. The installer bundles no webview: it uses the OS engine
  (bootstrapping WebView2 on Windows when missing).

## Verifying a download

Before installing, verify the archive:

```
# 1. Check the checksum
sha256sum --check SHA256SUMS        # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature of SHA256SUMS with the published signing key
#    (the key and the exact command are published on the release page)
```

The installer scripts perform this verification for you and refuse to proceed on
a mismatch — there is no blind `curl | sh`: the script is short, readable, and
its own hash is published.

## Key custody (maintainer)

The signing key is the responsibility of the **founding maintainer**. The custody
procedure — generation, storage (offline/hardware-backed), rotation schedule, and
revocation — is maintained by the maintainer and documented on the release
infrastructure. This document records that the procedure exists and is the
maintainer's; the key material itself is never in this repository.

## Installers

One-line, auditable installers place `meltemi` and `meltemid` on the user's
`PATH` and create the short alias **`mel`**:

- **Unix (macOS/Linux)**: [`scripts/install.sh`](../scripts/install.sh)
- **Windows (PowerShell)**: [`scripts/install.ps1`](../scripts/install.ps1)

Each script is short and legible; its published hash lets you verify it before
running. Manual installation (download, verify, extract, add to `PATH`, create
the `mel` alias) is documented in each script's header as the equivalent path.
Both are published as assets of every release, reachable at the version-free
latest-release URL, so the copy you download is the copy whose checksum is
signed.

The published site's [downloads page](https://meltemi.dev/downloads.html) is the
public entry point for all of the above.

## Release pipeline and budgets

The release pipeline (`.github/workflows/release.yml`) is triggered by a `vX.Y.Z`
tag and runs on Windows, macOS, and Linux with **hard gates**: the full test
suite, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny`, and the
**performance budgets** (constitution §12) — the TUI binary staying under
**25 MB** and every GUI installer under **15 MB**. Any red gate aborts the
release and **no artifact is published**. The GUI's runtime budgets (startup
under 1 s, idle memory under 80 MB) are measured per release and published in
`docs/qa/` — honest measurements, not blocking CI gates, because they depend
on the OS webview.

## Crate namespaces

To secure the project's namespace, the crates `meltemi`, `meltemid`, and
`meltemi-proto` are reserved on the registry: `meltemi-proto` is the real
published contract; `meltemi` and `meltemid` are honest placeholders that point
at this repository until their libraries are published. Publishing is a
maintainer action (the placeholder crates set `publish = false` in-tree as a
safety until the maintainer reserves the names).

## Summary (español)

Cada release publica archivos por plataforma con `meltemi`+`meltemid`, checksums
y firma. El usuario verifica checksum y firma con instrucciones publicadas; los
instaladores (`scripts/install.sh`, `scripts/install.ps1`) lo hacen por él, son
legibles y con hash publicado, e instalan los binarios y el alias `mel`. La
custodia de la clave es del mantenedor (documentada, nunca en el repo). El
pipeline de release corre las tres plataformas con gates duros —suite, clippy,
fmt, cargo-deny y presupuestos §12 (TUI < 25 MB; instalador GUI < 15 MB)— y
aborta sin publicar ante cualquier rojo. La GUI se publica como instalador por
plataforma (MSI/DMG/AppImage+deb) bajo la misma custodia de firmas; sus
presupuestos de arranque y memoria se miden y publican en `docs/qa/` por
release. Los crates `meltemi`/`meltemid`/`meltemi-proto` reservan el namespace.
