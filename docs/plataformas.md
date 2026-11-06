<!-- SPDX-License-Identifier: Apache-2.0 -->
# Platform notes

[Español](es/plataformas.md)

Meltemi supports Windows 10 1809+ / Windows 11, macOS 13+, and mainstream glibc
Linux. This page covers installation, local data and the environment settings you may need.

## The desktop app on Linux

The core — daemon and terminal client — runs on any mainstream glibc
distribution from the release archive. The desktop app is published **only as a
`.deb`**, which declares `libwebkit2gtk-4.1-0` and `libgtk-3-0` so the package
manager installs the engine it uses.

There is no published AppImage or RPM. On distributions outside the Debian family, use the core
archive for the daemon and terminal client, or build the desktop app from source using the
repository's build instructions. The desktop uses the operating system's webview rather than
bundling a browser engine.

## Data paths

The daemon keeps two kinds of files in the OS-conventional per-user folders. The
**data** folder holds session logs (append-only JSONL), the daemon's own logs, the
index of projects and local metrics; the **configuration** folder holds the
settings and the permission rules.

- **Windows** (the roaming profile): data in `%APPDATA%\meltemi\data\`,
  configuration in `%APPDATA%\meltemi\config\`
- **macOS**: both in `~/Library/Application Support/meltemi/`
- **Linux**: data in `$XDG_DATA_HOME/meltemi/` (or `~/.local/share/meltemi/`),
  configuration in `$XDG_CONFIG_HOME/meltemi/` (or `~/.config/meltemi/`)

Per-repository method artifacts live in `.meltemi/` inside each repository.

## The local socket

The daemon listens **only** on a local endpoint, never a network port:

- **Unix (macOS/Linux)**: a Unix domain socket with `0700` permissions.
- **Windows**: a named pipe with a user-scoped ACL.

### Remote use over SSH

The daemon never opens a network port: remote use rides the user's own SSH.
Which end runs what:

- **On the daemon's machine (macOS/Linux)**: `meltemi tunnel <user@host>`
  prints the `ssh -R` command that pushes this daemon's socket out to the
  remote host, with a heartbeat so a dead tunnel ends instead of lingering.
  The far end then points `MELTEMI_ENDPOINT` at the forwarded socket.
- **On a macOS/Linux client's machine**: the same socket can be pulled in with
  `ssh -L <local.sock>:<daemon.sock> <host>` — the mirror image, run from the
  other end.
- **When the daemon is on Windows**: its named pipe cannot be forwarded at
  all; `ssh <pc> meltemi bridge` carries the connection over SSH's own stdio,
  and the scriptable verbs (`ssh <pc> meltemi permissions`, …) run on the PC
  itself.
- **From any client, to any of the three**: an endpoint of the form
  `exec:<command>` makes the client launch that command and speak the protocol
  over its stdio, so `MELTEMI_ENDPOINT="exec:ssh <pc> meltemi bridge"` gives the
  desktop app and the terminal shell the same reach the bridge already gave a
  script. The command comes from the user's environment or user-scope
  configuration and never from a repository, and such an endpoint never starts a
  daemon locally.

A client whose `MELTEMI_ENDPOINT` points at a forwarded socket never starts a
daemon there: if the tunnel is down, it says so and names the tunnel. The
served Windows PC, the phone, the `~/.ssh/config` a command endpoint wants and
the waiting settings are in [acceso-remoto.md](en/acceso-remoto.md).

### Isolating an environment

`MELTEMI_ENDPOINT`, `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR` and
`MELTEMI_DAEMON_BIN` isolate a daemon from the user's own — for tests, scripts
or a second setup. A client starts a daemon on demand only at the user's own
endpoint; to let it start one at an isolated endpoint, also set
`MELTEMI_START_HERE=1`. Without it, a client that finds nothing there refuses
at once instead of starting one.

## Windows (git-bash)

Under git-bash / MSYS on Windows, the shell's
POSIX-path translation can *mangle* environment variables whose values look like
Unix paths. A variable such as `MELTEMI_ENDPOINT` set to a pipe or socket path
may be rewritten (e.g. a leading `/` turned into a drive path), so the client
cannot find the daemon.

**The remedy.** Disable the translation for that variable when setting it:

```
MSYS_NO_PATHCONV=1 MELTEMI_ENDPOINT='<endpoint>' meltemi status
```

Or set the variable from a native shell (PowerShell, `cmd`) instead of git-bash.
When in doubt, prefer PowerShell on Windows for setting Meltemi environment
variables.

## Accessibility

The client honors `NO_COLOR`, offers an ASCII-only fallback for box-drawing, and
`--json` on commands that call the daemon gives output that never depends on
terminal styling. See [accessibility](accesibilidad.md).
