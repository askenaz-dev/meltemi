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

- [x] 3.1 Añadir a `docs/release.md` la sección que distingue las dos rutas
  —qué produce cada una, qué garantías lleva cada una, y que un artefacto de
  ejecución no es distribución— con su resumen en el bloque en español, y cubrir
  «Aviso de build sin firmar donde se descarga» y «El artefacto lleva su propio
  aviso» con las aserciones sobre el aviso y la doc en
  `core/meltemid/tests/release.rs`

## 4. Verificación

- [x] 4.1 Gates locales — `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo deny check` y `meltemi validate
  artefactos-de-cada-push` limpio— más `meltemi verify` con su cobertura; y
  marcar como verificación documentada, pendiente del primer push a `main` tras
  el merge, lo que ningún test local puede probar: que los tres jobs completan
  en runner real, que el DMG de macOS sale por esta ruta igual que por la de
  tag, y el tiempo de pared por plataforma (design D8)
  <!-- 2026-08-06: gates verdes — `cargo fmt --all --check` limpio; `cargo
  clippy --workspace --all-targets -- -D warnings` sin avisos; `cargo test
  --workspace` **848 tests / 87 suites, 0 fallos** (incluye `release.rs` 19/19 y
  `site.rs` 12/12); `cargo deny check` **advisories ok, bans ok, licenses ok,
  sources ok**; `meltemi validate artefactos-de-cada-push` limpio; `meltemi
  verify artefactos-de-cada-push` **9/9 (complete)**, los nueve `linked` — ningún
  escenario quedó sin ancla, así que no hizo falta `sdd/verify-mark`. Se usó el
  binario de depuración (`target/debug/meltemi.exe`): el de release está tomado
  por procesos del mantenedor y no se mata nada.

  Además de los tests, se comprobó localmente lo que un archivo YAML sí permite
  comprobar sin runner: que `build.yml` parsea, que los catorce bloques `run:`
  pasan `bash -n`, que el lint de bloques (`every_workflow_script_stays_inside_
  its_block`) sigue verde con tres workflows, y que la secuencia
  presupuesto→normalización→checksums→aviso produce el `dist/` esperado en un
  sandbox con un DMG falso. Las tres guardas negativas se verificaron por
  mutación, no por observación: quitar el gate de tag del job `release` pone roja
  la segunda; añadir `contents: write` y un `gh release create` a `build.yml`
  pone roja la primera; cambiar un presupuesto en un solo archivo pone roja la de
  correspondencia nombrando los dos números y los dos archivos.

  **Verificación documentada, pendiente del primer push a `main` tras el merge**
  (design D8) — lo que ningún test local puede probar: (1) que los tres jobs de
  `build.yml` completan en runner real y dejan tres artefactos
  `meltemi-unsigned-<SO>-<sha-corto>` en la página del run; (2) que el de macOS
  trae el `meltemi-desktop-macOS.dmg` ya normalizado y el
  `meltemi-macOS.tar.gz` con los cuatro binarios; (3) el tiempo de pared por
  plataforma, que es el dato con el que se decide si la cadencia baja a
  `workflow_dispatch` o a `schedule`; (4) que el resumen del run muestra el aviso
  y que `UNSIGNED-BUILD.txt` viaja dentro del artefacto; (5) que `publish-site`
  de `release.yml` publica el sitio en ese mismo push sin esperar nada nuevo —la
  razón entera de no tocar aquel archivo—; y (6) que no se creó release alguna y
  que `releases/latest` sigue resolviendo a la última firmada. -->

- [x] 4.2 Anotar el resultado de esa verificación tras el primer push a `main`
  (los seis puntos de 4.1), con el tiempo de pared por plataforma, antes de
  archivar la change
  <!-- 2026-08-09: el primer push a `main` tras el merge ocurrió el 2026-08-06
  (commit `5b979c5c`) y disparó el run **31105696074** de `Build`,
  **completado con éxito en 15m3s**. Los seis puntos, medidos contra
  ese run y contra los artefactos descargados de él (siguen vivos hasta el
  2026-08-13; la retención de 7 días se comporta como se declaró):

  1. **Los tres jobs completan en runner real y dejan sus tres artefactos.**
     `unsigned build (ubuntu-latest)`, `(macos-latest)` y `(windows-latest)`,
     los tres verdes, con los 19 pasos de cada job en `success` — incluidos
     los tres presupuestos (`TUI size budget`, `Adapter size budget`, `GUI
     installer size budget`), que por tanto **gatean esta ruta de verdad y no
     de palabra**. Artefactos publicados en la página del run:
     `meltemi-unsigned-Linux-5b979c5c` (12 723 808 B),
     `meltemi-unsigned-macOS-5b979c5c` (12 106 948 B) y
     `meltemi-unsigned-Windows-5b979c5c` (10 782 152 B) — el nombre lleva SO y
     SHA corto del commit, como exige el requisito.
  2. **El artefacto de macOS trae lo que promete** (descargado y abierto, no
     inferido del YAML): `meltemi-desktop-macOS.dmg` (4 743 450 B) ya
     normalizado, `meltemi-macOS.tar.gz` (7 413 725 B) cuyo contenido es
     exactamente `meltemi`, `meltemid`, `meltemi-claude-acp` y
     `meltemi-codex-acp` —los cuatro binarios, con los dos adaptadores
     propios—, más `SHA256SUMS` de las dos piezas. El DMG que la máquina
     Windows del mantenedor no puede construir queda descargable desde la
     página del run, que era el pedido entero de esta change.
  3. **Tiempo de pared por plataforma** — el dato con el que se decide si la
     cadencia baja a `workflow_dispatch` o a `schedule`: **Linux 7m36s**,
     **macOS 9m35s**, **Windows 14m59s**; el run completo, 15m3s (los tres en
     paralelo, así que el reloj lo marca Windows). El costo declarado en la
     propuesta —«el gasto más caro que este repositorio haya añadido a
     `main`»— se confirma en su orden de magnitud y no lo excede: ningún job
     se acercó al límite del runner.
  4. **El aviso viaja a sus dos destinos.** El paso `Declare the build
     unsigned` cerró en `success` en los tres jobs (el que escribe al resumen
     del run y al artefacto), y `UNSIGNED-BUILD.txt` (1 311 B) está dentro del
     artefacto descargado, con el texto íntegro de `scripts/`: qué no lleva
     (firma, atestación, publicación), por qué la clave nunca toca CI, y la
     URL de `releases/latest` como la vía de instalación.
  5. **`publish-site` no esperó a nada nuevo, que era la razón entera de no
     tocar `release.yml`**: el run de `Release` en ese mismo push se creó a
     las 13:23:36Z y su job `publish site` arrancó a las 13:24:47Z — **71
     segundos después**, sin esperar empaquetado alguno (los jobs de tag
     quedaron `skipped` en 0s, como debía). La mitad medible del punto queda
     confirmada. **La otra mitad falló por causa ajena a esta change y se
     anota sin maquillar**: el paso `actions/deploy-pages@v4` de ese job
     agotó su espera («Timeout reached, aborting!») a los 10m9s, y el mismo
     paso del run del tag `v0.1.1` —empujado 18 s después— murió con
     «Deployment cancelled». El sitio sigue en pie (HTTP 200 en meltemi.dev,
     sirviendo el último despliegue bueno) y ningún job de `build.yml` está
     implicado: es el entorno Pages del repositorio, no el grafo de jobs.
     Queda anotado como hallazgo en `docs/plan-de-cambios.md`, no colado aquí.
  6. **No se creó release alguna por esta ruta y `releases/latest` sigue
     resolviendo a la última firmada.** `build.yml` no publicó nada —su
     cabecera `contents: read` lo hace imposible, y el conjunto negativo de
     tests lo pinea—; `releases/latest` sigue siendo **v0.1.0**, la firmada
     del 2026-07-26. El borrador `v0.1.1` que existe en el repositorio lo creó
     el run del **tag**, no esta ruta, y sigue en `Draft`: los enlaces de
     descarga del sitio no cambiaron de destino.

  **Segunda corrida, sobre el commit de hoy** (`5f01907`, run 31326960758,
  2026-08-09): **verde en las tres plataformas** — ubuntu 3m29s, macOS 3m36s,
  Windows 5m16s. La ruta no dependía del commit que la introdujo, y el número
  que importa para la cadencia es este par: **caché fría ~15 min, caché
  caliente ~5 min** (`Swatinem/rust-cache` por job, sin afinar). El costo por
  push, en el caso normal, es un tercio del que la propuesta declaró.

  **Y el punto 5 queda cerrado con evidencia posterior**: en este mismo push
  el workflow `Release` completó **con éxito**, `publish site` incluido. El
  fallo de `deploy-pages` del 2026-08-06 fue por tanto **transitorio, y su
  causa más probable es la concurrencia** —un push a `main` y un push de tag
  separados por 18 segundos, compitiendo por el mismo entorno Pages: el
  primero agotó su espera de 10 min y el segundo murió con «Deployment
  cancelled»—, no un defecto del pipeline ni de esta change. Se deja escrito
  para que un rojo idéntico no se lea mañana como una rotura nueva. -->

