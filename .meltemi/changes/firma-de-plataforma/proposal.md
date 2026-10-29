# firma-de-plataforma

> Vía completa (proposal → design → specs → tasks). Es la 6º de la tanda
> `entrega-sin-huecos`. Enmienda la política de firma por decisión del
> mantenedor del 2026-10-29: la firma de la release sigue siendo la atestación
> del workflow, sin clave, y las firmas de plataforma (Authenticode del MSI,
> Developer ID con notarización del DMG, la firma del APK) se admiten bajo
> custodia escrita. Los jobs que las usan corren en un entorno `signing`
> limitado a `main`, reciben lo ya construido y no ejecutan pasos de build.

## Why

Los instaladores de escritorio de v0.2.0 avisan en el primer arranque:
SmartScreen dice que protegió el equipo, y Gatekeeper se niega a abrir la app.
`docs/faq.md` explica cómo pasar el aviso, y `docs/release.md` lo declara deuda.
Para el primer perfil del rumbo (el desarrollador que ya usa agentes) es un
peaje menor. Para cualquier otro, es una razón para no seguir. Además el APK
del compañero no se puede publicar: Android no instala un APK sin firma, y una
firma que cambia entre releases impide actualizar.

La política escrita lo prohibía: «no existe clave de firma alguna, y una en un
secreto de Actions sigue prohibida». Esa frase tenía un motivo. SLSA exige que
el material secreto «MUST NOT be accessible to the environment running the
user-defined build steps», y un job que compila con una clave a mano es justo
ese caso. Pero el motivo prohíbe la clave **en el build**, no la firma. La
salida es la que SLSA deja abierta: firmar en jobs que no construyen nada.

## What Changes

- **La política, enmendada** en `docs/release.md` y en el requisito
  «Artefactos firmados con custodia documentada». La firma de la release sigue
  siendo la atestación, sin clave de largo plazo, y verifica igual que hoy. Las
  firmas de plataforma se admiten si cumplen tres condiciones:
  - su custodia está escrita: qué clave, dónde vive, quién la rota y cómo se
    revoca;
  - su material vive en el entorno `signing` (solo `main`) o en un servicio de
    firma;
  - ningún paso que compile puede leerla.
- **Jobs de firma aislados** en `release.yml`, uno por plataforma, en el
  entorno `signing`. Reciben el artefacto ya construido, firman y suben el
  resultado, sin checkout de código que se ejecute y sin `cargo`, `npm` ni
  `tauri`:
  - **Android**: `zipalign` + `apksigner` con la clave del entorno.
  - **macOS**: se extrae la `.app` del DMG, se firma con Developer ID y
    hardened runtime, se rehace el DMG, se firma, se notariza con
    `notarytool --wait` usando una clave de API de App Store Connect, y se
    grapa el ticket.
  - **Windows**: se firma el MSI con un servicio de firma con clave en
    hardware, porque desde 2023 las CA no emiten certificados de firma de
    código con clave exportable. El proveedor lo elige el mantenedor
    (design D4).
- **Una verificación independiente, sin secretos**, antes de publicar, contra
  la identidad esperada fijada en el repositorio (huella del certificado del
  APK, Team ID de Apple, sujeto del certificado de Windows). Usa `apksigner
  verify`, `codesign --verify`, `spctl`, `stapler validate` y
  `Get-AuthenticodeSignature`. Una identidad distinta falla la release.
- **Un interruptor por plataforma**: una variable del repositorio que el
  Terraform de meltemi-web enciende cuando existen los secretos de esa
  plataforma (`ide-delivery-infra`):
  - **apagado**: el instalador sale sin firma de plataforma, como hoy, y el APK
    no sale;
  - **encendido**: el instalador sale firmado y verificado, o la release no
    sale. Nunca se cae a sin firmar en silencio.
- **`capabilities.json` dice lo que se firmó**: la generación pasa al job de
  verificación, con `--signed` de lo que verificó
  (`estado-que-publica-la-release` D4).
- **El inventario de claves** en `docs/release.md`: cada clave, su custodio,
  dónde vive, su caducidad, cómo se rota, cómo se revoca y qué pasa si se
  pierde (la del APK no se puede reemplazar sin que los usuarios reinstalen).
- **`docs/faq.md` y `docs/quickstart.md`**: el aviso de primer arranque se
  describe por plataforma según el interruptor, y desaparece donde ya no
  aplica.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`:
  - **MODIFIED** «Artefactos firmados con custodia documentada»: la enmienda;
  - **ADDED** «Firma de plataforma de los instaladores y del APK».

## Impact

- Archivos: `.github/workflows/release.yml` (jobs de firma y de verificación,
  `package-gui` y `package-android` entregan a la firma),
  `scripts/signing-identities.toml` (nuevo; identidades públicas esperadas),
  `core/meltemid/tests/release.rs` (los controles nuevos),
  `docs/{release.md,faq.md,quickstart.md}`, `README.md`, `LEEME.md`.
- Dependencias de CI: las herramientas de firma de cada runner (Windows SDK,
  Xcode, build-tools de Android). La acción del proveedor de Windows se suma
  pineada por commit cuando el mantenedor lo elija (design D4).
- Lo que pone el mantenedor, en el entorno `signing` y declarado en el
  Terraform de meltemi-web:
  - la membresía del Apple Developer Program, el certificado Developer ID
    Application y una clave de API de App Store Connect;
  - el alta en el servicio de firma de Windows;
  - la clave del APK, generada y respaldada fuera de línea.
- Coste: unos 10 minutos de runner macOS por release (la notarización espera)
  y unos minutos de Windows y Linux. Los builds de integración no cambian.
- Riesgo: que una clave se filtre. Por eso vive en un entorno limitado a `main`
  y en jobs que no ejecutan código del repositorio. La identidad esperada está
  fijada, y la revocación está escrita antes de que haga falta.

## Fuera de alcance

- Firmar los binarios del núcleo (`meltemi`, `meltemid`, los adaptadores) y el
  `.exe` dentro del MSI: firmarlos exige firmar durante el build, donde ninguna
  clave puede estar. Se declara, y se revisa si aparece un servicio que firme
  antes del empaquetado sin exponer la clave al build.
- Firmar el `.deb`: Debian firma repositorios, no paquetes sueltos. Un
  repositorio APT propio es otra change.
- Firmar los builds de integración: siguen sin firma de plataforma, como
  declara `UNSIGNED-BUILD.txt`.
- Reemplazar la atestación: sigue siendo la firma de la release.
