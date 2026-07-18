<!-- SPDX-License-Identifier: Apache-2.0 -->
# Platform notes

Meltemi supports Windows 10 1809+ / Windows 11, macOS 13+, and mainstream glibc
Linux. CI runs on all three; **Windows is first class, not a later port**. This
page collects the real, discovered gotchas.

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
