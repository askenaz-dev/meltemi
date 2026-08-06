# Tasks — artefactos-de-cada-push

## 1. La ruta de integración

- [x] 1.1 Añadir `scripts/unsigned-build-notice.txt` con el aviso que viaja a
  los dos destinos (resumen del run y artefacto): qué garantías no acompañan al
  build —firma, atestación, publicación—, por qué la clave nunca toca CI
  (`docs/release.md`, «Key custody») y la URL de última release como vía de
  instalación (design D4)
- [x] 1.2 Añadir `.github/workflows/build.yml`: disparado por push a `main` y
  `workflow_dispatch`, `permissions: contents: read` en cabecera, un job por
  plataforma (`ubuntu-latest`, `macos-latest`, `windows-latest`) que construye
  en release, arma el archivo del núcleo con `meltemi`, `meltemid` y los dos
  adaptadores ACP, corre `tauri build`, mide los tres presupuestos con los
  valores de `release.yml`, normaliza los nombres de instalador, calcula
  `SHA256SUMS`, escribe el aviso al artefacto y al resumen del run, y sube
  `meltemi-unsigned-<SO>-<sha-corto>` con `retention-days: 7` (design D1, D3,
  D5). Sin heredoc en ningún `run: |`

## 2. Lo que la change afirma, pineado

- [x] 2.1 Añadir a `core/meltemid/tests/release.rs` la mitad afirmativa
  —disparador en `main`, las tres plataformas, los tres presupuestos con
  **idéntico valor** en los dos workflows, los dos adaptadores en los tres
  archivos del núcleo, el nombre con `unsigned` y SHA corto, la retención
  declarada— cubriendo «Push a main deja el build descargable», «Nombre del
  artefacto declara commit y build», «Retención acotada y declarada» y
  «Presupuesto excedido falla el build de integración» (design D6)
- [x] 2.2 Añadir la mitad negativa: `build.yml` sin `gh release`, sin
  `actions/attest`, sin `SHA256SUMS.minisig` y sin `contents: write`; y
  `release.yml` con su job `release` todavía condicionado a
  `startsWith(github.ref, 'refs/tags/')` — cubriendo «El build de integración no
  crea release», «El camino de release sigue siendo solo de tag» y «La última
  release firmada sigue resolviendo» (design D6)

## 3. Documentación

- [ ] 3.1 Añadir a `docs/release.md` la sección que distingue las dos rutas
  —qué produce cada una, qué garantías lleva cada una, y que un artefacto de
  ejecución no es distribución— con su resumen en el bloque en español, y cubrir
  «Aviso de build sin firmar donde se descarga» y «El artefacto lleva su propio
  aviso» con las aserciones sobre el aviso y la doc en
  `core/meltemid/tests/release.rs`

## 4. Verificación

- [ ] 4.1 Gates locales — `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo deny check` y `meltemi validate
  artefactos-de-cada-push` limpio— más `meltemi verify` con su cobertura; y
  marcar como verificación documentada, pendiente del primer push a `main` tras
  el merge, lo que ningún test local puede probar: que los tres jobs completan
  en runner real, que el DMG de macOS sale por esta ruta igual que por la de
  tag, y el tiempo de pared por plataforma (design D8)
