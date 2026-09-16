# familia-por-titulo

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — un delta solo ADDED sobre una capability existente,
> ninguna capability nueva, ningún MODIFIED ni REMOVED (design D4). Alcance de
> una tarde: una lista de un test, el requisito que la explica, y la razón
> escrita para que no vuelva a derivar en silencio.

## Why

`desktop/ui/tests/forms.test.ts` está rojo en `main`:

```
every form resolves to a schema of its own family
AssertionError: change/workspace resolved to workspace.schema.json
```

Lo introdujo `rama-por-change` (archivada el 2026-08-18) al añadir los métodos
`change/workspace` y `change/land`, y quedó **anotado sin colarse** en
`harness-global-y-por-agente` (tasks 6.3): «Un gate rojo que no es de esta
change […] se deja anotado en vez de colarse aquí». Esta change es ese
pendiente, cobrado por la puerta de siempre.

**El test no falla porque el contrato esté roto. Falla porque afirma una regla
que el contrato nunca tuvo.** El generador de formularios
(`desktop/ui/scripts/gen-method-forms.mjs`) no liga un método a un schema por el
nombre del archivo: lo liga por el `title` del schema, que declara qué métodos
posee (`claimsMethod`, líneas 49–56, sobre títulos como `worktree/*` o
`change/list · change/show`). El nombre del archivo es una etiqueta que el
generador arrastra al resultado, nunca una llave que consulte. La regla «el
archivo empieza por la familia» es, por tanto, una heurística sobre nombres con
una lista de excepciones escrita a mano — y la lista no se amplió cuando
`rama-por-change` añadió su schema.

**Y `workspace.schema.json` no es una anomalía: es la convención.** De los 32
schemas del contrato, cuatro llevan el nombre de sus verbos y no el de su
familia:

| archivo | `title` |
| --- | --- |
| `implement.schema.json` | `sdd/implement` |
| `validate.schema.json` | `sdd/validate` |
| `verify-archive.schema.json` | `sdd/verify · sdd/verify-mark · sdd/archive` |
| `workspace.schema.json` | `change/workspace · change/land` |

Los tres primeros son exactamente las entradas que **sostienen** la lista de
excepciones del test. El cuarto es el que falta. `workspace` es el gemelo
estructural de `verify-archive`: un archivo, varios verbos de una misma familia,
nombrado por los verbos porque es lo que el archivo contiene.

Dos hechos que conviene fijar porque el mensaje del test los esconde:

- **`change/land` resuelve al mismo archivo.** No existe `land.schema.json`:
  `workspace.schema.json` declara los dos métodos en su `title` y aloja
  `workspaceParams` y `landParams`. El test solo reporta `change/workspace`
  porque `assert` corta en el primero; los dos fallan.
- **Cuatro de las siete entradas de la lista actual son inertes.** `change`,
  `spec`, `repo-map` y `commit` ya pasan por la rama `file.startsWith(family)`
  sin necesidad de estar listadas. Las que de verdad cargan el peso son
  `validate`, `implement` y `verify-archive` — las tres del grupo de arriba.

## What Changes

- **`workspace` entra a la lista** de `desktop/ui/tests/forms.test.ts:38`, junto
  a `validate`, `implement` y `verify-archive`, que es el grupo al que pertenece.
- **El comentario que describe la lista se corrige.** Hoy dice «Single-method
  schemas keep their own names (validate, implement…)» y es falso por partida
  doble: `verify-archive` sirve a tres métodos y `workspace` a dos. Lo que
  describe al grupo no es cuántos métodos aloja, sino que **lleva el nombre de
  sus verbos en lugar del de su familia**.
- **Un requisito ADDED en `gui-shell`** que escribe la regla que faltaba: la
  ligadura método→schema la declara el `title`, el nombre del archivo no es
  llave, y el gate MUST NOT rehusar una ligadura correcta por la forma del
  nombre. Sin ese requisito escrito, el test seguiría siendo la única sede de
  una convención que nadie ratificó, que es justamente cómo derivó.
- **No se renombra ningún schema.** La alternativa evaluada —`change-workspace`
  / `change-land`— se descarta con razones en design D1.

## Capabilities

### Modified Capabilities

- `gui-shell`: + un requisito ADDED sobre qué decide la ligadura de un
  formulario a su schema. La capability que ya posee la paleta y sus
  formularios generados es la que debe poseer la regla por la que se ligan
  (design D2).

### New Capabilities

- Ninguna.

## Impact

- Archivos: `desktop/ui/tests/forms.test.ts` (una línea de lista y un
  comentario), `.meltemi/specs/gui-shell/spec.md` vía el delta de la change, y
  `docs/plan-de-cambios.md` para registrar la change.
- **Cero cambios de comportamiento.** No se toca el generador, ni el módulo
  generado, ni un schema, ni el contrato `proto/`, ni el daemon. La GUI renderiza
  exactamente los mismos formularios antes y después; `change/workspace` y
  `change/land` ya funcionaban y siguen igual. Lo único que cambia es qué acepta
  un gate del frontend.
- **Ninguna dependencia nueva**, ningún crate, ningún paquete npm.
- **No nace deber de paridad §4**: el daemon no gana capacidad. Esto es un gate
  del repositorio, no superficie de producto.
- `cargo test --workspace` no queda afectado —ningún schema se mueve, así que el
  test de conformidad de `proto/meltemi-proto` no ve diferencia—, pero se corre
  igual como gate de la change.

## Fuera de alcance

- **Renombrar `workspace.schema.json`.** Descartado en design D1; el costo
  (el `$id` publicado, ~12 sedes en `conformance.rs`, el módulo generado) se
  paga contra una regla que el código no usa, y para ser honesto el archivo
  tendría que partirse en dos.
- **Sustituir la lista por una comprobación derivada del `title`.** Es la idea
  correcta y no cabe aquí: cambiaría el test de vigilar nombres a vigilar
  ligaduras, que es otro alcance y otro gate. Queda anotado como propuesta
  futura en design D3, no colado en una change de una línea.
- **Limpiar las cuatro entradas inertes de la lista** (`change`, `spec`,
  `repo-map`, `commit`). Son ruido demostrable, pero borrarlas es aseo sin
  defecto detrás; se anota en D3 con la misma firma.
- **Tocar el generador o el módulo generado.** No hay nada que arreglar en
  ellos: ligan bien y la frescura ya la gatea `gen-method-forms.mjs --check`.
