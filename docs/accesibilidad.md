<!-- SPDX-License-Identifier: Apache-2.0 -->
# Accessibility

Meltemi's terminal surface is designed to be usable without color, without
Unicode box-drawing, and without a TTY at all. There is always a guaranteed
accessible path.

## The `--json` path

Every RPC-backed subcommand accepts `--json` and emits machine-readable output
that never depends on terminal styling. This is the guaranteed path for screen
readers, scripts, and CI:

```
meltemi --json status
meltemi --json sessions
```

A subcommand with `--json` is always scriptable and one-shot; it never falls back
to the interactive interface.

## Color

The client honors [`NO_COLOR`](https://no-color.org): when set, no ANSI color is
emitted. Color is only ever decorative — meaning is carried by text, never by
color alone.

## ASCII fallback

Where the interactive interface uses box-drawing characters, an ASCII-only
fallback is available for terminals or fonts that cannot render them, so the
layout stays legible.

## No hangs without a TTY

A bare invocation without a TTY is a usage error that points to `meltemi help` —
the client never waits on interactive input when there is no terminal. With a
subcommand it always runs scriptably, one-shot.

## Remote and constrained terminals

For remote use, tunnel the local socket over SSH (see
[platform notes](plataformas.md)); the `--json` path keeps working over any
transport and in the most constrained terminals.
