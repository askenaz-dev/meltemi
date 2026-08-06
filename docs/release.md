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
[platform notes](plataformas.md)), an archive containing the `meltemi` client,
the `meltemid` daemon and the two ACP adapters Meltemi ships —
`meltemi-claude-acp` and `meltemi-codex-acp` — which must land **in the same
directory as the daemon**, because that is the last place its detection looks
for them (`docs/agentes.md`). Plus:

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

Before installing, verify the archive. The steps are ordered by what each one
buys, so you can stop at the guarantee you need:

```bash
# 1. Check the checksum: the file arrived intact
sha256sum --check SHA256SUMS        # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature of the checksums file: the maintainer vouched for it
minisign -Vm SHA256SUMS -P <the public key below>

# 3. Verify the build provenance: which repository, commit and workflow produced it
gh attestation verify SHA256SUMS --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml
```

Each step answers a question the previous one cannot. A checksum proves the
file did not change in transit. A signature proves the maintainer signed these
checksums, on a machine GitHub does not control. The attestation proves what
*process* produced the published set — the one thing neither of the first two
can say. Steps 2 and 3 each cost a tool that ships with no operating system:
`minisign` for the signature, the GitHub CLI (`gh`) for the provenance.

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

### Provenance: what the attestation proves

The `release` job of
[`.github/workflows/release.yml`](../.github/workflows/release.yml) mints a
build attestation whose subjects are the entries of the merged `SHA256SUMS` —
the same single file the maintainer signs, so both mechanisms cover exactly the
same set of assets. `gh attestation verify` proves that the file you hold
matches an attestation minted by this repository's release workflow, for a
specific commit and run; `--signer-workflow` pins that workflow identity, and a
file built anywhere else fails the check.

Two limits, written down so the claim is never read as more than it is:

- **It attests the aggregation, not each build step.** The `release` job only
  downloads what the six packaging jobs built and merges their checksums. The
  attestation ties the published set to this workflow and this commit; it does
  not attest the compilation of each artifact in its packaging job. Attesting
  the builders themselves would be a future, stronger claim — this one is
  modest and true.
- **It is not a substitute for the maintainer's signature.** A compromised
  GitHub account can push a tag, let CI build it, and mint an attestation that
  verifies perfectly — it faithfully records the attacker's commit, which is
  its job. The manual signature on a machine GitHub does not control is the one
  step such an account cannot complete. The two mechanisms answer different
  questions, and neither replaces the other.

### Provenance: the public log, and verifying offline

Because this repository is public, every attestation is signed through the
Sigstore public-good instance and lands in a **public, permanent transparency
log**. What gets recorded: the names and digests of the published assets and
the identity of the workflow that minted the attestation — repository, workflow
path, commit. That is build metadata, published deliberately; it is never user
data, and it says nothing about who downloads or verifies anything. A project
that promises no hidden telemetry states this itself rather than leaving it for
a reader to discover.

The two verification tools are not symmetric offline, and for a local-first
product that deserves its own paragraph. minisign needs no network at all: the
public key and `SHA256SUMS.minisig` are enough, on any machine, forever. `gh
attestation verify` asks the GitHub API by default; an offline verification
exists, but it requires the attestation bundle and the trusted material to have
been downloaded beforehand from a machine that was online. Step 3 is the only
step of the three that assumes connectivity.

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

One-line, auditable installers place `meltemi`, `meltemid` and the two bundled
ACP adapters on the user's `PATH` — all in one directory — and create the short
alias **`mel`**:

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
**25 MB**, every GUI installer under **15 MB**, and each bundled ACP adapter
under **6 MB** (measured at ~3.15 MB apiece on 2026-07-28, see `docs/qa/`).
Any red gate aborts the release and **no artifact is published**. The GUI's runtime budgets (startup
under 1 s, idle memory under 80 MB) are measured per release and published in
`docs/qa/` — honest measurements, not blocking CI gates, because they depend
on the OS webview.

## Two build paths, one of which publishes

The repository builds the same per-platform files on two different paths, and
only one of them produces something installable.

| | Integration build | Release |
|---|---|---|
| Workflow | `.github/workflows/build.yml` | `.github/workflows/release.yml` |
| Trigger | every push to `main`, or by hand | a `vX.Y.Z` tag |
| Output | artifacts of the run, expiring after 7 days | assets of a published release |
| Signature | none, ever | detached, by the maintainer |
| Provenance | none | attestation over the merged checksums |
| What it is for | trying a build | installing Meltemi |

An artifact of a workflow run is **not distribution**. It has no stable URL, it
expires, it never appears under `releases/latest`, and it is served only after
authentication. That is exactly the right shape for trying the latest build —
the macOS DMG, on a machine that cannot build one — and exactly the wrong shape
for installing anything.

So the integration build signs nothing and publishes nothing, and it *cannot*:
it runs with `permissions: contents: read`, and that token cannot create a
release even if some future step asked it to. The reason is the same one that
keeps signing manual (see [Key custody](#key-custody)): the signature is the
step a compromised CI account cannot perform, so an automated path able to sign
would be worth exactly as much as the account that triggers it — which is to
say, not enough.

Each artifact is named for the platform it was built on and the commit it was
built from, and says `unsigned` in that name, so it can be told from a release
without being opened. The files inside keep the stable, version-free names of
the table above: the point of this path is to rehearse exactly what a tag
produces, name normalization included. The same three size budgets apply, with
the same values, because a packaging path nobody measures is precisely how a
budget rots.

Every run declares all of this where the download button is. The run summary and
an `UNSIGNED-BUILD.txt` beside the binaries both carry the same text, from
[`scripts/unsigned-build-notice.txt`](../scripts/unsigned-build-notice.txt).
The checksums file in an integration artifact is a manifest of what that run
built — not a verification of origin. Nothing is signed beside it, and nothing
will be.

Cadence is a dial, not a commitment: three release builds with a Tauri bundle on
every push to `main` is the most expensive thing this repository asks of CI.
`build.yml` is written so that dropping to manual runs only, or to a schedule,
or to a single platform, is an edit to its `on:` block or its matrix — not a
restructuring of the pipeline.

## Crate namespaces

To secure the project's namespace, the crates `meltemi`, `meltemid`, and
`meltemi-proto` are reserved on the registry: `meltemi-proto` is the real
published contract; `meltemi` and `meltemid` are honest placeholders that point
at this repository until their libraries are published. Publishing is a
maintainer action (the placeholder crates set `publish = false` in-tree as a
safety until the maintainer reserves the names).

## Summary (español)

Cada release publica archivos por plataforma con `meltemi`+`meltemid`+los dos
adaptadores ACP propios (que se instalan junto al daemon), checksums
y firma, más una atestación de procedencia acuñada por el workflow sobre el
`SHA256SUMS` fusionado (cubre la agregación, no cada paso de build; queda en un
log de transparencia público —metadato de build, jamás dato de usuario— y su
verificación con `gh` consulta la red por defecto, a diferencia de minisign,
que verifica sin conexión). El usuario verifica checksum, firma y procedencia
con instrucciones publicadas; los
instaladores (`scripts/install.sh`, `scripts/install.ps1`) lo hacen por él, son
legibles y con hash publicado, e instalan los binarios y el alias `mel`. La
custodia de la clave es del mantenedor: la privada jamás toca el repositorio ni
CI y se guarda offline —minisign no ofrece HSM ni revocación, así que repudiar
es publicar clave nueva en el repositorio con declaración fechada—; la pública
vive en el repositorio, que es el ancla, no en la página de release. El
pipeline de release corre las tres plataformas con gates duros —suite, clippy,
fmt, cargo-deny y presupuestos §12 (TUI < 25 MB; instalador GUI < 15 MB;
adaptador ACP < 6 MB cada uno)— y
aborta sin publicar ante cualquier rojo. La GUI se publica como instalador por
plataforma (MSI/DMG/deb) bajo la misma custodia de firmas; sus
presupuestos de arranque y memoria se miden y publican en `docs/qa/` por
release. Los crates `meltemi`/`meltemid`/`meltemi-proto` reservan el namespace.
Hay **dos rutas de build y solo una publica**: cada push a `main` deja los
mismos archivos por plataforma descargables desde la página de la ejecución
(`build.yml`, con los mismos presupuestos y sin crear release alguna); la
release la crea únicamente un tag. Un artefacto de ejecución no es
distribución: caduca, no tiene URL estable y jamás aparece en `releases/latest`.
No lleva firma ni procedencia y lo declara donde se descarga —en el resumen del
run y en `UNSIGNED-BUILD.txt` junto a los binarios—; su `SHA256SUMS` es el
manifiesto de lo que ese run construyó, no una verificación de origen. Sirve
para probar un build; para instalar, la release publicada.
