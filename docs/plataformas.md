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

To drive Meltemi on a remote machine, forward the **local socket** over an SSH
tunnel; the daemon itself never opens a network port. Forward the Unix socket
(`ssh -L`), then point the client at the forwarded endpoint.

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
