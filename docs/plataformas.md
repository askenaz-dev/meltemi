<!-- SPDX-License-Identifier: Apache-2.0 -->
# Platform notes

Meltemi supports Windows 10 1809+ / Windows 11, macOS 13+, and mainstream glibc
Linux. CI runs on all three; **Windows is first class, not a later port**. This
page collects the real, discovered gotchas.

## The desktop app on Linux: deb only, on purpose

The core — daemon and terminal client — runs on any mainstream glibc
distribution from the release archive. The desktop app is published **only as a
`.deb`**, which declares `libwebkit2gtk-4.1-0` and `libgtk-3-0` so the package
manager installs the engine it uses.

There is no AppImage, and the reason is the product's own promise rather than an
oversight: an AppImage is self-contained by construction, so it would have to
carry WebKitGTK and its whole closure. Built once to check, it measured
78,678,520 bytes — five times the 15 MB installer budget, which exists precisely
to encode "no bundled browser engine" (`instaladores-linux-sin-webview`). There is
no configuration that shrinks it: the pinned bundler copies the WebKit helper
processes and runs `linuxdeploy` with the GTK plugin in code, not in options.

So on Fedora, RHEL, openSUSE, Arch or NixOS there is no desktop installer yet.
Build it from source — the readme documents the Tauri invocation and the
development packages — or use the terminal client, which needs none of this. An
`.rpm` is the next step: the format can declare the engine as a dependency the
same way the `.deb` does, and it only waits on verifying those distributions'
package names rather than guessing them.

## Data paths

The daemon stores session logs (append-only JSONL) and local metrics under the
OS-conventional per-user data directory:

- **Windows**: `%LOCALAPPDATA%\meltemi\`
- **macOS**: `~/Library/Application Support/meltemi/`
- **Linux**: `$XDG_DATA_HOME/meltemi/` (or `~/.local/share/meltemi/`)

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

A client whose `MELTEMI_ENDPOINT` points at a forwarded socket never starts a
daemon there: if the tunnel is down, it says so and names the tunnel. The
served Windows PC, the phone and the waiting settings are in
[acceso-remoto.md](acceso-remoto.md).

### Isolating an environment

`MELTEMI_ENDPOINT`, `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR` and
`MELTEMI_DAEMON_BIN` isolate a daemon from the user's own — for tests, scripts
or a second setup. A client starts a daemon on demand only at the user's own
endpoint; to let it start one at an isolated endpoint, also set
`MELTEMI_START_HERE=1`. Without it, a client that finds nothing there refuses
at once instead of starting one.

## Windows (git-bash) {#windows-git-bash}

**The gotcha (QA finding H6).** Under git-bash / MSYS on Windows, the shell's
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
`--json` on every subcommand gives a machine-readable path that never depends on
terminal styling. See [accessibility](accesibilidad.md).
