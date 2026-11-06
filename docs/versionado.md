<!-- SPDX-License-Identifier: Apache-2.0 -->
# Versions and source identity

[Español](es/versionado.md)

Meltemi uses **SemVer, pre-1.0**, with one delivered version across the daemon, terminal client,
desktop and bundled adapters. Run `meltemi version` to see the installed terminal client's
version. `meltemi status` and `meltemi doctor` help identify a daemon from an older installation.

## What a release number means

A successful CI run for a push to `main` starts a new release of that exact source commit.
Ordinary changes advance the patch version automatically. Version ordering follows source
history rather than the order in which builds finish.

A tag can be reserved while a build is still running or has failed. A gap in numbers does not
mean an asset is missing from your installation. Download a **published release** with its
manifest and checksums, rather than relying on a tag alone.

## Source and delivered version

`release-manifest.json` records the delivered `version`, `sourceSha`, declared `sourceVersion`,
CI run and tag. The source archive can retain an older declared base version: packaging stamps
the assigned release version into its isolated build checkout. This does not mean that the
published binaries came from another source.

To identify the code behind a download, inspect that manifest and verify the asset's attestation.
The [download guide](release.md) gives the checksum and signature commands.

## Compatibility

Before 1.0, a breaking change requires a **minor** version increase; after 1.0 it requires a
major increase. Compatibility covers:

- `proto/`: the public client/daemon methods, fields and schemas;
- **CLI grammar**: commands, flags, arguments and exit codes;
- **artifact format**: project specifications, changes and on-disk registries.

Read the release notes before an update that changes the minor version. A common version number
does not claim that every external agent or provider supports the same features; see [agents](agentes.md).

## Immutability and retries

Published assets and tags are not replaced in place. A failed publication retries with the same
reserved version and source. Recovering an older build does not move `latest` backwards.
A correction to an already published release is delivered as another version.
