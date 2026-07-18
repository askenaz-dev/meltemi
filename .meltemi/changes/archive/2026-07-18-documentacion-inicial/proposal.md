## Why

Un producto sin README es invisible; uno sin quickstart es intocable. Antes del
repo público y del hito v0.1 hace falta la documentación mínima que lleve a
alguien de cero a su primer `propose` en minutos — sin mencionar productos de
terceros en lo público (política del proyecto), con Windows/macOS/Linux como
iguales y con las trampas reales ya descubiertas por QA documentadas.

## What Changes

- **README raíz**: qué es Meltemi (plano de control spec-driven), el lema, la
  arquitectura en un vistazo, estado honesto del proyecto y cómo empezar.
- **Quickstart**: instalar → `meltemi` (TUI con onboarding) → configurar un
  agente → primer `propose` con revisión — en terminal puro, por plataforma.
- **Esqueleto de `docs/`** navegable: arquitectura, método SDD, referencia CLI
  (generable desde la gramática), accesibilidad (NO_COLOR/ASCII/SSH), matriz de
  plataformas (`docs/plataformas.md`, documento transversal del plan).
- **Notas de plataforma reales**: la nota git-bash/MSYS del hallazgo H6 de QA
  (mangling de `MELTEMI_ENDPOINT`), rutas de datos por SO, túnel SSH para uso
  remoto.
- **Tooling de docs**: decisión mínima (markdown en repo primero; sitio en
  `meltemi.dev` cuando el dominio esté operativo).

## Capabilities

### New Capabilities
- `initial-docs`: spec de gobernanza documental — qué documentos existen, qué
  promete cada uno y su criterio de frescura (el quickstart se verifica contra
  binarios reales en cada release).

### Modified Capabilities
- _Ninguna._

## Impact

- `README.md`, `docs/` (estructura), sin código. Idiomas: público en inglés con
  espejo/resumen en español (decisión fina en el design; los artefactos del
  método siguen en español).

## Fuera de alcance

- Sitio web/branding de `meltemi.dev` (post-dominio); videos/tutoriales.
- Referencia del SDK (fase 2).
