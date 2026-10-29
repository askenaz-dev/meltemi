# Tasks — firma-de-plataforma

Vía completa. Un commit atómico por tarea, con referencia
`(firma-de-plataforma N.M)` y sin trailers de co-autoría. **Ningún secreto
entra a este repositorio, a un fixture, a un log ni a un commit**: el código
lee nombres de secretos, nunca valores. Las tareas *(mantenedor)* no son de un
agente.

## 1. La política

- [ ] 1.1 `docs/release.md`: la enmienda (design D1) y el inventario de claves
  (D7), en inglés y en el resumen en español; «Platform signing» deja de ser
  deuda declarada y pasa a describir el interruptor — escenarios «Ninguna
  clave de firma de release que custodiar» y «Clave de plataforma solo con
  custodia escrita» (la parte documental)
- [ ] 1.2 Los controles estáticos en `core/meltemid/tests/release.rs`: los
  jobs del entorno `signing` sin checkout ni herramientas de build, sus
  secretos en ningún otro job, `build.yml` sin el entorno, y cada secreto de
  firma nombrado en el inventario (D1) — escenarios «Ningún job que construye
  ve una clave de plataforma», «Clave de plataforma solo con custodia escrita»
  y «Los builds de integración no firman»; gate: la suite

## 2. Los jobs

- [ ] 2.1 El interruptor: `package-gui` y `package-android` suben fuera del
  patrón `meltemi-*` cuando su variable está en `on`, y `publish` depende de
  `verify-signatures` (D2) — escenarios «Sin activar se publica como hoy» y
  «Activada y sin firma verificada no se publica»
- [ ] 2.2 `sign-macos`: llavero temporal, firma de la `.app` con hardened
  runtime, DMG rehecho y firmado, `notarytool --wait`, grapado y el
  presupuesto medido otra vez; si `spctl` sin red rechaza la app copiada, la
  notarización de la `.app` antes del DMG (D3)
- [ ] 2.3 `sign-windows`: el paso del proveedor que nombra
  `MELTEMI_SIGN_WINDOWS_PROVIDER`, empezando por el elegido en 3.2, con su
  acción pineada por commit si la tiene (D4)
- [ ] 2.4 `sign-android`: keystore temporal, `zipalign` + `apksigner` v2/v3 y
  el keystore borrado (D5)
- [ ] 2.5 `scripts/signing-identities.toml` y `verify-signatures`: la matriz
  sin secretos contra la identidad fijada, y `capabilities --signed` con lo
  verificado (D6) — escenarios «Instalador firmado y verificado antes de
  publicar», «Una firma de otra identidad falla» y «El estado publicado dice
  lo que se firmó»

## 3. Del mantenedor

- [ ] 3.1 *(mantenedor)* Apple Developer Program, el certificado Developer ID
  Application exportado a `.p12` y una clave de API de App Store Connect con
  rol de desarrollador; sus valores en las variables del Terraform de
  meltemi-web (`ide-delivery-infra`), y el Team ID en
  `scripts/signing-identities.toml`
- [ ] 3.2 *(mantenedor)* El proveedor de Windows elegido entre los del design
  D4, su alta y su credencial en el Terraform; el sujeto del certificado en
  `scripts/signing-identities.toml`
- [ ] 3.3 *(mantenedor)* La clave del APK generada en su máquina, respaldada
  fuera de línea en dos lugares, cargada en el Terraform, y su huella en
  `scripts/signing-identities.toml` (D5)

## 4. Los textos y la verificación

- [ ] 4.1 `docs/faq.md`, `docs/quickstart.md`, `README.md` y `LEEME.md`: el
  aviso de primer arranque por plataforma, según lo que la release firma
- [ ] 4.2 *(tras la primera release firmada)* Descargar el MSI, el DMG y el APK
  en máquinas limpias: sin aviso de SmartScreen ni de Gatekeeper, el APK se
  instala y actualiza sobre el anterior, y la verificación del usuario con
  `gh attestation verify` sigue igual; anotarlo aquí
- [ ] 4.3 `meltemi validate` y `verify`; archivar
