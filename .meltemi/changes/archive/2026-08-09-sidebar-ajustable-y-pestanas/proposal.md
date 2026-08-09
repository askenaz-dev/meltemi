# sidebar-ajustable-y-pestanas

> Vía completa (proposal → design → specs → tasks). Los deltas son **solo
> ADDED** sobre una capability existente, lo que por la letra del criterio de
> vía rápida la haría elegible; se declina por la otra mitad del criterio —
> «alcance de un día». Esto son tres superficies, dos módulos puros nuevos,
> dos componentes nuevos, un par de persistencia que cruza TypeScript y Rust,
> una guardia de accesibilidad enmendada y un smoke conducido sobre el binario
> de release. La **forma** del delta califica; el **alcance** no (design D9).

## Why

El mantenedor mandó una captura de la barra lateral y tres frases: la línea
divisoria entre las opciones y los proyectos «debe poder moverse», el scroll de
la sección de proyectos «se ve fea», y la parte de sesiones «deben ser tabs, de
esa forma puedo tener varios tabs abiertos». Son tres cosas de naturaleza
distinta —geometría, cromo y modelo de navegación— y **una sola change**, por
una razón medible que se explica abajo.

**La línea no existe.** No hay elemento divisor: hay un `border-top: 1px solid
var(--hair)` sobre `.section`, la fila del título PROYECTOS
(`Sidebar.svelte:493`). Se lee como raya de encabezado porque eso es lo que es.
Y no hay nada que arrastrar en toda la superficie de escritorio: un barrido de
`desktop/ui/src` no encuentra un solo `pointerdown`, `pointermove` ni
`setPointerCapture`. El reparto entre las siete entradas de navegación y el
árbol de proyectos está decidido por el navegador, no por el usuario: `nav`
toma su alto natural y `.tree` se queda con lo que sobre. Con seis proyectos
abiertos —los que el mantenedor tiene— el árbol vive en un canal de tres filas
mientras la navegación ocupa la mitad de la columna.

**El scroll es feo porque no está estilado en absoluto.** El repositorio no
declara `scrollbar-width`, `scrollbar-color` ni `::-webkit-scrollbar` en ningún
sitio; lo único que toca el pintado nativo es `color-scheme: light dark`
(`app.css:61`), que hace que la barra siga el tema pero no cambia ni su ancho ni
sus botones de paso. Lo que se ve en la captura es la barra clásica de Windows:
unos 17 px con dos flechas, dentro de una columna cuyo contenido mide unos 200.
Se come un doceavo del ancho y añade dos cuadrados de cromo a un árbol de filas
de 28 px.

**Y las sesiones se pisan unas a otras.** Hoy `App.svelte:51` guarda
`detailSession: string | null`: **una**. Abrir una sesión estando en otra
reemplaza el transcript en la misma instancia del componente, así que el
borrador sin enviar que se escribió en la sesión A sobrevive dentro de la
sesión B, y un `session/log` que falla deja el transcript de A bajo la cabecera
de B. No es que falten pestañas: es que la única sesión visible es una variable
que se sobrescribe.

**Por qué son una change y no tres.** El divisor **fabrica** la segunda barra
de scroll. Darle a `nav` un alto puesto por el autor obliga a `min-height: 0;
overflow-y: auto` sobre él —que hoy no tiene— y en ese momento la columna de
216 px estrena una segunda barra clásica con sus flechas. Entregar la petición
1 sin la 2 duplica exactamente el defecto del que trata la 2. Son una decisión
de diseño, no tres tickets.

## What Changes

- **La línea se vuelve un control.** Un elemento propio con `role="separator"`,
  foco, nombre accesible y `aria-valuenow`/`min`/`max`, que arrastra con el
  puntero y se mueve con ArrowUp/ArrowDown/Home/End. El hairline se muda del
  encabezado al control: la raya que ahora divide debe **leerse** como divisor.
  La aritmética vive en un módulo puro (`nav-split.ts`) con suelos por los dos
  lados, probado con `node --test`, no dentro del componente.
- **El reparto se recuerda** junto al tema, la geometría de ventana y el
  pliegue, en `desktop-ui.json`. Perfil nuevo arranca en el reparto de hoy.
  Plegada la barra, el separador se retira con el árbol y el reparto guardado
  queda inerte hasta desplegar.
- **La barra de scroll se estrecha en toda la superficie**, no solo en el
  árbol: dos declaraciones estándar heredadas en `:root`
  (`scrollbar-width: thin`, `scrollbar-color: var(--text-faint) transparent`).
  El mantenedor señaló la barra lateral porque es donde 17 px duelen, pero la
  misma barra está en el transcript y en Ajustes. Se usan **propiedades
  estándar**, no `::-webkit-scrollbar`.
- **Las sesiones se abren en pestañas**, hasta ocho, con la lista como primera
  pestaña —no cerrable— y cada sesión abierta como su propio panel montado.
  Abrir una que ya está abierta enfoca en vez de duplicar. Una pestaña de fondo
  conserva su transcript, su búsqueda y su borrador, y declara cuántos eventos
  llegaron sin leer. El estado de cada una se dice con símbolo y palabra, nunca
  con color solo.
- **La tira de pestañas es un componente genérico** (`TabStrip.svelte`) con el
  patrón ARIA completo: `tablist`/`tab`/`tabpanel`, `tabindex` rotatorio,
  flechas con ciclo, Home, End y Delete. Es la primera implementación correcta
  de pestañas de esta superficie; el Editor tiene la suya propia y podrá
  adoptarla después.
- **La guardia de accesibilidad se enmienda, y hacia arriba.** Hoy el barrido
  de `tabindex` solo ve el literal `tabindex="-1"`; el `tabindex` rotatorio que
  el patrón ARIA exige es una expresión, así que hoy **pasaría sin ser vista**.
  El detector se ensancha para verla y se abre una excepción estrecha —solo
  elementos con `role="tab"`— con un test positivo que cobra el precio: la tira
  debe recorrerse entera con el teclado.

## Capabilities

### Modified Capabilities

- `gui-shell`: + tres requisitos ADDED — el reparto ajustable entre navegación
  y proyectos con su persistencia y sus suelos, las barras de desplazamiento de
  la superficie, y varias sesiones abiertas a la vez en pestañas. Ninguno
  reemplaza texto vivo: el modelo de drill-in y el breadcrumb siguen intactos
  (una pestaña de sesión **es** un drill-in; lo que cambia es que pueden
  coexistir), y el contrato de dígitos 1–5 no se toca.

### New Capabilities

- Ninguna.

## Impact

- **Cero dependencias nuevas, cero métodos del contrato, cero movimiento de
  `proto/`.** El daemon no gana capacidad: esto es cromo de una superficie, y
  por tanto **no nace deber de paridad §4** — el mismo argumento escrito que
  usaron `panel-opaco-y-nav-plegable` y sus precedentes.
- Un solo archivo Rust de producto cambia: `desktop/src/uistate.rs`, que gana
  `nav_split: Option<u32>` tras `#[serde(default)]`. Todo `desktop-ui.json`
  existente carga sin tocarse.
- La TUI **no cambia**: la superficie de terminal es de una sesión por
  construcción (`tui/src/shell/state.rs`, `drill: Option<Drill>`), y multiplicar
  eso es su propia change con su propio modelo de foco.
- **Lo que solo el smoke puede confirmar y se declara ahora**: el consumo con
  ocho transcripts montados contra el presupuesto de 80 MB en reposo, el
  comportamiento de `hidden` sobre hijos flex en WebView2, dónde aterriza el
  foco al cerrar una pestaña, y cómo se comporta `setPointerCapture` en
  WebView2 —del que este repositorio no tiene precedente alguno.

## Fuera de alcance

- **El ancho de la barra lateral**: los 216 px siguen fijos. El mantenedor
  señaló la línea horizontal entre dos zonas, no el borde vertical.
- **Persistir el conjunto de pestañas abiertas.** Se olvidan al reiniciar, como
  las del Editor. Tres razones concretas en el design (D7); condición de
  reapertura escrita allí.
- **Adoptar `TabStrip` en el Editor**: el componente nace genérico para que la
  conversión sea mecánica después, pero hacerla aquí arrastra una segunda vista.
- **Cualquier atajo global nuevo** (Ctrl+W, Ctrl+Tab, teclas de pliegue o
  reparto): el conjunto de teclas de la GUI es materia de su propia revisión, y
  WebView2 reclama esos dos como aceleradores del navegador.
- **Reordenar pestañas arrastrando, desprenderlas a otra ventana, fijarlas.**
- **Un tope de líneas del transcript**: la GUI no tiene ninguno y la TUI sí.
  Ocho transcripts montados lo vuelven pertinente; el smoke lo mide y, si hace
  falta, es su propia change.
- **Hacer arrastrables los otros repartos fijos** (las columnas del Editor, las
  calles de la revisión).
- **La auditoría de intuitividad** que el mantenedor pidió aparte, incluida la
  etiqueta textual de nivel verificado en Flota.
