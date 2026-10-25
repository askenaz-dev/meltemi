# docs-en-el-sitio

> Vía completa (proposal → design → specs → tasks): trae una **enmienda de la
> constitución (§11, idioma)** con ratificación del mantenedor como gate, el
> índice de las docs y la política de idioma escrita donde se aplique; el
> generador de páginas, si llega, es de meltemi-web. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha P7).

> **Nota 2026-10-24**, tras la mudanza del sitio:
>
> - **Servir las docs en el sitio es primero una decisión de meltemi-web.** Su
>   `product-site` declara como no-objetivo que la documentación de los
>   productos vive en el repositorio de cada uno y que el sitio la enlaza. Para
>   cambiarlo hace falta una decisión de allí (la siguiente es la 0006; existen
>   de la 0001 a la 0005) que resuelva dos cosas: si el sitio renderiza las
>   docs, y cómo llegan los `docs/*.md` del IDE a un build que, por la
>   decisión 0003, no necesita red ni nube. La salida natural es la de los
>   medios y los tokens: una copia confirmada en la web, verificada contra el
>   IDE. Esa decisión es del mantenedor; esta change no la toma.
> - **Lo que no depende de ella se queda aquí**: la enmienda de §11 (con su
>   ratificación como gate), el índice fuente `docs/README.md`, la política de
>   idioma por página y las traducciones. Valen aunque el sitio nunca sirva las
>   docs, porque hoy ya hay una contradicción entre §11 y la práctica.
> - **El generador y las páginas pasan a meltemi-web**, si la decisión es
>   servir: `packages/site`, no `scripts/` de este repositorio.

## Why

Orca tiene quince secciones de documentación navegables en su dominio.
Meltemi tiene `docs/` en el repositorio y, en meltemi.dev, las páginas del IDE
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
- **Del lado del sitio, en meltemi-web, tras su decisión y fuera de esta
  change**: un generador de páginas estáticas `docs/*.md` → HTML sin
  JavaScript, en los dos idiomas, con la navegación lateral, los tokens del
  sitio y los tests de allí como gate (sin orígenes externos, enlaces
  internos válidos, texto alternativo). La dependencia de Markdown, si hace
  falta, se justifica en el design de esa change como herramienta de build
  que no viaja en ningún binario.
- **La estructura de docs se ordena para leerse**: empezar aquí, el método,
  agentes, sesiones, permisos, acceso remoto, releases, referencia del CLI,
  FAQ, privacidad — un índice fuente en `docs/README.md` que el generador
  respeta. Lo que hoy está en un idioma se traduce al otro dentro de esta
  change o se marca «solo en …» hasta que llegue.
- **Los README apuntan al sitio** para leer, y al repositorio para
  contribuir, cuando el sitio sirva las docs; hasta entonces apuntan a
  `docs/` como hoy.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `initial-docs`: + índice fuente y política de idioma por página.
- Constitución §11: **enmienda** (gate de ratificación).

## Impact

- Archivos: `.meltemi/constitution.md` (enmienda), `meltemi.md` (§9 o donde
  se cite la política), `docs/README.md` (índice), `README.md`, `LEEME.md`,
  `CONTRIBUTING.md`, y las traducciones de `docs/`. En meltemi-web, en una
  change propia tras su decisión: el generador, las páginas de docs y la
  navegación.
- Riesgo real: mantener dos idiomas al día. La regla «toda página dice en
  qué idiomas existe» convierte el desfase en una frase visible en vez de en
  una mentira silenciosa.

## Fuera de alcance

- Un buscador en el sitio: exigiría JavaScript (spec `product-site` de
  meltemi-web); el índice y la navegación bastan por ahora.
- La decisión de servir o no las docs, el generador y las páginas:
  meltemi-web.
- Traducir los artefactos del método: siguen en español por §11.
