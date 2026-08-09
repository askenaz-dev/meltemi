# Tareas — panel-opaco-y-nav-plegable

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(panel-opaco-y-nav-plegable N.M)` y sin trailers de co-autoría. Gates del
repo en cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la
suite del crate tocado.

## 1. El fondo que faltaba, y el lint que lo habría visto

- [x] 1.1 `ProjectSwitcher.svelte`: el panel pinta `--surface` (token
  definido) en vez del inexistente `--surface-1` (design D1) — escenario «El
  conmutador de proyectos cubre lo que tapa» — gates: suite de cableado
- [x] 1.2 Lint de variables de estilo en `desktop/tests/scenarios_shell.rs`:
  reúne cada `var(--x)` usada y cada `--x` definida en `desktop/ui/src`, y
  falla nombrando archivo y línea de las usadas sin definir (design D1) —
  escenario «Ninguna variable de estilo se usa sin existir» — gates: `cargo
  test -p meltemi-desktop`

## 2. La barra que se pliega

- [x] 2.1 Control de pliegue en la cabecera de la barra y riel angosto: las
  entradas conservan etiqueta accesible, foco y dígito; el contador de
  permisos permanece visible; strings ES/EN (design D2) — escenarios «Plegar
  y desplegar desde la cabecera» y «Plegada no pierde alcance» — gates: suite
  de cableado
- [x] 2.2 Persistencia junto a tema y geometría de ventana; perfil nuevo
  arranca desplegado (design D3) — escenario «El pliegue se recuerda, el
  primer arranque no» — gates: suite de cableado

## 3. Cierre

- [x] 3.1 `meltemi validate panel-opaco-y-nav-plegable` limpio y `meltemi
  verify` con los cinco escenarios enlazados (meta: cero marcas manuales);
  suite completa, clippy y fmt verdes; comprobación sobre el binario de
  release de que el panel cubre y la barra pliega, con captura
