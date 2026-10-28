# Tasks — antes-de-anunciar

Vía rápida. Un commit atómico por tarea, con referencia
`(antes-de-anunciar N.M)` y sin trailers de co-autoría. Encender Discussions
es del mantenedor y no es tarea de esta change.

## 1. Canales

- [x] 1.1 Plantillas `bug.md` y `question.md`, `config.yml` sin issue en blanco
  y con la política de seguridad, y la de propuesta con el orden de la
  constitución; el lint de gobernanza las exige (design D1) — escenarios «Cada
  tipo tiene su plantilla con sus secciones» y «La seguridad no va a un issue
  público»; gates: `governance.rs`
  <!-- 2026-10-28: `bug.md` pide versión, plataforma, superficie, agente y la
  salida de diagnóstico, y avisa de `SECURITY.md`; `question.md` apunta antes a
  la FAQ y al quickstart; `config.yml` apaga el issue en blanco y enlaza el
  reporte privado de vulnerabilidades. La de propuesta decía «proposal → specs
  → design → tasks»; ahora el orden de la constitución. `governance.rs` 3/3. -->
- [x] 1.2 Insignias de CI, licencia y última release en `README.md` y
  `LEEME.md` (design D2) — gate: pruebas de docs
  <!-- 2026-10-28: bajo el título de los dos README; la de CI es la del propio
  GitHub Actions y las otras dos de shields.io, solo en los README. Prueba de
  docs nueva que las exige; las de enlaces internos siguen verdes (la de
  licencia apunta a `LICENSE`). Pruebas de docs 18/18. -->

## 2. El changelog

- [x] 2.1 `meltemi-release changelog <x.y.z> [--until <fecha>]` (design D3) —
  escenarios «Cada release lista las changes que archiva» y «Las notas de la
  release salen de esa sección»; gates: tests de `meltemi-release`
  <!-- 2026-10-28: `archived_changes`, `summary_of` y `write_changelog` en la
  biblioteca y el subcomando en el binario, que no toca versión alguna. Tests
  en `tests/changelog.rs`: dos secciones escritas sobre un archivo de tres
  changes, sin repetir y en orden; reescribir da el mismo archivo; las notas
  las lee el binario real (`notes`), no una copia de su regla; el resumen sale
  del inicio en negrita de la primera viñeta, de su primera frase o de la de
  «Why»; una fecha mal formada es error de uso y no escribe nada. Suite de
  `meltemi-release` verde; clippy `-D warnings` limpio. -->
- [x] 2.2 `CHANGELOG.md` con la 0.1.0 y la 0.2.0 escritas por la herramienta, y
  la regla y la cadencia en `docs/release.md` (design D4) — gate:
  `meltemi-release notes 0.2.0`
  <!-- 2026-10-28: `changelog 0.1.0 --until 2026-07-26` lista 42 changes y
  `changelog 0.2.0 --until 2026-10-23` lista 20; `notes 0.2.0` devuelve su
  sección. Las archivadas desde el 2026-10-24 quedan para la sección de la
  release siguiente, que escribe el mantenedor con el mismo comando antes de
  `set-version`. `docs/release.md` gana la regla, la cadencia, el paso y por
  qué la 0.1.1 no tiene sección, en inglés y en el resumen en español. -->

## 3. Docs

- [ ] 3.1 El quickstart desde los instaladores; compilar, en el README y en
  `CONTRIBUTING.md` (design D5) — escenario «El quickstart no pide compilar»;
  gates: pruebas de docs
- [ ] 3.2 `docs/faq.md`, enlazada desde los dos README y el quickstart (design
  D6) — escenario «Cada pregunta tiene su respuesta»; gates: pruebas de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate antes-de-anunciar` limpio y `meltemi verify` con los
  seis escenarios enlazados; entrada en `docs/plan-de-cambios.md`
