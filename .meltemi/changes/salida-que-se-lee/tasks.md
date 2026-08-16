# Tareas — salida-que-se-lee

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(salida-que-se-lee N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite del crate
tocado. **Ojo concurrencia**: hay otras sesiones con changes en vuelo —
commitear en cuanto cada tarea cierre, nunca dejar el árbol sucio entre pasos.

## 1. El formato como elección

- [ ] 1.1 `Format { Human, Json, Yaml }` sustituye al booleano en
  `render_outcome`/`render_error`; `--yaml` en la gramática y el rehúso de
  usarlo junto a `--json`; la dependencia YAML pineada y justificada en el
  design, auditada por `cargo-deny` (design D1, D2) — escenarios «YAML emite un
  documento y nada más», «El error en YAML también es un documento» y «Dos
  formatos de máquina a la vez se rehúsan» — gates: suite del crate,
  `cargo deny check`

## 2. La lectura

- [ ] 2.1 Decisión de color en un solo lugar: `--no-color`, `NO_COLOR`,
  `TERM=dumb` y stdout sin TTY apagan; los formatos de máquina nunca pintan; el
  pintado son secuencias ANSI propias, sin crate (design D3, D4) — escenarios
  «La salida redirigida no lleva color» y «El usuario apaga el color» — gates:
  suite del crate
- [ ] 2.2 `specs` y `changes` ganan resumen y alineación calculada del
  contenido, y el color codifica estado y tipo **de forma redundante**; el test
  compara la salida pintada con la monocroma tras retirar los escapes y exige
  el mismo texto (design D3, D5) — escenarios «El listado abre con su resumen»,
  «Las columnas se alinean con el contenido» y «Sin color no se pierde
  información» — gates: suite del crate

## 3. Cierre

- [ ] 3.1 `meltemi validate salida-que-se-lee` limpio y `meltemi verify` con
  los ocho escenarios enlazados (meta: cero marcas manuales); referencia CLI
  regenerada; suite completa, clippy, fmt y `cargo deny` verdes; comprobación
  en la terminal real —color donde hay TTY, sin color en un pipe— con captura;
  entrada en `docs/plan-de-cambios.md`
