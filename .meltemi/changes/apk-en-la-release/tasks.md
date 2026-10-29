# Tasks — apk-en-la-release

Vía completa. Un commit atómico por tarea, con referencia
`(apk-en-la-release N.M)` y sin trailers de co-autoría. Ninguna clave de
publicación entra a este repositorio ni a sus workflows en esta change: la
firma de publicación es de `firma-de-plataforma`.

## 1. El build

- [ ] 1.1 `release.yml`: el job `package-android` con el toolchain del design
  D2, clippy para el target (D6), el APK sin firmar medido contra
  `MELTEMI_APK_BUDGET_BYTES` y subido como `android-unsigned`, que `publish` no
  recoge (D1, D3, D4) — escenarios «APK sobre el techo falla» y «Sin firma de
  publicación la release sale sin APK»; gate: los controles estructurales de
  workflows
- [ ] 1.2 `build.yml`: la pierna de Android con el mismo toolchain y techo, la
  firma efímera generada y destruida en la ejecución, y el APK con el resto de
  los artefactos de integración (D1, D4) — escenario «Push a main deja el APK
  descargable»
- [ ] 1.3 `scripts/unsigned-build-notice.txt`: el párrafo de la firma efímera
  (D4)
- [ ] 1.4 `core/meltemid/tests/release.rs`: el presupuesto del APK igual en
  los dos workflows, y el nombre `android-unsigned` fuera del patrón que
  recoge `publish` (D3, D4)

## 2. Los documentos

- [ ] 2.1 `mobile/README.md`: CI lo compila en las dos rutas; dónde bajar el
  APK de integración y qué significa su firma; el build local sin cambios
- [ ] 2.2 `docs/release.md`: `meltemi-companion-Android.apk` en la tabla de
  assets, publicado solo firmado; en inglés y en el resumen en español
- [ ] 2.3 `docs/prueba-de-uno.md`: el paso 12 opcional, el compañero en un
  teléfono, con la fila de la plantilla del informe (D7)

## 3. Verificación

- [ ] 3.1 La primera ejecución de `build.yml` con la pierna de Android: el APK
  medido, instalado en un emulador (`adb install` → `Success`) y anotado aquí
- [ ] 3.2 *(mantenedor)* La corrida en un teléfono real con el APK de la
  primera release que lo publique, y su informe en `docs/qa/` (D7) —
  escenarios «La corrida en el teléfono queda escrita» y «Sin informe del
  teléfono no se declara publicado»
- [ ] 3.3 `meltemi validate` y `verify`; archivar
