# Design — familia-por-titulo

## Context

El gate `every form resolves to a schema of its own family`
(`desktop/ui/tests/forms.test.ts:30`) está rojo en `main` desde que
`rama-por-change` añadió `change/workspace` y `change/land`. La pregunta de este
design no es cómo ponerlo verde —hay dos formas obvias— sino **cuál de las dos
dice la verdad sobre el contrato**, porque la que se elija queda escrita como
convención para todos los schemas que vengan.

Estado verificado al escribir esto:

- El generador liga por `title`, no por nombre de archivo:
  `claimsMethod(schema, method)` (`gen-method-forms.mjs:49–56`) compara el método
  contra las cláusulas del `title`, partidas por `·` y `&`, con soporte de
  comodín (`worktree/*`). El nombre del archivo entra al resultado como etiqueta
  (`forms[method] = { schema: found.file, … }`, línea 131) y **no se consulta en
  ninguna decisión**.
- De los 32 archivos de `proto/schemas/v1/`, cuatro llevan el nombre de sus
  verbos en vez del de su familia: `implement` (`sdd/implement`), `validate`
  (`sdd/validate`), `verify-archive` (`sdd/verify · sdd/verify-mark ·
  sdd/archive`) y `workspace` (`change/workspace · change/land`).
- La lista de excepciones tiene siete entradas y cuatro son inertes: `change`,
  `spec`, `repo-map` y `commit` ya pasan por `file.startsWith(family)`.
- `workspace.schema.json` aloja `workspaceParams`, `workspaceResult`,
  `landParams`, `landCommit` y `landResult`; su `description` trata taller y
  aterrizaje como un solo ciclo de vida.
- `workspace.schema.json` se nombra en cuatro sedes: su propio `$id`
  (`https://meltemi.dev/proto/v1/workspace.schema.json`), dos líneas del módulo
  generado, y ~12 llamadas de `proto/meltemi-proto/tests/conformance.rs` que
  pasan `"workspace"` como nombre de archivo.

## Goals / Non-Goals

**Goals**: dejar el gate verde diciendo algo cierto; escribir la regla de
ligadura donde se pueda citar; y que la convención de nombres del contrato quede
enunciada en vez de inferida de una lista.

**Non-Goals**: cambiar comportamiento; tocar el generador, el módulo generado o
el contrato; rediseñar el gate; y limpiar el ruido que no causó el defecto.

## Decisions

### D1 — No se renombra: el `title` manda, y el nombre por verbos es la convención

Las dos opciones sobre la mesa eran añadir `workspace` a la lista, o renombrar
`workspace.schema.json` a `change-workspace.schema.json` para que el prefijo de
familia se cumpliera. **Se elige la primera**, por tres razones de hecho y no de
gusto.

**La primera: el prefijo de familia no es un invariante, es decoración.** El
generador resuelve por `title`. Un método solo puede ligarse a un schema que lo
declare, y esa comprobación ya la hace `claimsMethod` antes de buscar el `$defs`.
El nombre del archivo no participa. Renombrar para satisfacer el test sería
cambiar el contrato **para que una heurística de test deje de quejarse**, que es
exactamente al revés de por qué existen los tests.

**La segunda: `workspace` no es la excepción, es el cuarto miembro de un grupo
que ya tiene tres.** `implement`, `validate` y `verify-archive` llevan el nombre
de sus verbos y no el de su familia `sdd`, y son precisamente las tres entradas
que hoy sostienen la lista. `workspace` es el gemelo estructural de
`verify-archive`: un archivo, varios verbos de una familia, nombrado por los
verbos. Si renombrar fuera lo correcto para `workspace`, lo sería para los otros
tres — y nadie ha propuesto renombrar `verify-archive.schema.json` a
`sdd-verify-archive.schema.json`.

**La tercera: el renombrado obliga a partir el archivo, o a mentir.** Un archivo
llamado `change-workspace.schema.json` que aloja `landParams`, `landCommit` y
`landResult` describe mal la mitad de su contenido. Para ser honesto habría que
partirlo en `change-workspace` y `change-land`, rompiendo un schema cuya
`description` trata deliberadamente el taller y su aterrizaje como un ciclo. Y
el costo se paga entero: el `$id` es una URL publicada bajo
`https://meltemi.dev/proto/v1/`, ~12 sedes de `conformance.rs` pasan
`"workspace"` como literal, y el módulo generado se regenera. Todo eso a cambio
de satisfacer una regla que el código no consulta.

Lo que sí queda mal dicho hoy y se corrige: el comentario de la lista afirma
«Single-method schemas keep their own names». Es falso — `verify-archive` sirve
a tres métodos y `workspace` a dos. Lo que define al grupo no es el número de
métodos sino **el criterio de nombre**: llevan el de sus verbos, no el de su
familia. El comentario pasa a decir eso.

### D2 — La capability es `gui-shell`, y el requisito es la regla de ligadura

El formulario generado vive en el requisito «Paleta con difusa, grupos,
recientes y formularios tipados» de `gui-shell` (spec.md:350–376), que ya exige
generarlos desde `proto/schemas/v1` con los obligatorios marcados y verificar la
frescura en CI. Lo que ese requisito **no** dice en ninguna parte es *cómo se
decide de qué schema sale el formulario de un método*. Ese silencio es el hueco
por el que el test se convirtió en la única sede de una convención que nadie
ratificó, y por el que la convención pudo derivar sin que nadie lo notara hasta
que otro trabajo tropezó con el rojo.

Por eso el delta es **ADDED sobre `gui-shell`**, no MODIFIED: no se corrige nada
de lo que aquel requisito afirma —todo sigue siendo cierto—, se escribe una regla
que faltaba a su lado. La capability que posee la paleta y sus formularios es la
que debe poseer la regla por la que se ligan.

El requisito se escribe **acotado a lo que esta change entrega**: que el `title`
es lo que declara la ligadura y que el gate no debe rehusar un nombre por su
forma. No se le hace afirmar que la lista desaparece, porque la lista no
desaparece aquí (D3). Un escenario que ningún test de esta change cubriera sería
deuda disfrazada de rigor.

### D3 — Lo que se ve y no se toca, anotado en vez de colado

Dos cosas quedan a la vista al leer el gate, y ninguna entra:

**La lista podría derivarse del contrato.** En lugar de nombres a mano, el test
podría leer los `title` de `proto/schemas/v1/*.schema.json` y afirmar que el
schema al que cada método se liga lo declara. Eso vigilaría la ligadura real en
vez de la forma de los nombres, y haría imposible la deriva que causó este
defecto. Es la idea correcta y **no cabe en una change de una línea**: cambia
qué vigila el gate, le da acceso al árbol de schemas, y merece su propio análisis
de qué se pierde al dejar de vigilar nombres. Queda como propuesta futura.

**Cuatro entradas de la lista son inertes.** `change`, `spec`, `repo-map` y
`commit` pasan ya por `file.startsWith(family)`; listarlas no cambia ningún
resultado. Borrarlas dejaría la lista diciendo exactamente lo que carga. Pero es
aseo sin defecto detrás, y el rumbo de estructura es explícito: «lo que surja se
anota como propuesta futura, no se cuela». Se anota.

Las dos son la misma propuesta futura y conviene que viajen juntas: si la lista
se deriva del contrato, las cuatro inertes desaparecen solas.

### D4 — Elegibilidad fast-forward

La vía rápida pide deltas solo ADDED sobre capabilities existentes, sin
capability nueva y sin MODIFIED ni REMOVED. Esta change cumple los cuatro: un
requisito ADDED sobre `gui-shell`, cero capabilities nuevas, cero MODIFIED, cero
REMOVED. El alcance material es una entrada de lista, un comentario y el
requisito que los explica, con cero cambios de comportamiento.

La constitución prohíbe la vía nula, no la rápida: un defecto de una línea sigue
necesitando su propuesta, su razón escrita y su escenario. Lo que la vía rápida
ahorra es el gate intermedio, no el rastro.

### D5 — Qué verifica qué

Los dos escenarios del delta se cubren con el mismo test que hoy falla, una vez
corregida la lista:

- «Un schema nombrado por sus verbos pasa el gate» — el test recorre
  `METHOD_FORMS` entero, de modo que los cuatro schemas del grupo (`implement`,
  `validate`, `verify-archive`, `workspace`) pasan por la aserción en cada
  ejecución.
- «Los dos métodos del taller se ligan a su schema» — `change/workspace` y
  `change/land` están ambos en `METHOD_FORMS` apuntando a
  `workspace.schema.json`, así que el recorrido los afirma a los dos. Es el
  escenario que prueba que el arreglo cubre el defecto **completo** y no solo el
  primero que `assert` reportaba.

Como verificación negativa, y por mutación en vez de por observación: quitar
`workspace` de la lista vuelve a poner el test rojo con el mensaje original. Se
comprueba al implementar y se anota en la tarea.
