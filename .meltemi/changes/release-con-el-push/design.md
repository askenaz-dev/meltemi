# Design — release-con-el-push

## Context

El pipeline de release de hoy (`.github/workflows/release.yml`) se dispara con
un tag `v*.*.*` y con cada push a `main`. Con el tag corre los gates en las
tres plataformas, empaqueta con nombres estables, fusiona los `SHA256SUMS`,
acuña una atestación sobre ellos y deja un **borrador**. El mantenedor debía
descargar `SHA256SUMS`, firmarlo con minisign en su máquina
(`scripts/sign-release.ps1`), subir el `.minisig` y publicar. Con el push a
`main`, el mismo archivo publica el sitio en GitHub Pages.

Lo verificado el 2026-10-23, que sostiene las decisiones de abajo:

- `releases/latest` es v0.1.0 (2026-07-26). v0.1.1 es un borrador sin firmar
  desde el 2026-08-06. La clave pública minisign nunca entró al árbol
  (`procedencia-de-release` 2.2 abierta).
- El repositorio tiene **releases inmutables activadas**
  (`gh api repos/askenaz-dev/meltemi/immutable-releases` responde
  `{"enabled":true}`): una release publicada no admite assets nuevos.
- Los permisos por defecto de los workflows son de **escritura**
  (`default_workflow_permissions: write`), y `ci.yml` no declara bloque
  `permissions`.
- No hay secretos de repositorio ni más entorno que `github-pages`. Ningún
  ruleset protege `main` ni los tags.
- meltemi.dev lo sirve `askenaz-dev/meltemi-web` desde el 2026-10-21
  (decisión 0004 de ese repositorio). El sitio de este repositorio ya no
  responde en el dominio, pero cada push lo sigue publicando en Pages.
- El `deploy.yml` de meltemi-web acepta `workflow_dispatch` sin entradas.
- La versión `0.1.1` vive en cinco archivos editables —`Cargo.toml`,
  `desktop/tauri.conf.json`, `mobile/tauri.conf.json`,
  `desktop/ui/package.json` y `desktop/ui/package-lock.json` (dos entradas)—
  más las dieciocho entradas de los crates del workspace en `Cargo.lock`.
- El motor exige que todo `MODIFIED` y todo `REMOVED` apunte a un requisito
  vivo, y `migration.rs` falla si desaparece, aun renombrado, un requisito de
  la era de la migración, o uno de sus escenarios sin registro en
  `SUPERSEDED`. De `release-distribution`, esa era dejó cinco requisitos y
  cinco escenarios; ninguno de los cinco puede renombrarse.

## Goals / Non-Goals

**Goals:** que la release del IDE salga de un push a `main` sin paso humano;
que su verificación siga siendo posible con un comando, con red y sin ella;
que la ausencia del paso humano se compense con controles estructurales
declarados en código; que la web se entere de cada release; que este
repositorio deje de publicar un sitio que ya no es suyo.

**Non-Goals:** firma de plataforma (Authenticode, notarización); gestores de
paquetes y aviso de versión en el cliente (`distribucion-y-actualizacion`);
CHANGELOG generado y quickstart (`antes-de-anunciar`); la página `/ide/` con
versión, que es una change de meltemi-web; un paso humano por entorno con
revisores obligatorios, que es la opción que el mantenedor descartó.

## Decisions

### D1 — La release nace de un push a `main` que trae una versión nueva; el tag es la salida

`release.yml` se dispara con `push` a `main` y con `workflow_dispatch`
restringido a `main`, este último solo para volver a correr una publicación
fallida. Deja de escuchar tags. Su primer job, `version`, decide:

1. lee `[workspace.package] version` y exige que los cinco portadores digan
   lo mismo (D2);
2. comprueba que no exista el tag `v<versión>` (`git ls-remote --tags`);
3. comprueba que la versión sea mayor que la mayor ya publicada, por SemVer:
   un `0.1.2` empujado después de `0.2.0` no se publica;
4. comprueba que el commit sea alcanzable desde `main`
   (`git merge-base --is-ancestor`).

Si las cuatro pasan, sale `release=true` y corren los gates, el empaquetado,
la publicación (D4) y el aviso a la web (D7). Si no, el workflow termina sin
hacer nada más, y el build de integración de `build.yml` sigue siendo lo que
deja cada push. La lógica vive en `meltemi-release gate` (D2), que el job
ejecuta y que sus tests prueban contra repositorios git temporales, porque
las cuatro negativas son escenarios y un `if:` de YAML no se puede probar.

El gesto del mantenedor pasa a ser un commit: un diff con autor y fecha que
está en la historia antes de que nada se publique. Nadie empuja tags. El
pipeline crea el tag sobre el commit exacto que pasó los gates.

`ci.yml` y `build.yml` siguen corriendo en paralelo sobre el mismo push.
`release.yml` vuelve a correr los gates en vez de fiarse del veredicto de
otro workflow, como hoy. Un push de versión paga tres veces lo mismo; ocurre
una vez por release, y el dial está escrito en Riesgos.

*Alternativas.* El tag como entrada, como en Meltemi Diagrams: deja un gesto
manual aparte del commit y admite un tag sobre un commit que no está en
`main`. Un job programado que publique «lo último de main»: publica sin que
nadie haya dicho «esta es una versión».

### D2 — La versión vive en cinco archivos, un script la fija en todos y un guardián falla si divergen

`meltemi-release set-version <x.y.z>`, un crate del workspace sin
dependencias (`core/meltemi-release`): rehúsa lo que no sea `x.y.z` o no sea
mayor que la versión actual, reescribe los cinco portadores, refresca las
entradas del workspace en `Cargo.lock` con `cargo update --workspace` y, si el
lockfile movió algo más que esas entradas, devuelve todos los archivos a como
estaban. Un test del crate exige que los portadores de este repositorio digan
lo mismo y nombra los que no.

*Desviación al implementar (2026-10-23).* El plan decía dos scripts gemelos,
`set-version.ps1` y `set-version.sh`, y un `release-gate.sh` probado desde
`release.rs`. Probar un script de bash desde los tests en Windows depende de
qué `bash` resuelva el PATH, que puede ser el de WSL y no el de Git, y dos
scripts gemelos son dos implementaciones que divergen. Una herramienta en
Rust, sin dependencias fuera de la biblioteca estándar, es una sola
implementación, se prueba igual en las tres plataformas y es el lenguaje
único del producto. Vive en `core/` con los demás crates para no abrir un
directorio que `rumbo/structure.md` no declara, y no viaja en ningún
archivo publicado.

Una regla nueva en la parte operativa de `AGENTS.md`: un agente no cambia la
versión salvo que el mantenedor lo pida con palabras, porque desde esta
change un commit de versión es una publicación. Es un control blando y se
declara así en Riesgos.

*Alternativas.* `cargo-release` o `cargo-edit`: una herramienta nueva para
reescribir cinco líneas, contra §10.

### D3 — La atestación es la firma: se retira minisign y no hay clave de largo plazo

Se retiran el borrador que esperaba a una persona, `scripts/sign-release.ps1`
y toda instrucción de verificar con minisign. La firma de una release es la
atestación de procedencia que acuña el workflow que la publica: Sigstore sin
clave, ligada a la identidad del workflow, en un log de transparencia
público.

El paso de atestación sigue donde lo puso `procedencia-de-release`, sobre el
`SHA256SUMS` fusionado con `subject-checksums`. Sus sujetos son los assets
que ese archivo lista. El **bundle** de la atestación se publica como asset
de la release (`attestation.sigstore.json`), y eso resuelve la asimetría que
aquella change dejó escrita: la verificación sin red deja de exigir una
descarga previa desde una máquina en línea, salvo la raíz de confianza, que
se baja una vez con `gh attestation trusted-root`.

La verificación publicada queda en dos pasos: el checksum, y
`gh attestation verify <artefacto> --repo askenaz-dev/meltemi
--signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml`, con su
variante sin red (`--bundle` y `--custom-trusted-root`). La documentación de
hoy dice `gh attestation verify SHA256SUMS`; con `subject-checksums` el
archivo de checksums no es sujeto, así que esa forma probablemente no
verifica. Queda en «Sin verificar» y se resuelve en la primera corrida.

**El límite, escrito como límite y no como riesgo escondido**: una cuenta con
permiso de escritura puede publicar una release que verifica perfectamente,
porque la atestación registra fielmente su commit. El argumento de D1 de
`procedencia-de-release` sigue siendo cierto. Lo que cambia es que el
mantenedor eligió la permuta con ese dato a la vista, y lo que la compensa
es D5.

§6 favorece el camino nuevo: el predicado es SLSA, el formato in-toto y el
firmante Sigstore, tres estándares abiertos. minisign era la desviación
consciente que aquel design dejó anotada.

*Alternativas.* La firma manual, que es la opción descartada. La clave
minisign en un secreto de Actions, que SLSA prohíbe en lenguaje normativo y
sigue prohibida aquí, escrito en `docs/release.md`. Un HSM o KMS en la nube:
una cuenta más y una credencial dentro de CI, sin comprar nada que la
atestación no dé.

### D4 — La publicación es idempotente y respeta las releases inmutables

El job `publish` corre en el entorno `release` (D5) con `contents: write`,
`id-token: write`, `attestations: write` y `artifact-metadata: write`. En
orden:

1. fusiona los checksums de los seis jobs de empaquetado, como hoy;
2. acuña la atestación y guarda el bundle;
3. crea la release como **borrador** con el tag apuntando al commit
   (`gh release create v<versión> --draft --target <sha>`), con las notas;
4. sube todos los assets, el bundle incluido;
5. la publica (`gh release edit --draft=false`).

El borrador es estado interno durante minutos, no una espera humana. Con
releases inmutables, adjuntar después de publicar es imposible, así que el
orden no es estética. Una re-ejecución salta lo hecho: si la release ya está
publicada, pasa al aviso; si es un borrador, sube lo que falte y publica.

Las notas salen de la sección `## <versión>` de `CHANGELOG.md` cuando existe
y de `--generate-notes` cuando no.

**Lo que sigue a la publicación corre en el mismo workflow, por `needs`.**
Un tag o una release creados con `GITHUB_TOKEN` no disparan otros workflows:
GitHub lo impide para evitar bucles. Un `on: release: published` nunca se
dispararía. Esto corrige la premisa de `distribucion-y-actualizacion`, que
proponía colgar de ese evento; su design lo recogerá cuando le toque.

### D5 — Controles estructurales en lugar del paso humano

1. **Acciones pineadas por SHA.** Todo `uses:` de los tres workflows apunta a
   un commit completo, con la versión como comentario. Sin un humano entre el
   push y la publicación, una acción con tag flotante que alguien mueva
   publica. Un test de `release.rs` lo exige en todos los workflows.
2. **Permisos mínimos.** Todo workflow declara `permissions: contents: read`
   arriba y solo los jobs que publican elevan los suyos. El permiso por
   defecto del repositorio baja a lectura (D6).
3. **Un entorno para publicar.** `release`, con política de despliegue
   limitada a `main`. Los jobs `publish` y `notify-web` corren en él y son los
   únicos que ven la credencial del aviso.
4. **Las negativas de D1**, probadas.
5. **Rulesets.** En `main`: sin borrado ni push forzado. En los tags `v*`:
   sin movimiento ni borrado para nadie. La creación de tags `v*` se restringe
   a GitHub Actions solo si un ruleset admite esa integración como excepción,
   cosa sin verificar. Si no lo admite, la creación queda libre: ningún
   workflow escucha tags, así que un tag empujado a mano no publica nada, y
   las releases inmutables ya bloquean el tag de toda release publicada.
6. **Repudio de una release en vez de repudio de una clave.** Una release
   publicada no se corrige en el sitio. Se publica la versión siguiente y se
   declara la mala, con fecha, en sus notas y en `docs/release.md`. El log de
   transparencia es la evidencia de qué se publicó y desde qué commit.

*Lo que no se adopta.* Revisores obligatorios en el entorno `release`: es la
opción 1 que el mantenedor descartó. Queda escrito en `docs/release.md` como
la vía de vuelta: una línea de Terraform.

### D6 — El repositorio del IDE entra al Terraform de la familia sin importar el recurso del repositorio

En meltemi-web, `packages/infra/github.tf` gana, para `askenaz-dev/meltemi`,
los dos rulesets, el entorno `release` con su política de rama, el secreto
`WEB_DISPATCH_TOKEN` de ese entorno y los permisos por defecto en lectura.
Todo se declara referenciando el repositorio por su nombre, como recursos
sueltos. El recurso `github_repository` **no** se importa: importarlo pondría
bajo Terraform ajustes que ese recurso reconcilia —descripción, topics, Pages,
features— y el primer apply podría cambiar cosas que nadie pidió.

Esa parte es trabajo de meltemi-web y se nombra aquí. En las tareas de esta
change figura como precondición externa.

### D7 — La web se entera por un disparo y lee la verdad en GitHub, no en el mensaje

Tras publicar, el job `notify-web` ejecuta `gh workflow run deploy.yml --repo
askenaz-dev/meltemi-web --ref main` con `WEB_DISPATCH_TOKEN`: un token de
grano fino, con la organización como propietaria del recurso, acceso solo a
`meltemi-web` y el único permiso **Actions: lectura y escritura**.

No es `repository_dispatch`, como decía la propuesta: ese endpoint exige
**Contents: escritura** en el repositorio destino, es decir, poder empujar
código a la web. `workflow_dispatch` solo pide poder lanzar workflows.

El disparo no lleva datos. La web lee la última release de la API pública de
GitHub, que es la fuente de verdad, así que un aviso perdido no deja una
versión equivocada, solo una tardía. Un horario propio de la web lo repara;
eso lo decide la change hermana. El `deploy.yml` de hoy ya acepta el
disparo, así que funciona antes de que esa change aterrice: redespliega un
sitio igual.

Un aviso fallido no deshace nada. El job queda en rojo con la release ya
publicada, y se vuelve a correr solo.

El token es credencial de una persona y caduca. Lo crea el mantenedor y lo
deja en el entorno `infrastructure` de meltemi-web; Terraform lo coloca en
el entorno `release` de este repositorio, como ya hace con los tokens de
publicación de npm. Rotarlo es cambiar el valor y aplicar.

*Alternativas.* Una GitHub App de la organización: sin caducidad personal y
la mejor opción a largo plazo, pero con más montaje del que este paso
necesita hoy; queda anotada. Solo sondeo desde la web: ninguna credencial,
pero una release tarda hasta un ciclo de sondeo en verse.

### D8 — El sitio sale de este repositorio, y con él `product-site`

Salen `site/`, los jobs `site-lint` y `publish-site`, y los tests `site.rs`
y `scenarios_sitio.rs`. **Antes** se trasladan a `release.rs` los tests que
cubren requisitos que se quedan en `release-distribution`: el renombrado de
instaladores a nombres estables, los scripts de instalación publicados como
assets con su alias, y la base canónica de descarga única entre los
instaladores y `docs/release.md`. La regla de tokens del sitio ya vive en
meltemi-web (`pnpm site:tokens --ide`).

`scripts/capture-desktop.ps1` y `tui/examples/capture_svg.rs` escriben en
`docs/media/`, de donde la web copia. GitHub Pages se apaga y el entorno
`github-pages` se borra; lo hace el mantenedor, porque el recurso del
repositorio no está bajo Terraform (D6).

La capacidad `product-site` sale entera de la verdad viva de este
repositorio: vive en `openspec/specs/product-site` de meltemi-web. No era
de la era de la migración, así que `migration.rs` no la reclama. Si el motor
deja un spec vacío al archivar, la carpeta se borra en la misma tarea.

### D9 — Orden con las dos changes pendientes

**`artefactos-de-cada-push` se archiva primero, tal como está.** Se verificó
en un runner real el 2026-08-09 y sus requisitos son ciertos hoy. Uno de
ellos, «Builds de integración descargables sin crear release», dice que el
camino de release se dispara solo desde un tag; esta change lo modifica.
Hasta que esa change se archive, `meltemi validate release-con-el-push`
reporta **exactamente un** diagnóstico: `modified-requirement-missing` para
ese requisito. Es esperado y queda escrito aquí para que nadie lo lea como un
delta roto. Sus otros tres requisitos siguen siendo ciertos con esta change y
no se tocan.

**`procedencia-de-release` se retira sin fundir sus deltas.** Archivarla
metería en la verdad viva una clave pública en el árbol que no va a existir.
Lo que sigue siendo cierto de ella —la atestación, su alcance declarado, el
log público— entra por esta change como requisito añadido, reescrito. Lo que
deja de ser cierto —la clave, su custodia— no entra nunca. Su carpeta pasa
al histórico con fecha, sin su delta y con una cabecera «Retirada sin
fundir», de modo que la historia conserva su propuesta, su design y sus
tareas y nada pretende haber entrado a la verdad viva. Si el motor o algún
guardián no admite un histórico sin delta, la tarea lo resuelve antes de
seguir.

**`sucesion-y-custodia`** se re-acota cuando le toque: sin clave, lo que se
sucede es el control del repositorio y de la organización. No se edita aquí.

### D10 — La primera release por este camino es 0.2.0

`turno-vacio-es-error` cambió la taxonomía de códigos de salida, y
`docs/versionado.md` clasifica eso como ruptura.

**El orden importa, porque desde el merge cada push a `main` puede
publicar.** La versión del workspace es hoy 0.1.1 y la mayor publicada es
0.1.0, así que lo único que impide publicar 0.1.1 sola es que su tag ya
existe, el del borrador. Por eso:

1. el merge de esta change llega a `main` con la versión intacta; el job
   `version` responde `release=false` porque el tag `v0.1.1` existe, y esa
   corrida es la primera evidencia real de la negativa;
2. el entorno `release`, su secreto y los rulesets ya están aplicados desde
   meltemi-web (D6); si no, GitHub crearía el entorno al vuelo, sin política
   de rama;
3. el mantenedor fija 0.2.0 con el script y empuja: esa es la release;
4. solo entonces se borran el borrador v0.1.1 y su tag, que con `main` en
   0.2.0 ya no protegen nada.

La primera corrida es también la que resuelve lo que este design deja sin
verificar, y su evidencia queda en las tareas.

### D11 — La primera eliminación del proyecto destapa que los dos validadores discrepan

Esta es la primera change del proyecto con un delta `REMOVED`: el histórico
no tiene ninguno. Al escribirla aparecieron dos lecturas del mismo bloque.
`meltemi validate release-con-el-push` lo acepta con su **Reason** y su
**Migration**, como pide `artifact-format`. `validate_tree`, que corre
`dogfood.rs` sobre todo el árbol, pasa los deltas de cada change por la regla
de la verdad viva y le exige a cada bloque eliminado un escenario y un verbo
normativo: dieciocho diagnósticos, nueve bloques por dos reglas.

La lectura correcta es la primera. Un bloque `REMOVED` nombra un requisito que
deja de existir; pedirle escenarios es pedir que se demuestre lo que se está
retirando. `validate_tree` debe saltar las reglas de requisito dentro de una
sección `## REMOVED Requirements` y comprobar allí lo que el formato pide.

La corrección es la primera tarea, y `artifact-format` gana un escenario que
lo deja escrito para los dos validadores, de modo que la próxima eliminación
no tenga que descubrirlo. **Hasta esa tarea, `dogfood.rs` está en rojo en
esta rama**, y solo en ella: `main` no tiene el delta.

## Risks / Trade-offs

- **Una cuenta con escritura publica.** Asumido y declarado (D3). Los
  controles de D5 cierran las puertas de al lado; esta no.
- **Un agente que cambia la versión publica.** Las sesiones de agente empujan
  con las credenciales del mantenedor, así que ningún control por identidad
  las distingue. El control es la regla de `AGENTS.md` (D2), y es blando.
- **Servicios externos en la ruta crítica.** Sigstore y la API de GitHub
  pueden fallar a mitad. Cada paso salta lo hecho (D4), así que re-ejecutar
  termina lo que falte.
- **Una release no se corrige, solo se sustituye** (releases inmutables).
- **Tres corridas de gates por push de versión** (CI, Build, Release). El
  dial: que `build.yml` se salte los pushes de versión. No se adopta hasta
  que el costo lo pida.
- **El token del aviso caduca.** El aviso falla en rojo y la web se repara
  sola por su horario.
- **Verificar exige `gh`.** El paso 1 no lo exige; el paso 2 sí, con y sin
  red.
- **Los instaladores siguen sin firma de plataforma.** Deuda ya declarada.

## Sin verificar al escribir este design

Se comprueban al implementar o en la primera corrida, y no se convierten en
afirmación antes:

- qué forma verifica con `subject-checksums`: `gh attestation verify
  SHA256SUMS`, como dice hoy la documentación, o cada artefacto;
- el nombre de la salida del bundle en la versión pineada de `actions/attest`;
- si un ruleset admite a GitHub Actions como excepción para crear tags `v*`;
- el recurso del proveedor de GitHub para los permisos por defecto de los
  workflows y los campos de política de rama del entorno, en la versión
  pineada en meltemi-web;
- si la organización admite tokens de grano fino sobre sus repositorios y con
  qué caducidad máxima;
- si una release inmutable publicada puede borrarse y si su tag se puede
  reutilizar, que afecta al procedimiento de repudio;
- cómo tratan `meltemi` y los guardianes una change retirada en el histórico
  sin delta;
- ~~si `cargo update --workspace` refresca las versiones del workspace en
  `Cargo.lock` sin tocar dependencias de terceros~~. **Resuelto el
  2026-10-23**: `set-version 0.2.0` sobre un worktree desechable movió los doce
  crates del workspace y ninguna de las 168 dependencias de terceros.
