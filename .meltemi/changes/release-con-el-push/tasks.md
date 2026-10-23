# Tareas — release-con-el-push

Vía completa. Un commit atómico por tarea, con referencia
`(release-con-el-push N.M)` y sin trailers de co-autoría. Gates del repo en
cada tarea: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, la suite del crate tocado y, cuando la tarea toca código bajo
`cfg(unix)` o `not(windows)`, el mismo clippy con `--target
x86_64-linux-android` sobre los crates del núcleo. **La change se desarrolla en
su rama `release-con-el-push` y llega a `main` por merge en la tarea 7.1, no
antes**: desde ese merge cada push a `main` puede publicar (design D10).

Las tareas marcadas *(meltemi-web)* o *(mantenedor)* no son de este
repositorio o no son de un agente; están aquí porque esta change no se puede
cerrar sin ellas.

## 1. Precondiciones

- [x] 1.1 El motor: `validate_tree` deja de exigir escenario y verbo normativo
  a los bloques de una sección `## REMOVED Requirements` y les comprueba lo que
  el formato pide, **Reason** y **Migration**; test en `validate.rs` con un
  delta que solo elimina, y otro que confirma que un `REMOVED` sin
  **Migration** sigue siendo inválido en las dos validaciones (design D11) —
  escenarios «Requisito eliminado con razón y migración», «Requisito eliminado
  sin migración», «Requisito modificado con bloque parcial» (vigentes) — gates: `cargo test -p meltemi-spec`, con
  `dogfood.rs` de vuelta en verde
- [x] 1.2 Archivar `artefactos-de-cada-push` tal como está —7/7, verificada en
  runner real el 2026-08-09— para que «Builds de integración descargables sin
  crear release» entre a la verdad viva y `meltemi validate
  release-con-el-push` quede limpio (design D9) — gates: `meltemi validate`,
  `cargo test -p meltemi-spec`
  <!-- 2026-10-23: simulado sobre una copia temporal de `.meltemi/` con ese
  delta fundido: `validate` limpio. Sin fundir, un único diagnóstico,
  `modified-requirement-missing` sobre ese requisito.
  Hecho el mismo día con los binarios de esta rama: `verify` 9/9, `archive`
  la preservó en `archive/2026-10-23-artefactos-de-cada-push` y regeneró la
  proyección; `validate release-con-el-push` y `validate` limpios. -->
- [x] 1.3 Retirar `procedencia-de-release` sin fundir: su carpeta pasa a
  `.meltemi/changes/archive/<fecha>-procedencia-de-release/` sin `specs/`, con
  la cabecera «Retirada sin fundir — absorbida por `release-con-el-push`» en
  su `proposal.md`; comprobar que `meltemi` lista el histórico y que
  `dogfood.rs` y `migration.rs` lo aceptan, y resolver aquí lo que no (design
  D9) — gates: `cargo test -p meltemi-spec`
  <!-- 2026-10-23: `meltemi changes` la lista como `archived 2026-10-23`; el
  listado solo lee la fecha del nombre y qué artefactos hay, así que una
  entrada sin `specs/` no rompe nada. `validate`, `validate
  release-con-el-push`, `dogfood.rs` y `migration.rs` en verde. -->
- [x] 1.4 Pinear por commit completo cada `uses:` de `ci.yml`, `build.yml` y
  `release.yml`, con la versión como comentario, y declarar `permissions:
  contents: read` arriba de `ci.yml`; tests en `release.rs` que recorren todos
  los workflows (design D5) — escenarios «Acción sin pinear rechazada»,
  «Workflow sin permisos declarados rechazado» — gates: `cargo test -p
  meltemid --test release`

## 2. La versión

- [x] 2.1 `core/meltemi-release`, crate sin dependencias, con `version` y
  `set-version <x.y.z>`: rehúsa lo que no sea `x.y.z` o no sea mayor que la
  actual, reescribe los cinco portadores, refresca las entradas del workspace
  en `Cargo.lock` y, si el lockfile movió algo ajeno, devuelve todo a como
  estaba; tests de que los portadores de este repositorio coinciden y de que
  una versión menor se rehúsa sin escribir nada (design D2, con su desviación)
  — escenario «Versión divergente entre portadores» — resolver el «sin
  verificar» de `cargo update --workspace`
  <!-- 2026-10-23: `set-version 0.2.0` sobre un worktree desechable reescribió
  los cinco portadores; `cargo update --workspace` movió los doce crates del
  workspace y ninguna dependencia de terceros; repetirlo se rehusó con salida
  1. Nueve tests; clippy limpio en Windows y para `x86_64-linux-android`. -->
- [x] 2.2 `AGENTS.md`, en su parte operativa y fuera del bloque gestionado: un
  agente no cambia la versión salvo que el mantenedor lo pida con palabras, y
  por qué; `docs/versionado.md`: la release nace de un commit de versión en
  `main` y el tag es su resultado (design D1, D2) — escenarios «Ruptura exige
  minor» (vigente), «Tag empujado a mano no publica»

## 3. El pipeline

- [x] 3.1 `meltemi-release gate --commit <sha>`: las cuatro comprobaciones de
  D1, con salida `release=true|false`, la versión y el motivo; tests en
  `core/meltemi-release/tests/` contra repositorios git temporales, uno por
  negativa y uno por la positiva (design D1, D5) — escenarios «Versión nueva en la rama principal publica» (la
  decisión), «Push sin versión nueva no publica», «Versión ya publicada no se
  republica», «Versión menor no se publica», «Commit fuera de la rama
  principal no se publica»
- [x] 3.2 `release.yml`: disparadores `push` a `main` y `workflow_dispatch`
  limitado a `main`, sin tags; job `version` con el script; `gates`, `deny`,
  `package` y `package-gui` condicionados a `release == 'true'`; fuera
  `site-lint` y `publish-site`; tests de estructura en `release.rs` (design
  D1, D8) — escenarios «Tag empujado a mano no publica», «Presupuesto
  excedido aborta» (vigente)
  <!-- 2026-10-23: el YAML parsea con seis jobs (`version`, `gates`, `deny`,
  `package`, `package-gui`, `release`); el borrador se crea con `--target` sobre
  el commit en vez de `--verify-tag`, y la 3.3 lo convierte en publicación.
  El `gate` preguntado por el commit de esta rama contra `origin/main`
  respondió `release=false` porque `v0.1.1` ya existe, que es la protección
  del merge de la 7.1. Sale con el job el test de `publish-site`
  (`scenarios_sitio.rs`), cuyo requisito retira esta change. -->
- [x] 3.3 Job `publish` en el entorno `release`: fusión de checksums,
  atestación con su bundle como asset `attestation.sigstore.json`, borrador
  con el tag en el commit, subida de todos los assets, publicación; cada paso
  salta lo hecho; notas desde la sección de `CHANGELOG.md` o generadas (design
  D3, D4) — escenarios «Procedencia publicada con la release», «Re-ejecución
  tras fallo parcial termina lo que falta», «Nada se adjunta tras publicar»,
  «Notas desde el changelog», «Instalador atestiguado por plataforma», «Script
  instalador publicado y atestiguado», «Credenciales solo en el entorno de
  publicación» — resolver el «sin verificar» de la salida del bundle en la
  versión pineada de `actions/attest`
  <!-- 2026-10-23: la salida es `bundle-path` (action.yml del commit pineado).
  Las notas las toma el job `version` con `meltemi-release notes`, que sale
  con 3 si `CHANGELOG.md` no tiene la sección; este repositorio no tiene
  `CHANGELOG.md` todavía, así que la 0.2.0 llevará notas generadas salvo que
  `antes-de-anunciar` llegue antes. Un borrador no crea su tag: tras un fallo
  a mitad, el `gate` vuelve a abrir el camino y `publish` completa el
  borrador. El test viejo de la firma deja de exigirle `SHA256SUMS.minisig`
  al workflow; se reescribe entero en la 4.3. -->
- [x] 3.4 Job `notify-web` tras `publish`, en el entorno `release`: `gh
  workflow run deploy.yml --repo askenaz-dev/meltemi-web --ref main` con
  `WEB_DISPATCH_TOKEN`, sin entradas; un fallo deja la release intacta y se
  repite solo (design D7) — escenarios «La web se redespliega tras publicar»,
  «Sin publicación no hay aviso», «Aviso fallido no deshace la release», «El
  aviso no lleva datos»
- [x] 3.5 `build.yml` y `scripts/unsigned-build-notice.txt`: el aviso deja de
  hablar de minisign y distingue el build de integración por lo que de verdad
  le falta, atestación y publicación; test de que `build.yml` no concede
  `contents: write` a ningún job (design D3) — escenarios «La última release
  publicada sigue resolviendo», «La ruta de integración no puede publicar»,
  «El build de integración no crea release» (vigente)

## 4. La verificación publicada

- [x] 4.1 `docs/release.md` reescrito: la atestación como firma; los dos pasos
  con red y sin ella, con la raíz de confianza descargada una vez; el límite
  de la cuenta con escritura y los controles que lo rodean; el repudio de una
  release; la vía de vuelta a un paso humano; fuera la custodia de clave y la
  firma del mantenedor (design D3, D5) — escenarios «Verificación por el
  usuario», «Límite de la firma del workflow declarado», «Ninguna clave de
  firma que custodiar», «Repudio de una release», «Alcance de la atestación
  declarado sin exagerar», «Registro público de la atestación declarado»,
  «Verificación sin red con el bundle publicado»
- [ ] 4.2 `README.md`, `LEEME.md` y las cabeceras de `scripts/install.ps1` y
  `scripts/install.sh`: los dos pasos de verificación y ningún minisign;
  retirar `scripts/sign-release.ps1` (design D3) — escenario «Ninguna clave de
  firma que custodiar»
- [ ] 4.3 `core/meltemid/tests/release.rs`: los tests que afirman la firma
  manual y el camino de tag —`signed_artifacts_and_verification_are_documented`,
  `the_trust_anchor_is_never_sourced_from_the_release_page`,
  `the_signing_tools_limits_are_declared_not_promised_away`,
  `the_release_path_still_belongs_to_a_tag_alone`,
  `the_version_free_download_urls_still_answer_from_the_signed_release` y
  `the_integration_build_cannot_create_or_sign_a_release`— se reescriben
  contra los escenarios nuevos o se retiran, uno por uno, cada uno con su
  marcador `// Scenario:`; ningún escenario del delta queda sin test o sin
  marca manual justificada. Los escenarios que viajan sin cambios dentro de
  los requisitos modificados conservan sus tests de hoy, y se comprueba que
  siguen vinculados — escenarios «Presupuesto de tamaño como gate», «Formato
  autocontenido no se publica», «El paquete declara el motor del sistema»,
  «Push a main deja el build descargable» (vigentes)

## 5. El sitio sale

- [ ] 5.1 Trasladar a `release.rs`, antes de borrar nada, los tests de
  `scenarios_sitio.rs` y `site.rs` que cubren requisitos que se quedan: la
  normalización de nombres de instaladores, los scripts de instalación como
  assets con su alias y la base canónica única entre instaladores y
  `docs/release.md` (design D8) — escenarios «Instalador de escritorio
  normalizado antes de publicar», «Descarga sin conocer la versión»,
  «Instalación con alias» (vigentes)
- [ ] 5.2 Retirar `site/`, `core/meltemid/tests/site.rs` y
  `core/meltemid/tests/scenarios_sitio.rs`; `scripts/capture-desktop.ps1` y
  `tui/examples/capture_svg.rs` escriben en `docs/media/`; actualizar las
  referencias a `site/` en `docs/` (design D8) — gates: la suite completa
  con `--no-fail-fast`
- [ ] 5.3 Al archivar, si el motor deja vacío `.meltemi/specs/product-site/`,
  borrar la carpeta en el mismo commit y comprobar que `dogfood.rs` sigue
  verde (design D8)

## 6. Fuera de este repositorio, antes del merge

- [ ] 6.1 *(meltemi-web)* `packages/infra/github.tf`: para
  `askenaz-dev/meltemi`, el ruleset de `main` (sin borrado ni push forzado),
  el de tags `v*` (sin movimiento ni borrado; creación restringida a GitHub
  Actions solo si el ruleset lo admite), el entorno `release` con política de
  rama `main`, los permisos por defecto de los workflows en lectura y el
  secreto `WEB_DISPATCH_TOKEN`, todo sin importar `github_repository`; plan
  comentado en su pull request y aplicado (design D5, D6) — resolver los «sin
  verificar» de los rulesets y del proveedor
- [ ] 6.2 *(mantenedor)* Crear el token de grano fino —propietario
  `askenaz-dev`, acceso solo a `meltemi-web`, permiso Actions: lectura y
  escritura— y dejarlo en el entorno `infrastructure` de meltemi-web con
  `pnpm infra:bootstrap` (design D7) — resolver el «sin verificar» de la
  política de tokens de la organización
- [ ] 6.3 *(meltemi-web)* Change hermana: `/ide/` muestra la versión, la fecha
  y las notas de la última release, leídas de GitHub, y un horario propio
  repara un aviso perdido (design D7); no bloquea el merge, porque el
  `deploy.yml` de hoy ya acepta el disparo

## 7. Merge y primera release, en este orden (design D10)

- [ ] 7.1 Merge de la rama a `main` con la versión intacta en 0.1.1 y el
  borrador v0.1.1 todavía en su sitio; la corrida de `release.yml` debe decir
  `release=false` porque el tag `v0.1.1` existe; anotar aquí el run — escenarios
  «Versión ya publicada no se republica», «Push sin versión nueva no publica»
  (evidencia real)
- [ ] 7.2 *(mantenedor)* `scripts/set-version.ps1 0.2.0`, commit y push a
  `main`; seguir la corrida y anotar aquí la evidencia: el tag sobre el
  commit, los assets con el bundle, los comandos de verificación que
  funcionaron con red y sin ella, y la corrida de despliegue de la web;
  resolver en el design cada punto de «Sin verificar» que siga abierto —
  escenarios «Versión nueva en la rama principal publica», «Procedencia
  publicada con la release», «Verificación sin red con el bundle publicado»,
  «La web se redespliega tras publicar» (evidencia real)
- [ ] 7.3 *(mantenedor)* Con `main` ya en 0.2.0: borrar el borrador v0.1.1 y su
  tag, apagar GitHub Pages en este repositorio y borrar su entorno
  `github-pages` (design D8, D10)
- [ ] 7.4 `meltemi verify release-con-el-push` sin escenarios sin vínculo;
  `meltemi archive release-con-el-push`; actualizar `docs/plan-de-cambios.md`
  y el gate de `antes-de-anunciar`
