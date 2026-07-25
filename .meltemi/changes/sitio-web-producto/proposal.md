## Why

El mantenedor lo formuló exacto: "esto no es una shell, es un producto
completo con múltiples UI (desktop y terminal) para macOS, Linux y Windows" —
y hoy no existe ni la página que lo cuente ni el lugar canónico que guíe a un
usuario desde "lo encontré en GitHub" hasta "mi agente corre bajo mis specs".
El dominio `meltemi.dev` está reservado (plan de cambios, namespaces), el
design system ya define la identidad, y la guía de agentes
(flota-deteccion-guia) necesita una casa pública además del repo.

## What Changes

- **Sitio estático** en el repo (`site/`), desplegable a `meltemi.dev` vía
  GitHub Pages: sin backend, sin cookies, sin analytics de terceros —
  coherente con §9 hasta en la web.
- **Contenido**: la historia del producto (daemon + escritorio + terminal,
  tres SO, paridad de núcleo; "un rumbo, muchas velas"), capturas reales de
  ambas superficies, descarga por plataforma (MSI/DMG/AppImage+deb y el
  instalador de una línea, con checksums/firma enlazados), la guía de
  agentes y perfiles multi-suscripción, y el manifiesto/constitución
  enlazados — el método como diferenciador, presentado como herramienta
  poderosa, no como peaje.
- **Identidad del design system**: tokens, marca y tipografía de
  `design-system/` aplicados al sitio (una sola identidad producto↔web).
- **Verificación**: lint de enlaces y build del sitio como gate de CI; las
  descargas apuntan a la release firmada más reciente sin hardcodear
  versiones (patrón verificado en test, como la referencia CLI).

## Capabilities

### New Capabilities
- `product-site`: el sitio estático, su contenido mínimo verificable y su
  postura de privacidad (sin rastreo, sin cookies, sin CDN de terceros para
  ejecutar código).

### Modified Capabilities
- `release-distribution`: + publicación del sitio en el pipeline (Pages) con
  las URLs de descarga de la release.

## Impact

- Nuevo `site/` (HTML/CSS estático con los tokens; sin framework — §10
  aplica también aquí), `.github/workflows` (job de Pages), `docs/` (la guía
  de agentes se comparte, no se duplica: fuente única).
- Sin impacto en daemon/contrato/superficies.

## Fuera de alcance

- Blog, changelog dinámico, buscador, i18n del sitio más allá de ES/EN.
- Cualquier backend, formulario o recolección de datos.
- Marketplace/registro comunitario (fase 3).
