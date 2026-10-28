# antes-de-anunciar

> Vía rápida (fast-forward): plantillas, un CHANGELOG generado, el quickstart
> reescrito desde los instaladores y una FAQ. Sin código de producto. Tenía un
> gate que no era suyo, `procedencia-de-release`, y ya se cumplió (ver la nota
> de abajo). Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas P2, P3, P6 y
> P-V1). Absorbe las propuestas nombradas allí como `canales-de-comunidad`,
> `cadencia-y-changelog` y `quickstart-sin-cargo`.

> **Nota 2026-10-24**, tras la mudanza del sitio y la release automática:
>
> - **La página de privacidad ya no es de esta change.** El sitio vive en
>   `askenaz-dev/meltemi-web`, que ya tiene `/privacy/` y `/es/privacy/` con el
>   IDE sin cuenta y sin telemetría. Las tres afirmaciones que le faltaban,
>   que el daemon no abre puertos de red, que no lee credenciales y dónde
>   guarda sus datos, las añade la change `ide-privacy-statements` de
>   meltemi-web.
> - **El gate se cumplió.** `procedencia-de-release` se retiró y la sustituyó
>   `release-con-el-push`: la v0.2.0 se publicó el 2026-10-23 y se verificó
>   con `gh attestation verify`, con red y sin ella. Ya no hay clave minisign
>   que publicar.
> - **El CHANGELOG tiene un lector.** `meltemi-release notes <versión>` toma de
>   `CHANGELOG.md` la sección titulada `## <versión>`; lo que genere esta
>   change tiene que respetar ese formato (ver «What Changes»).

## Why

Orca tiene 77k estrellas, un changelog con entradas casi diarias, testimonios
con nombre y Discord. Meltemi tiene cero estrellas, Discussions apagado, una
sola plantilla de issue (`.github/ISSUE_TEMPLATE/change-proposal.md`), 35
commits desde la v0.2.0 sin `CHANGELOG.md` (las notas de la 0.2.0 las generó
GitHub), y un `docs/quickstart.md`
que empieza en `cargo build` y en un `config.toml` escrito a mano — cuando los
instaladores firmados existen y `meltemi session "…" --agent claude-code` o
el chip de flota ya evitan ambas cosas. No hay página de preguntas frecuentes
ni de resolución de problemas. El sitio, cuyo argumento es «sin telemetría,
sin cuentas, sin red en el daemon», ya tiene una página de privacidad en
meltemi-web que dice las dos primeras; la tercera la añade
`ide-privacy-statements`.

Nada de esto es producto; todo es embudo. Y el orden importaba: un visitante
que llegara por un anuncio tenía que poder verificar lo que descarga, y
anunciar antes quema la primera impresión con la misma gente a la que se le
pide confianza. Eso ya se cumple: desde la v0.2.0 lo publicado se verifica con
la atestación del workflow, sin ninguna clave del mantenedor.

## What Changes

- **Canales**: Discussions encendido (lo hace el mantenedor), plantillas de
  issue para error, pregunta y propuesta (la de propuesta apunta al método:
  una change), la plantilla de PR verificada contra `CONTRIBUTING.md`, e
  insignias en los dos README (CI, licencia, última release) — no en el
  sitio, que se mantiene sin orígenes externos (spec `product-site` de
  meltemi-web).
- **`CHANGELOG.md` generado del archivo**: `.meltemi/changes/archive/` ya es
  el changelog en bruto (65 changes con fecha); un script lo pliega por
  release en prosa breve, y la regla queda escrita en `docs/release.md`: toda
  release lista las changes que archiva, y la cadencia es «una release al
  cerrar cada tanda, y nunca más de un mes sin release mientras `main` se
  mueva». **Formato**: cada release es una sección titulada
  `## <versión> — <fecha>` (la versión sin corchetes y con `v` opcional, la
  fecha opcional), porque `meltemi-release notes <versión>` extrae de ahí las
  notas de la release; sin sección, el flujo cae a las notas que genera
  GitHub (código de salida 3), que es como salió la 0.2.0. El script escribe
  la sección de la versión que se va a publicar, y se comprueba con
  `meltemi-release notes <versión>`, el lector real, no con una copia de su
  regla.
- **Quickstart desde los instaladores**: instalar → `meltemi fleet` → una
  sesión gobernada con el agente que ya tienes, en terminal y en la app; sin
  `cargo`, sin TOML; el `.meltemi/` del proyecto aparece con `propose`, no
  antes. La vía de compilar queda en `CONTRIBUTING.md`, que es donde va.
- **`docs/faq.md`** (preguntas frecuentes y resolución de problemas),
  alimentada por lo que ya costó aprender: `not_launchable` y los shims de
  Windows, «el CLI deniega todo» y por qué, el named pipe y el túnel, el
  agente que termina sin decir nada, dónde están los logs.

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

## Impact

- Archivos: `.github/ISSUE_TEMPLATE/`, `.github/PULL_REQUEST_TEMPLATE.md`,
  `README.md`, `LEEME.md`, `CHANGELOG.md` (nuevo), `scripts/` (generador),
  `docs/release.md`, `docs/quickstart.md`, `docs/faq.md` (nuevo). El sitio
  no se toca desde aquí: la página de privacidad es de meltemi-web.
- Cero dependencias nuevas; cero código de producto.
- Gate del anuncio: `ide-privacy-statements` en meltemi-web, porque la
  página de privacidad es lo que enlaza el anuncio y tiene que decir lo que
  el anuncio afirma del producto. El de la procedencia ya se cumplió con la v0.2.0. Los
  artefactos de esta change se pueden preparar antes; el anuncio, no.

## Fuera de alcance

- Docs navegables en el sitio y la enmienda de idioma: `docs-en-el-sitio`.
- Gestores de paquetes y aviso de versión nueva:
  `distribucion-y-actualizacion`.
- Un demo animado en la portada: `demo-animada-del-sitio`.
- La página de privacidad y cualquier otra página del sitio: viven en
  meltemi-web (change `ide-privacy-statements`).
- Telemetría, métricas de crecimiento o insignias de terceros en el sitio:
  constitución §9 y spec `product-site` de meltemi-web.
