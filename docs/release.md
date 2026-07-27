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
| Linux | `meltemi-Linux.tar.gz` | `meltemi-desktop-Linux.deb` |

The installer scripts (`install.sh`, `install.ps1`) are published as release
assets too, with their checksums inside the signed `SHA256SUMS`: the site links
a script whose hash travels signed, instead of hosting a copy of it.

Each release publishes, per supported platform (see
[platform notes](plataformas.md)), an archive containing the `meltemi` client and
the `meltemid` daemon, plus:

- a `SHA256SUMS` file with the checksum of every archive;
- a detached signature for the checksums file;
- the **desktop client installer** (gui-tauri-paridad): MSI on Windows, DMG on
  macOS, deb on Linux, with its own `SHA256SUMS` under the same signing custody.
  No installer bundles a webview: it uses the OS engine — bootstrapping WebView2
  on Windows when missing, already present on macOS, declared as a package
  dependency (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) on Linux. No self-contained
  format is published for exactly that reason: an AppImage would have to carry
  the engine, and measured ~79 MB against a 15 MB budget
  (instaladores-linux-sin-webview). Outside the Debian family the desktop app is
  built from source until an `.rpm` exists; the core archive runs anywhere.

## Verifying a download

Before installing, verify the archive:

```bash
# 1. Check the checksum
sha256sum --check SHA256SUMS        # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature of the checksums file with the project's public key
minisign -Vm SHA256SUMS -P <the public key below>
```

Step 2 answers a question step 1 cannot: a checksum proves the file did not
change in transit, a signature proves *who* published it. Both are needed, and
in that order.

### The public key

> **Not yet published.** The key exists and `v0.1.0` is signed with it, but its
> public half has not landed in this repository yet, so step 2 above cannot be
> completed by a reader today. Until it does, only step 1 is actually available.
> This is stated plainly rather than papered over with a placeholder.

The key belongs **here, in the repository**, and not on the release page — that
distinction is the whole point of an anchor. Anyone who can publish a release can
also edit the text beside it, so a key printed only there proves nothing: an
attacker would replace the artifacts, sign them with their own key, print their
own key next to them, and every published instruction would still pass. A key
committed to the tree has a git history: changing it is a diff, with an author
and a date, in a file thousands of clones already hold.

**The installer scripts do step 1 only.** They fetch the asset and `SHA256SUMS`,
compare the hash and refuse on a mismatch — there is no blind `curl | sh`: the
script is short, readable, and its own hash is published. They do **not** verify
the signature, because `minisign` is not present by default on any of the three
platforms and a one-line installer that first demands a package install is not
one line. Step 2 is yours to run, once, on the checksums file.

## Signing a release (maintainer)

The tool is [minisign](https://jedisct1.github.io/minisign/): one small signature
per release, a public key short enough to print in a README, and no keyring to
manage. It is what the artifact checksums are signed with; the alternative worth
knowing about is a GPG detached signature (`gpg --detach-sign --armor`), which
buys nothing here except a familiar command and a much larger surface.

**Once, when the key is created.** Do this on a machine you trust, and answer the
password prompt with a passphrase you keep in your password manager:

```bash
minisign -G -p meltemi.pub -s meltemi.key
```

`meltemi.pub` is public, and its home is this file, in the repository — the
release notes and the downloads page point here instead of printing a copy that
whoever publishes a release could swap. `meltemi.key` is the secret: it never
touches this repository, never touches CI, and its backup lives offline. Losing
it is recoverable (publish a new key and say so); leaking it means repudiating
it the only way minisign allows — a new key in this file and a dated statement
(see key custody below).

**Per release.** The pipeline produces a draft release with every artifact and a
recomputed `SHA256SUMS`. Sign that one file — the checksums cover everything
else, so one signature is enough:

```powershell
./scripts/sign-release.ps1 -Tag vX.Y.Z
```

It downloads `SHA256SUMS`, signs it (minisign asks for the key's passphrase on
the terminal — the script never sees or stores it), verifies the signature it
just produced, uploads `SHA256SUMS.minisig`, and then asks whether to publish the
draft.

**That order is mandatory, not tidy.** This repository has immutable releases
enabled: once a release is published its assets can no longer be added, changed
or removed. Publish before attaching the signature and that version can never be
signed at all — the only remedy is cutting a new one. The script checks the draft
state before it asks for your passphrase, so the mistake costs a prompt rather
than a version number. Publishing is the one step it never does on its own: it is the most
visible, least reversible action in the procedure, so it stays a decision you
make each release, not a default a script reaches for. It runs only on the
maintainer's own machine, never in CI — see the key-custody note below for why.

The manual equivalent, if you would rather run each step yourself:

```bash
gh release download vX.Y.Z --pattern SHA256SUMS --dir .
minisign -Sm SHA256SUMS -s meltemi.key
minisign -Vm SHA256SUMS -p meltemi.pub   # verify before publishing, always
gh release upload vX.Y.Z SHA256SUMS.minisig
gh release edit vX.Y.Z --draft=false
```

Either way, the release notes point at this file for the key and the procedure —
they never carry their own copy of the key, which could drift from the anchor or
be swapped along with the assets. A release whose signature you have not
verified yourself is not ready — the check above costs a second and catches the
case where the wrong key signed.

**Installer signing is a separate matter.** The MSI and the DMG carry no
platform signature yet: Windows Authenticode and Apple notarization need
purchased certificates, and until those exist the installers warn on first run.
That is declared debt, not an oversight — the checksums and the minisign
signature still cover exactly what was published.

### Key custody

The signing key is the **founding maintainer's** responsibility: generation,
offline storage, and a rotation and repudiation procedure. The key material is
never in this repository, and no CI job ever holds it.

Two limits of the chosen tool, written down so nobody plans against a capability
that is not there. Storage is **offline, not hardware-backed**: minisign has no
HSM or PKCS#11 support, so "the key lives in a token" is not reachable without
changing tools. And minisign has **no revocation mechanism** — there is no
message a holder can publish that makes an old signature stop verifying. So
"revoke" here can only mean: publish a new public key in this file, state in the
release notes and on the site that the old key is repudiated from a given date,
and re-sign what still matters. That works only because the key lives in the
repository, where the replacement is an auditable commit; it is the same property
that makes the anchor worth anything in the first place.

## Crate namespace (maintainer)

The three names on crates.io — `meltemi`, `meltemid`, `meltemi-proto` — are
verified free and unreserved. Every crate in this workspace carries
`publish = false` on purpose: crates.io is append-only, so an accidental
`cargo publish` is a permanent fact. Nothing reaches it without the maintainer
deliberately lifting that flag.

Reserving a name means publishing something real under it, which is why this is
not automated. When the moment comes:

```bash
cargo login                       # paste the crates.io API token, never store it in the repo
# drop `publish = false` from the manifest being reserved, then:
cargo publish -p meltemi-proto --dry-run   # read what it would upload
cargo publish -p meltemi-proto
```

`meltemi-proto` is the honest first name to claim: it is the contract crate, it
is useful to a third party on its own, and it does not promise a working product
the way `meltemi` does. Publish the binaries' names when the binaries are worth
installing from source — a squatting-prevention placeholder that never becomes a
real crate is its own kind of squatting.

Restore `publish = false` afterwards for any crate not meant to be published on
every release, so the guard keeps working.

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
**performance budgets** (meltemi.md §12, "Métricas de Éxito") — the TUI binary staying under
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
custodia de la clave es del mantenedor: la privada jamás toca el repositorio ni
CI y se guarda offline —minisign no ofrece HSM ni revocación, así que repudiar
es publicar clave nueva en el repositorio con declaración fechada—; la pública
vive en el repositorio, que es el ancla, no en la página de release. El
pipeline de release corre las tres plataformas con gates duros —suite, clippy,
fmt, cargo-deny y presupuestos §12 (TUI < 25 MB; instalador GUI < 15 MB)— y
aborta sin publicar ante cualquier rojo. La GUI se publica como instalador por
plataforma (MSI/DMG/deb) bajo la misma custodia de firmas; sus
presupuestos de arranque y memoria se miden y publican en `docs/qa/` por
release. Los crates `meltemi`/`meltemid`/`meltemi-proto` reservan el namespace.
