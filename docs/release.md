<!-- SPDX-License-Identifier: Apache-2.0 -->
# Releases: packaging, signing, and installation

This document describes how Meltemi is packaged, signed, verified, and installed.
The promise is a **single self-contained binary, installable with one command**
(foundational document §4.5). See [versioning](versionado.md) for how versions
are decided.

Every push to main whose CI succeeds automatically publishes its exact source
SHA. Release validates the CI execution through GitHub's API, reserves an
immutable version tag, runs all release gates on three platforms, stamps the
build version in isolated checkouts, packages and attests the artifacts. No
manual version commit or release PR is needed (release-automatica-main).

_Resumen en español al final._

## Canonical download base

Every download resolves against **one** base, declared here once and nowhere
else. The installer scripts, the workspace manifest and this document must all
name it, and `core/meltemid/tests/release.rs` fails the build if they diverge;
the site, which lives in [meltemi-web](https://github.com/askenaz-dev/meltemi-web),
links the same base:

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
assets too, with their checksums inside the `SHA256SUMS` the attestation covers:
the site links a script whose hash travels attested, instead of hosting a copy of
it.

Each release publishes, per supported platform (see
[platform notes](plataformas.md)), an archive containing the `meltemi` client,
the `meltemid` daemon and the two ACP adapters Meltemi ships —
`meltemi-claude-acp` and `meltemi-codex-acp` — which must land **in the same
directory as the daemon**, because that is the last place its detection looks
for them (`docs/agentes.md`). Plus:

- a `SHA256SUMS` file with the checksum of every asset, the installers and the
  installer scripts included;
- `attestation.sigstore.json`, the bundle of the attestation that signs the
  release, so a download can be verified without network;
- the **desktop client installer** (gui-tauri-paridad): MSI on Windows, DMG on
  macOS, deb on Linux, covered by the same `SHA256SUMS` and the same
  attestation. No installer bundles a webview: it uses the OS engine —
  bootstrapping WebView2 on Windows when missing, already present on macOS,
  declared as a package dependency (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) on
  Linux. No self-contained format is published for exactly that reason: an
  AppImage would have to carry the engine, and measured ~79 MB against a 15 MB
  budget (instaladores-linux-sin-webview). Outside the Debian family the
  desktop app is built from source until an `.rpm` exists; the core archive
  runs anywhere.

## Verifying a download

Two steps, ordered by what each one buys, so you can stop at the guarantee you
need:

```bash
# 1. Check the checksum: the file arrived intact
sha256sum --check --ignore-missing SHA256SUMS   # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature: this repository's release workflow published this very file
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml
```

A checksum proves the file did not change in transit. The attestation proves
which repository, which commit and which workflow published it; `gh` verifies it
against the public log. Verify the artifact you downloaded, not `SHA256SUMS`:
the attestation's subjects are the entries of that file, one per published
asset, and the checksums file itself is not one of them.

The installer scripts do step 1 for you, not step 2: `gh` ships with no
operating system, and a one-line installer that first demands a package install
is not one line.

### Without network

The bundle published with every release makes step 2 possible offline. The only
thing that still comes from the network is the trusted root, which you download
once, on any machine that is online:

```bash
gh attestation trusted-root > trusted_root.jsonl
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml \
  --bundle attestation.sigstore.json --custom-trusted-root trusted_root.jsonl
```

### What the signature proves, and what it does not

The attestation **is the signature of the release**; there is no other. The
`publish` job of [`.github/workflows/release.yml`](../.github/workflows/release.yml)
mints it with the job's own identity (Sigstore, no long-term key), over the
entries of the merged `SHA256SUMS`, so one attestation covers every published
asset.

Two limits, written down so the claim is never read as more than it is:

- **It attests the aggregation, not each build step.** The `publish` job only
  downloads what the six packaging jobs built and merges their checksums. The
  attestation ties the published set to the publication workflow revision. With
  workflow_run, that revision can differ from the tested source SHA: the attested
  release-manifest.json records the exact source and its CI run. The attestation does
  not attest the compilation of each artifact in its packaging job. Attesting
  the builders themselves would be a future, stronger claim — this one is
  modest and true.
- **An account with write access to this repository can publish a release that
  verifies.** It pushes a commit to `main`, CI approves it, the pipeline builds it, and
  the attestation faithfully records that commit — which is its job. A signature
  made by hand on a machine GitHub does not control was the one step such an
  account could not complete; the maintainer gave that step up on 2026-10-23,
  with this limit in view, and the [controls](#controls-around-a-release-nobody-signs-by-hand)
  below are what surrounds the door it leaves.

The trust anchor is the identity of the workflow,
`askenaz-dev/meltemi/.github/workflows/release.yml`, which `--signer-workflow`
pins and which lives in this repository by construction. Never take the anchor,
or any key, from the text of a release page: whoever can publish a release can
edit the text beside it.

### The public log

Because this repository is public, every attestation is signed through the
Sigstore public-good instance and lands in a **public, permanent transparency
log**. What gets recorded: the names and digests of the published assets and
the identity of the workflow that minted the attestation — repository, workflow
path, commit. That is build metadata, published deliberately; it is never user
data, and it says nothing about who downloads or verifies anything. A project
that promises no hidden telemetry states this itself rather than leaving it for
a reader to discover.

## Automatic publication and recovery

Push the reviewed change to main. The `CI` workflow must complete successfully;
Release consumes its exact head_sha, not the moving tip of main. A pull request,
fork, different branch, different workflow or failed CI cannot publish. A
separate trusted control-plane checkout validates the run before source code is
checked out. The source must also remain an ancestor of origin/main.

The pipeline has five stages:

1. `authorize`: validate ci.yml id/path, repository identity, push/main and success.
2. `version`: allocate a deterministic version by first-parent ancestry, reserve
   an annotated tag and write release-manifest.json and notes from its commit range.
3. `gates` and `deny`: validate the stamped tree on Linux, macOS and Windows.
4. `package` and `package-gui`: build the same SHA and stamped version everywhere.
5. `publish`: merge assets, attest SHA256SUMS, complete a draft, then publish.

The source archive behind the tag deliberately retains its declared version.
The attested **release-manifest.json** records sourceSha, sourceVersion, delivered
version, CI run and the numbering anchor. To reproduce, checkout sourceSha and
run `RELEASE_VERSION=X.Y.Z node scripts/automatic-release.mjs stamp` before the
normal build (PowerShell: `$env:RELEASE_VERSION = 'X.Y.Z'` then the Node command).
The stamp uses the existing Rust version tool and changes no dependency versions.

**Notes** list actual commit subjects from the reserved ancestor-tag range. A
version-specific CHANGELOG.md section can add curated notes and a tested milestone
report; `meltemi-release changelog X.Y.Z --report <path>` remains available for
that purpose. No generated text claims manual testing took place.

**Recovery:** re-run the failed jobs or dispatch from main with the successful
CI run id:

```bash
gh workflow run release.yml --ref main -f ci_run_id=123456789
```

The API validation is identical for both entry points. Runs predating this
automation are refused clearly; historical unreserved tags are not rebuilt.
The tag is reserved before the release gates and remains if a build or upload
fails. A retry reuses the same tag/version and completes the draft. An already
published release is never modified: a retry verifies its required assets,
manifest and checksums before skipping. An incomplete immutable release fails
explicitly. A valid retry can still repair the web notice. Internal Actions
artifacts use overwrite for rerun-all; failed-only keeps artifacts from the
successful jobs. This does not overwrite published release assets.
An older recovered source cannot replace latest: promotion requires both a
higher version and ancestry from the current latest source.

CI and Release use `queue: max`. GitHub retains up to **100 pending runs** per
concurrency group; excess runs are canceled by the service. Recover a canceled
publication with its successful CI run id. If CI itself was canceled, rerun it
first. FIFO follows queue arrival, not commit order; source-based numbering
handles completion in either order. See [GitHub concurrency](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#concurrency).

The previous proposal `version-que-propone-ci` is superseded for its manual
release-PR gate. Its future readiness calculations are separate work, not a
prerequisite for automatic patch releases.

## Controls around a release nobody signs by hand

With no person between a successful CI push and a publication, the doors next to
the one written above are closed in code:

- every action the workflows use is **pinned by full commit**, with its version
  as a comment, so nobody can move what runs by moving a tag;
- every workflow starts **read-only**, and only the jobs that publish raise their
  permissions;
- the jobs that publish or hold a publication credential run in the **`release`
  environment**, which admits `main` only;
- authorization refuses CI outside this repository's successful push-to-main
  workflow; source ancestry and immutable reservations are verified again;
- existing reservations are reused, historical tags are not rebuilt, and latest
  is protected by both source ancestry and version;
- **rulesets** keep `main` from being deleted or rewritten and version tags from
  being moved or deleted; they are declared in the family's Terraform
  (`packages/infra` in [meltemi-web](https://github.com/askenaz-dev/meltemi-web)),
  with the environment and its credential.

**No signing key exists.** None is kept in this repository, in CI or on any
machine, and none is to be added: a key in an Actions secret is exactly the case
SLSA forbids in normative language — secret material "MUST NOT be accessible to
the environment running the user-defined build steps".

**The way back to a human step**, should the trade be revisited, is one line in
that Terraform: required reviewers on the `release` environment. Every
publication would then wait for a person to approve it.

## Repudiating a release

A published release cannot be corrected in place: releases are immutable. When
one turns out compromised or broken, the procedure is to publish the next
version and to declare the bad one, with its date, in the new release's notes
and in the list below. The transparency log keeps the evidence of what was
published, from which commit and by which workflow.

Repudiated releases: none.

## The web notice

After `publish`, the `notify-web` job starts the deploy workflow of
[meltemi-web](https://github.com/askenaz-dev/meltemi-web), which serves
meltemi.dev. The notice carries no data: the site reads the published release
from GitHub, so a lost notice leaves the site late, never wrong.

Its credential is the `WEB_DISPATCH_TOKEN` secret of this repository's
`release` environment, where only jobs on `main` can read it and only the
notice step names it. The design calls for a fine-grained token owned by the
`askenaz-dev` organisation, with access to `meltemi-web` only and the single
permission **Actions: read and write**, which can start that repository's
workflows and cannot write its code. Until one exists, the maintainer chose on
2026-10-23 to fill it with the organisation's Terraform admin token, which can
do far more; a narrow token stored later in meltemi-web's `infrastructure`
environment takes its place with no other change, and Terraform places either
value here. The token expires: when it does, the notice fails in red with the
release already published, and the value is replaced and applied again.

## Platform signing

The MSI and the DMG carry no platform signature yet: Windows Authenticode and
Apple notarization need purchased certificates, and until those exist the
installers warn on first run. That is declared debt, not an oversight — the
checksums and the attestation still cover exactly what was published.

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
attested.

The published site's [downloads page](https://meltemi.dev/ide/downloads/) is the
public entry point for all of the above.

## Release pipeline and budgets

The release pipeline (`.github/workflows/release.yml`) starts after successful
CI for a push to main and checks out that exact approved source SHA. It runs on Windows,
macOS, and Linux with **hard gates**: the full test suite, `cargo clippy -- -D
warnings`, `cargo fmt --check`, `cargo deny`, and the **performance budgets**
(meltemi.md §12, "Métricas de Éxito") — the TUI binary staying under **25 MB**,
every GUI installer under **15 MB**, and each bundled ACP adapter under **6 MB**
(measured at ~3.15 MB apiece on 2026-07-28, see `docs/qa/`). Any red gate aborts
the release and **no artifact is published**. The GUI's runtime budgets (startup
under 1 s, idle memory under 80 MB) are measured per release and published in
`docs/qa/` — honest measurements, not blocking CI gates, because they depend on
the OS webview.

## Two build paths, one of which publishes

The repository builds the same per-platform files on two different paths, and
only one of them produces something installable.

| | Integration build | Release |
|---|---|---|
| Workflow | `.github/workflows/build.yml` | `.github/workflows/release.yml` |
| Trigger | every push to `main`, or by hand | successful CI for every push to `main` |
| Output | artifacts of the run, expiring after 7 days | assets of a published release |
| Signature | none, ever | the attestation its workflow mints |
| What it is for | trying a build | installing Meltemi |

An artifact of a workflow run is **not distribution**. It has no stable URL, it
expires, it never appears under `releases/latest`, and it is served only after
authentication. That is exactly the right shape for trying the latest build —
the macOS DMG, on a machine that cannot build one — and exactly the wrong shape
for installing anything.

So the integration build signs nothing and publishes nothing, and it *cannot*:
it runs with `permissions: contents: read`, and that token cannot create a
release even if some future step asked it to. Publishing belongs to
`release.yml` alone, and only when its gate opens the path.

Each artifact is named for the platform it was built on and the commit it was
built from, and says `unsigned` in that name, so it can be told from a release
without being opened. The files inside keep the stable, version-free names of
the table above: the point of this path is to rehearse exactly what a release
produces, name normalization included. The same three size budgets apply, with
the same values, because a packaging path nobody measures is precisely how a
budget rots.

Every run declares all of this where the download button is. The run summary and
an `UNSIGNED-BUILD.txt` beside the binaries both carry the same text, from
[`scripts/unsigned-build-notice.txt`](../scripts/unsigned-build-notice.txt).
The checksums file in an integration artifact is a manifest of what that run
built — not a verification of origin.

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

Cada push a main con CI verde publica su SHA exacto, sin bump ni PR manual.
La versión se reserva en un tag anotado inmutable y se estampa solo en los
checkouts de build. El manifiesto atestado declara fuente, versión y CI.
Reintentar conserva la reserva y completa el borrador; recuperar exige el id de
un CI push/main válido. La cola conserva hasta 100 pendientes por grupo; un
reintento antiguo nunca retrocede latest. La prueba manual de un hito sigue
siendo evidencia aparte y no se inventa en las notas automáticas.

Cada release publica, por plataforma, el archivo con `meltemi`, `meltemid` y los
dos adaptadores ACP propios (que se instalan junto al daemon), el instalador de
escritorio (MSI/DMG/deb) y los scripts de instalación, todos con nombre estable
y su checksum en un único `SHA256SUMS`. **La firma es la atestación** que acuña
el workflow de release sobre las entradas de ese archivo, sin clave de largo
plazo; su bundle, `attestation.sigstore.json`, viaja con la release para
verificar sin red. Se verifica en dos pasos: el checksum y
`gh attestation verify <artefacto> --signer-workflow
askenaz-dev/meltemi/.github/workflows/release.yml`. La atestación cubre la
agregación, no cada paso de build, y queda en un log de transparencia público
—metadato de build, jamás dato de usuario—. El límite se declara: una cuenta con
permiso de escritura puede publicar una release que verifica; lo rodean las
acciones pineadas por commit, los permisos de solo lectura, el entorno
`release` limitado a `main`, las negativas del gate y los rulesets. No existe
clave de firma alguna, y una en un secreto de Actions sigue prohibida. Una
release no se corrige: se publica la siguiente y se declara la repudiada con su
fecha. Tras publicar, el pipeline avisa a meltemi-web, sin datos, con un token
que solo puede lanzar sus workflows.

El pipeline de release corre las tres plataformas con gates duros —suite,
clippy, fmt, cargo-deny y presupuestos §12 (TUI < 25 MB; instalador GUI < 15 MB;
adaptador ACP < 6 MB cada uno)— y aborta sin publicar ante cualquier rojo. Los
crates `meltemi`/`meltemid`/`meltemi-proto` reservan el namespace. Hay **dos
rutas de build y solo una publica**: cada push a `main` deja los mismos archivos
por plataforma descargables desde la página de la ejecución (`build.yml`, con
los mismos presupuestos y sin crear release alguna); la release la crea
únicamente el workflow de release después de CI verde y validación de su SHA. Un
artefacto de ejecución no es distribución: caduca, no tiene URL estable y jamás
aparece en `releases/latest`. No lleva firma ni procedencia y lo declara donde
se descarga —en el resumen del run y en `UNSIGNED-BUILD.txt` junto a los
binarios—; su `SHA256SUMS` es el manifiesto de lo que ese run construyó, no una
verificación de origen. Sirve para probar un build; para instalar, la release
publicada.
