<!-- SPDX-License-Identifier: Apache-2.0 -->
# Security Policy

Meltemi is a small open-source project. This policy is honest about what we can
promise: a private reporting channel, a clear scope, and realistic response
times — no paid program, no guaranteed SLAs.

_Resumen en español al final._

## Reporting a Vulnerability

**Please do not open a public issue for a security vulnerability.**

Report privately using GitHub's **private vulnerability reporting** on this
repository (Security tab → "Report a vulnerability"). If that is unavailable to
you, contact the founding maintainer directly through their GitHub profile and
ask for a private channel before sharing details.

Include, as far as you can: the affected component and version, a description of
the issue, reproduction steps, and the impact you observed.

## Scope

Meltemi's threat model is described in the foundational document
(`meltemi.md`, §8). In scope:

- the headless daemon `meltemid` and its local transport (Unix socket / Windows
  named pipe) — the daemon **never opens a network port**;
- the permission proxy and the deny-by-default posture (no client → denied);
- the worktree isolation, checkpoints, and per-task commit machinery;
- handling of agent credentials — Meltemi **must never read, store, or reuse
  them** (fair play); a violation of this is a security issue.

Out of scope: vulnerabilities in third-party agent binaries themselves, in the
user's own git hooks, or in the operating system.

## Response

As a small project we aim to:

- acknowledge a report within a few days;
- assess and reproduce it, keeping you informed;
- fix confirmed issues in a reasonable time frame proportional to severity, and
  credit reporters who wish to be credited.

These are honest intentions of a community project, not contractual guarantees.

## Summary (español)

No abras un issue público para una vulnerabilidad. Repórtala en privado por el
reporte privado de vulnerabilidades de GitHub (o contacta al mantenedor por su
perfil). Alcance: el daemon local sin red, el proxy de permisos, el aislamiento
por worktrees y el juego limpio con credenciales de agentes. Respondemos con
prontitud razonable; sin SLAs ni programa pagado.
