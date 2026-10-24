## REMOVED Requirements

### Requirement: Sitio estático sin rastreo ni orígenes de terceros
**Reason**: El sitio del producto se mudó a `askenaz-dev/meltemi-web`, que sirve meltemi.dev desde el 2026-10-21; este repositorio deja de alojarlo y de publicarlo.
**Migration**: La regla vive en la capacidad `product-site` de meltemi-web (`openspec/specs/product-site`), cuyo pipeline la verifica.

### Requirement: Historia del producto completa y honesta
**Reason**: Las páginas del IDE se mudaron a `askenaz-dev/meltemi-web` (`/ide/` y `/es/ide/`); este repositorio deja de alojarlas.
**Migration**: La regla vive en la capacidad `product-site` de meltemi-web.

### Requirement: Descargas resueltas a la última release firmada
**Reason**: La página de descargas se mudó a `askenaz-dev/meltemi-web`; este repositorio deja de alojarla.
**Migration**: La web enlaza `releases/latest/download/<nombre estable>` y su suite lo verifica; los nombres estables siguen garantizados aquí por «Nombres de artefacto estables por plataforma» de `release-distribution`.

### Requirement: Fuente única compartida con la documentación
**Reason**: El sitio se mudó a `askenaz-dev/meltemi-web`; la regla de no copiar documentación al sitio ya no tiene sitio que vigilar en este repositorio.
**Migration**: La regla vive en la capacidad `product-site` de meltemi-web, que enlaza la documentación de este repositorio en vez de copiarla.

### Requirement: Identidad del design system aplicada al sitio
**Reason**: El sitio se mudó a `askenaz-dev/meltemi-web`; el design system y la marca siguen siendo normativos aquí, pero el sitio que los aplica ya no.
**Migration**: meltemi-web comprueba sus tokens contra `desktop/ui/src/app.css` de este repositorio con `pnpm site:tokens --ide <ruta>`.

### Requirement: Capturas reales desde un proyecto fixture
**Reason**: Las capturas se publican en el sitio de `askenaz-dev/meltemi-web`, no en este repositorio.
**Migration**: `scripts/capture-desktop.ps1` y `tui/examples/capture_svg.rs` siguen aquí, con el proyecto fixture y el agente simulado, y escriben en `docs/media/`, de donde la web las copia.

### Requirement: Paridad de idiomas ES/EN del sitio
**Reason**: El sitio se mudó a `askenaz-dev/meltemi-web`, que mantiene sus páginas en los dos idiomas.
**Migration**: La regla vive en la capacidad `product-site` de meltemi-web, cuya suite comprueba los gemelos de idioma.

### Requirement: Verificación del sitio como gate de CI
**Reason**: El lint del sitio (`core/meltemid/tests/site.rs`) sale de este repositorio junto con el sitio.
**Migration**: El sitio se verifica con la suite de meltemi-web (`pnpm test`), que su CI corre en cada push y pull request.
