# Design — artefactos-de-cada-push

## Context

El mantenedor pidió que «esto sea automático con cada push». Lo que quiere
descargar —los archivos por plataforma, y sobre todo el DMG que su máquina
Windows no puede construir— ya lo produce el pipeline: `release.yml` los arma y
los sube con `actions/upload-artifact` (187, 289). Lo que no puede volverse
automático es el acto de **publicar una release firmada**, y esa frontera está
ratificada en `procedencia-de-release`: la firma minisign ocurre en la máquina
del mantenedor porque es el único paso que una cuenta de GitHub comprometida no
puede completar, y una clave en un secreto de Actions es el caso que SLSA v1.2
prohíbe en lenguaje normativo. Con **immutable releases** activado, además,
firmar precede a publicar sin orden alternativo.

Este design decide cómo dar lo primero sin rozar lo segundo, y deja escritas las
razones para que nadie «simplifique» más adelante en la dirección equivocada.

Estado verificado del pipeline al escribir esto:

- Cada job de `release.yml` salvo `site-lint` y `publish-site` lleva
  `if: startsWith(github.ref, 'refs/tags/')` (39, 119, 129, 194, 303).
- `publish-site` declara `needs: [site-lint, gates, deny, package, package-gui]`
  (382) y `!cancelled()` en su condición (384).
- `ci.yml` corre en cada push a `main`, en las tres plataformas: `npm ci`,
  `npm audit`, checks del frontend, `cargo fmt --check`, `cargo clippy -D
  warnings`, `cargo build`, `cargo test --workspace` y el lint del sitio; más
  `cargo-deny` en un job propio.

## Goals / Non-Goals

**Goals**: que un push a `main` deje descargables los mismos artefactos por
plataforma que produce el camino de tag; que se distingan de una release a
simple vista y por escrito; que los presupuestos de tamaño los gateen igual; que
el costo esté dicho y sea fácil de bajar; y que el camino que firma y publica
quede exactamente como está.

**Non-Goals**: firmar, atestiguar o publicar desde CI; crear releases de
cualquier tipo, incluidas draft y prerelease; tocar los enlaces libres de
versión del sitio; empaquetar desde pull requests o forks; optimizar el pipeline
existente.

## Decisions

### D1 — Un workflow aparte, no una condición relajada en `release.yml`

La forma obvia —quitar el `if:` de tag a `gates`, `deny`, `package` y
`package-gui`— se descarta por dos hallazgos concretos, no por gusto.

**El primero es `publish-site`.** Declara `needs: [site-lint, gates, deny,
package, package-gui]`, y `needs` es una dependencia dura: un job espera a sus
`needs` aunque el `if:` de esos jobs los salte. Hoy en `main` los cuatro se
saltan al instante y el sitio se publica en cuanto pasa su lint. Si empiezan a
correr, **cada publicación del sitio pasa a esperar un build de tres plataformas
con Tauri**. Peor: la condición lleva `!cancelled()`, así que en cuanto alguien
cancele el run largo —y un run largo en cada push se cancela— la publicación del
sitio se salta con él. Se rompería, de rebote, la única cosa que `main` hacía.

**El segundo es el duplicado.** `gates` corre fmt, clippy, la suite completa y
el release build en tres plataformas; `deny` corre `cargo-deny`. `ci.yml` ya
corre exactamente eso —los mismos comandos, las mismas tres plataformas— en cada
push a `main`. Encenderlos en `main` sería pagar dos veces la parte más cara del
día para no aprender nada nuevo.

Se elige entonces `.github/workflows/build.yml`: archivo propio, disparador
propio, jobs propios. Tres consecuencias buenas y una mala, todas asumidas:

- El grafo de `release.yml` **no cambia ni una línea**. La forma más segura de
  no debilitar el camino firmado es no editarlo. Es la propiedad que más vale de
  esta decisión.
- La cadencia se cambia editando el bloque `on:` de un archivo (D5).
- El costo es visible como workflow propio en la pestaña Actions, no escondido
  dentro del que dice «Release».
- **La mala**: dos archivos pueden divergir, y un ensayo que ya no ensaya lo
  mismo no ensaya nada. No se resuelve con disciplina sino con test: la
  correspondencia de presupuestos, nombres estables y adaptadores queda pineada
  en `core/meltemid/tests/release.rs` (D6).

Se consideró y se descarta un tercer camino, extraer el empaquetado a un
workflow reusable (`workflow_call`) invocado desde ambos: es la solución
correcta al problema de la divergencia, y también una reestructuración del
archivo del que depende la única release publicada, sin poder probarla sin
empujar. No es el momento; queda anotada como promoción con evidencia si la
divergencia llega a doler.

### D2 — La capability es `release-distribution`, y el requisito es una frontera

Un artefacto de run no es distribución —no tiene URL estable, caduca, y GitHub
lo sirve tras autenticación—, así que la tentación es decir que no pertenece a
`release-distribution`. Se decide lo contrario, por dos razones.

La primera es de hecho: `release-distribution` es la capability que posee el
pipeline de empaquetado, sus gates duros y sus presupuestos de tamaño. Los
requisitos nuevos hablan de ese pipeline.

La segunda es de propósito: lo que estos requisitos fijan es **la línea que el
empaquetado no puede cruzar**. «Produce artefactos y no crea release», «lleva
presupuestos aunque no publique», «declara que no está firmado» son
afirmaciones que solo significan algo al lado de los requisitos que definen qué
es una release firmada. Alojarlas en otra capability las dejaría huérfanas de
aquello contra lo que se contrastan. No se crea capability nueva: sería
inventar superficie normativa para infraestructura de repositorio.

### D3 — El artefacto lleva el commit; los archivos de dentro conservan su nombre estable

Dos capas, dos reglas opuestas y ambas deliberadas.

**El artefacto del run** se llama `meltemi-unsigned-<SO>-<sha-corto>`. Lleva el
commit porque un build que no dice de dónde salió no se puede volver a
producir, y en una lista de runs el SHA es lo único que distingue el de hace
diez minutos del de ayer. Lleva `unsigned` porque esa es exactamente la
propiedad que lo separa de una release, y el nombre del `.zip` que GitHub
descarga es lo primero —a veces lo único— que un humano lee.

**Los archivos de dentro** conservan los nombres estables y libres de versión:
`meltemi-macOS.tar.gz`, `meltemi-desktop-macOS.dmg`, `meltemi-Windows.zip`. Es
tentador marcarlos también, y sería un error: el valor entero de esta ruta es
producir **lo mismo** que produce el camino de tag, incluido el paso de
normalización de nombres que `sitio-web-producto` D2 exige. Un archivo renombrado
dejaría de ensayar ese paso, que es justamente uno de los que más se puede
romper sin que nadie lo note hasta el tag.

Esto no contradice el requisito vigente «Nombres de artefacto estables por
plataforma», que habla de **artefacto publicado**: aquí no se publica nada, y el
requisito nuevo lo dice con esas palabras para que la lectura conjunta no
dependa de la buena fe del lector.

El `SHA256SUMS` sí se genera, porque el paso de checksum es parte de lo que se
ensaya y porque quien descarga dos artefactos quiere saber que el DMG es el DMG.
Pero un `SHA256SUMS` sin `.minisig` al lado es precisamente la forma que invita a
verificar y no encuentra contra qué: por eso el aviso viaja en el mismo
directorio y lo dice en su primera línea (D4).

### D4 — La ausencia de firma se declara donde se descarga, no solo donde se documenta

Tres sitios, en orden de probabilidad de ser leídos:

1. **El resumen del run** (`$GITHUB_STEP_SUMMARY`), que es la misma página donde
   está el botón de descarga. Es el único lugar que el mantenedor ve seguro.
2. **`UNSIGNED-BUILD.txt` dentro del artefacto**, junto a los binarios, para que
   sobreviva a la descarga y al reenvío.
3. **`docs/release.md`**, para quien llega por la documentación.

El texto vive en `scripts/unsigned-build-notice.txt`, no incrustado en el YAML:
así es revisable en un diff, testeable desde Rust y no se escribe dos veces
(el workflow lo `cat`ea a los dos destinos). Dice qué **no** tiene —firma,
atestación, publicación—, por qué (la clave nunca toca CI, `docs/release.md`
«Key custody») y adónde ir para instalar de verdad. Va en inglés, como el resto
de `docs/release.md` y de los workflows: no es superficie de producto sujeta a
la i18n de §11, es un archivo de build.

Detalle de implementación decidido por el lint del repositorio: el aviso se
escribe con `cat` de un archivo y `echo`s, nunca con heredoc. Un heredoc
necesita su terminador en la columna 0 y eso rompe el bloque YAML —el mismo
tropiezo que `release.yml` ya documenta en su gate de tamaño (líneas 235-237) y
que `every_workflow_script_stays_inside_its_block` vigila.

### D5 — Un job por plataforma, cadencia con dos diales, retención de 7 días

**Un job por plataforma, no los dos del camino de tag.** Allí `package` y
`package-gui` están separados porque un bundle roto no debe ocultar un archivo
sano y porque paralelizan. Aquí se fusionan: mismo artefacto, la mitad de
runners, y el `cargo build --workspace --release` se comparte con el `tauri
build` en vez de pagarse dos veces por plataforma. La secuencia es la unión de
ambos jobs: dependencias del sistema, `npm ci`, `npm run build` (que
`tauri::generate_context!` necesita antes de compilar `desktop/`), release
build, empaquetado del núcleo, `tauri build`, presupuestos, normalización,
checksums, aviso, subida.

**Costo, sin maquillar.** Hoy un push a `main` paga la matriz de `ci.yml` (3
jobs), `cargo-deny`, el lint del sitio y la publicación. Esta change añade tres
jobs con un release build y un bundle de Tauri cada uno: es el gasto más caro que
se le haya puesto a `main`. Cuánto tarda de verdad no se sabe sin empujar, y no
se va a inventar aquí una cifra.

**Dos diales, a mano y sin reestructurar.** El bloque `on:` es el primero:
cambiar `push: branches: [main]` por `schedule:` —o dejar solo el
`workflow_dispatch`, que se incluye desde el día uno— es editar cuatro líneas de
un archivo que nada más usa. La matriz de plataformas es el segundo: dejar solo
`macos-latest` cubriría el caso que motivó la petición a un tercio del costo. Se
declaran los dos aquí para que la change que los use no tenga que justificar una
reestructuración.

**Retención: 7 días.** El propósito es «probar el último build»; un build de
hace dos semanas no es el último, y a nadie le sirve. Además acota el
almacenamiento sin depender de qué plan tenga el repositorio. Es un número, está
en el YAML, y el requisito exige que sea acotado y declarado, no que sea siete.

**Tres ajustes decididos al implementar**, los tres en la línea de esta
decisión y anotados aquí para que no queden solo en un comentario del YAML:

- `concurrency` con `cancel-in-progress`, agrupado por ref. Si el propósito es
  «el último build», un run que ya fue superado por otro push es gasto puro.
  Es el tercer dial y el único que no hay que girar a mano.
- `fail-fast: false` en la matriz. En el camino de tag la matriz aborta entera
  porque una release parcial no existe; aquí es al revés: un fallo en Linux no
  debe llevarse por delante el DMG de macOS, que es el artefacto que motivó
  todo esto.
- `scripts/install.sh` y `install.ps1` **no** viajan en el artefacto, a
  diferencia del camino de tag. Instalan desde la última release publicada, y
  un instalador de release al lado de un build sin firmar es exactamente la
  confusión que `UNSIGNED-BUILD.txt` existe para evitar. No se ensaya ese
  `cp` porque ensayarlo tendría un costo peor que el que evita.

### D6 — La imposibilidad de publicar es estructural, y el test la vigila por el lado negativo

El workflow declara `permissions: contents: read` en su cabecera. No es
decoración: con ese permiso, un paso que intentara `gh release create` fallaría
aunque alguien lo añadiera por descuido. La garantía queda en el mecanismo, no
en un comentario que pide buena conducta.

Los tests de `core/meltemid/tests/release.rs` se escriben en el estilo que ya
usa el archivo —leer los ficheros del repositorio y afirmar sobre su texto— y se
reparten en dos mitades:

- **Afirmativa**: `build.yml` se dispara en `main`, corre las tres plataformas,
  nombra los tres presupuestos con **los mismos valores** que `release.yml`,
  mete los dos adaptadores ACP en los tres archivos del núcleo, nombra el
  artefacto con `unsigned` y un SHA corto, y declara `retention-days`.
- **Negativa, que es la que de verdad protege**: `build.yml` **no** contiene
  `gh release`, ni `actions/attest`, ni `SHA256SUMS.minisig`, ni
  `contents: write`; y `release.yml` **sigue** condicionando su job `release` a
  `startsWith(github.ref, 'refs/tags/')`. Si alguien convierte esta ruta en una
  ruta de publicación, la suite se pone roja antes que la release.

El test de correspondencia de presupuestos es el que ataca el riesgo de D1: si
un valor sube en un archivo y no en el otro, falla nombrando los dos.

### D7 — Elegibilidad fast-forward: por criterio, con tripwire

El criterio del motor (`fast_forward_eligible`): ninguna capability nueva y
deltas sin MODIFIED ni REMOVED. Esta change cumple los dos genuinamente.
`release-distribution` existe en la verdad viva, y los cuatro requisitos son
superficie normativa que **no existía**: el pipeline nunca prometió nada sobre lo
que produce fuera de una release, los presupuestos nunca se declararon
aplicables fuera del camino de publicación, y nadie había escrito que un
artefacto no firmado deba decirlo. Ningún requisito vigente necesita
reescribirse: «Nombres de artefacto estables por plataforma» y «Pipeline de
release con gates duros» hablan de artefactos **publicados** y del **release**, y
esta ruta no publica ni releasea (D3).

Tripwire declarado: si al implementar apareciera la necesidad de enmendar un
requisito vigente —por ejemplo, si resultara imposible correr los presupuestos
fuera del camino de tag sin retocar el requisito de gates duros—, esa enmienda
sale a su propia change o esta pasa a spec-full. Se dice ahora, no se descubre
después.

Nota de convivencia: `procedencia-de-release` está activa y también delta
`release-distribution`, con un MODIFIED sobre «Artefactos firmados con custodia
documentada». Los cuatro requisitos de esta change son ADDED y de nombre
distinto, así que las dos deltas se aplican sin tocarse. Esta change **no**
reproduce ni adelanta aquel MODIFIED.

### D8 — Qué queda como verificación documentada

Un pipeline no se prueba del todo sin empujar, y esta change no empuja. La
partición honesta, en la línea que `conformidad-manual` estableció:

- **Automatizado** (`cargo test`): la estructura del workflow, sus disparadores,
  la igualdad de presupuestos entre los dos archivos, el nombre del artefacto,
  la retención, el conjunto negativo, el contenido del aviso, y que el YAML
  siga pasando el lint de bloques.
- **Verificación documentada, con verify-mark**: que los tres jobs completen en
  un runner real, que el DMG de macOS salga por esta ruta igual que por la de
  tag, y el tiempo de pared por plataforma. Se anota en la tarea de verificación
  como pendiente del primer push a `main` tras el merge, con lo que hay que
  mirar.
