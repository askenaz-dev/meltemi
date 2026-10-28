# Design — enfasis-con-color

## Context

`ShellCtx::emphasis()` (`tui/src/shell/render.rs`) es el único estilo de
énfasis de la TUI: `REVERSED`, más `BOLD` cuando `present.color` lo permite.
Lo usan diecisiete sitios, de cuatro clases: el banner de permisos y los
contadores de la cabecera; el título del panel enfocado (`▸` más inverso, como
pide la spec); la fila seleccionada de cada lista; y los títulos y cabeceras
de grupo de las vistas (flota, harness, tablero, sesiones por proyecto,
registro de proyectos, bandeja). `Presentation` (`present.rs`) resuelve color
y Unicode a partir del entorno y es una función pura con tests.

`tui/examples/capture_svg.rs` dibuja la vista de sesiones sobre datos fixture
con `TestBackend` y la escribe como SVG: hoy solo distingue tramos en inverso.
meltemi-web la copia tal cual a `/ide/`.

## Decisions

### D1. Dos estilos: bloque y cabecera

`emphasis()` sigue siendo el bloque (banner, contadores, título enfocado, fila
seleccionada, etiqueta de agente configurado). `heading()` es nuevo, para los
títulos de vista y las cabeceras de grupo. Con color: bloque = fondo acento +
texto blanco + negrita; cabecera = negrita en acento sobre el fondo del
terminal. Sin color, `heading()` devuelve `emphasis()`, que es el inverso, así
que el render monocromo no cambia ni un atributo.

Alternativa rechazada: un solo estilo con fondo azul para todo. Cinco bloques
azules en la vista de sesiones son el mismo problema de hoy en otro color.

### D2. El acento según la capacidad del terminal

`Presentation` gana `truecolor`, verdadero cuando `color` lo es y `COLORTERM`
vale `truecolor` o `24bit` (la convención que siguen los terminales que lo
soportan). Con él, `Color::Rgb(37, 99, 235)`, el Egeo exacto; sin él,
`Color::Indexed(27)`, el más cercano de la paleta de 256 colores. Texto en
`Color::White`. No hay detección de 16 colores: un terminal que no entiende
el índice 27 es hoy raro, y la pareja glifo + etiqueta sigue ahí.

Alternativa rechazada: `Color::Blue` del tema del terminal. Cambia con cada
tema y a veces es casi negro sobre fondo oscuro.

### D3. Monocromo intacto

Con `NO_COLOR`, `--no-color` o `TERM=dumb`, `present.color` es falso y ninguno
de los dos estilos nombra un color: el test
`no_color_render_emits_no_colored_cells` sigue comprobando que ninguna celda
tiene color, y la spec «NO_COLOR … MUST NOT pintar fondo» se cumple sin
excepción.

### D4. La captura escribe los colores de las celdas

`capture_svg` agrupa cada fila en tramos por estilo (fondo, texto, negrita,
inverso) y escribe, para cada tramo, un `rect` con el `fill` del fondo cuando
lo hay y el `fill` del texto cuando lo hay, más `font-weight="700"` en
negrita. Los colores salen de las celdas del `TestBackend` como hexadecimal
(`Rgb` y `White`); el ejemplo fija `COLORTERM=truecolor` en su
`PresentationEnv`, así que la captura lleva el Egeo exacto. Los tramos en
inverso sin color siguen saliendo como hoy, por si alguien captura en
monocromo.

## Verificación

- `present.rs`: tests de `truecolor` con `COLORTERM` y sin él, y de que
  `NO_COLOR` lo apaga.
- `render.rs`: un test de que con color el banner y la fila seleccionada
  llevan el acento de fondo y las cabeceras de grupo el acento de texto, y el
  test monocromo existente.
- `tui/tests/docs.rs`: la captura sigue siendo SVG bien formado.
- Gates: `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, suite de `meltemi` (tui).
