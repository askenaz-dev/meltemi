# Design — apk-en-la-release

## Context

El build de Android ya se hizo una vez, y su diario está en las tareas 3.1 y
3.2 de `companero-movil` (archivada) y en
`docs/qa/2026-10-02-companero-movil-smoke.md`. El toolchain medido fue:

- SDK con plataformas 33–36;
- NDK 27.0.12077973;
- JDK 21;
- Tauri CLI nativo 2.12.1.

El APK de release arm64 sin firmar midió 10,69 MB. Firmado con `zipalign` y
`apksigner` sin tocar el proyecto generado, midió 10,75 MB, y verificó con los
esquemas v2 y v3.

El fallo que costó encontrar sigue escrito en `mobile/README.md`. Un proyecto
Gradle generado desde el CLI de npm registra `node` como el comando al que
volver, y Gradle no puede lanzarlo. Con el CLI nativo registra `cargo`. CI
tiene que usar el nativo.

## Decisions

### D1 — Un job propio en `release.yml`, una pierna en la matriz de `build.yml`

En la ruta de release, `package-android` es un job aparte, en paralelo a
`package-gui` y con las mismas dependencias (`version`, `gates`, `deny`). Así
un APK roto no tapa un instalador sano, que es la misma razón por la que
`package` y `package-gui` están separados.

En la ruta de integración, la matriz de `build.yml` gana una entrada
`android`, que corre sobre `ubuntu-latest` y salta los pasos de escritorio. Es
la forma que ya tiene el archivo, con un solo job y una pierna por plataforma.
Si los `if:` lo hacen ilegible, se separa en un job `build-android`, y lo
decide la tarea 1.2 al escribirlo.

### D2 — El toolchain, pineado como en la medición

| Pieza | Cómo |
| --- | --- |
| JDK 21 | `actions/setup-java`, pineada por commit (dependencia nueva de CI: la imagen trae varios JDK, y el que gana en el `PATH` cambia entre imágenes) |
| SDK | el de la imagen del runner (`$ANDROID_HOME`); `sdkmanager` instala `platforms;android-36` si falta |
| NDK | `sdkmanager "ndk;27.0.12077973"`, y `NDK_HOME` apuntando a él |
| Rust | `rustup target add aarch64-linux-android` sobre el toolchain pineado |
| Tauri CLI | `cargo install tauri-cli --version 2.12.1 --locked`, con la caché de cargo |
| Frontend | `npm ci --prefix desktop/ui`; `beforeBuildCommand` ya lo construye |

Los comandos son los de `mobile/README.md`: `cargo tauri android init` y luego
`cargo tauri android build --apk --target aarch64`, desde `mobile/`. `gen/`
sigue sin versionarse: se regenera en cada ejecución, como en local.

### D3 — El techo, igual en los dos workflows

`MELTEMI_APK_BUDGET_BYTES: "26214400"` (25 MiB, el techo declarado por
`companero-movil`) en el `env` de los dos workflows. El paso de medida falla
con el tamaño y el techo, como los otros presupuestos. El test de
`core/meltemid/tests/release.rs` que ya exige que los presupuestos coincidan
entre `build.yml` y `release.yml` suma este. Se mide el APK sin firmar, porque
la firma añade unos 60 KB y es igual en las dos rutas. El resumen del job
imprime el tamaño y lo que pesa la biblioteca nativa.

### D4 — Firmado efímero en integración, firma de publicación en la release

Un APK sin firma no se instala, y un build de integración que no se puede
instalar no sirve para probar nada. En `build.yml` se hace esto:

1. se genera una clave con `keytool -genkeypair` en el directorio temporal del
   runner, con una contraseña aleatoria de la ejecución;
2. se firma con `zipalign` + `apksigner` y se verifica;
3. se borra la clave;
4. `UNSIGNED-BUILD.txt` gana un párrafo: el APK lleva una firma efímera,
   generada y destruida en esa ejecución, que no prueba origen y no coincide
   con la de la release, así que Android no lo instalará encima de un
   compañero de release.

Ese último efecto es deseable, porque hace imposible confundir los dos.

En `release.yml`, `package-android` sube el APK sin firmar como artefacto
`android-unsigned`. El nombre no empieza por `meltemi-` a propósito: el
`pattern: meltemi-*` de `publish` no lo recoge. Lo consume el job de firma de
`firma-de-plataforma`, que sube `meltemi-companion-Android` firmado y
verificado. Sin ese job, o sin su clave, la release sale sin APK, y el job de
firma lo dice en su resumen.

Firmar con la clave efímera en la release está descartado: publicaría algo que
no se puede actualizar nunca, porque cada release tendría otra clave y Android
rechaza la actualización.

### D5 — El coste, y su dial

La pierna de Android en `build.yml` cuesta unos 15 minutos de runner Linux por
push, con caché, que es lo más barato que hay. Se acepta, porque es lo que
convierte «un cambio rompe Android sin que nadie lo vea» en un build rojo. El
dial existe y ya está escrito en la cabecera de `build.yml`: la matriz.

### D6 — Clippy con el NDK, donde ya está

`cargo clippy -p meltemi-mobile --target aarch64-linux-android -- -D warnings`
corre en el job de Android, antes del build. Ponerlo en `ci.yml` obligaría a
montar el NDK en una cuarta pierna solo para clippy, porque las dependencias
del cliente SSH compilan C para el target. Revisar en el mismo job que compila
cuesta segundos.

### D7 — La pierna en un teléfono

`docs/prueba-de-uno.md` gana un paso 12 opcional, que se escribe «no corrido»
si no hay teléfono:

- instalar `meltemi-companion-Android.apk` desde la release;
- conectar por el túnel SSH con la clave del compañero (`mobile-companion`:
  el compañero habla SSH por sí mismo);
- ver la flota;
- aprobar una petición de permiso desde el teléfono.

El informe del teléfono va en `docs/qa/`, con el modelo, la versión de Android
y lo que no se pudo ejercer. Mientras no exista ese informe, la nota del
compañero en `docs/capacidades.toml` no puede decir «probado en un teléfono».

## Risks / Trade-offs

- **La imagen del runner cambia el SDK.** El NDK y el JDK van pineados. El SDK
  se completa con `sdkmanager` si falta la plataforma. Un cambio de imagen que
  rompa el build se ve en la siguiente ejecución, no en la release, que es el
  punto de tener la pierna de integración.
- **Tauri cambia la ruta de salida del APK.** El paso busca el único `*.apk`
  bajo `gen/android/app/build/outputs/apk/` y falla si hay cero o más de uno,
  nombrando lo que encontró.
- **La release sale sin APK hasta que haya clave.** Es lo honesto: el estado
  del compañero sigue «en desarrollo» y lo dice.
