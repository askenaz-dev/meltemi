## Context

Sin README no hay proyecto; sin quickstart no hay usuarios. Todo el producto de
Fase 1 existe en este punto del orden; la documentación describe lo real,
incluidas las trampas de plataforma ya descubiertas (QA H6). Política del
proyecto: sin nombres de terceros en lo público salvo datos factuales.

## Goals / Non-Goals

**Goals:** README, quickstart verificado, esqueleto docs/ navegable, notas de
plataforma reales, referencia CLI generada desde la gramática.
**Non-Goals:** sitio de `meltemi.dev` (post-dominio); videos; SDK (fase 2).

## Decisions

### D1 — El quickstart se verifica contra binarios
El quickstart es un guion ejecutable: sus pasos scriptables corren en CI por
plataforma (donde el TTY no es necesario) y el resto se verifica en el pipeline
de release. Documentación que miente = build rojo.

### D2 — Referencia CLI generada, no redactada
La referencia de subcomandos/flags/códigos de salida se genera desde la gramática
y la taxonomía del código (fuente única); regenerarla es parte del pipeline. El
keymap de la TUI se genera del keymap-como-dato.

### D3 — Inglés público con espejo en español
README/quickstart en inglés + `LEEME.md` breve en español enlazado (coherente con
la política de #21). Los artefactos del método siguen en español.

### D4 — Notas de plataforma con las cicatrices reales
Windows: rutas de datos, git-bash/MSYS (H6: mangling de `MELTEMI_ENDPOINT`),
conhost/ASCII. Unix: XDG, permisos del socket. Remoto: túnel SSH del socket
local (nunca puertos). Accesibilidad: NO_COLOR/ASCII/`--json` como ruta
garantizada.

## Risks / Trade-offs

- **Docs que enveजecen** → generación desde fuentes + verificación en CI; lo no
  verificable se marca con fecha.

## Migration Plan

Solo documentos y tooling de generación; `docs/qa/` y research permanecen.

## Open Questions

- Herramienta del sitio cuando llegue el dominio (fuera de esta change).
