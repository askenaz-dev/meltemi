# release-con-el-push

> Vía completa (proposal → design → specs → tasks). Decisión del mantenedor
> del 2026-10-23: la release del IDE sale **sin paso manual**, como el
> despliegue de Meltemi Diagrams sale de un push. Revierte a conciencia la
> decisión D1 de `procedencia-de-release` (la firma minisign en la máquina del
> mantenedor) y la retira sin fundir sus deltas; absorbe la tarea 7.4 de `delivery` en meltemi-web
> (retirar el sitio de este repositorio); re-acota `sucesion-y-custodia`.
> Nace de retomar el proyecto el 2026-10-23 con tres hechos sobre la mesa:
> v0.1.1 lleva desde el 6 de agosto como borrador sin firmar, `main` lleva
> 375 commits sin release, y la web ya vive en `meltemi-web` desde el 21 de
> octubre mientras este repositorio sigue publicando un sitio fantasma.

## Why

El pipeline de release de hoy hace todo salvo publicar: construye y prueba en
tres plataformas, empaqueta con nombres estables, acuña una atestación y deja
un **borrador** esperando a que una persona descargue el `SHA256SUMS`, lo firme
con minisign en su portátil, suba el `.minisig` y pulse publicar. Ese paso se
diseñó como la garantía más fuerte del esquema —lo único que una cuenta de
GitHub comprometida no puede completar— y a cambio costó esto: v0.1.1 nunca
se publicó, la clave pública nunca llegó al árbol (tarea 2.2 abierta desde
julio), y `antes-de-anunciar` lleva un trimestre con el gate rojo. La
garantía existe en el papel; en la práctica el paso manual produjo cero
releases firmadas desde agosto y un «latest» de julio.

Meltemi Diagrams, el otro producto de la familia, llegó a producción el 20 de
octubre con dos reglas: toda la infraestructura es Terraform y **nada se
publica desde una máquina**. Un push a `main` despliega el servicio; un tag
publica diecinueve paquetes y la extensión con procedencia. El IDE no tiene
servicio alojado: su despliegue **es** la release. Así que «desplegar con el
push» significa aquí que la release nace del push, entera, con los mismos
gates de siempre, y que nadie vuelve a firmar nada a mano.

Lo que se pierde se dice con todas las letras: un atacante con escritura en
el repositorio podrá publicar una release que verifique perfectamente,
porque la atestación registra fielmente su commit. Lo que se gana es que la
release sea un hecho del repositorio y no de una persona: reproducible,
registrada en un log público, verificable con un comando, y sin una clave de
largo plazo que custodiar, rotar, suceder o perder. Los controles que
compensan la pérdida van en la sección siguiente; ninguno reintroduce un paso
humano. La constitución §6 además favorece el camino nuevo: SLSA, in-toto y
Sigstore son estándares abiertos; minisign era una desviación consciente que
el propio design de `procedencia-de-release` dejó anotada.

La web entra en la misma change por una razón mecánica: `release.yml` publica
hoy el sitio a GitHub Pages en cada push, y ese sitio dejó de existir el 21 de
octubre cuando `meltemi-web` tomó meltemi.dev. Reescribir el workflow sin
retirar ese job sería dejar el fantasma; y «actualizar la web» con cada
release, que es la segunda mitad de lo que el mantenedor pidió, necesita que
la web sepa que hubo release: un aviso del pipeline a `meltemi-web`.

## What Changes

- **La release nace de un push a `main` que cambia la versión.** El pipeline
  compara la versión del workspace con la última release publicada; si es
  nueva, corre los gates en las tres plataformas, empaqueta con los nombres
  estables, acuña la atestación, crea el tag `v<versión>` sobre ese commit y
  **publica** la release, sin borrador. Un push que no cambia la versión sigue
  dejando el build de integración de `artefactos-de-cada-push`. El tag deja de
  ser la entrada y pasa a ser la salida: la aprobación explícita que la
  constitución §3 pide para un efecto externo irreversible es el commit de
  versión que el mantenedor empuja, y ninguna otra cosa decide publicar.
- **La firma es la del workflow.** Se retira minisign: el `SHA256SUMS` queda
  cubierto por la atestación de build (Sigstore keyless, identidad del
  workflow, log de transparencia público), y la verificación publicada pasa
  de tres pasos a dos: checksum y `gh attestation verify … --signer-workflow`.
  El bundle de la atestación viaja como asset de la release para que la
  verificación sin red exista. Los scripts de instalación siguen haciendo
  solo el paso 1, como hoy.
- **Controles que compensan la ausencia del paso humano**, todos
  estructurales: las acciones de terceros pineadas por SHA y no por tag
  flotante, porque sin gate humano una acción comprometida publica; un ruleset
  que reserva la creación de tags `v*` y bloquea borrar y reescribir `main`;
  el pipeline rehúsa publicar un commit que no sea alcanzable desde `main` o
  cuya versión ya tenga release; la credencial del aviso a la web vive en un
  entorno de despliegue cuya política solo admite la referencia que publica;
  y la repudiación de una release sustituye a la de una clave: borrar la
  release, publicar la siguiente versión y declararlo, con el log público
  como evidencia de qué pasó. El repositorio del IDE **entra al Terraform de
  la familia** para que rulesets, entorno y credencial se declaren en código
  como todo lo demás (decisión 0004 de meltemi-web).
- **Una versión en cinco archivos, verificada.** `Cargo.toml`,
  `desktop/tauri.conf.json`, `mobile/tauri.conf.json`,
  `desktop/ui/package.json` y `desktop/ui/package-lock.json` llevan la
  versión, y `Cargo.lock` la repite por crate; un script la fija en todos y
  un guardián falla si divergen, como `release:version` en Diagrams. La primera release de este camino es
  **0.2.0**: `turno-vacio-es-error` cambió la taxonomía de códigos de salida y
  `versionado.md` lo clasifica como ruptura. El borrador v0.1.1 se borra.
- **Notas de release desde el repositorio**: la sección de `CHANGELOG.md` de
  esa versión cuando existe, las notas generadas cuando no;
  `antes-de-anunciar` convierte la sección en regla.
- **La web se entera.** Al publicarse la release, el pipeline dispara el
  despliegue del sitio de `askenaz-dev/meltemi-web` con un token que solo
  puede lanzar sus workflows; el disparo no lleva datos y el sitio lee la
  release publicada en GitHub (design D7). La página de descargas sigue sin literal de versión:
  sus enlaces resuelven a «latest» y eso no cambia. La parte del sitio es una
  change hermana en meltemi-web; aquí va el disparo y su credencial.
- **El sitio se retira de este repositorio** (tarea 7.4 de `delivery` en
  meltemi-web): `site/`, el job de Pages de `release.yml`, los tests
  `site.rs` y `scenarios_sitio.rs`, y GitHub Pages apagado. Los dos scripts de
  captura escriben a un destino dentro del IDE del que el sitio copia. La
  capacidad `product-site` sale de la verdad viva de este repositorio porque
  vive en el de la web.

## Enmienda a `procedencia-de-release`

La change sigue activa con 6 de 8 tareas; esta la retira sin fundir sus
deltas y conserva, reescrito, lo que sigue siendo cierto (design D9):

- **D1 se revierte.** «Los dos mecanismos conviven; ninguno reemplaza al otro»
  pasa a «la atestación es la firma». El argumento de D1 —la cuenta
  comprometida— sigue siendo cierto y queda escrito en `docs/release.md`
  como límite declarado, no como riesgo escondido. La cláusula «qué
  invalidaría esta decisión» no se cumplió: GitHub no ganó una aprobación
  fuera de banda; el mantenedor decidió la permuta con ese dato a la vista.
- **D2 se conserva** (un solo subject: el `SHA256SUMS`) y se amplía: la
  atestación cubre el conjunto publicado y el bundle viaja con él.
- **D3 cambia de objeto.** Ya no hay clave pública que anclar en el árbol; el
  ancla es la identidad del workflow (`--signer-workflow`), que vive en el
  repositorio por construcción. El principio —nunca tomar la confianza de la
  página que se autentica— se mantiene.
- **D4 queda sin objeto**: no hay herramienta de firma cuyos límites declarar.
  Su lugar lo ocupa la asimetría offline (D5 del design), ahora resuelta con
  el bundle como asset.
- **Tarea 1.2** (corrida real de la atestación) la cumple la release 0.2.0 de
  este camino. **Tarea 2.2** (publicar la clave pública) se retira: no hay
  clave. Su carpeta pasa al histórico sin delta y con la cabecera «Retirada
  sin fundir»; `artefactos-de-cada-push`, verificada en agosto, se archiva
  antes que esta. `antes-de-anunciar` cambia su gate a «0.2.0 publicada y verificada
  con `gh attestation verify`».
- **`sucesion-y-custodia` se re-acota**: sin clave de largo plazo, lo que hay
  que suceder es el control del repositorio y de la organización, no una
  clave; su propuesta se reescribe en ese sentido cuando llegue su turno.
- El comparativo con Orca anotó «releases automáticas» entre lo que no se
  copia. Se precisa: lo que no se copia es publicar sin gates ni procedencia;
  la ausencia de un paso manual, con gates en tres plataformas y atestación,
  es otra cosa, y es esta.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: ADDED «Procedencia verificable de la release»
  (la de `procedencia-de-release`, reescrita, con el bundle como asset);
  ADDED «Publicación idempotente y compatible con releases inmutables»;
  ADDED «Controles estructurales del camino sin paso humano»; ADDED «Aviso a
  la web al publicar»; MODIFIED «Versionado con política escrita» (la release
  nace de un commit de versión en `main`; el tag es su resultado); MODIFIED
  «Artefactos firmados con custodia documentada» (la firma es la atestación
  del workflow, sin clave de largo plazo; repudio de release en vez de
  repudio de clave; conserva su nombre porque es de la era de la migración);
  MODIFIED «Instalador auditable», «Instaladores de la GUI de escritorio» y
  «Nombres de artefacto estables por plataforma» (de «firmado» a «cubierto
  por la atestación»); MODIFIED «Builds de integración descargables sin crear
  release» (el camino de release ya no es «solo de tag»; exige archivar antes
  `artefactos-de-cada-push`); REMOVED «Publicación del sitio con la release».
- `product-site`: REMOVED sus ocho requisitos; la capacidad vive en
  `meltemi-web`.
- `artifact-format`: MODIFIED «Reglas de contenido de delta» (un bloque
  `REMOVED` no exige escenario ni verbo normativo en ninguna validación del
  motor). Es la primera eliminación del proyecto y destapó que la validación
  de una change y la del árbol discrepaban (design D11).

## Impact

- Archivos: `.github/workflows/release.yml` (reescrito), `build.yml` (el
  caso «sin cambio de versión»), `scripts/sign-release.ps1` (retirado),
  `scripts/unsigned-build-notice.txt`, `scripts/set-version.{ps1,sh}`
  (nuevo), `docs/release.md`, `docs/versionado.md`, `README.md`, `LEEME.md`,
  `core/meltemid/tests/release.rs` (los guardianes que afirman la firma
  manual cambian con sus escenarios), `core/meltemid/tests/{site.rs,
  scenarios_sitio.rs}` (retirados), `site/` (retirado),
  `scripts/capture-desktop.ps1`, `tui/examples/capture_svg.rs`,
  `scripts/release-gate.sh` (nuevo), `Cargo.toml` (versión y `repository`,
  que hoy apunta a una organización que no es la nuestra), `Cargo.lock`,
  `desktop/tauri.conf.json`, `mobile/tauri.conf.json`,
  `desktop/ui/package.json`, `desktop/ui/package-lock.json`, `AGENTS.md`
  (parte operativa), `docs/plan-de-cambios.md`,
  `core/meltemi-spec/src/validate.rs` (los bloques `REMOVED` en la
  validación del árbol).
- Fuera de este repositorio, nombrado aquí: en `meltemi-web`, `github.tf`
  adopta el repositorio del IDE (rulesets, entorno, credencial del aviso) y
  una change hermana reconstruye `/ide/` con la versión; el mantenedor
  apaga Pages y borra el borrador v0.1.1.
- Cero dependencias en los binarios. Una credencial nueva acotada al aviso a
  la web, colocada por Terraform; su alcance queda en `docs/release.md`.
- Riesgo asumido: una cuenta con escritura publica. Riesgo nuevo: un paso
  que habla con un servicio externo (la atestación, el aviso) aborta o deja
  a medias una release; cada job salta lo que ya está publicado para que una
  corrida repetida termine lo que falta, como en Diagrams.
- Los 177 commits locales sin push y la posible deuda de gates sobre ellos no
  son de esta change, pero van antes: la primera release 0.2.0 corre sobre lo
  que CI haya aprobado.

## Fuera de alcance

- Firma de plataforma (Authenticode, notarización): dinero, no criptografía;
  deuda ya declarada.
- Gestores de paquetes y aviso de versión en el cliente:
  `distribucion-y-actualizacion`, que ahora cuelga de `release: published`
  sin esperar a nadie.
- CHANGELOG generado, quickstart, FAQ, plantillas: `antes-de-anunciar`.
- La página `/ide/` con versión y novedades: change hermana en
  `meltemi-web`.
- Volver a un paso humano por entorno con revisores obligatorios: es la
  opción 1 que el mantenedor descartó; queda anotada como la vía de vuelta
  si la permuta se revisa.
