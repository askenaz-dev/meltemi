# distribucion-y-actualizacion

> Vía completa (proposal → design → specs → tasks): manifiestos en gestores
> de paquetes publicados por el pipeline **después** de la release firmada, y
> la actualización resuelta por esos gestores, no por red desde Meltemi.
> Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas P8 y la parte de
> aviso de versión de D-V1). Absorbe las propuestas nombradas allí como
> `distribucion-gestores-paquetes` y `avisos-de-version`.

> **Nota 2026-10-24**, tras la mudanza del sitio y la release automática:
>
> - **La firma cambió de naturaleza.** `procedencia-de-release` se retiró y la
>   sustituyó `release-con-el-push`: la firma de la release es la atestación
>   del workflow (OIDC, sin clave del mantenedor), que cubre el `SHA256SUMS`
>   publicado, y las releases son inmutables. Los hashes de los manifiestos
>   salen de ese `SHA256SUMS`; no hay clave de firma que distinguir del token.
> - **El sitio no está aquí.** `site/downloads.html` y su espejo ya no
>   existen; la página de descargas es de meltemi-web. Lo que le toca a la
>   web, en changes de allí y después de esta: declarar en su Terraform los
>   repositorios del tap y del bucket y el secreto (decisión 0004: toda
>   configuración de infraestructura es Terraform, como ya hizo con el
>   repositorio del IDE en `ide-releases`), y mostrar los comandos cuando los
>   manifiestos estén publicados, porque la web no anuncia lo que no existe.
> - **El tap es una decisión del mantenedor.** Existe
>   `askenaz-dev/homebrew-tap` (público, de 2026-05-13, con un solo archivo,
>   `forge.rb`, de otro proyecto suyo). El design presenta las dos salidas: un
>   tap propio, `homebrew-meltemi` (`brew tap askenaz-dev/meltemi`), o
>   reutilizar ese (`brew tap askenaz-dev/tap`, con los dos productos en él).
>   Crear los repositorios y el token de publicación de manifiestos lo hace
>   él, no un agente.

> **Nota 2026-10-29**, al entrar a la tanda `entrega-sin-huecos` (8º):
>
> - **La credencial es la GitHub App** del mantenedor (`entrega-sin-huecos`
>   D4), instalada en el tap y en el bucket: reemplaza al «token de
>   publicación de manifiestos». `winget` es la excepción: su PR va a
>   `microsoft/winget-pkgs`, donde la App no está instalada, y necesita un
>   token propio (design D3).
> - **El cask depende de `firma-de-plataforma`**: Homebrew deja de aceptar
>   casks que Gatekeeper rechaza, así que el cask sale cuando el DMG está
>   notarizado. La fórmula del núcleo no espera.
> - **Los artefactos de esta change** (design, specs, tasks) se escribieron
>   ese día; la decisión del tap sigue siendo del mantenedor (design D2).

## Why

Instalar Meltemi es descargar un `.msi`, un `.dmg` o un `.deb` desde la
página de releases y, para el núcleo, un archivo comprimido. Funciona y está
firmado. Pero el desarrollador que vive en terminal —el primer perfil del
rumbo— instala con `winget`, `brew` o `scoop`, y actualiza con los mismos.
Orca está en la App Store y en un APK; Meltemi no está en ningún gestor, y
tampoco avisa cuando hay versión nueva.

Avisar tiene una frontera que conviene decir antes de diseñar: el daemon
no abre red (§3), el webview no alcanza ningún origen remoto, y
`deny.toml` prohíbe con razón todo cliente HTTP y toda pila TLS en el
workspace. Meltemi **no puede** consultar «¿hay versión nueva?» sin romper
una de las tres. Y no hace falta: el gestor de paquetes del usuario ya lo
sabe. La distribución por gestores **es** el mecanismo de actualización.

## What Changes

- **Manifiestos por gestor, publicados tras la release**: `winget`
  (`askenaz-dev.Meltemi` y `askenaz-dev.MeltemiDesktop`, o la forma que el
  design verifique), un tap de Homebrew (fórmula para el núcleo, cask para
  el `.dmg`; propio o compartido, ver la nota) y un bucket de Scoop propio.
  Un job de `release.yml` **posterior** al job `publish` (como `notify-web`)
  genera los manifiestos con los hashes del `SHA256SUMS` publicado y abre las
  PR o actualiza los repos de destino. El token que necesite es un token de
  publicación de manifiestos, acotado a esos repositorios; no es
  `WEB_DISPATCH_TOKEN` ni toca la atestación, que no usa ninguno. El design
  lo acota, y la atestación de la release sigue intacta (releases
  inmutables: los assets no cambian después).
- **Los README** muestran los comandos (`winget install …`,
  `brew install …`, `scoop install …`) junto a los instaladores, con la
  advertencia de que la atestación se verifica igual
  (`gh attestation verify`). La página de descargas hace lo mismo desde
  meltemi-web, cuando los manifiestos estén publicados.
- **El cliente sabe cómo se instaló** (heurística por ruta y por registro
  del gestor) y en «Acerca de» / `meltemi version --verbose` muestra el
  canal y el comando de actualización correspondiente; «Ver releases» abre
  la página en el navegador del usuario por el sistema, sin que Meltemi
  toque la red.
- **Ninguna comprobación de versión por red desde Meltemi**: la propuesta
  lo escribe para que no se «arregle» después con un cliente HTTP.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisitos «Manifiestos de gestores publicados
  tras la release firmada, con los hashes publicados» y «La actualización la
  hace el gestor del usuario; Meltemi no consulta versiones por red».
- `gui-shell`, `cli-contract`: + canal de instalación y comando de
  actualización en «Acerca de» y `version --verbose`.

## Impact

- Archivos: `.github/workflows/release.yml` (job nuevo tras `publish`),
  repositorios de tap y bucket (fuera de este monorepo, con su README),
  `README.md`, `LEEME.md`, `docs/release.md`,
  `desktop/ui/src/lib/views/Settings.svelte`, `tui/src/cli.rs`. En
  meltemi-web, en changes propias: el Terraform de los repositorios y del
  secreto, y la página de descargas.
- Cero dependencias en los binarios. Un token de Actions nuevo, acotado a
  manifiestos y creado por el mantenedor; su alcance queda documentado en
  `docs/release.md`.
- Riesgo: los manifiestos de `winget` los revisa un tercero y tardan; la
  página lo dice y los instaladores directos siguen siendo la vía principal.

## Fuera de alcance

- App stores, `.rpm` (change propia con sus nombres de paquete verificados),
  AppImage (rechazado por `instaladores-linux-sin-webview`).
- Auto-actualización dentro de Meltemi: es red desde el cliente; no.
- Telemetría de instalaciones: §9.
- La página de descargas y el Terraform de los repositorios de destino:
  meltemi-web.
