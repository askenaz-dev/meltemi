# Design — firma-de-plataforma

## Context

`release.yml` construye en `package` (núcleo) y `package-gui` (instaladores),
y `apk-en-la-release` suma `package-android`, que entrega un APK sin firmar
como `android-unsigned`. `publish` recoge todo lo que se llame `meltemi-*`,
escribe un `SHA256SUMS`, lo atesta y publica. El entorno `release` (solo
`main`) guarda las credenciales de publicación.

La regla que hay que conservar es la de SLSA que cita `docs/release.md`: el
material secreto no puede ser accesible al entorno que ejecuta los pasos de
build definidos por el usuario. En GitHub Actions, el límite de aislamiento
real es el job, porque cada uno corre en una máquina nueva. Un job que no hace
checkout del código, no ejecuta nada que el build produjo y solo invoca
herramientas de firma del sistema sobre un artefacto descargado no ejecuta
pasos de build.

## Decisions

### D1 — La enmienda, en una frase que se pueda comprobar

> La firma de la release es la atestación del workflow, sin clave de largo
> plazo. Las firmas de plataforma se admiten con su custodia escrita. Su
> material vive en el entorno `signing`, limitado a `main`, o en un servicio de
> firma, y solo lo leen jobs que no compilan.

`docs/release.md` reemplaza «No signing key exists» por esa regla y por el
inventario (D7). El escenario «Ninguna clave de firma que custodiar» pasa a
decir «de release», y un test estático de `core/meltemid/tests/release.rs`
comprueba tres cosas:

- ningún job con `environment: signing` hace `actions/checkout` ni corre
  `cargo`, `npm`, `npx`, `tauri` o `node`;
- ningún otro job referencia los secretos de firma;
- `build.yml` nunca nombra el entorno `signing`.

*Alternativa descartada*: firmar dentro de `package-gui` con la integración
nativa de Tauri. Es lo más simple y lo que hace casi todo el mundo, y pone la
clave en el job que corre `npm ci` y `cargo build` con cientos de dependencias.
Es exactamente el caso que la política prohibía con razón.

### D2 — Un camino por artefacto, y un interruptor por plataforma

Tres variables de repositorio, `MELTEMI_SIGN_WINDOWS`, `MELTEMI_SIGN_MACOS` y
`MELTEMI_SIGN_ANDROID`, con valor `on` u otro. Las declara el Terraform de
meltemi-web, que las enciende solo cuando los secretos de esa plataforma están
presentes (`ide-delivery-infra`). Así el interruptor y la credencial cambian en
el mismo `apply`.

| Interruptor | `package-gui` / `package-android` suben | Firma | Llega a `publish` |
| --- | --- | --- | --- |
| apagado | `meltemi-gui-<os>` (como hoy); el APK, `android-unsigned` | no corre | el instalador sin firmar; el APK no |
| encendido | `unsigned-gui-<os>` / `android-unsigned` (fuera del patrón `meltemi-*`) | `sign-<plataforma>` sube `signed-<plataforma>`; `verify-signatures` lo verifica y lo re-sube como `meltemi-gui-<os>` o `meltemi-companion-Android` | solo lo verificado |

`publish` depende de `verify-signatures`. Si el interruptor está encendido y la
firma o la verificación fallan, `publish` no corre: la release no sale. Nunca
se cae a sin firmar en silencio. Un mantenedor que necesite publicar sin firma
apaga el interruptor en el Terraform, que es un gesto visible y revisado.

### D3 — macOS: firmar la app, rehacer el DMG, notarizar, grapar

Job `sign-macos` en `macos-latest`, entorno `signing`:

1. Importa el certificado Developer ID Application (`.p12` en base64 y su
   contraseña) en un llavero temporal creado para el job y borrado al final.
2. `hdiutil attach` del DMG sin firmar y copia de `Meltemi.app`.
3. `codesign --force --options runtime --timestamp --sign "<identidad>"`
   sobre los binarios anidados y luego sobre la `.app`, sin `--deep`, como
   recomienda Apple.
4. `hdiutil create` de un DMG nuevo con la `.app` y un enlace a
   `/Applications`, y `codesign` del DMG.
5. `xcrun notarytool submit --wait` con la clave de API de App Store Connect
   (`.p8`, id de la clave, emisor); un rechazo imprime el log de
   `notarytool log` y falla.
6. `xcrun stapler staple` del DMG.

El DMG rehecho pierde el fondo y la disposición del de Tauri. Se acepta: lo que
importa del DMG es la `.app` y el enlace. El presupuesto de 15 MB se vuelve a
medir sobre el DMG nuevo. Si `spctl` sobre la app copiada fuera del DMG, sin
red, no la acepta, la tarea 2.2 suma una notarización y un grapado de la `.app`
antes del paso 4. Son dos envíos y el doble de espera.

*Alternativa descartada*: firmar con la integración de Tauri (`APPLE_*` en el
build). Es la misma objeción de D1.

### D4 — Windows: un servicio con la clave en hardware

Desde junio de 2023, las CA no emiten certificados de firma de código con
clave privada exportable: la clave vive en un HSM o en un token. Un `.pfx` en
un secreto ya no es una opción, así que la firma la hace un servicio. Hay tres
candidatos, y el mantenedor elige al contratar (tarea 3.2 de
`entrega-sin-huecos`):

| Opción | Cómo autentica desde CI | Editor que ve el usuario | Coste | A verificar |
| --- | --- | --- | --- | --- |
| **Azure Artifact Signing** (antes Trusted Signing) | OIDC federado, sin secreto de larga vida | el nombre validado del mantenedor | cuota mensual baja | la elegibilidad por país de la validación de identidad |
| **SignPath Foundation** | token de API del proyecto; SignPath verifica que el artefacto viene del workflow del repositorio | «SignPath Foundation», con el proyecto en la descripción | gratuito para proyectos open source aprobados | la aprobación del proyecto |
| **SSL.com eSigner** (o DigiCert KeyLocker) | credenciales del servicio + TOTP | el nombre validado del mantenedor | certificado OV anual + cuota del servicio | ninguna restricción de país conocida |

**Recomendación**: Azure Artifact Signing si el mantenedor es elegible, porque
no deja secreto de larga vida (OIDC) y el editor es él. Si no es elegible,
SignPath Foundation, que es gratuita y además verifica el origen del
artefacto. SSL.com queda como respaldo.

El job `sign-windows` corre en `windows-latest`, entorno `signing`, con un
paso por proveedor: el que corre es el que nombra la variable
`MELTEMI_SIGN_WINDOWS_PROVIDER`. Firma el MSI y no el `.exe` de dentro, que
exigiría firmar durante el build (fuera de alcance, declarado).

### D5 — Android: la clave que no se puede perder

Job `sign-android` en `ubuntu-latest`, entorno `signing`. Decodifica el
keystore de `MELTEMI_ANDROID_KEYSTORE_B64` a un archivo temporal, corre
`zipalign -p 4` y `apksigner sign` (esquemas v2 y v3, como en la medición de
`companero-movil`), y borra el keystore. La clave la genera el mantenedor en su
máquina:

```
keytool -genkeypair -alias meltemi-companion -keyalg RSA -keysize 4096 -validity 10000
```

La respalda fuera de línea en dos lugares. **Perderla es irreversible**:
Android no instala una actualización firmada con otra clave, y cada usuario
tendría que desinstalar. Por eso el inventario (D7) la trata distinto de las
otras.

### D6 — Verificar contra una identidad fijada, sin secretos

`scripts/signing-identities.toml`, versionado, guarda lo público que se espera:

- la huella SHA-256 del certificado del APK;
- el Team ID de Apple y la identidad «Developer ID Application: …»;
- el sujeto (o la huella) del certificado de Windows.

`verify-signatures` es una matriz sin entorno y sin secretos (Windows, macOS y
Linux). Descarga lo firmado y lo comprueba con las herramientas del sistema:

- **APK**: `apksigner verify --print-certs`, con la huella igual a la fijada;
- **DMG**: `codesign --verify --strict` de la `.app`, `spctl -a -t open
  --context context:primary-signature` del DMG, `xcrun stapler validate` y el
  Team ID igual al fijado;
- **MSI**: `Get-AuthenticodeSignature` con estado `Valid`, el sujeto igual al
  fijado y sello de tiempo presente.

Una firma válida de otra identidad falla. Es la defensa contra una clave
equivocada o sustituida en el entorno. La pierna de Linux también corre
`meltemi-release capabilities --signed <lo verificado>`, que no necesita
secretos, y sube `meltemi-capabilities` (`estado-que-publica-la-release` D4
pasa aquí).

### D7 — El inventario de claves

`docs/release.md` gana una sección «Platform signing keys». Por cada clave
dice:

- qué es y qué firma;
- dónde vive (secreto del entorno `signing`, servicio externo o la máquina del
  mantenedor para el original);
- quién la custodia;
- cuándo caduca;
- cómo se rota;
- cómo se revoca;
- qué pasa si se filtra o se pierde.

Las filas son el Developer ID con la clave de API de notarización, la
identidad de firma de Windows según el proveedor, el keystore del APK y la
clave privada de la GitHub App (que no firma artefactos, pero se custodia
igual). La sección dice también lo que no cambia: la atestación sigue siendo
la firma de la release, y `gh attestation verify` sigue siendo la forma de
verificar el origen.

## Risks / Trade-offs

- **La notarización tarda o falla** por causas de Apple. `--wait` tiene un
  tope de 30 minutos; vencido, el job falla y la release no sale. Se reintenta
  el workflow, y la publicación ya es idempotente.
- **El proveedor de Windows cambia de nombre o de condiciones.** Solo cambia el
  paso del proveedor. El interruptor, la verificación y la identidad fijada no
  dependen de él.
- **Rehacer el DMG** pierde su estética. Se acepta (D3).
- **Más jobs, más minutos.** Solo en releases; los builds de integración no
  firman.
