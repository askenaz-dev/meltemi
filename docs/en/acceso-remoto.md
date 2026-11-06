<!-- SPDX-License-Identifier: Apache-2.0 -->
# Remote access

[Español](../acceso-remoto.md)

Supervise and direct an agent from another machine over your SSH connection. `meltemid` listens
only on a local socket or named pipe; Meltemi opens no network port and provides no hosted relay.
Configure and test SSH before using it from the app.

## The boundary: a live connection

Monitoring, approval and direction need an established connection to the daemon. For the desktop
or TUI, that connection can be a forwarded socket or the standard streams of
`ssh … meltemi bridge`. You can also run individual commands directly over SSH: they execute on
the remote PC and use its local endpoint.

This privacy boundary keeps the daemon off network ports. Without a connection there is no
control or daemon push. An optional notice from a connected client is different and is configured
later in this guide. The same permission policy applies from every surface.

## From a phone today

### Approve and follow without the TUI

These commands need no socket forward, bridge or TUI. Use a terminal app with SSH on the phone;
each command executes on the daemon's machine:

```
ssh <tu-pc> meltemi permissions
ssh <tu-pc> meltemi approve perm-3
ssh <tu-pc> meltemi watch <sesión>
```

`permissions` lists waiting requests and the options their agents offered. `approve <id>` selects
a one-time approval when exactly one was offered; `deny <id>` selects rejection, or cancels when
none was offered. `--option <id>` chooses a particular option. Each decision concerns one request,
never a batch. The first decision wins; deciding an already resolved request exits with `12`.

The full call is in `ssh <tu-pc> meltemi log <sesión>`. `watch` follows the log until the session
ends; Ctrl+C stops following without cancelling. Use `meltemi cancel <sesión>` to cancel it.

A connected `watch` counts as a client. Under `while-connected`, it lets a request wait while
you open another terminal to answer it.

### The TUI over SSH

```
ssh -t <tu-pc> meltemi
```

The TUI needs a terminal at least **80×24**. On a phone you may need landscape orientation or a
smaller font. `MELTEMI_BELL=1` enables the bell; the terminal app decides whether to turn it into a
notification. There is no push without a connection.

## Connect the desktop or TUI to your PC

An `exec:` endpoint launches a command on your machine and transports the connection over its
standard input and output:

```
MELTEMI_ENDPOINT="exec:ssh mi-pc meltemi bridge" meltemi status
```

For the desktop app, put the same endpoint in your **user** configuration:

```toml
# <config-dir>/config.toml
[client]
endpoint = "exec:ssh mi-pc meltemi bridge"
```

`<config-dir>` is the configuration folder shown by `meltemi doctor`; see
[platforms](../plataformas.md#data-paths). The endpoint is never accepted from a repository.
An `exec:` endpoint does not start a local daemon, even with `MELTEMI_START_HERE=1`.
The app's header identifies the SSH connection.

### Prepare SSH

Set up the alias in your `~/.ssh/config`:

```
Host mi-pc
    HostName 203.0.113.10
    User tu-usuario
    ServerAliveInterval 30
    ServerAliveCountMax 3
```

First test the connection in a terminal and accept the host fingerprint there:

```
ssh mi-pc true
ssh-add ~/.ssh/id_ed25519
```

The `exec:` transport uses its streams for the protocol and cannot also use them to ask for a
passphrase or new host fingerprint. Meltemi adds no options, manages no keys and does not edit
`known_hosts`. If you need pipes or redirections, put them in a script and name that script as the
endpoint: the endpoint command is not executed through a shell.

## The bridge: the final hop

`meltemi bridge` connects the local endpoint on the machine where it runs to its standard streams.
It is the final hop of the connection and also reaches a Windows named pipe:

```
ssh <tu-pc> meltemi bridge
```

If the daemon is not listening, the bridge fails and gives the next step. Start the daemon on the
PC with the desktop app or `meltemi status` before connecting from elsewhere.

## Unix sockets and the tunnel helper

With a macOS/Linux daemon, `meltemi tunnel <user@host>` prints the reverse `ssh -R` forward,
equivalent configuration and endpoint the other end should use. `--exec` runs your SSH visibly.
You can also forward locally from a macOS/Linux client:

```
ssh -L <local.sock>:<daemon.sock> <host>
```

The helper includes heartbeats and `ExitOnForwardFailure=yes` to detect a broken connection.
A client pointed at a forwarded socket does not start a different daemon when the tunnel fails.

A Windows named pipe cannot be forwarded like a Unix socket; use `meltemi bridge`.
Windows can be a client of a Unix daemon. In Git Bash, see the `MSYS_NO_PATHCONV` setting in
[platforms](../plataformas.md#windows-git-bash).

## A Windows PC as the remote host

Enable OpenSSH Server in an administrator PowerShell on the PC:

```
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
Start-Service sshd
Set-Service -Name sshd -StartupType Automatic
```

With `meltemi` on the remote `PATH`, use `ssh <tu-pc> meltemi bridge`, individual commands, or
`ssh -t <tu-pc> meltemi`. That machine's owner manages SSH, user access and the firewall.

**Measured** by the product's tests: the bridge over a named pipe and commands without a terminal.
**Not measured**: the path through an actual Windows `sshd`, or starting the daemon from that SSH
session. Test your complete route before relying on it; local tests do not replace that check.
Historical evidence is in the [verification record](../qa/2026-09-25-acceso-por-tunel-smoke.md).

## When both machines are away from home

Use your own VPN or an SSH rendezvous host you administer. With a reverse forward, the PC dials
out and the client also connects out to that host; the final hop runs `meltemi bridge` inside the PC.

| PC | Client | Route |
| --- | --- | --- |
| Home | Away | client → rendezvous → reverse tunnel → PC |
| Away | Home | the PC keeps the same outbound route |
| Away | Away | both reach the same rendezvous |
| Home | Home | the same route or a direct SSH connection |

A stopped rendezvous means no remote access; local work remains available. This is your own
infrastructure (BYO-network), not something bundled with Meltemi. The project's local tests need
no account or network. A VPN provides the route for SSH, not new daemon permissions.

## Identity from your own mesh

Meltemi has no accounts. It can optionally read the identity and presence already known by your
Tailscale/Headscale client. Configure this only in your **user** file:

```toml
# <config-dir>/config.toml
[identity]
mesh = "tailscale"
login-server = "https://headscale.mi-casa"
command = "tailscale"
```

An `[identity]` table in the project is ignored with a diagnostic. `meltemi identity` makes a
read-only query to the configured client (`status --json`), with a timeout. The daemon never runs
`up`, `login` or `logout`. Presence is not control: an online machine does not prove that it runs
`meltemid`; connect to or test its endpoint.

- `not_configured`: add the setting if you want this feature.
- `client_missing`: check the binary and configured path.
- `not_linked`: sign in using your own mesh client.

`meltemi identity-link` composes the login gesture; the daemon never performs that login.
With `meltemi identity-link --exec`, the CLI runs your client with inherited input and output so
you can complete its own sign-in. The desktop app can copy the gesture. In the TUI use `:identity`
or `:identity-link`; in the desktop app open **Settings → Connections**.

Interoperability was checked against the published Tailscale `v1.102.5` type definitions; a field
check with an installed, linked client remains pending. It is not a guarantee about another
client version.

## Named machines

Save endpoints in your **user** configuration:

```toml
# <config-dir>/config.toml
[[client.machines]]
name = "esta"
endpoint = "local"

[[client.machines]]
name = "la-torre"
endpoint = "exec:ssh la-torre meltemi bridge"
```

The list contains no key material; that stays in your `~/.ssh/config`. An unsupported key field
rejects the entire machine instead of being silently accepted.

`meltemi machines` lists machines and does not switch another client's destination. In the TUI,
`:machines` lists them and `:machine la-torre` selects one. In the desktop app use
**Settings → Connections**, **Test** and **Use this one**. Switching reconnects; it first checks for
unsaved work. **Test** starts nothing: it connects, greets and closes. Presence is read on demand,
never polled, and does not say a daemon is running. Sessions are not aggregated across machines;
you see those of the active machine.

## The mobile companion

The Android companion can monitor, approve, review and direct; it is not a code or specification
editor. It exists in the product source, but the published release does not distribute an APK.
The [build guide](../../mobile/README.md) describes building it for developers. No iOS distribution
is published.

The phone uses its own SSH client to reach `meltemi bridge`. The app generates its own key and
shows the public part for you to add to the server's `authorized_keys`. Revoke access by removing
that line. The private key is encrypted in app storage; the system keystore protects the material
used to encrypt it. A changed host key refuses the connection and is reported explicitly.

The app does not keep the socket when backgrounded. On returning it reconnects and refreshes
before presenting state as current. There is no Meltemi relay or offline daemon notification.

## The optional waiting notice

A connected desktop or TUI client can run your command when something begins waiting. It is off
by default. The command is read only from your user configuration:

```toml
# <config-dir>/config.toml
[notices]
on-waiting = "curl -s -d Meltemi https://ntfy.mi-casa/mi-topic-largo-y-privado"
```

Replace the example with your own service and a command installed on your machine. The daemon
does not read or run it. The command receives only these event variables:

| Variable | Content |
| --- | --- |
| `MELTEMI_WAITING` | `permission`, `instruction`, `gate` or `test` |
| `MELTEMI_SESSION` | Opaque session id |
| `MELTEMI_MACHINE` | Client machine name |
| `MELTEMI_AT` | RFC 3339 time in UTC |
| `MELTEMI_COUNT` | Number of waiting items |

The event includes no project name or path, request text, tool name or session title. It sends a
notice on entering a waiting state, with at least thirty seconds between real notices; resolving
the wait sends no follow-up notice.

Test it from **Settings → Tools** or with `notice-test` in the TUI palette (also `aviso-prueba`).
The test uses `MELTEMI_WAITING=test` and ignores the minimum interval. A failed command does not
stop work, but you will receive no notice. You manage its service, access and credentials;
Meltemi does not verify that the destination delivers a notification.

## How long a request waits

These settings go in user `config.toml` or the project's `.meltemi/config.toml`; the project wins:

| Setting | Default | Effect |
|---|---|---|
| `[permissions] wait` | `"while-connected"` | Interactive wait while a client is connected, or a number of seconds. Expiry denies the request. |
| `[permissions] implement-wait` | `30` | Seconds to wait in `implement` turns. |
| `[permissions] no-client-grace` | `30` | Seconds tolerated without a client before pending requests are denied. |
| `[sessions] idle-timeout` | `900` | Seconds to keep an agent waiting for another instruction; `0` does not wait. |
| `[sessions] max-idle` | `3` | Maximum waiting sessions; reaching it closes the oldest wait. |
| `[sessions] lever-wait` | `30` | Seconds for an agent to confirm a requested model or effort. |

A connected client counts for `while-connected`, including the desktop, TUI or `meltemi watch`
over SSH. Connecting grants no permission automatically: a person or a rule must decide.
