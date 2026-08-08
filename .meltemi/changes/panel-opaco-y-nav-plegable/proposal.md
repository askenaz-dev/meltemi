# panel-opaco-y-nav-plegable

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre una capability existente
> (`gui-shell`), ninguna capability nueva, ningún MODIFIED ni REMOVED. Alcance
> de un día: un defecto de una línea, un control que falta, y sus tests.

## Why

El mantenedor mandó una captura de la aplicación con la superficie rota y dos
frases: «errores gráficos» y «no me deja colapsar el side nav». Son dos cosas
distintas y conviene no confundirlas.

**Lo primero es un defecto, y su causa es de una línea.** El panel del
conmutador de proyectos pinta su fondo con `var(--surface-1)`, un token que
**no está definido en ninguna parte**: los tokens del design system son
`--surface` y `--surface-2`. CSS no falla ante un `var()` sin definir ni
avisa — simplemente no pinta fondo, así que el panel queda transparente y el
árbol de navegación, la Flota y las etiquetas de estado se leen unos sobre
otros, que es exactamente lo que muestra la captura. Un barrido de toda la
superficie de escritorio confirma que es el **único** token fantasma del
proyecto: no hay una familia de errores, hay uno, y está justo donde el
mantenedor lo vio.

Lo que el defecto expone es más grande que el defecto: nada impedía que un
token inexistente llegara a la aplicación empaquetada. Los tests de cableado
leen fuentes y comprueban que un nodo existe, no que tenga fondo; el smoke
visual es por release, no por commit. Un lint que compare tokens usados contra
tokens definidos cuesta una función y cierra la clase entera de fallo.

**Lo segundo no es un fallo: es una ausencia.** La barra lateral es un
`<aside>` de 216 px fijos sin control alguno para plegarla; el único «plegar»
que existe cierra grupos de proyecto **dentro** del árbol. En una ventana
estrecha —o simplemente cuando el usuario quiere el lienzo entero para un
diff— la navegación se queda ocupando su ancho sin manera de retirarla.

## What Changes

- **El fondo del panel vuelve a existir**: el conmutador de proyectos pinta
  `--surface`, el token que el design system sí define, y queda opaco sobre
  todo lo que cubre.
- **Lint de tokens como gate**: un test recorre la superficie de escritorio,
  reúne cada `var(--x)` usado y cada `--x` definido, y falla nombrando
  cualquiera que se use sin existir. La clase de error muere aquí, no en la
  próxima captura del mantenedor.
- **La barra lateral se pliega**: un control en su cabecera la lleva a un
  riel angosto de iconos y la devuelve, sin perder ninguna entrada — cada una
  conserva su etiqueta accesible y su atajo de dígito. El estado se recuerda
  entre arranques, junto al tema y la geometría de ventana que ya se guardan.

## Capabilities

### Modified Capabilities

- `gui-shell`: + el fondo opaco como propiedad verificable de toda superficie
  flotante (con el lint de tokens que lo sostiene), + la barra lateral
  plegable con su estado recordado y sin pérdida de alcance.

## Impact

- `desktop/ui/src/lib/components/ProjectSwitcher.svelte` (una línea),
  `Sidebar.svelte` y `App.svelte` (el pliegue y su persistencia),
  `messages.ts` (ES/EN), `desktop/tests/scenarios_shell.rs` (el lint y los
  escenarios).
- Cero dependencias nuevas, cero movimiento del contrato `proto/`, ningún
  método nuevo: esta change no toca el daemon ni nace deber de paridad §4 —
  el cromo exclusivo de una superficie no lo tiene.

## Fuera de alcance

- **La auditoría de intuitividad completa** (leyendas, glosario, el check de
  nivel verificado que el mantenedor preguntó): es su propia change, con su
  propio alcance; aquí solo entra lo que la captura mostró roto.
- **Rediseñar la navegación** o cambiar el contrato de vistas numeradas: el
  pliegue oculta la barra, no reorganiza lo que contiene.
- **Un smoke visual por commit**: sigue siendo por release; lo que esta change
  añade es el lint que no necesita ojos.
