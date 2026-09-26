# antes-de-anunciar

> Vía rápida (fast-forward): plantillas, un CHANGELOG generado, el quickstart
> reescrito desde los instaladores, una FAQ y una página de privacidad. Sin
> código de producto. Tiene un gate que no es suyo: **no se anuncia nada hasta
> que `procedencia-de-release` cierre** (clave pública en el árbol). Nace del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas P2, P3, P6 y
> P-V1). Absorbe las propuestas nombradas allí como `canales-de-comunidad`,
> `cadencia-y-changelog` y `quickstart-sin-cargo`.

## Why

Orca tiene 77k estrellas, un changelog con entradas casi diarias, testimonios
con nombre y Discord. Meltemi tiene cero estrellas, Discussions apagado, una
sola plantilla de issue (`.github/ISSUE_TEMPLATE/change-proposal.md`), 198
commits desde v0.1.1 sin release ni `CHANGELOG.md`, y un `docs/quickstart.md`
que empieza en `cargo build` y en un `config.toml` escrito a mano — cuando los
instaladores firmados existen y `meltemi session "…" --agent claude-code` o
el chip de flota ya evitan ambas cosas. No hay página de preguntas frecuentes
ni de resolución de problemas, y el sitio, cuyo argumento es «sin
telemetría, sin cuentas, sin red en el daemon», no tiene una página que lo
diga (`grep telemetr site/index.html` → nada).

Nada de esto es producto; todo es embudo. Y el orden importa: la clave
minisign con la que se firmaron v0.1.0 y v0.1.1 no está en el repositorio,
así que un visitante que llegue por un anuncio no puede verificar lo que
descarga. Anunciar antes de cerrar la procedencia quema la primera impresión
con la misma gente a la que se le pide confianza.

## What Changes

- **Canales**: Discussions encendido (lo hace el mantenedor), plantillas de
  issue para error, pregunta y propuesta (la de propuesta apunta al método:
  una change), la plantilla de PR verificada contra `CONTRIBUTING.md`, e
  insignias en los dos README (CI, licencia, última release) — no en el
  sitio, que la spec `product-site` mantiene sin orígenes externos.
- **`CHANGELOG.md` generado del archivo**: `.meltemi/changes/archive/` ya es
  el changelog en bruto (57 changes con fecha); un script lo pliega por
  release en prosa breve, y la regla queda escrita en `docs/release.md`: toda
  release lista las changes que archiva, y la cadencia es «una release al
  cerrar cada tanda, y nunca más de un mes sin release mientras `main` se
  mueva».
- **Quickstart desde los instaladores**: instalar → `meltemi fleet` → una
  sesión gobernada con el agente que ya tienes, en terminal y en la app; sin
  `cargo`, sin TOML; el `.meltemi/` del proyecto aparece con `propose`, no
  antes. La vía de compilar queda en `CONTRIBUTING.md`, que es donde va.
- **`docs/faq.md`** (preguntas frecuentes y resolución de problemas),
  alimentada por lo que ya costó aprender: `not_launchable` y los shims de
  Windows, «el CLI deniega todo» y por qué, el named pipe y el túnel, el
  agente que termina sin decir nada, dónde están los logs.
- **Página de privacidad en el sitio** (`site/privacy.html` y `site/es/`):
  qué no hace Meltemi (telemetría, cuentas, red en el daemon, lectura de
  credenciales) y qué guarda en local y dónde; sin JS, pasa el lint del
  sitio.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `community-governance`: + plantillas de issue por tipo y canal de
  discusión declarado.
- `release-distribution`: + `CHANGELOG.md` por release generado del archivo
  y cadencia declarada.
- `initial-docs`: + quickstart desde instaladores; + FAQ y resolución de
  problemas.
- `product-site`: + página de privacidad.

## Impact

- Archivos: `.github/ISSUE_TEMPLATE/`, `.github/PULL_REQUEST_TEMPLATE.md`,
  `README.md`, `LEEME.md`, `CHANGELOG.md` (nuevo), `scripts/` (generador),
  `docs/release.md`, `docs/quickstart.md`, `docs/faq.md` (nuevo),
  `site/privacy.html`, `site/es/privacy.html`, enlaces en el pie del sitio.
- Cero dependencias nuevas; cero código de producto.
- Gate externo: `procedencia-de-release` cerrada (clave en el árbol, v0.1.2
  firmada) antes de publicar el anuncio. Los artefactos de esta change se
  pueden preparar antes; el anuncio, no.

## Fuera de alcance

- Docs navegables en el sitio y la enmienda de idioma: `docs-en-el-sitio`.
- Gestores de paquetes y aviso de versión nueva:
  `distribucion-y-actualizacion`.
- Un demo animado en la portada: `demo-animada-del-sitio`.
- Telemetría, métricas de crecimiento o insignias de terceros en el sitio:
  constitución §9 y spec `product-site`.
