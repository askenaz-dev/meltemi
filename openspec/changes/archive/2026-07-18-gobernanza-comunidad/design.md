## Context

Antes del repo público: reglas escritas de gobernanza, contribución (vía specs),
conducta, seguridad y CLA (§9.3). `method-bootstrap` ya formaliza la ratificación
de enmiendas; esto lo extiende a documentos de comunidad. Cero código.

## Goals / Non-Goals

**Goals:** GOVERNANCE/CONTRIBUTING/CODE_OF_CONDUCT/SECURITY en la raíz, texto del
CLA, plantillas `.github/`, y una spec de gobernanza que fije qué existe y qué
promete (verificable por lint de presencia+secciones).
**Non-Goals:** tooling de firma del CLA (decisión del mantenedor); fundación u
organización legal; bug bounty.

## Decisions

### D1 — Idioma: inglés público, espejo breve en español
Los documentos comunitarios se publican en inglés (audiencia global) con un
resumen en español enlazado; los artefactos del método siguen en español. La
política queda escrita en CONTRIBUTING.

### D2 — Contribución es spec-driven, sin excepciones de fondo
CONTRIBUTING exige: toda feature entra como propuesta de cambio con artefactos;
la plantilla de PR incluye la checklist (change enlazada, clippy/fmt/tests 3
plataformas, SPDX, convención de commits **sin trailers de co-autoría**). Fixes
triviales documentados como vía corta explícita (typo/docs) para no espantar.

### D3 — Gobernanza honesta del momento
GOVERNANCE declara la realidad: mantenedor fundador con decisión final +
ratificación de enmiendas (formaliza `method-bootstrap`), y el camino declarado
para sumar mantenedores (criterios, no promesas).

### D4 — CoC y SECURITY estándar adoptados
Código de conducta: estándar ampliamente adoptado, adaptado con contacto real.
SECURITY: divulgación responsable, alcance (daemon local, sin red — el modelo de
amenaza ya escrito en meltemi.md §8), tiempos de respuesta honestos de un
proyecto pequeño.

### D5 — CLA acotado con hueco de firma
El texto del CLA (§9.3: contribución bajo Apache-2.0, sin cesión de copyright)
queda en el repo; el mecanismo de firma se integra cuando el mantenedor lo elija
(pendiente registrado). La spec exige el texto, no el tooling.

## Risks / Trade-offs

- **Documentos-promesa que nadie cumple** → la spec exige contenido mínimo
  verificable por lint (secciones presentes), no aspiraciones.

## Migration Plan

Solo documentos nuevos; nada existente cambia.

## Open Questions

- Contacto de seguridad definitivo (correo del mantenedor vs alias del dominio
  cuando `meltemi.dev` esté operativo).
