# entrega-sin-huecos

> Vía completa (proposal → design → specs → tasks): una change paraguas, como
> `release-para-un-desarrollador`, que inventaría lo que falta entre «está en
> el repositorio» y «una persona lo instala desde una release», y ordena las
> changes que lo cierran. Su eje es automatizar: que CI/CD **proponga** cada
> versión según la política escrita, que **construya y publique** todo lo que
> el producto tiene (también el móvil), y que el estado que declara el sitio
> **salga de la release** en vez de escribirse a mano. Nace del pedido del
> mantenedor del 2026-10-29, tras ver en la página del IDE un compañero
> móvil «Not yet» que ya está construido.

## Why

La página del IDE en meltemi.dev declara tres cosas sobre el cliente: el
escritorio «en desarrollo», el SDK público «todavía no» y el compañero móvil
«especificado para la fase 3, nada publicado». Revisado contra el repositorio
y contra la release v0.2.0, solo la del SDK sigue siendo exacta.

- **El móvil está hecho y no se entrega.** `companero-movil` cerró 8/8 con
  verify 7/7 y se archivó el 2026-10-28; el APK de release arm64 mide
  10,69 MB y se instaló y arrancó en un emulador el 2026-10-02. Pero ningún
  workflow lo compila para Android (`ci.yml`, `build.yml` y `release.yml` no
  lo nombran), ninguna release lo publica, nadie lo probó en un teléfono, y
  firmarlo exige una clave que la política de release prohíbe tener en CI.
- **El escritorio se publica, pero no se puede decir «publicado».** v0.2.0
  trae `meltemi-desktop-Windows.msi`, `meltemi-desktop-macOS.dmg` y
  `meltemi-desktop-Linux.deb` bajo el mismo `SHA256SUMS` y la misma
  atestación. Lo que lo deja en «en desarrollo» es concreto: los instaladores
  de Windows y macOS no llevan firma de plataforma y avisan en el primer
  arranque; el transcript pinta las herramientas como píldoras
  (`herramientas-a-la-vista`, solo propuesta); la actualización depende de
  que el usuario vuelva a la página de releases (`distribucion-y-actualizacion`,
  solo propuesta); y la prueba de un desarrollador está escrita y sin correr.
- **El SDK no existe, y es lo honesto.** No hay change abierta (solo el nombre
  `plugins-skills-sdk`), y `meltemi-client` y `meltemi-proto` llevan
  `publish = false`. Lo que sí existe y nadie publica como tal es el contrato:
  los JSON Schemas de `proto/schemas/`, la fuente de verdad del protocolo.
- **La versión la decide una persona, a mano, y se equivoca.** La release sale
  sola con el push (`release-con-el-push`), pero escribir la versión y el
  `CHANGELOG.md` es un paso manual del mantenedor. La política
  (`docs/versionado.md`) dice que antes de 1.0 lo aditivo sube el patch y solo
  la ruptura sube el minor. Aun así, `release-para-un-desarrollador` D5 y
  `docs/release.md` recomiendan **0.3.0** para una tanda sin ruptura, que por
  la política es **0.2.1**. La política no la aplica ninguna herramienta, y
  esa recomendación lo prueba.
- **El estado del sitio se escribe a mano y envejece.** La tabla de
  capacidades de `/ide/` y `/es/ide/` es HTML en meltemi-web, sin test que la
  ate a nada. El sitio ya sabe leer la release del IDE en cada despliegue: el
  script `ide-release.mjs` escribe versión y fecha entre marcadores. Solo que
  no lee lo que la release contiene.

La forma de cerrar esto sin agrandar el riesgo ya está en la casa. La release
la publica un workflow, no una mano. El sitio lee lo publicado, no lo
declarado. Un gesto del mantenedor, el merge, sigue siendo lo que publica.

## What Changes

- **Inventario de brechas**, con su dueño. Es la tabla de abajo, y se
  mantiene en `docs/plan-de-cambios.md` como la tanda de esta paraguas.
- **Cinco changes nuevas, nombradas aquí y abiertas como propuesta propia**
  (el rumbo de estructura: lo que surge se anota, no se cuela):
  - `version-que-propone-ci` (completa): un workflow, tras cada push a
    `main`, calcula la versión siguiente aplicando `docs/versionado.md` a lo
    archivado desde el último tag. La ruptura se detecta en los tres
    contratos de la política, por diff contra el tag: los schemas de `proto/`,
    la gramática de la CLI vía `docs/referencia-cli.md` (que ya se genera) y
    el formato de `.meltemi/`; una change puede además declararla. El
    workflow escribe la sección del `CHANGELOG.md` (con `--report` si existe
    el informe de esa versión) y la versión en una rama `release/next`, y abre
    o actualiza una PR «Release X.Y.Z» con el porqué de cada número. **Mergear
    esa PR es publicar**, así que el gesto del mantenedor no cambia de dueño:
    deja de ser escribir la versión y pasa a ser aprobarla. Un agente sigue
    sin escribir versiones.
  - `estado-que-publica-la-release` (completa): el estado de cada capacidad
    (núcleo, escritorio, móvil, SDK) vive en un archivo versionado del IDE,
    con un test que lo ata a los hechos. Una capacidad no puede decir
    «publicada» si `release.yml` no publica su artefacto. Tampoco puede decir
    «todavía no» si su change está archivada. Cada release lo publica como un
    asset más (`capabilities.json`), bajo el mismo `SHA256SUMS` y la misma
    atestación. En meltemi-web, una change hermana hace que el despliegue
    escriba la tabla de `/ide/` y `/es/ide/` desde ese asset, entre
    marcadores, como `ide-release.mjs` ya hace con la versión. La tabla deja
    de escribirse a mano.
  - `apk-en-la-release` (completa): el APK de Android se compila en
    `build.yml` (como artefacto de integración sin firma, igual que los demás)
    y en `release.yml`, con el techo de 25 MB como gate duro y su checksum en
    `SHA256SUMS`. Se publica en la release en cuanto haya firma (la change
    siguiente). Se suma una pierna en un teléfono real a
    `docs/prueba-de-uno.md` y a la conformidad manual.
  - `firma-de-plataforma` (completa, **gate: decisión del mantenedor**):
    Authenticode para el MSI, Developer ID con notarización para el DMG y la
    clave de firma del APK. Firmar se hace en un job aislado, en un entorno
    `signing` limitado a `main`. Ese job recibe los artefactos ya construidos
    y nunca ejecuta un paso de build, así que la clave no es accesible a los
    pasos que el usuario define, que es lo que la cita de SLSA de
    `docs/release.md` prohíbe. Hoy esa página dice que no se agregará clave
    alguna. Esta change la enmienda, o elige proveedores que firman sin clave
    en CI (firma en la nube por OIDC donde exista). El design presenta las dos
    salidas y lo que cuesta cada una. Comprar los certificados es del
    mantenedor.
  - `contrato-publicado` (rápida): el primer paso honesto hacia el SDK. Los
    JSON Schemas de `proto/schemas/` viajan en cada release como un asset
    versionado y atestado, con su `PROTOCOL_VERSION`. La decisión de publicar
    `meltemi-proto` y `meltemi-client` como crates (publicación por OIDC, sin
    token) queda escrita con su condición: que el contrato se asiente, como
    dice el rumbo. El SDK propiamente dicho sigue siendo de fase 2
    (`plugins-skills-sdk`).
- **Dos changes ya propuestas entran en el orden**: `distribucion-y-actualizacion`
  (los gestores de paquetes **son** el canal de actualización; no hay
  actualización desde la app por red), después de la firma, porque un cask
  de Homebrew de una app sin notarizar choca con la política vigente de
  Homebrew, que el design verifica; y `herramientas-a-la-vista`, la brecha de
  producto que separa el escritorio de «publicado».
- **La web, ahora y a mano, una vez**: las filas de `/ide/` y `/es/ide/` pasan
  a decir lo que hoy es cierto (el móvil «en desarrollo, se compila desde la
  fuente, sin APK publicado»; el escritorio con el porqué; el SDK con el
  contrato público), con un test que fije las filas hasta que
  `estado-que-publica-la-release` las escriba sola. Es una change de
  meltemi-web, aprobada por el mantenedor el 2026-10-29.
- **La política de versiones y la recomendación, reconciliadas**: o la
  política gana una regla escrita («un hito probado de punta a punta puede
  subir el minor», decidida por el mantenedor en la PR de release), o la
  recomendación de `release-para-un-desarrollador` D5 y de `docs/release.md`
  pasa a 0.2.1. Es una decisión del mantenedor. Mientras no llegue,
  `version-que-propone-ci` aplica la política tal como está escrita.

### Las brechas

| Brecha | Hoy | La cierra |
| --- | --- | --- |
| Elegir la versión y escribir el changelog | a mano, y contradice la política | `version-que-propone-ci` |
| La tabla de estado del sitio | HTML a mano, sin test; el móvil desactualizado | la web ahora; luego `estado-que-publica-la-release` |
| El APK de Android | sin build en CI, sin asset en la release | `apk-en-la-release` |
| Firma de plataforma (MSI, DMG, APK) | sin firma; la política prohíbe la clave en CI | `firma-de-plataforma` (decisión) |
| Actualizar Meltemi | volver a la página de releases | `distribucion-y-actualizacion` |
| El transcript del escritorio | píldoras «título · estado» | `herramientas-a-la-vista` |
| El SDK | no existe; el contrato no se publica como tal | `contrato-publicado`; el SDK, fase 2 |
| El móvil en un teléfono real | solo emulador | pierna de `apk-en-la-release` |
| La prueba de un desarrollador | escrita, sin correr | el mantenedor (`release-para-un-desarrollador` 5.1) |

### El orden

| Orden | Change | Vía | Por qué aquí | Depende de |
| --- | --- | --- | --- | --- |
| 0º | web: filas al día | rápida, en meltemi-web | el sitio dice algo falso hoy | — |
| 1º | `version-que-propone-ci` | completa | sin ella cada release depende de recordar la política; sirve ya a la próxima | — |
| 2º | `estado-que-publica-la-release` | completa, con hermana en meltemi-web | que el 0º no vuelva a envejecer | 1º (la PR de release lo regenera) |
| 3º | `apk-en-la-release` | completa | el móvil construido en CI, aunque aún sin publicar | — |
| 4º | `firma-de-plataforma` | completa, gate: decisión | publica el APK y saca a MSI y DMG del aviso | decisión y certificados |
| 5º | `distribucion-y-actualizacion` | completa (ya propuesta) | actualizar sin red desde Meltemi | 4º para el cask |
| 6º | `contrato-publicado` | rápida | primer paso del SDK, sin prometerlo | 1º |
| deseable | `herramientas-a-la-vista` | completa (ya propuesta) | lo que falta para llamar «publicado» al escritorio | — |

## Capabilities

### New Capabilities

- Ninguna en esta paraguas: cada capacidad nueva o modificada la declara su
  change.

### Modified Capabilities

- `release-distribution`: + requisito «Las brechas de entrega se inventarían
  con su dueño»: cada capacidad que el sitio declara tiene una fila con su
  estado verificable y la change que la cierra.

## Impact

- Archivos de esta paraguas: este directorio y `docs/plan-de-cambios.md` (la
  tanda y los pendientes del mantenedor). **No toca código de producto ni
  workflows**: cada cosa que los toca tiene su change en la tanda.
- Cero dependencias nuevas en esta paraguas. Las que traiga cada change, se
  justifican en su design (constitución §10).
- Lo que solo el mantenedor puede hacer, y qué bloquea cada cosa:
  - la decisión de firma y la compra de certificados (bloquea 4º y el APK
    publicado);
  - la regla de versión para hitos (no bloquea: mientras tanto se aplica la
    política escrita);
  - el permiso para que Actions abra PRs, o una GitHub App declarada en el
    Terraform de meltemi-web (bloquea 1º; una PR abierta con el token del
    workflow no dispara CI, y el design lo resuelve o lo declara);
  - el tap de Homebrew (ya escrito en `distribucion-y-actualizacion`);
  - correr la prueba de un desarrollador (sigue en `release-para-un-desarrollador`).
- Riesgo: que la automatización publique algo que nadie miró. Por eso la
  versión llega como PR y el merge sigue siendo el gesto. Las firmas se hacen
  en un job que no construye nada. El sitio solo pinta lo que una release
  atestada publicó.

## Fuera de alcance

- Publicar sin un gesto humano: la PR de release no se mergea sola.
- Actualización desde la app por red: §3 y `deny.toml`; la cubren los gestores.
- App stores (Play, App Store) e iOS: el compañero es Android primero.
- El SDK completo, con API estable y paquetes por lenguaje: fase 2, cuando el
  contrato se asiente.
- Telemetría de instalaciones o de uso: §9.
