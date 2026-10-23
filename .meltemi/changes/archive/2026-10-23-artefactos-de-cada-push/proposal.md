# artefactos-de-cada-push

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre una capability existente,
> ninguna capability nueva, ningún MODIFIED ni REMOVED (design D7). Alcance de
> un día: un workflow nuevo, sus tests y una sección de documentación.

## Why

El mantenedor lo pidió en una frase: «Me gustaría que esto sea automático con
cada push (para eso tenemos CICD)». Quiere probar el build del día —el DMG de
macOS en particular, que no puede construir en su máquina Windows— sin
ceremonia, cada vez que `main` se mueve.

Lo que **no** puede volverse automático es la release firmada, y no por
comodidad: `procedencia-de-release` lo decidió con lenguaje normativo detrás.
Una cuenta de GitHub comprometida empuja un tag, CI construye, la atestación se
acuña y **verifica perfectamente** —porque registra fielmente el commit del
atacante. La firma manual en una máquina que GitHub no controla es el único
paso que esa cuenta no puede completar. Y la clave en un secreto de Actions es
el caso que SLSA v1.2 prohíbe textualmente («MUST NOT be accessible to the
environment running the user-defined build steps»). A eso se suma que este
repositorio tiene **immutable releases** activado: publicada una release, sus
assets no admiten añadidos, así que firmar precede a publicar y no hay orden
alternativo. Nada de esto se toca aquí.

Pero entre «release firmada» y «nada» hay un escalón que el pipeline ya casi
tiene construido. `release.yml` empaqueta los seis artefactos por plataforma y
**ya los sube con `actions/upload-artifact`** (líneas 187 y 289) antes de que el
job `release` los baje y los fusione. Es decir: la ruta que produce exactamente
lo que el mantenedor quiere descargar existe, funciona, y está apagada fuera de
un tag —cada job lleva `if: startsWith(github.ref, 'refs/tags/')` (39, 119,
129, 194, 303). En un push a `main` hoy solo corren `site-lint` y
`publish-site`. El último metro que falta no es empaquetar: es **dejar que la
ruta de empaquetado corra también en `main` y que su salida quede descargable
desde la página del run**, sin release, sin versión, sin firma y sin insinuar
ninguna de las tres.

La distinción importa porque un artefacto de run **no es distribución**. No
tiene URL estable, caduca, no aparece en `releases/latest`, y GitHub lo sirve
tras autenticación. Es exactamente la forma correcta de «probar un build» y
exactamente la forma incorrecta de instalar Meltemi — y esta change escribe esa
frontera en vez de dejarla a la intuición del que descarga.

## What Changes

- **Workflow nuevo `.github/workflows/build.yml`**, disparado por push a `main`
  y por `workflow_dispatch`. Un job por plataforma que construye en modo
  release, arma el archivo del núcleo (`meltemi`, `meltemid` y los dos
  adaptadores ACP, como manda `adaptadores-propios-acp`), construye el
  instalador de la GUI con `tauri build`, **mide los tres presupuestos** y sube
  todo como artefacto del run.
- **`release.yml` no se toca en su grafo de jobs.** La ruta que crea releases
  sigue siendo solo de tag; la ruta nueva vive en su propio archivo con su
  propio disparador. La razón es medible, no estética: `publish-site` declara
  `needs: [site-lint, gates, deny, package, package-gui]` (línea 382), y `needs`
  no es condicional —encender el empaquetado en `main` dentro de ese archivo
  haría que **cada publicación del sitio esperara un build de tres plataformas
  con Tauri**, y con `!cancelled()` en su condición (384), cancelar el build
  ahora caro se llevaría por delante la publicación del sitio que hoy es lo
  único que `main` hace. Además, `gates` y `deny` en `main` serían un duplicado
  literal de `ci.yml`, que ya corre fmt, clippy, build, la suite y el lint del
  sitio en las tres plataformas, más `cargo-deny`, en cada push a `main`
  (design D1).
- **El artefacto dice lo que es**: `meltemi-unsigned-<SO>-<sha-corto>`. El
  nombre lleva el commit porque un build sin commit es un build que nadie puede
  volver a producir, y lleva `unsigned` porque esa es la propiedad que lo separa
  de una release. Los **archivos dentro conservan los nombres estables** —
  `meltemi-macOS.tar.gz`, `meltemi-desktop-macOS.dmg`— porque el valor del
  ensayo es producir exactamente lo que produce el tag (design D3).
- **Aviso de «sin firmar» donde el humano mira**: un `UNSIGNED-BUILD.txt` dentro
  del artefacto y el mismo texto en el resumen del run, que es la página donde
  está el botón de descarga. Dice qué no tiene (firma, atestación, publicación)
  y adónde ir para instalar de verdad. El texto vive en `scripts/` como archivo
  del árbol, revisable y testeable, no incrustado en el YAML.
- **Los presupuestos gatean esta ruta también**:
  `MELTEMI_TUI_BUDGET_BYTES`, `MELTEMI_GUI_INSTALLER_BUDGET_BYTES` y
  `MELTEMI_ADAPTER_BUDGET_BYTES` con los mismos valores y el mismo `exit 1`. Un
  build que nadie mide es exactamente como se pudre un presupuesto: creciendo un
  poco por commit hasta que el tag lo descubre.
- **`permissions: contents: read` en la cabecera del workflow**, para que la
  ruta nueva no pueda crear una release aunque alguien le añada un paso mañana.
  La imposibilidad es estructural, no una promesa en un comentario.
- **Retención de 7 días**, declarada y justificada: el propósito es «probar el
  último build», y un build de hace dos semanas no es el último.
- **Tests en `core/meltemid/tests/release.rs`** que pinean lo que la change
  afirma: que el empaquetado corre en `main`, que el nombre lleva commit y
  `unsigned`, que los presupuestos son los mismos en los dos archivos, que los
  adaptadores viajan en los tres archivos, y —el conjunto negativo, el que
  importa— que `build.yml` **no** contiene creación de release, ni atestación,
  ni `.minisig`, ni `contents: write`.
- **`docs/release.md`** gana una sección corta que distingue las dos rutas para
  quien lea la documentación en vez del aviso.

## Capabilities

### Modified Capabilities

- `release-distribution`: + cuatro requisitos ADDED sobre la frontera entre
  empaquetar y publicar — builds de integración descargables sin crear release,
  su identidad y caducidad, los presupuestos aplicados a toda ruta que
  empaquete, y la declaración de ausencia de firma donde se descarga. La
  capability que ya posee el pipeline y sus presupuestos es la que debe poseer
  la línea que el pipeline no puede cruzar (design D2).

### New Capabilities

- Ninguna.

## Impact

- Archivos: `.github/workflows/build.yml` (nuevo),
  `scripts/unsigned-build-notice.txt` (nuevo), `core/meltemid/tests/release.rs`,
  `docs/release.md`, `docs/plan-de-cambios.md`. **`release.yml` no cambia.**
- **Costo, dicho y no escondido**: cada push a `main` gana tres jobs que hoy no
  existen, cada uno con un `cargo build --workspace --release` y un `tauri
  build`. Es el gasto más caro que este repositorio haya añadido a `main`, y se
  asume porque el mantenedor lo pidió sabiendo lo que pedía. Se abarata donde se
  puede: un job por plataforma en vez de los dos del camino de tag (el build
  release se comparte con el bundle en vez de compilarse dos veces), y sin
  duplicar los gates que `ci.yml` ya corre sobre el mismo commit. Y se deja con
  dos diales a mano —el bloque `on:` y la matriz de plataformas— para que
  bajarlo a `workflow_dispatch` o a `schedule` sea editar cuatro líneas de un
  archivo, no reestructurar un pipeline (design D5).
- Cero dependencias nuevas; ningún crate, ninguna action de terceros que no use
  ya `release.yml`. El contrato `proto/` no se mueve, el daemon no gana
  capacidad y por tanto **no nace deber de paridad §4**: esto es infraestructura
  del repositorio, no superficie de producto.
- Los enlaces de descarga del sitio (`releases/latest/download/…`) siguen
  resolviendo a la última release **firmada**, porque esta ruta no crea, no
  modifica y no publica release alguna. El test lo pinea por el lado negativo.
- **Lo que solo un push real puede confirmar**: que los tres jobs completan
  dentro del tiempo de runner, que el `tauri build` de macOS produce el DMG en
  esta ruta igual que en la de tag, y cuánto tarda de verdad. Se declara ahora
  para que no se lea como verificado.

## Fuera de alcance

- **Firmar, atestiguar o publicar nada desde esta ruta.** Es el punto entero:
  `procedencia-de-release` decidió que la firma es manual y no se relitiga aquí.
- **Prereleases, tags `nightly` o cualquier release automática**, incluida la
  marcada como draft: contaminaría el espacio de versiones y pondría en riesgo
  `releases/latest`, que es lo que el sitio y los dos README prometen.
- **Builds en pull requests o en ramas de trabajo**: el mantenedor pidió `main`,
  y un fork abriría la ruta a ejecutar empaquetado con código de terceros. Si se
  quiere, es otra change con su análisis de amenaza.
- **Caché compartida entre `ci.yml` y `build.yml`**: `Swatinem/rust-cache` ya
  gestiona lo suyo por job; afinarlo es optimización con evidencia, no de
  entrada.
- **Instaladores para plataformas que el camino de tag no produce** (`.rpm`,
  AppImage): siguen fuera por las razones de `instaladores-linux-sin-webview`.
- **Firma de plataforma** (Authenticode, notarización): deuda declarada en
  `docs/release.md`, ajena a esta change.
