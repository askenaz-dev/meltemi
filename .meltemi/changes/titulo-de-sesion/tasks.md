# Tareas — titulo-de-sesion

Vía completa. Un commit atómico por tarea, con referencia
`(titulo-de-sesion N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, la suite del crate
tocado, y en `desktop/ui` además `npm run check`, `npm run lint:i18n`,
`npm run check:forms` y `npm test`.

## 1. La derivación

- [x] 1.1 Función pura de derivación en el daemon: primera línea no vacía,
  espacios colapsados, truncado con elipsis a 64 **caracteres** (jamás bytes),
  con tests de unicode, líneas vacías, instrucción de una palabra, instrucción
  toda en blanco y truncado exacto en el límite (design D1) — escenario «Título
  derivado de la primera instrucción»
  <!-- 2026-08-09: módulo propio `core/meltemid/src/title.rs`, puro y sin
  dependencias. La elipsis **sustituye** al último carácter en vez de sumarse,
  para que un título recortado nunca supere el presupuesto que declara. Siete
  tests, dos de ellos con texto que solo un corte por bytes rompería. -->
- [ ] 1.2 Deuda declarada al implementar: el título se deriva **por
  caracteres**, que es lo correcto para no partir un carácter, pero el ancho
  visible depende del glifo. Si alguna vez importa el ancho en columnas de
  terminal, es medida de la TUI y no de la derivación

## 2. El índice y el contrato

- [x] 2.1 `SessionRecord` gana `title: Option<String>` con
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, y **`merge_into`
  gana su rama**: un registro sin título conserva el que había (design D4) —
  escenario «El título sobrevive al cierre de la sesión» — el test pliega inicio
  + cierre y exige que siga ahí
- [x] 2.2 `record_from_log` recupera el título del evento de inicio (design D3)
  — escenario «El título se recupera del registro»
- [x] 2.3 `SessionInfo` y el evento `session_started` ganan `title` opcional en
  `meltemi-proto` y en sus dos schemas, fuera de `required`; conformidad de tres
  vías en `conformance.rs` (presente, omitido, byte-igualdad de la forma
  omitida) y `npm run gen:forms` con el generado commiteado (design D3) — gates:
  `check:forms`

## 3. Los caminos que derivan

- [x] 3.1 La sesión libre deriva el título del **texto crudo** de la
  instrucción, antes de expandir referencias, y lo escribe en el registro de
  inicio (design D1, D2). El evento llega con 2.3 — escenario «El título sale
  del texto que se escribió», que se enlaza al llegar su test e2e
- [x] 3.2 `propose` deriva de la idea que lo inicia; el **dispatch no deriva** y
  deja el campo ausente (design D2) — escenario «Sin instrucción de usuario no
  hay título inventado»
  <!-- 2026-08-09: el flujo SDD quedó **también sin título** y el design lo
  registra como enmienda: a `run_turn` no llega la frase del usuario sino un
  prompt que el método compone; nombrar la sesión con texto generado es el
  mismo error que nombrarla con una referencia expandida. -->
- [x] 3.3 El resume hereda el título de la sesión que continúa, junto a
  `resumed_from`, sin re-derivar (design D5) — escenario «Una sesión reanudada
  conserva el título», que se enlaza con su e2e al llegar el contrato
  <!-- 2026-08-09, gotcha que costará tiempo si se repite: editar estos
  archivos con un script de Python en Windows los reescribe en **CRLF**, y un
  test de la analítica compara contra `\n` literal — la suite entera se pone
  roja por un cambio de una línea. Normalizar a LF antes de correr nada. -->

## 4. Las superficies

- [x] 4.1 GUI: el rótulo de la pestaña pasa a ser el título con el hash en el
  emergente; lista, detalle, árbol y recientes lo muestran junto al id; sin
  título, lo de hoy (design D6, D7) — escenarios «La pestaña dice de qué trata
  la sesión» y «Una sesión sin título se nombra como antes»
- [x] 4.2 GUI: el proyecto se antepone al rótulo **solo** cuando las pestañas
  abiertas cruzan más de un proyecto (design D6) — escenarios «El proyecto se
  antepone ante ambigüedad» y «Con un solo proyecto el rótulo no lo repite»
- [ ] 4.3 TUI: `SessionRow` gana el campo y la lista lo muestra recortado al
  ancho, sin desplazar columnas (design D6) — escenario «La lista de sesiones
  muestra el título»

## 5. Cierre

- [ ] 5.1 `meltemi validate titulo-de-sesion` limpio y `meltemi verify` con los
  nueve escenarios enlazados (meta: cero marcas manuales); suite completa,
  clippy, fmt y los gates del frontend verdes; `docs/paridad-nucleo.md`
  revisado (esta change no añade métodos: se comprueba que no haga falta fila
  nueva)
- [ ] 5.2 Smoke conducido sobre el binario de release con la receta de
  `docs/qa/2026-08-09-piel-de-pestanas-smoke.md` (patch de puerto + user data
  folder propio y nuevo + revertir el patch): seis sesiones con instrucciones
  distintas, pestañas nombradas, una sesión sin título junto a otras con él, y
  dos proyectos abiertos para ver el nombre antepuesto. Nota en `docs/qa/`
