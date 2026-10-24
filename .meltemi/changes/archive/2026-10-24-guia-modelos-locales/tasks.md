# Tasks — guia-modelos-locales

## 1. La guía y lo que la rodea

- [x] 1.1 `docs/local-models.md`: las tres recetas verificadas en forma con
  OpenCode 1.18.32 el 2026-09-25 —el archivo de OpenCode y el perfil de Meltemi
  de cada una, la alternativa del `opencode.json` del proyecto, el modelo por el
  archivo o por la palanca, qué se mide, la advertencia sobre referencias dentro
  de un texto— y el lint que parsea sus bloques y rehúsa claves en claro (design
  D1, D2, D3, D4, D8) — escenarios «Tres recetas con su perfil», «Ninguna clave
  en claro», «La guía declara con qué se verificó» — gates: suite de `meltemid`
  (el test nuevo y la guía de agentes)

- [x] 1.2 La pierna «proveedor local» en `docs/conformidad-manual.md`, pendiente
  y con lo verificado en forma (design D5) — escenarios «La pierna declara sus
  criterios y lo que registra», «Sin corrida en vivo la pierna queda pendiente»
  — gates: tests de docs

- [x] 1.3 La afirmación sobre Aider corregida en `docs/plan-de-cambios.md` y en
  la propuesta de `motor-propio-byok` (design D6) — escenario «Lo que no se
  lanza no se promete» — gates: tests de docs

- [x] 1.4 Enlaces: la sección OpenCode de `docs/agentes.md`, el README y el
  LEEME, sin nombres de terceros donde no se pueden nombrar (design D7) —
  gates: tests de docs, guía de agentes
  <!-- 2026-10-24: esta tarea también enlazó la página de agentes del sitio en
  sus dos idiomas y llevaba el escenario «La página de agentes enlaza la guía de
  modelos propios», con el lint del sitio como gate. Se hizo entonces en `site/`.
  El sitio se mudó a meltemi-web y ese requisito, con su test, pasa a su change
  `agents-page-own-model`. Antes de archivar se retiró de aquí el delta de
  `product-site`, para que el archivo no recreara esa spec en este repositorio. -->

## 2. Cierre

- [x] 2.1 `meltemi validate guia-modelos-locales` limpio, `meltemi verify
  guia-modelos-locales` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  guia-modelos-locales` y `cargo clean` del taller
