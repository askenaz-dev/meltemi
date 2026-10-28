<!-- SPDX-License-Identifier: Apache-2.0 -->
# Questions and troubleshooting

What people ask first, and what took the longest to find out. Each answer
links the guide that has the detail. Something not here? Open an issue with
the question template.

## Where do I start when something is wrong?

With what Meltemi can see:

```
meltemi version      # the client
meltemi status       # the daemon: version, uptime, sessions
meltemi fleet        # which agents are ready, and what is missing
```

If the client and the daemon report different versions, an old daemon is still
running: `meltemi stop`, and the next command starts the installed one.

## The fleet says my agent is not launchable

On Windows, an agent installed through npm leaves small launcher scripts next
to each other: a `.cmd` the system can run and a `.ps1` it cannot run directly.
When only the `.ps1` is there, the fleet says `not_launchable` instead of
handing the launcher a path that will fail. Reinstall the agent globally so
its `.cmd` shim exists. See the [agents guide](agentes.md#windows).

## The fleet says a layer is missing

Some agents are two layers: the official CLI and an adapter that speaks the
Agent Client Protocol. The row names the missing layer and the exact command
that installs it. Meltemi's own adapters travel in its installers, beside the
daemon; if one of them is missing, repair the Meltemi installation. See the
[agents guide](agentes.md).

## The command line denies every permission

A command run without a terminal — from a script, a pipe or CI — has nobody to
ask, so every request is denied with the agent's own rejection, and stderr
says so. In a terminal you are asked. To grant without a human, write a rule
in `.meltemi/permissions.toml` of the project or in your global
`permissions.toml`; pressing `r` at a question writes the most specific rule
for you. See [free sessions](sesion-libre.md#terminal).

## I keep being asked for the same command

Press `r` when it is asked: Meltemi writes a rule for that exact command or
path and stops asking from the next session. A rule that allows a command does
not allow another command chained, piped, redirected or substituted after it
(`;`, `&&`, `|`, `>`, `$(…)`), so those still ask. A deny rule reaches them all.

## The agent edited files without asking me

Meltemi decides only what an agent asks. An agent that edits without asking
over the protocol is recorded in the session log but not governed. The
[agents guide](agentes.md) says, per agent, what the last verification run
measured; the session log shows every tool call either way.

## The session ended without saying anything

A turn in which the agent neither wrote nor used a tool is reported as an
empty turn, and the command line exits with code `15`, instead of passing for
a success. It usually means the agent's own CLI could not work: signed out,
out of quota, or a model it does not offer. Run the agent's own CLI once, by
hand, in the same project. The exit codes are listed in the
[CLI reference](referencia-cli.md#exit-codes).

## How do I use Meltemi from another machine?

The daemon never opens a network port. Remote use rides your own SSH: a tunnel
to the daemon's local socket, or `meltemi bridge` run over `ssh`, which also
reaches a Windows daemon, whose named pipe no SSH forward can reach. The
[remote access guide](acceso-remoto.md) has both, and the commands to approve
and follow a session from a phone.

## The installer warns that it is not signed

The MSI and the DMG carry no platform signature yet: that needs purchased
certificates. Check the download first —
`sha256sum --check --ignore-missing SHA256SUMS` and
`gh attestation verify <file> --repo askenaz-dev/meltemi` (see
[releases and verification](release.md)) — and then:

- **Windows**: SmartScreen says it protected your PC. Choose *More info*, then
  *Run anyway*.
- **macOS**: Gatekeeper refuses to open it. Control-click the app and choose
  *Open*, or open it from *System Settings → Privacy & Security → Open Anyway*.

The Debian package and the core archives are not affected.

## Where are my data and my logs?

In the per-user folders of your operating system: session logs (append-only
JSONL), the daemon's own logs and the index of projects in the data folder;
settings and permission rules in the configuration folder. The paths of each
platform are in the [platform notes](plataformas.md#data-paths). Nothing leaves
your machine: there is no telemetry.

## What does Meltemi send over the network?

Nothing of its own. The daemon opens no port and makes no request; agents use
the network their own CLIs use, under the permissions you grant. See the
privacy section of the [README](../README.md#privacy-and-posture).
