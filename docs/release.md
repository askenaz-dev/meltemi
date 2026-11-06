<!-- SPDX-License-Identifier: Apache-2.0 -->
# Downloads, installation and verification

[Español](es/release.md)

Install Meltemi from a published GitHub release. Choose the desktop installer for the graphical
app and the core archive or install script for the daemon, terminal client and bundled adapters.
See [quickstart](quickstart.md) for the first session and [versions](versionado.md) for the source
information recorded with each release.

## Canonical download base

The download base is:

```
CANONICAL_DOWNLOAD_BASE=https://github.com/askenaz-dev/meltemi/releases
```

Links using `releases/latest/download/<stable-name>` resolve to the latest published release.
Use a release's own page when you need a specific version.

## Artifacts

| Platform | Core archive | Desktop installer |
|---|---|---|
| Windows | `meltemi-Windows.zip` | `meltemi-desktop-Windows.msi` |
| macOS | `meltemi-macOS.tar.gz` | `meltemi-desktop-macOS.dmg` |
| Linux | `meltemi-Linux.tar.gz` | `meltemi-desktop-Linux.deb` |

The core archive contains `meltemi`, `meltemid`, `meltemi-claude-acp` and `meltemi-codex-acp`.
Keep the adapters in the same directory as the daemon. Install the provider's official CLI
separately; the [agents guide](agentes.md) explains what each integration needs.

The desktop uses the operating system's webview. Windows installs WebView2 if needed; macOS
uses its system webview; the Debian package declares `libwebkit2gtk-4.1-0` and `libgtk-3-0`.
There is no published RPM or AppImage. See [platforms](plataformas.md) for supported systems.

Each release also includes:

- `install.sh` and `install.ps1`, the core installation scripts;
- `SHA256SUMS`, with the file digests used to verify release assets;
- `attestation.sigstore.json`, the release attestation bundle;
- `release-manifest.json`, which records the delivered version, exact source SHA and approved CI.

## Verifying a download

Download the asset and `SHA256SUMS` from the same release. Check the checksum first, then verify
its publication origin if you have GitHub CLI installed:

```bash
# 1. Check the checksum: the file arrived intact
sha256sum --check --ignore-missing SHA256SUMS   # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature: this repository's release workflow published this very file
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml
```

Use the name of the asset you downloaded in the second command. The attestation subjects are
the assets listed in `SHA256SUMS`; the checksum file itself is not a subject. The install scripts
perform the checksum check for you, but do not perform attestation verification.

The trust anchor is the identity of the workflow,
`askenaz-dev/meltemi/.github/workflows/release.yml`. Never take the anchor, or any key, from the text
of a release page: verify against the expected repository and workflow.

### Without network

Download the trusted root once on an online machine, then transfer it, your asset and the release
bundle to the machine where you will verify:

```bash
gh attestation trusted-root > trusted_root.jsonl

gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml \
  --bundle attestation.sigstore.json --custom-trusted-root trusted_root.jsonl
```

### What the attestation proves

The attestation is the signature of the release. The `publish` job signs the asset set with its
workflow identity; it attests the aggregation, not each build step, and does not attest the
compilation of each individual artifact. `release-manifest.json` identifies the source approved
by CI, which can differ from the workflow revision recorded by the publication attestation.

A valid signature proves origin and integrity, not that the program is safe or free of defects.
An account with write access to this repository can publish a release that verifies. Read the
release notes and check the expected source as well as the signature.

The public transparency log records the names and digests of the assets and the identity of the
workflow that published them. This is build metadata, never user data or a record of downloads.

## Installers

Download an install script, read it, then run it. Both scripts place the daemon, terminal client
and bundled adapters together on `PATH` and create the short `mel` alias:

- Unix: [install.sh](../scripts/install.sh).
- PowerShell: [install.ps1](../scripts/install.ps1).

For manual installation, extract the verified core archive, keep its binaries together, and add
that directory to `PATH`. The script headers describe the equivalent manual steps.

After installation:

```bash
meltemi version
meltemi status
meltemi fleet
```

`status` starts the local daemon when needed. After an update, stop an older running daemon with
`meltemi stop`; the next command starts the installed version. The desktop and daemon should use
the same release. `meltemi doctor` reports a mismatch.

## Platform signing

The MSI and DMG do not currently have Windows Authenticode or Apple platform signatures.
SmartScreen or Gatekeeper may warn before opening them. Checksums and the Sigstore attestation
verify the published files; they do not remove those platform warnings. The [FAQ](faq.md#the-installer-warns-that-it-is-not-signed)
explains the available opening controls after you verify the download.

## Published releases and temporary builds

Install from [published releases](https://github.com/askenaz-dev/meltemi/releases), rather than
from an artifact of `.github/workflows/build.yml`. An integration-run artifact is not distribution:
it expires, has no stable latest-release URL and includes `UNSIGNED-BUILD.txt` to mark its lack
of a release signature. Its checksum only describes that temporary build.

Successful CI for a push to `main` starts automatic publication of that exact source. A tag can
exist before packaging finishes; use a published release with its required assets. Automatic
release notes describe source changes and do not claim that a manual acceptance test was run.

## Repudiating a release

A published release is immutable. The recovery procedure is to publish the next version and to
declare the bad one, with its date, in the release notes. Check those notes before installing an
older version.

Repudiated releases: consult the published release notes for current notices.
