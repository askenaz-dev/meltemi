# cromo-que-no-estorba

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre una capability existente
> (`gui-shell`), ninguna capability nueva, ningún MODIFIED ni REMOVED, y alcance
> de un día: tres defectos de cromo, sus tests y una comprobación sobre el
> binario.

## Why

Tres frases del mantenedor sobre la misma captura: «la barra de la derecha
también tiene el scroll feo», «los mensajes de arriba se ven feos y se quedan
pegados», «en la paleta de comandos no se cierra al hacer click fuera del área».
Las tres son cromo que estorba, y ninguna es una función que falte.

**El cajón se desplaza de lado.** `Drawer.svelte` declara `overflow: auto` en su
cuerpo —los dos ejes— con un ancho fijo de 268 px. Basta un hijo cuyo contenido
mínimo no quepa para que aparezca una barra horizontal debajo de todo el panel.
Un cajón de 268 px no debería desplazarse de lado nunca: su contenido tiene que
partirse, que es lo que hace el resto de la superficie.

**Los avisos no caducan, ninguno.** `pushNotice` los apila y solo los quita un
clic. Eso es correcto —y obligatorio— para lo que la spec ya exige: el
vencimiento de un permiso «MUST superficializarse con un aviso persistente que
no se descarta en silencio». Pero esa regla habla de vencimientos y errores de
sesión, no de un «enlace creado» que confirma lo que el usuario acaba de hacer y
ya ve en pantalla. Hoy los dos se tratan igual, así que dos confirmaciones se
quedan clavadas encima de la vista tapando la tabla. La frontera existe en la
spec; lo que falta es aplicarla.

**La paleta no se cierra al hacer clic fuera.** Tiene su velo (`.scrim`) y
atiende a Escape, pero el velo no lleva `onclick`. El conmutador de proyectos
—el otro panel con velo de esta superficie— sí lo lleva. Es una incoherencia de
un atributo, y de las que enseñan al usuario que el gesto no funciona *aquí*.

## What Changes

- **El cajón parte su contenido en vez de desplazarlo de lado**:
  desplazamiento solo vertical, y los hijos del cuerpo pueden encogerse por
  debajo de su contenido mínimo para que las rutas largas se partan.
- **Los avisos se separan por consecuencia**: los informativos —confirmaciones
  de algo que el usuario acaba de hacer— se retiran solos tras unos segundos, y
  se pueden retirar antes; los de aviso y de error **siguen siendo persistentes
  y solo se van con un gesto**, exactamente donde la spec lo exige. Pasar el
  puntero o el foco sobre uno detiene su cuenta: nada desaparece bajo la mano
  que iba a leerlo.
- **Todo velo cierra lo que cubre.** El velo de la paleta gana lo que el del
  conmutador ya tenía, y un test barre la superficie: cualquier `.scrim` de
  cualquier componente debe cerrar al hacer clic. La clase de fallo muere aquí.

## Capabilities

### Modified Capabilities

- `gui-shell`: + la frontera entre aviso transitorio y aviso persistente, con la
  pausa al apuntar; + que ninguna superficie flotante se desplace de lado; + que
  todo velo cierre al hacer clic fuera.

### New Capabilities

- Ninguna.

## Impact

- `desktop/ui/src/lib/stores.ts` (la caducidad por tono),
  `desktop/ui/src/lib/components/Notices.svelte` (la pausa y el tratamiento),
  `desktop/ui/src/lib/components/Drawer.svelte` (un eje),
  `desktop/ui/src/lib/components/Palette.svelte` (el velo),
  `desktop/tests/scenarios_shell.rs` (los tests y el barrido de velos).
- Cero dependencias, cero métodos del contrato, ningún archivo Rust de producto:
  el daemon no gana capacidad y no nace deber de paridad §4.

## Fuera de alcance

- **Hacer transitorio ningún aviso de vencimiento o de error**: es exactamente
  lo que la spec prohíbe, y esta change lo refuerza en vez de tocarlo.
- **Rediseñar el banner de daemon inalcanzable**, que tiene sus propias reglas.
- **Las pestañas al estilo Chrome y la flota por suscripción**: cada una es su
  propia change, pedidas en el mismo mensaje y de tamaño muy distinto.
