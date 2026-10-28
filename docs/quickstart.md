<!-- SPDX-License-Identifier: Apache-2.0 -->
# Quickstart

From the release assets to a governed session with the agent you already have,
and then to your first reviewed proposal — with no compiler and no configuration
written by hand. The scriptable steps below run against the built binaries in
the docs CI, so a divergence between this page and the product fails the build.

Building from source is for contributors: see the README ("Option B — from
source") and [CONTRIBUTING.md](../CONTRIBUTING.md).

## 1. Install from the release

Every link resolves to the latest published release; you never need a version
number.

| Platform | Desktop app | Core: daemon and terminal client |
|---|---|---|
| Windows 10 1809+ / 11 | [`meltemi-desktop-Windows.msi`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Windows.msi) | [`install.ps1`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.ps1) or [`meltemi-Windows.zip`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Windows.zip) |
| macOS 13+ | [`meltemi-desktop-macOS.dmg`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-macOS.dmg) | [`install.sh`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.sh) or [`meltemi-macOS.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-macOS.tar.gz) |
| Linux (Debian/Ubuntu) | [`meltemi-desktop-Linux.deb`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Linux.deb) | [`install.sh`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.sh) or [`meltemi-Linux.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Linux.tar.gz) |

The install scripts are short: download them, read them, then run them. They
put `meltemi`, `meltemid` and the `mel` alias on your `PATH`, and they refuse a
download whose checksum does not match. To check a file yourself:

```bash
sha256sum --check --ignore-missing SHA256SUMS      # 1. it arrived intact
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml   # 2. it is the release's
```

(`shasum -a 256` on macOS; `Get-FileHash` on Windows.) What each check proves,
and the offline variant, are in [releases and verification](release.md).

**The first launch of the desktop app warns you.** The MSI and the DMG carry no
platform signature yet — that needs purchased certificates — so Windows
SmartScreen and macOS Gatekeeper ask before opening them. The checks above are
what tell you the file is the one the release published. The
[FAQ](faq.md#the-installer-warns-that-it-is-not-signed) shows the exact clicks.

## 2. Check it worked

```
meltemi version      # prints the client version, no daemon needed
meltemi help         # the full scriptable surface
meltemi status       # starts the daemon on demand and reports it
```

`version` and `help` are local: they never touch the daemon. `status` starts it
on demand over a local socket — no network port is ever opened.

## 3. Your agents

Meltemi runs the official binary of the coding agent you already have, with
the agent's own authentication: it never reads or stores an agent's
credentials.

```
meltemi fleet        # the agent catalog crossed with what is installed here
```

Each row says whether the agent is ready and, when it is not, which layer is
missing and the exact command that installs it. Nothing needs configuring: the
agent is named with `--agent <id>` from that list, or chosen with a chip in the
desktop app. Several subscriptions of one provider are linked with
`meltemi link <agent> <name>`. The detail per agent is in the
[agents guide](agentes.md).

## 4. A governed session

```
cd your-project
meltemi session "summarise what this project does" --agent <id>
```

The session runs under the permission proxy: every tool the agent wants to use
is asked for, here in your terminal, with the agent's own options — a digit
picks one, `d` denies, `r` approves and remembers the rule, Ctrl+C stops the
session. Everything is written to an append-only log, and a restore point is
taken before the first turn. The [free-session guide](sesion-libre.md) says what
governs a session and what a rule allows.

In the desktop app, **New session** opens the composer: pick the agent with its
chip, write the instruction, and send it. Permission requests appear inline in
the conversation, and the tray holds every request still waiting.

## 5. Your first proposal

```
meltemi propose "add a dark-mode toggle to the settings page" --agent <id>
```

This is when the project gains its `.meltemi/` folder: Meltemi scaffolds the
change under `.meltemi/changes/` and delegates the proposal to your agent, under
the same permission proxy. Review the spec deltas as a checklist:

```
meltemi review add-dark-mode-toggle
```

## 6. The rest of the cycle

```
meltemi plan add-dark-mode-toggle          # refine design, sequence tasks
meltemi verify add-dark-mode-toggle        # per-requirement checklist
meltemi implement add-dark-mode-toggle <agent>   # deploy over tasks.md
meltemi archive add-dark-mode-toggle       # fold deltas into the living truth
```

Every task runs in an isolated worktree with a pre-task checkpoint and an atomic
commit that traces back to its requirement.

## Accessibility and remote use

- `--json` on any subcommand emits machine-readable output.
- `NO_COLOR` and an ASCII fallback are honored (see
  [accessibility](accesibilidad.md)).
- For remote use, tunnel the **local socket** over SSH — the daemon never
  listens on the network. See [remote access](acceso-remoto.md) and the
  [platform notes](plataformas.md).

## Windows note

Under git-bash / MSYS, environment variables that look like paths (e.g.
`MELTEMI_ENDPOINT`) can be mangled. See the
[platform notes](plataformas.md#windows-git-bash) for the remedy before setting
them.

## When something goes wrong

Start with the [questions and troubleshooting](faq.md) page.
