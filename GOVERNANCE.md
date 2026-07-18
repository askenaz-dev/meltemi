<!-- SPDX-License-Identifier: Apache-2.0 -->
# Governance

Meltemi is an open-source project (Apache-2.0). This document describes how the
project is governed **today**, honestly, and the declared path to sharing that
governance as the community grows. It is not aspirational: it reflects the
current reality.

_Resumen en español al final._

## Governance model

Meltemi currently follows a **founding-maintainer** model. The founding
maintainer (Guillmar Ortiz) holds final decision authority over the direction of
the project, the acceptance of changes, and the foundational documents.

Decisions are made in the open. Substantive work — every feature — flows through
the spec-driven change process described in [CONTRIBUTING.md](CONTRIBUTING.md):
a proposal is reviewed before code is written, and the reviewed scenarios are the
definition of "done". Governance decisions themselves are recorded as changes so
they are traceable.

The project's non-negotiable principles live in
[`.meltemi/constitution.md`](.meltemi/constitution.md). They bind the maintainer
too.

## Amendment ratification

The constitution and the project's direction (`rumbo`) are foundational. Any
modification to them requires an **approved change proposal**, following the
two-stage method bootstrap (`method-bootstrap`): the proposal is authored,
reviewed against the existing constitution, and only then ratified by the
founding maintainer. A ratified amendment records its date and author in the
document header (as the current constitution does: "RATIFICADA v1.0").

No amendment is silent: the change and its ratification are part of the git
history.

## Becoming a maintainer

The project intends to grow beyond a single maintainer. This is a declared path,
not a promise of a seat. A contributor is considered for maintainer when they
have, over a sustained period:

- landed several non-trivial changes through the full spec-driven process;
- shown sound judgment in reviews (of both specs and code);
- upheld the constitution's principles, especially quality (clippy/fmt/tests
  green on the three platforms) and the fair-play and no-hidden-telemetry rules.

The founding maintainer proposes and ratifies the addition of a new maintainer
as a governance change. As the maintainer group grows, this document will be
amended to describe shared decision-making.

## Summary (español)

Meltemi se gobierna hoy con un **mantenedor fundador** con decisión final; toda
funcionalidad entra por el proceso spec-driven (ver `CONTRIBUTING.md`). Las
enmiendas a la constitución o al rumbo exigen una propuesta de cambio aprobada y
ratificada (bootstrap del método). El camino a mantenedor es declarado
(contribuciones sostenidas, buen criterio en revisiones, respeto a los
principios), no una promesa de puesto.
