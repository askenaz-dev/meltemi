# Tareas — enfasis-con-color

Vía rápida. Un commit atómico por tarea, con referencia
`(enfasis-con-color N.M)` y **sin trailers de co-autoría**. Gates del repo en
cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite de `meltemi` (tui). **Ninguna tarea añade una
dependencia**; el daemon no se toca y no hay RPC nuevo.

## 1. El énfasis

- [x] 1.1 `tui/src/shell/present.rs`: `Presentation::truecolor` resuelto de
  `COLORTERM` (`truecolor` o `24bit`), apagado con el color; tests
  (escenario «El acento según el terminal»).
- [x] 1.2 `tui/src/shell/render.rs`: `accent()`, `emphasis()` con fondo acento
  y texto blanco en negrita cuando hay color, `heading()` en negrita acento
  sin fondo; los títulos de vista y cabeceras de grupo pasan a `heading()`;
  test del escenario «Énfasis con color», y el monocromo existente sigue
  verde (escenario «Monocromo conserva el inverso»).
- [x] 1.3 `tui/examples/capture_svg.rs`: tramos por estilo con `fill` de
  fondo y texto y `font-weight` en negrita, `COLORTERM=truecolor` en el
  entorno del ejemplo; `docs/media/tui-sessions.svg` regenerada; el lint de
  `tui/tests/docs.rs` verde (escenario «La captura muestra los colores»).
