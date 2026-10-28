# Tasks — antes-de-anunciar

Vía rápida. Un commit atómico por tarea, con referencia
`(antes-de-anunciar N.M)` y sin trailers de co-autoría. Encender Discussions
es del mantenedor y no es tarea de esta change.

## 1. Canales

- [ ] 1.1 Plantillas `bug.md` y `question.md`, `config.yml` sin issue en blanco
  y con la política de seguridad, y la de propuesta con el orden de la
  constitución; el lint de gobernanza las exige (design D1) — escenarios «Cada
  tipo tiene su plantilla con sus secciones» y «La seguridad no va a un issue
  público»; gates: `governance.rs`
- [ ] 1.2 Insignias de CI, licencia y última release en `README.md` y
  `LEEME.md` (design D2) — gate: pruebas de docs

## 2. El changelog

- [ ] 2.1 `meltemi-release changelog <x.y.z> [--until <fecha>]` (design D3) —
  escenarios «Cada release lista las changes que archiva» y «Las notas de la
  release salen de esa sección»; gates: tests de `meltemi-release`
- [ ] 2.2 `CHANGELOG.md` con la 0.1.0 y la 0.2.0 escritas por la herramienta, y
  la regla y la cadencia en `docs/release.md` (design D4) — gate:
  `meltemi-release notes 0.2.0`

## 3. Docs

- [ ] 3.1 El quickstart desde los instaladores; compilar, en el README y en
  `CONTRIBUTING.md` (design D5) — escenario «El quickstart no pide compilar»;
  gates: pruebas de docs
- [ ] 3.2 `docs/faq.md`, enlazada desde los dos README y el quickstart (design
  D6) — escenario «Cada pregunta tiene su respuesta»; gates: pruebas de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate antes-de-anunciar` limpio y `meltemi verify` con los
  seis escenarios enlazados; entrada en `docs/plan-de-cambios.md`
