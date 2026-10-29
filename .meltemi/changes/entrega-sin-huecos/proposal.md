# entrega-sin-huecos

> Vía completa (proposal → design → specs → tasks): una change paraguas, como
> `release-para-un-desarrollador`, que inventaría lo que falta entre «está en
> el repositorio» y «una persona lo instala desde una release», y ordena las
> changes que lo cierran. Su eje es automatizar: que CI/CD **proponga** cada
> versión según la política escrita, que **construya, firme y publique** todo
> lo que el producto tiene (también el móvil), y que el estado que declara el
> sitio **salga de la release** en vez de escribirse a mano. Nace del pedido
> del mantenedor del 2026-10-29, tras ver en la página del IDE un compañero
> móvil «Not yet» que ya está construido.

## Decisiones del mantenedor (2026-10-29)

- **Todo automatizado.** Somos dos —el mantenedor y su agente— y cada paso
  manual frena las releases. Lo que pueda hacer CI, lo hace CI; el gesto
  humano que queda es mergear la PR de release.
- **La política de firma se enmienda.** Las firmas de plataforma (Authenticode,
  Developer ID con notarización, la firma del APK) se admiten bajo custodia
  escrita. La firma de la release sigue siendo la atestación, sin clave. Lo
  hace `firma-de-plataforma`.
- **Un hito probado sube el minor: la 0.3.0.** La regla que faltaba quedó en
  `docs/versionado.md` (tarea 1.2 de esta change), y con ella la recomendación
  de `release-para-un-desarrollador` queda conforme a la política.
- **1.0.0 cuando estemos listos, y que lo diga CI.** La PR de release lleva la
  lista de preparación para 1.0, calculada de los hechos, y propone 1.0.0 en
  cuanto todo está en verde (`version-que-propone-ci`). Hoy no lo está: lo dice
  el design, D3.
- **Una GitHub App, la que el mantenedor ya tiene**, abre las PRs de release y
  da los tokens del camino de entrega. Sus credenciales se configuran al
  implementar, desde la máquina del mantenedor.

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
  firmarlo exigía una clave que la política de release prohibía tener en CI.
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
  `CHANGELOG.md` es un paso manual del mantenedor. La recomendación de 0.3.0
  contradecía la política escrita, que daba 0.2.1. La política no la aplicaba
  ninguna herramienta, y esa recomendación lo prueba.
- **El estado del sitio se escribe a mano y envejece.** La tabla de
  capacidades de `/ide/` y `/es/ide/` es HTML en meltemi-web, sin test que la
  ate a nada. El sitio ya sabe leer la release del IDE en cada despliegue: el
  script `ide-release.mjs` escribe versión y fecha entre marcadores. Solo que
  no lee lo que la release contiene.

La forma de cerrar esto sin agrandar el riesgo ya está en la casa. La release
la publica un workflow, no una mano. El sitio lee lo publicado, no lo
declarado. Y un gesto del mantenedor, el merge, sigue siendo lo que publica.

## What Changes

- **Inventario de brechas**, con su dueño: la tabla de abajo, mantenida en
  `docs/plan-de-cambios.md` como la tanda de esta paraguas.
- **La regla del hito en la política** (`docs/versionado.md`): un hito probado
  de punta a punta sube el minor. Es texto; la aplica `version-que-propone-ci`.
- **Cinco changes nuevas, abiertas aquí con sus cuatro artefactos**, listas
  para implementar:
  - `version-que-propone-ci`: tras cada push a `main`, CI calcula la versión
    siguiente aplicando la política a lo archivado sin release. La ruptura se
    detecta en los tres contratos, por diff contra el último tag; el hito, por
    el informe de la prueba. CI escribe el changelog y la versión en una rama
    y abre o actualiza una PR «Release X.Y.Z» con el porqué de cada número y
    la lista de preparación para 1.0. **Mergearla es publicar.**
  - `estado-que-publica-la-release`: el estado de cada capacidad vive en un
    archivo versionado del IDE. Un test lo ata a los hechos: una capacidad no
    puede decir «publicada» sin su artefacto en `release.yml`, ni «todavía no»
    con su change archivada. Cada release lo publica como asset atestado
    (`capabilities.json`), y el sitio pinta su tabla desde ahí.
  - `apk-en-la-release`: el APK se compila en `build.yml` y `release.yml`, con
    el techo de 25 MB como gate, y se publica firmado. Suma una pierna en un
    teléfono real.
  - `firma-de-plataforma`: la enmienda de la política y la firma de MSI, DMG y
    APK en jobs aislados de un entorno `signing` limitado a `main`, que reciben
    lo ya construido y no ejecutan pasos de build. Cada plataforma se activa
    cuando sus credenciales existen, sin romper el pipeline mientras tanto.
  - `contrato-publicado`: los JSON Schemas del contrato viajan en cada release
    como asset atestado; el SDK sigue siendo de fase 2.
- **`distribucion-y-actualizacion`, ya propuesta, gana design, specs y tasks**:
  los gestores de paquetes como canal de actualización, con la GitHub App como
  credencial donde alcanza.
- **Tres changes en meltemi-web**, con sus artefactos: `ide-state-as-it-is`
  (las filas de `/ide/` al día ahora, con test), `ide-state-from-release` (la
  tabla pintada desde `capabilities.json`) e `ide-delivery-infra` (el Terraform
  de los entornos `signing` y `release-pr` del IDE y sus secretos, y la App
  como credencial del aviso a la web en lugar del token de administración).
- **`herramientas-a-la-vista`, ya propuesta, queda como deseable**: es la
  brecha de producto que separa el escritorio de «publicado», no de entrega.

### Las brechas

| Brecha | Hoy | La cierra |
| --- | --- | --- |
| Elegir la versión y escribir el changelog | a mano | `version-que-propone-ci` |
| La regla del hito | faltaba; la recomendación contradecía la política | esta change, 1.2 (hecho) |
| La tabla de estado del sitio | HTML a mano, sin test; el móvil desactualizado | `ide-state-as-it-is`; luego `estado-que-publica-la-release` e `ide-state-from-release` |
| El APK de Android | sin build en CI, sin asset en la release | `apk-en-la-release` |
| Firma de plataforma (MSI, DMG, APK) | sin firma; la política prohibía la clave | `firma-de-plataforma` |
| Credenciales del camino de entrega | token de administración de respaldo para el aviso | `ide-delivery-infra` (App) |
| Actualizar Meltemi | volver a la página de releases | `distribucion-y-actualizacion` |
| El transcript del escritorio | píldoras «título · estado» | `herramientas-a-la-vista` |
| El SDK | no existe; el contrato no se publica como tal | `contrato-publicado`; el SDK, fase 2 |
| El móvil en un teléfono real | solo emulador | pierna de `apk-en-la-release` |
| La prueba de un desarrollador | escrita, sin correr | el mantenedor (`release-para-un-desarrollador` 5.1) |

### El orden

| Orden | Change | Repositorio | Por qué aquí | Depende de |
| --- | --- | --- | --- | --- |
| 0º | `ide-state-as-it-is` | meltemi-web | el sitio dice algo falso hoy | — |
| 1º | `ide-delivery-infra` | meltemi-web | los entornos y secretos que los demás leen | la App del mantenedor |
| 2º | `version-que-propone-ci` | IDE | la próxima release ya sale por PR | 1º |
| 3º | `estado-que-publica-la-release` | IDE | el estado viaja con la release | — |
| 4º | `ide-state-from-release` | meltemi-web | la tabla deja de escribirse a mano | 3º publicado en una release |
| 5º | `apk-en-la-release` | IDE | el móvil construido en CI | — |
| 6º | `firma-de-plataforma` | IDE | MSI y DMG sin aviso; el APK publicable | 1º, 5º, credenciales |
| 7º | `contrato-publicado` | IDE | primer paso del SDK, sin prometerlo | — |
| 8º | `distribucion-y-actualizacion` | IDE | actualizar sin red desde Meltemi | 6º para el cask |
| deseable | `herramientas-a-la-vista` | IDE | lo que falta para llamar «publicado» al escritorio | — |

## Capabilities

### New Capabilities

- Ninguna en esta paraguas: cada capacidad nueva o modificada la declara su
  change.

### Modified Capabilities

- `release-distribution`: + requisito «El estado de cada capacidad tiene
  dueño»: cada capacidad que el producto declara tiene su estado verificable y
  la change que la cierra, inventariados.

## Impact

- Archivos de esta paraguas: este directorio, `docs/plan-de-cambios.md`,
  `docs/versionado.md` (la regla del hito), `docs/release.md` (la frase de la
  0.3.0), la nota de `release-para-un-desarrollador` D5, los directorios de las
  cinco changes nuevas y los artefactos que faltaban a
  `distribucion-y-actualizacion`. En meltemi-web, los de sus tres changes.
  **No toca código de producto ni workflows**: cada cosa que los toca tiene su
  change.
- Cero dependencias nuevas en esta paraguas. Las que traiga cada change, se
  justifican en su design (constitución §10).
- Lo que solo el mantenedor puede hacer: instalar la GitHub App en los
  repositorios del camino y darle sus permisos; contratar las firmas (Apple
  Developer Program, un servicio de firma para Windows) y generar la clave del
  APK; y correr la prueba de un desarrollador.
- Riesgo: que la automatización publique algo que nadie miró. Por eso la
  versión llega como PR y el merge sigue siendo el gesto. Las firmas se hacen
  en jobs que no construyen nada. El sitio solo pinta lo que una release
  atestada publicó.

## Fuera de alcance

- Publicar sin un gesto humano: la PR de release no se mergea sola.
- Actualización desde la app por red: §3 y `deny.toml`; la cubren los gestores.
- App stores (Play, App Store) e iOS: el compañero es Android primero.
- El SDK completo, con API estable y paquetes por lenguaje: fase 2, cuando el
  contrato se asiente.
- Telemetría de instalaciones o de uso: §9.
