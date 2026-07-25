## 1. Fundaciones del sitio

- [x] 1.1 Estructura `site/` (portada, método, agentes, descargas) en HTML estático sin framework, sin generador y sin JavaScript, con el archivo de dominio del sitio _(Req: Sitio estático sin rastreo ni orígenes de terceros; design D1)_
- [x] 1.2 `site/tokens.css` derivado de `desktop/ui/src/app.css` + hoja base con la pila tipográfica de sistema, densidad, radios 4/8, filetes y sombra única para superposiciones _(Req: Identidad del design system aplicada al sitio; design D4)_
- [x] 1.3 Marcas de `brand/` puestas en escena en el artefacto publicado, sin copias binarias en `site/` y sin fuentes remotas _(Req: Identidad del design system aplicada al sitio; design D4, D7)_

## 2. Contenido

- [x] 2.1 Portada: producto completo (daemon + dos superficies en paridad, tres plataformas), lema, promesa BYO, estado honesto y "qué no es" _(Req: Historia del producto completa y honesta; design D3)_
- [x] 2.2 Página del método como herramienta (spec-first, deltas sobre verdad viva, EARS, trazabilidad) con manifiesto y constitución enlazados íntegros _(Req: Historia del producto completa y honesta; design D3)_
- [x] 2.3 Página de agentes: BYO-agent, niveles de integración y perfiles multi-suscripción como resumen breve que enlaza `docs/agentes.md` y el quickstart _(Req: Fuente única compartida con la documentación; design D3)_
- [~] 2.4 Capturas de ambas superficies desde un repositorio fixture temporal con `mock-agent`, con texto alternativo y procedencia declarada — **parcial**: la captura de terminal se genera del propio renderizador (`cargo run -p meltemi --example capture_svg`) sobre datos fixture y viaja con alt y procedencia; la de escritorio exige una ventana real y queda como acción del mantenedor con su procedimiento documentado en `docs/ux/capturas.md` (el sitio declara la ausencia en vez de fingirla) _(Req: Capturas reales desde un proyecto fixture; design D5)_
- [x] 2.5 Árbol gemelo ES/EN con raíz en inglés, `lang`, `hreflang` y conmutador por enlace plano _(Req: Paridad de idiomas ES/EN del sitio; design D6)_

## 3. Descargas

- [x] 3.1 Página de descargas por plataforma con enlaces de última release (archivo del núcleo, instalador de escritorio, script instalador) y verificación de checksum y firma enlazada a su fuente _(Req: Descargas resueltas a la última release firmada; design D2)_
- [x] 3.2 Pipeline: normalizar los nombres de artefacto por plataforma —incluidos los instaladores del cliente de escritorio— y publicar los scripts instaladores como artefactos con su checksum en el `SHA256SUMS` firmado _(Req release-distribution: Nombres de artefacto estables por plataforma; Instalador auditable; design D2)_
- [x] 3.3 Base canónica de descarga declarada una sola vez y compartida por el sitio y los scripts instaladores _(Req: Descargas resueltas a la última release firmada; design D2)_

## 4. Publicación

- [x] 4.1 Job de Pages en `release.yml` dependiente del empaquetado: publica el sitio solo tras gates y artefactos verdes, dejando intacta la edición anterior ante un rojo _(Req release-distribution: Publicación del sitio con la release; design D7)_
- [x] 4.2 Publicación desde `main` para cambios solo de contenido, con el lint del sitio como condición previa y sin alterar los enlaces de descarga _(Req release-distribution: Publicación del sitio con la release; design D7)_

## 5. Verificación

- [x] 5.1 Lint del sitio (`core/meltemid/tests/site.rs`): páginas y secciones requeridas, enlaces internos y hacia `docs/` resueltos, cero JavaScript y cero orígenes externos _(Req: Verificación del sitio como gate de CI; Sitio estático sin rastreo ni orígenes de terceros; design D8)_
- [x] 5.2 Lint de descargas: sin literal de versión y nombres de artefacto cruzados contra `release.yml` _(Req: Descargas resueltas a la última release firmada; design D2, D8)_
- [x] 5.3 Lint de identidad: tokens del sitio idénticos en nombre y valor a los del cliente de escritorio _(Req: Identidad del design system aplicada al sitio; design D4)_
- [x] 5.4 Lint de fuente única y contenido: rehúsa bloques de seis o más líneas idénticos a `docs/`, exige gemelas de idioma, alt y procedencia por captura, y rehúsa nombres de productos de terceros fuera de datos de interoperabilidad _(Req: Fuente única compartida con la documentación; Paridad de idiomas ES/EN del sitio; Capturas reales desde un proyecto fixture; Historia del producto completa y honesta; design D3, D5, D6)_
- [x] 5.5 CI: el lint del sitio corre en cada PR en las tres plataformas y su rojo bloquea merge y publicación _(Req: Verificación del sitio como gate de CI; design D8)_

## 6. Calidad y cierre

- [x] 6.1 README, `LEEME.md` y `docs/release.md` enlazan el sitio publicado como puerta de entrada, con una sola dirección canónica _(Req: Fuente única compartida con la documentación; design D3)_
- [x] 6.2 `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las tres plataformas _(constitución §7)_
