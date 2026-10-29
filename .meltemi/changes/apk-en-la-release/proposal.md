# apk-en-la-release

> Vía completa (proposal → design → specs → tasks). Es la 5º de la tanda
> `entrega-sin-huecos`. El compañero móvil se compila en CI. Cada push a `main`
> deja un APK descargable, y la release lo publica cuando está firmado, con el
> techo de 25 MB como gate en las dos rutas. Suma la pierna que faltaba: el
> compañero en un teléfono real.

## Why

`companero-movil` se archivó el 2026-10-28 con 8/8 tareas y 7/7 en verify. El
APK de release arm64 mide 10,69 MB contra un techo declarado de 25 MB, y se
instaló y arrancó en un emulador. Todo eso se hizo en la máquina del
mantenedor. Ningún workflow lo nombra:

- `ci.yml` corre clippy y la suite sobre el crate `mobile` para el host, nunca
  para Android;
- `build.yml` deja el núcleo y los instaladores de escritorio de cada push,
  pero no el APK;
- `release.yml` publica todo lo demás y no el APK.

Las consecuencias:

- Un cambio que rompe el build de Android no lo ve nadie hasta que alguien
  compila a mano.
- El techo de 25 MB se midió una vez y nadie lo vuelve a medir.
- Quien quiere probar el compañero tiene que montar el SDK, el NDK, un JDK y el
  CLI nativo de Tauri.

La firma es la otra mitad. Un APK sin firma no se instala, y firmarlo exigía una
clave que la política de release prohibía tener en CI. Esa política se enmienda
en `firma-de-plataforma`. Esta change construye y mide; aquella firma. Con las
dos, la release publica el APK.

## What Changes

- **El build de Android en las dos rutas**: un job en `release.yml` y una
  pierna en `build.yml`, sobre `ubuntu-latest`, con:
  - JDK 21 y el NDK pineado (27.0.12077973, el de la medición);
  - el target `aarch64-linux-android`;
  - el CLI nativo de Tauri pineado e instalado con `--locked`, como exige
    `mobile/README.md`;
  - `cargo tauri android init` y `cargo tauri android build --apk --target
    aarch64`.

  La única ABI es `arm64-v8a`, la de la medición; `x86_64` sigue siendo solo
  para el emulador.
- **El techo como gate**: `MELTEMI_APK_BUDGET_BYTES` (25 MiB) con el mismo
  valor en los dos workflows, fijado por el test que ya ata los otros
  presupuestos. Pasarlo falla el build.
- **El nombre estable**: `meltemi-companion-Android.apk`.
- **En la ruta de integración, instalable**: firmado con una clave efímera
  generada en la propia ejecución y descartada al terminar, con el aviso de
  `UNSIGNED-BUILD.txt` ampliado para decirlo. Esa firma no prueba origen y no
  sirve para actualizar un compañero instalado desde una release.
- **En la release, solo firmado**: el job deja el APK sin firmar como artefacto
  interno, que `publish` no recoge. Lo recoge el job de firma de
  `firma-de-plataforma`, y solo un APK firmado y verificado llega a la release.
  Mientras no haya clave, la release sale sin APK y el resto no cambia.
- **Clippy para Android**: el crate `mobile` se revisa también para su target,
  en el job que ya tiene el NDK, para que un `cfg(target_os = "android")` roto
  no espere a la release.
- **La pierna en un teléfono real**: un paso opcional en
  `docs/prueba-de-uno.md` (instalar el APK de la release, conectar por el
  túnel, aprobar una petición desde el teléfono) y su informe en `docs/qa/`.
- **Los documentos**: `mobile/README.md` (CI ya lo compila; el build local
  sigue igual), `docs/release.md` (el asset y su firma) y `docs/capacidades.toml`
  (el asset del compañero, cuando `estado-que-publica-la-release` exista).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisito «El compañero móvil en las dos rutas de
  build».
- `mobile-companion`: + requisito «Probado en un teléfono antes de publicarse».

## Impact

- Archivos: `.github/workflows/{build.yml,release.yml}`,
  `scripts/unsigned-build-notice.txt`, `core/meltemid/tests/release.rs` (el
  presupuesto nuevo), `mobile/README.md`, `docs/release.md`,
  `docs/prueba-de-uno.md`, `docs/qa/`.
- Dependencias de CI nuevas: `actions/setup-java`, pineada por commit (design
  D2). El SDK de Android y `sdkmanager` vienen en la imagen del runner; el NDK
  se instala pineado. Ningún crate nuevo.
- Coste: unos 15 minutos de runner Linux por push en `build.yml` (design D5).
- Lo que pone el mantenedor: nada para construir. Para publicar, la clave del
  APK (`firma-de-plataforma`). Para la pierna, un teléfono Android con el
  túnel SSH a su máquina.

## Fuera de alcance

- Google Play, F-Droid o cualquier tienda: el APK se publica en la release,
  como todo lo demás.
- iOS: el compañero es Android primero (`mobile-companion`).
- Un AAB: lo piden las tiendas, no la instalación directa.
- La firma del APK: es de `firma-de-plataforma`.
