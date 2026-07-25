## Context

El producto ya existe completo: daemon `meltemid`, superficie de terminal
(`meltemi`), cliente de escritorio Tauri y releases firmadas con instaladores
por plataforma (`docs/release.md`, `.github/workflows/release.yml`). Lo que no
existe es el lugar público que lo cuente: hoy la única puerta de entrada es el
README de GitHub, que asume que el visitante ya sabe qué busca. El dominio
`meltemi.dev` está reservado (plan de cambios, namespaces), `design-system/` es
la fuente visual normativa y la guía de agentes (`flota-deteccion-guia`)
necesita casa pública. Esta change añade una superficie de *lectura* — no una
superficie de producto: no consume el daemon, no toca el contrato y no suma
métodos RPC, por lo que la paridad de núcleo (§4) no se ve afectada.

## Goals / Non-Goals

**Goals:** sitio estático en `site/` con la historia del producto completo (dos
superficies en paridad, tres plataformas), descargas que resuelven a la release
firmada más reciente sin literales de versión, identidad única
producto↔web derivada del design system, postura de privacidad sin rastreo ni
terceros (§9) y verificación del sitio como gate bloqueante de CI.
**Non-Goals:** blog, changelog dinámico, buscador, idiomas más allá de ES/EN;
backend, formularios o recolección de datos de cualquier tipo; hospedar copias
de artefactos, de scripts instaladores o de la documentación; marketplace o
registro comunitario (fase 3).

## Decisions

### D1 — HTML y CSS a mano, sin generador y con cero JavaScript
`site/` son páginas HTML estáticas y una hoja de estilos; no hay framework, no
hay generador de sitios y no hay paso de build: lo que está en el repositorio es
byte a byte lo que se publica. Alternativas evaluadas: Astro/Hugo (árbol de
dependencias y una toolchain más que pinear y auditar para media docena de
páginas — §10 no lo justifica), Zola (tentador por ser un único binario Rust,
pero seguiría siendo una dependencia pineada y un lenguaje de plantillas para
contenido que cambia poco), Tailwind (paso de build y clases utilitarias que
duplicarían los tokens que ya tenemos). Cero JavaScript no es ascetismo: es la
forma de que la postura de privacidad sea verificable por inspección del
contenido y no una promesa. Todo lo que la página necesita — navegación,
conmutador de idioma, tema claro/oscuro, detalles plegables — se resuelve con
HTML y CSS.

### D2 — Descargas por redirector de "última release", nombres estables
El sitio no conoce ninguna versión: cada descarga apunta a
`…/releases/latest/download/<nombre-estable>`, la URL que el alojamiento de
releases resuelve a la release más reciente. Para que ese patrón funcione, los
nombres de artefacto deben ser estables y libres de versión: los archivos del
núcleo ya lo son (`meltemi-Windows.zip`, `meltemi-Linux.tar.gz`,
`meltemi-macOS.tar.gz`), pero los instaladores que emite el bundler de Tauri
traen la versión en el nombre, así que el job de empaquetado los renombra a un
esquema por plataforma antes de subirlos. Los scripts instaladores se publican
también como assets de cada release, con su checksum dentro del `SHA256SUMS`
firmado: así el sitio enlaza un script cuyo hash viaja firmado, sin hospedar
copia alguna (una copia en el dominio sería una segunda verdad con su hash
propio, exactamente lo que el "sin `curl | sh` a ciegas" quiere evitar). La base
canónica de descarga se declara una sola vez y el lint comprueba que el sitio y
los scripts instaladores nombren el mismo repositorio. Alternativa rechazada:
interpolar la versión en tiempo de publicación (obligaría a republicar el sitio
por cada parche y a que el sitio pudiera mentir entre release y despliegue).

### D3 — Frontera de contenido: el sitio narra, `docs/` opera
El sitio hospeda únicamente narrativa de producto — qué es, para quién, el
método como herramienta, capturas, descargas — y enlaza lo operativo a su fuente
única: quickstart, guía de agentes con sus perfiles multi-suscripción,
referencia CLI, verificación de checksum y firma, notas de plataforma,
manifiesto y constitución. Cada tema aparece con un resumen breve y su enlace,
nunca con el procedimiento repetido. Para que la regla no dependa de la
disciplina del autor, el lint rehúsa cualquier bloque de seis o más líneas
consecutivas idéntico a un documento de `docs/`; el umbral deja pasar la línea
de comando de instalación (el gancho legítimo de la página de descargas) y
atrapa el copiado de secciones. Alternativa rechazada: generar el sitio desde
`docs/` (volvería a exigir generador, contra D1) o espejar la documentación en
el dominio (dos verdades que se desincronizan en la primera release).

### D4 — Identidad derivada y verificada, tipografía de sistema
`site/tokens.css` declara los mismos tokens que el cliente de escritorio
(`desktop/ui/src/app.css`, derivado a su vez de `design-system/` y
`docs/ux/design-system.md`) y el lint compara nombre por nombre: un valor que
divergiera falla el build, igual que una referencia CLI desactualizada. El sitio
hereda del sistema la densidad, los radios de 4 y 8 px, los filetes de 1 px con
un único nivel de sombra reservado a superposiciones, el vocabulario de estado
símbolo + palabra y la regla dura de no animar el layout de sus bandas de aviso.
La tipografía es la misma pila de sistema del cliente (Inter preferida, jamás
requerida): no se cargan fuentes remotas — un `<link>` a un proveedor de fuentes
es una petición de terceros con la IP del visitante, inaceptable bajo §9 — ni se
autohospedan familias completas por peso y licencia; el wordmark de la marca ya
viene vectorizado, así que la identidad sobrevive sin ninguna fuente instalada.
Las marcas se toman de `brand/` (fuente única) y el job de publicación las pone
en escena dentro del artefacto, sin copias binarias en `site/`.

### D5 — Capturas desde un fixture con el agente simulado
Las capturas de ambas superficies se toman sobre un repositorio fixture temporal
con `mock-agent`, nunca sobre un proyecto real ni una cuenta de agente: así
ninguna imagen filtra rutas personales, nombres de proyecto, identidades ni
marcas de terceros, y la regla de fixtures que gobierna los e2e gobierna también
el material de marketing (§2 y la regla del README de no nombrar productos de
terceros fuera de datos factuales de interoperabilidad). Cada captura declara su
procedencia — versión del producto, plataforma y superficie — y lleva texto
alternativo descriptivo; el lint exige ambos, porque lo que hay dentro del píxel
no lo puede juzgar un test.

### D6 — ES/EN como árboles gemelos, raíz en inglés
Dos árboles paralelos: inglés en la raíz y español bajo su prefijo de idioma,
con `lang` y alternancia `hreflang` declaradas y un conmutador que es un enlace
plano. La raíz en inglés replica la convención pública ya establecida por el
repositorio (README en inglés con `LEEME.md` como espejo) y evita que el
alojamiento estático tenga que negociar idioma — algo que GitHub Pages no hace.
No hay detección por IP ni redirección automática: además de exigir código
(contra D1), inferir el idioma del visitante desde su red es precisamente el
tipo de inferencia que §9 no quiere. El lint exige que cada página tenga su
gemela: un idioma a medias es peor que un idioma ausente.

### D7 — Publicación con GitHub Pages dentro del pipeline existente
El despliegue es un job de Pages en `.github/workflows/release.yml` que depende
del empaquetado (`package` y `package-gui`): el sitio no puede anunciar una
release cuyos artefactos no se publicaron, así que un gate rojo deja el sitio
anterior intacto. Los cambios que solo tocan contenido se publican desde `main`
con el mismo job y el lint como condición previa — las URLs de descarga son
libres de versión (D2), así que republicar contenido nunca desalinea las
descargas. Alojamiento: Pages sirve estático sobre el dominio propio con HTTPS,
sin backend y sin cookies de aplicación. Alternativas rechazadas: hosts de
terceros tipo Netlify/Vercel (dependencia extra, y sus capas de edge traen
analítica y cookies por defecto: exactamente lo que §9 prohíbe) y un VPS propio
(custodia de TLS y operación sin ganancia alguna para contenido estático).

### D8 — Verificación: un lint del workspace, sin red y sin navegador
La verificación es un test Rust del propio workspace
(`core/meltemid/tests/site.rs`, junto al lint de release, con el que comparte el
cruce contra `release.yml`): lee los archivos del repositorio y falla con un
diagnóstico localizado. Nada de navegador headless ni de comprobación de enlaces
externos por red — los e2e y los gates del proyecto no salen a la red, y un lint
que dependa de terceros vivos es un lint intermitente. Cubre: secciones y
páginas requeridas, enlaces internos y hacia `docs/` resueltos, ausencia de
JavaScript y de orígenes externos, descargas sin literal de versión con nombres
cruzados contra el pipeline, coherencia de tokens, gemelas de idioma, capturas
con alt y procedencia, y el anti-duplicación de D3. Corre en la suite del
workspace, luego en las tres plataformas y en cada PR, y su rojo bloquea tanto
el merge como la publicación.

## Risks / Trade-offs

- **El redirector de "última release" es un contrato del alojamiento** → está
  documentado y es estable; el pipeline normaliza los nombres y el lint los
  cruza, y si cambiara solo hay un lugar que ajustar (la tabla de descargas).
- **Renombrar los instaladores del bundler** podría chocar con un actualizador
  automático futuro → hoy no existe (ni red ni telemetría en el cliente); si
  llega, su change decide el esquema de nombres con este precedente a la vista.
- **Las capturas envejecen respecto de la interfaz** → procedencia declarada por
  captura y refresco como tarea de release; el lint no puede juzgar un PNG y no
  finge lo contrario.
- **El anti-duplicación puede dar falsos positivos** con listas cortas de
  comandos → umbral de seis líneas y una única excepción declarada (la línea de
  instalación); mejor rehusar de más que dejar entrar una segunda verdad.
- **La postura de privacidad cubre el contenido, no el alojamiento** → el lint
  garantiza que el sitio no rastrea; que Pages sirva estático sin cookies es una
  propiedad del alojamiento que se declara honestamente, no se testea.
- **Secuencia con `flota-deteccion-guia`**: la página de agentes enlaza
  `docs/agentes.md`, que produce esa change; el lint exige el enlace, así que
  esta change entra después.
- **El slug del repositorio no es único hoy** (los scripts instaladores apuntan
  a una base distinta del namespace declarado en el plan) → el lint exige una
  sola base canónica y la tarea de descargas la unifica; sin eso, "la release
  más reciente" podría resolver a un repositorio que no es el nuestro.

## Migration Plan

Aditivo por completo: se suma `site/`, un job de publicación, un lint y los
enlaces de entrada desde README/LEEME. Sin cambios en el daemon, en `proto/`, en
la TUI ni en la GUI, y sin métodos RPC nuevos: la matriz de paridad no se toca.
El dominio propio se apunta cuando el mantenedor habilite el DNS; hasta
entonces la URL por defecto de Pages sirve el sitio y ni el lint ni los enlaces
de descarga dependen del dominio. Reversión: retirar el job de publicación y el
directorio `site/`; nada más del repositorio depende de ellos.

## Open Questions

- ¿Versionar el sitio por release (una copia navegable por tag) o mantener una
  sola edición viva? Se decide con demanda real; hoy "última" es honesto porque
  las descargas también lo son.
- Si el mantenedor quisiera una URL corta para el instalador bajo el dominio,
  ¿un redirect estático de Pages hacia el asset firmado, en vez de hospedar el
  script? Queda por decidir con la infraestructura de DNS en la mano.
- ¿La raíz debe seguir siendo inglés cuando existan datos de uso? El cambio
  sería un delta menor; hoy manda la convención del repositorio.
