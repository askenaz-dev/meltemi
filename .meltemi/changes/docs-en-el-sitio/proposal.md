# docs-en-el-sitio

> Vía completa (proposal → design → specs → tasks): trae una **enmienda de la
> constitución (§11, idioma)** con ratificación del mantenedor como gate, un
> generador de páginas estáticas y la política de idioma escrita donde se
> aplique. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha P7).

## Why

Orca tiene quince secciones de documentación navegables en su dominio.
Meltemi tiene `docs/` en el repositorio y cuatro páginas en meltemi.dev
(inicio, agentes, descargas, método, con su espejo en `es/`). Un visitante
que llega desde GitHub no puede leer el quickstart, la referencia del CLI
ni la página de agentes sin abrir archivos Markdown en el repositorio.

Y hay una contradicción que ya no se puede posponer. La constitución §11
manda «documentación de producto y artefactos del método: español neutro»,
y la práctica es otra: `docs/quickstart.md`, `docs/agentes.md`,
`docs/release.md`, `docs/sesion-libre.md`, `docs/paridad-nucleo.md` y
`docs/plataformas.md` están en inglés; `docs/acceso-remoto.md` y
`docs/conformidad-manual.md`, en español. El README es inglés con `LEEME.md`
al lado. Nadie decidió eso a medias: pasó porque el público al que se
escribe es global y el método se escribe en la lengua del mantenedor. Servir
las docs en el sitio obliga a decidir, y decidir bien es enmendar §11 en vez
de fingir que se cumple.

## What Changes

- **Enmienda de §11, con ratificación como gate**: los artefactos del método
  (`.meltemi/`, propuestas, designs, specs, tasks, backlog) siguen en
  español neutro; la **documentación de producto** se escribe con el inglés
  como lengua fuente y se publica en inglés y en español, y toda página
  pública dice en qué idiomas existe. El texto exacto lo trae el design; la
  constitución solo cambia con la firma del mantenedor fundador. Hasta
  entonces, la enmienda queda aplicada y marcada pendiente, como las
  anteriores.
- **Generador de páginas estáticas**: `docs/*.md` → `site/docs/` y
  `site/es/docs/`, HTML sin JavaScript, con la navegación lateral, los
  tokens del sitio y el lint como gate (sin orígenes externos, enlaces
  internos válidos, texto alternativo). El generador vive en `scripts/` o en
  el crate del lint; la dependencia de Markdown, si hace falta, se justifica
  en el design como herramienta de build que no viaja en ningún binario
  (§10).
- **La estructura de docs se ordena para leerse**: empezar aquí, el método,
  agentes, sesiones, permisos, acceso remoto, releases, referencia del CLI,
  FAQ, privacidad — un índice fuente en `docs/README.md` que el generador
  respeta. Lo que hoy está en un idioma se traduce al otro dentro de esta
  change o se marca «solo en …» hasta que llegue.
- **Los README apuntan al sitio** para leer, y al repositorio para
  contribuir.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `product-site`: + requisito «Documentación navegable generada de `docs/`,
  sin JS, con lint y en los dos idiomas declarados».
- `initial-docs`: + índice fuente y política de idioma por página.
- Constitución §11: **enmienda** (gate de ratificación).

## Impact

- Archivos: `.meltemi/constitution.md` (enmienda), `meltemi.md` (§9 o donde
  se cite la política), `docs/README.md` (índice), `scripts/` o
  `site/lint` (generador), `site/docs/`, `site/es/docs/`, `site/*.html`
  (navegación), `README.md`, `LEEME.md`, `CONTRIBUTING.md`.
- Dependencia candidata de build (Markdown → HTML), fuera de los binarios
  distribuidos.
- Riesgo real: mantener dos idiomas al día. La regla «toda página dice en
  qué idiomas existe» convierte el desfase en una frase visible en vez de en
  una mentira silenciosa.

## Fuera de alcance

- Un buscador en el sitio: exigiría JavaScript (`product-site`); el índice y
  la navegación bastan por ahora.
- Traducir los artefactos del método: siguen en español por §11.
