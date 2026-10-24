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

## Motion

Movement says that something is working or loading, and never anything else.
The terminal has exactly one: the header's spinner, drawn only while a session
is running a turn and with an ASCII twin of its own (`-\|/`) wherever the
ASCII fallback applies. At rest nothing is animated and the pulse adds no
redraw. The time a working session has been running is shown in its detail;
a session that lives without working shows no time, because nothing would keep
it current.

The desktop surface honors `prefers-reduced-motion`: each animation is
withdrawn rather than frozen. A skeleton's bars stand still and its "loading…"
word becomes visible, the brand mark appears already drawn, and the ambient
ring gives way to an accent frame. Clocks keep counting — a figure that changes
is not motion — and the time beside a working turn is hidden from the
transcript's live region, so a screen reader hears "working…" once instead of
a second every second. The full vocabulary, with where each movement is allowed
and where it never is, lives in [the design system](ux/design-system.md#motion).

## No hangs without a TTY

A bare invocation without a TTY is a usage error that points to `meltemi help` —
the client never waits on interactive input when there is no terminal. With a
subcommand it always runs scriptably, one-shot.

The same holds for the permissions an agent asks for. They are asked only when
stdin and stderr are a terminal; otherwise each one is denied at once with the
agent's own rejection and a notice on stderr, and nothing waits for a key. When
they are asked, the question is plain text on stderr — no color, no box drawing,
no full-screen mode — and every answer is a single key: a digit for one of the
agent's options, `d` or Esc to deny, Ctrl+C to stop. Enter never decides. Keys
typed before a question could be read are set aside rather than taken as an
answer, and once it can be read there is no time limit on the human: the only
deadline is the daemon's own `[permissions] wait`. stdout is untouched, so
`--json` stays one object.

## Remote and constrained terminals

For remote use, tunnel the local socket over SSH (see
[platform notes](plataformas.md)); the `--json` path keeps working over any
transport and in the most constrained terminals.
