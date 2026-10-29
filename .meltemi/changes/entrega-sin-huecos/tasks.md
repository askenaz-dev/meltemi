# Tasks — entrega-sin-huecos

Vía completa, paraguas. Un commit atómico por tarea, con referencia
`(entrega-sin-huecos N.M)` y sin trailers de co-autoría. Esta change no
implementa nada que otra change de la tanda posea: sus tareas del grupo 2 se
marcan cuando la change nombrada se archiva.

Las tareas marcadas *(meltemi-web)* o *(mantenedor)* no son de este repositorio
o no son de un agente; están aquí porque la tanda no se cierra sin ellas.

## 1. El inventario y las propuestas

- [x] 1.1 La propuesta y la tanda en `docs/plan-de-cambios.md`
- [x] 1.2 La regla del hito en `docs/versionado.md`, la frase de la 0.3.0 en
  `docs/release.md` y la nota en `release-para-un-desarrollador` D5 (design
  D2) — escenario «La política dice qué hace un hito»
  <!-- 2026-10-29: decisión del mantenedor; la 0.3.0 queda conforme a la
  política. -->
- [x] 1.3 Las cinco changes nuevas con sus cuatro artefactos, validadas:
  `version-que-propone-ci`, `estado-que-publica-la-release`,
  `apk-en-la-release`, `firma-de-plataforma`, `contrato-publicado`
  <!-- 2026-10-29: las cinco con `meltemi validate` limpio. Escribiéndolas
  aparecieron dos cosas: la frase de `release-para-un-desarrollador` sobre quién
  escribe la versión, que ahora admite el merge de la PR de release, y el
  índice de `proto/README.md`, que nombra 19 de los 34 schemas (lo cierra
  `contrato-publicado` 1.1). -->
- [x] 1.4 Design, specs y tasks de `distribucion-y-actualizacion`, con la App
  como credencial (design D4)
  <!-- 2026-10-29: la App para el tap y el bucket; winget necesita un token
  propio (su PR va a `microsoft/winget-pkgs`), con la cuenta de máquina como
  recomendación; el cask espera a la notarización. -->
- [x] 1.5 *(meltemi-web)* `ide-state-as-it-is`, `ide-state-from-release` e
  `ide-delivery-infra`, validadas con `openspec validate`
  <!-- 2026-10-29: las tres válidas, en `openspec/changes/` de meltemi-web.
  Los nombres cruzan los dos repositorios: `MELTEMI_APP_ID` y
  `MELTEMI_APP_PRIVATE_KEY` en `release` y `release-pr`, los secretos de firma
  en `signing`, los interruptores `MELTEMI_SIGN_*`; y las notas de la tabla son
  las mismas palabras en `ide-state-as-it-is` D1 y en
  `estado-que-publica-la-release` D3. -->
- [x] 1.6 El inventario de brechas, con su dueño (design D6) — escenario
  «Cada brecha con su dueño»
  <!-- 2026-10-29: la tabla «Las brechas» de la propuesta, y la tanda con las
  decisiones y el orden en `docs/plan-de-cambios.md`. -->

## 2. La tanda

- [ ] 2.1 *(meltemi-web)* `ide-state-as-it-is` archivada
- [ ] 2.2 *(meltemi-web)* `ide-delivery-infra` archivada
- [ ] 2.3 `version-que-propone-ci` archivada
- [ ] 2.4 `estado-que-publica-la-release` archivada
- [ ] 2.5 *(meltemi-web)* `ide-state-from-release` archivada
- [ ] 2.6 `apk-en-la-release` archivada
- [ ] 2.7 `firma-de-plataforma` archivada
- [ ] 2.8 `contrato-publicado` archivada
- [ ] 2.9 `distribucion-y-actualizacion` archivada

## 3. Del mantenedor

- [ ] 3.1 *(mantenedor)* La GitHub App instalada en este repositorio y en
  meltemi-web con los permisos que piden `version-que-propone-ci` e
  `ide-delivery-infra`; su id y su clave como variables del Terraform
- [ ] 3.2 *(mantenedor)* Las firmas contratadas y la clave del APK generada y
  respaldada, como piden `firma-de-plataforma` y `apk-en-la-release`
- [ ] 3.3 *(mantenedor)* Confirmar la lectura de 1.0 del design D3: los
  entregables de fase 2 que el texto del Hito v1.0 no nombra, ¿son gate o
  informativos?

## 4. Cierre

- [ ] 4.1 `meltemi validate` y `verify`; la tanda cerrada en
  `docs/plan-de-cambios.md`; archivar
