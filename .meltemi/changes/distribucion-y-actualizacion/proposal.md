# distribucion-y-actualizacion

> Vía completa (proposal → design → specs → tasks): manifiestos en gestores
> de paquetes publicados por el pipeline **después** de la release firmada, y
> la actualización resuelta por esos gestores, no por red desde Meltemi.
> Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas P8 y la parte de
> aviso de versión de D-V1). Absorbe las propuestas nombradas allí como
> `distribucion-gestores-paquetes` y `avisos-de-version`.

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
  design verifique), un tap de Homebrew propio (`askenaz-dev/homebrew-meltemi`:
  fórmula para el núcleo, cask para el `.dmg`) y un bucket de Scoop propio.
  Un job de `release.yml` **posterior** a `release: published` genera los
  manifiestos con los hashes del `SHA256SUMS` publicado y abre las PR o
  actualiza los repos propios. El token que necesite es un token de
  publicación de manifiestos, nunca la clave de firma; el design lo acota y
  `procedencia-de-release` sigue intacta (immutable releases: los assets no
  cambian después).
- **La página de descargas y los README** muestran los comandos
  (`winget install …`, `brew install …`, `scoop install …`) junto a los
  instaladores, con la advertencia de que la firma se verifica igual.
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
- `product-site`: + comandos de instalación por gestor.
- `gui-shell`, `cli-contract`: + canal de instalación y comando de
  actualización en «Acerca de» y `version --verbose`.

## Impact

- Archivos: `.github/workflows/release.yml` (job nuevo tras `published`),
  repositorios propios de tap y bucket (fuera de este monorepo, con su
  README), `site/downloads.html` y `site/es/downloads.html`, `README.md`,
  `LEEME.md`, `docs/release.md`, `desktop/ui/src/lib/views/Settings.svelte`,
  `tui/src/cli.rs`.
- Cero dependencias en los binarios. Un token de Actions nuevo, acotado a
  manifiestos; su alcance queda documentado en `docs/release.md`.
- Riesgo: los manifiestos de `winget` los revisa un tercero y tardan; la
  página lo dice y los instaladores directos siguen siendo la vía principal.

## Fuera de alcance

- App stores, `.rpm` (change propia con sus nombres de paquete verificados),
  AppImage (rechazado por `instaladores-linux-sin-webview`).
- Auto-actualización dentro de Meltemi: es red desde el cliente; no.
- Telemetría de instalaciones: §9.
