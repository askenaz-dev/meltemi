<!-- SPDX-License-Identifier: Apache-2.0 -->
# Release pipeline reference (maintainers)

Operational release procedures and CI controls belong in the repository, outside the public user
portal. For downloads, installation and verification, see [the user guide](../release.md).
This reference changes no policy, secret or infrastructure and does not authorize a publication.

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
that purpose. Follow the [one-developer test](../prueba-de-uno.md) to exercise an
installed release end to end and record the milestone's evidence. No generated
text claims manual testing took place.

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

GitHub's tag lookup only returns published releases. Draft recovery therefore
uses the authenticated, paginated release list and retains the numeric release
ID for verification before and after publication. A 404 from the tag endpoint
alone does not mean no draft exists. Multiple entries for the same tag are
refused rather than selecting a draft to overwrite. See the
[GitHub release API](https://docs.github.com/en/rest/releases/releases#get-a-release-by-tag-name).

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
[`scripts/unsigned-build-notice.txt`](../../scripts/unsigned-build-notice.txt).
The checksums file in an integration artifact is a manifest of what that run
built — not a verification of origin.

Cadence is a dial, not a commitment: three release builds with a Tauri bundle on
every push to `main` is the most expensive thing this repository asks of CI.
`build.yml` is written so that dropping to manual runs only, or to a schedule,
or to a single platform, is an edit to its `on:` block or its matrix — not a
restructuring of the pipeline.
