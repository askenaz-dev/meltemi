# Design — texto-intacto-al-agente

## Context

`expand_refs` (en `core/meltemid/src/repo_map.rs`) es el único paso que todo
prompt atraviesa antes de salir hacia el agente: `free_session.rs:248` lo llama
con la instrucción del usuario y `propose.rs:204` con la idea. Su forma actual
recorre `text.as_bytes()` con un índice y, para todo byte que no sea `@`, hace
`out.push(bytes[i] as char)`. Los índices de byte no son un capricho: el token
de una referencia se rebana con `&text[start..end]`, y el escaneo necesita
posiciones absolutas. Por eso el bucle nació sobre bytes — y por eso el
`as char` pasó desapercibido, porque en un prompt ASCII es la identidad.

El smoke del 2026-07-31 lo midió a través de la tubería con nombre, sin GUI:
20 caracteres entran, 24 se registran. La corrección tiene que arreglar la
codificación **sin** romper la lógica del token, que es donde está la tentación
de reescribir con `chars()` y quedarse sin los índices que el rebanado exige.
Este design fija cómo, y de paso resuelve el defecto hermano que la misma
función esconde: un token que solo admite ASCII no puede nombrar un archivo
acentuado.

## Goals / Non-Goals

**Goals**: que el texto no referenciado llegue al agente exactamente como se
escribió, carácter por carácter y sea cual sea su alfabeto; que un archivo con
acentos o eñes se referencie como cualquier otro; que ambas promesas queden
escritas en la verdad viva en vez de depender de que nadie vuelva a tocar el
bucle; y que las conductas hoy solo implícitas (`@@` literal, `@` suelto como
texto) queden ancladas por requisito, ya que la corrección pasa por encima de
ellas.

**Non-Goals**: cambiar la gramática de las referencias más allá del alfabeto
admitido, normalizar Unicode al resolver rutas, tocar el contrato `proto/`, o
reescribir logs ya escritos.

## Decisions

### D1 — Copiar tramos literales, no caracteres: los índices de byte se quedan

Tres formas de arreglarlo estaban sobre la mesa.

(a) **Recorrer con `chars()`**. Elimina el `as char` de un plumazo, pero pierde
el índice absoluto que `&text[start..end]` necesita; recuperarlo obliga a
llevar un contador de `len_utf8()` en paralelo — la misma aritmética de bytes
de hoy, ahora duplicada y desincronizable. Se descarta: pelea con la lógica del
token en vez de dejarla en paz.

(b) **`char_indices()` para todo el bucle**. Conserva el índice, pero el cuerpo
del bucle tiene que saltar hacia adelante tras consumir una referencia, y
`char_indices` no ofrece salto: exige `while let` con un iterador `peekable` o
recrear el iterador tras cada referencia. Funciona, y es más ruidoso que el
problema que resuelve.

(c) **Copiar el tramo literal entero de una vez**, que es la elegida. El bucle
conserva su índice de byte `i` y su forma; lo que cambia es que en vez de
preguntar «¿este byte es `@`?» pregunta «¿dónde está el próximo `@`?» con
`text[i..].find('@')`, empuja `&text[i..at]` con `push_str` —una sola rebanada
`&str`, imposible de partir un carácter porque el compilador no deja rebanar
fuera de frontera— y salta a `at`. Si no queda ningún `@`, empuja el resto y
termina.

La propiedad que sostiene la corrección es que **`i` está siempre en frontera
de carácter**, y se puede demostrar caso por caso: arranca en 0; tras un tramo
literal vale la posición de un `@` (ASCII, frontera); tras `@@` avanza 2 desde
un `@` (dos bytes ASCII); tras un `@` suelto avanza 1; y tras una referencia
vale el `end` del token, que D2 deja en frontera por construcción. Como `i`
está en frontera, `text[i..]` nunca puede entrar en pánico — la rebanada que
antes era una conversión silenciosa ahora es una invariante que el propio tipo
`&str` vigila.

Efecto colateral bienvenido: copiar por tramos es más rápido que empujar byte
a byte. Se anota porque conviene saber que la corrección no compra corrección
al precio de rendimiento; no se mide ni se promete, porque nadie lo pidió.

### D2 — El token admite letras y dígitos de cualquier alfabeto, no «todo lo no ASCII»

`is_ref_char` decide dónde termina una referencia dentro de la prosa. Hoy es
`b: u8 -> b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_')`,
así que `@informé.md` corta en la «é» y diagnostica «no encontrado» sobre
`informe` — una ruta que el usuario nunca escribió. Con archivos acentuados
ordinarios en esta máquina, eso es un defecto, no una restricción.

Dos ampliaciones posibles, y la diferencia importa. **Aceptar cualquier byte no
ASCII** (`b >= 0x80`) es de una línea y sería seguro para las fronteras — los
bytes de una secuencia multibyte son todos ≥ 0x80, así que un carácter se
consume entero o no se consume — pero se traga la puntuación: `@lib.rs¿ves?`
absorbería `¿ves`, y en español la puntuación no ASCII abunda («¿», «¡», «—»,
«…», ««»»). El arreglo crearía un defecto nuevo del mismo tamaño.

Se elige **operar sobre `char` con `is_alphanumeric()`**: `is_ref_char(ch:
char) -> ch.is_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_')`, y el
escaneo del token recorre `text[start..].char_indices()` acumulando
`ch.len_utf8()`, de modo que `end` cae siempre en frontera de carácter (la
premisa que D1 usa). La clasificación Unicode hace exactamente la distinción
que hace falta: las letras acentuadas y las eñes son alfanuméricas, la
puntuación tipográfica no lo es. Sobre ASCII es una ampliación estricta —
`is_alphanumeric` y `is_ascii_alphanumeric` coinciden para todo `char` ASCII —
así que ninguna referencia que hoy resuelve deja de resolver.

Dos límites se declaran en vez de esconderse. Uno: una palabra acentuada pegada
a una referencia sin separador (`@lib.rsñandú`) se absorbe dentro del token,
igual que hoy `@lib.rsx` absorbe la `x`; es la conducta consistente, y
delimitar por otra vía (comillas, espacios escapados) sería cambiar la gramática
de las referencias, que está fuera de alcance. Dos: un nombre en forma
descompuesta (NFD, «e» seguida de acento combinante U+0301) corta en la marca,
porque la biblioteca estándar no clasifica marcas y añadir tablas Unicode sería
una dependencia nueva que este arreglo no justifica (§10). En Windows y macOS
los nombres llegan compuestos en la práctica; el caso queda escrito para que
quien lo encuentre sepa que se conocía.

### D3 — Los escenarios afirman caracteres, no bytes

El defecto es de codificación, así que un test que compare `String` contra
`String` puede pasar por la razón equivocada si ambos lados se corrompen igual.
Los tests de esta change afirman sobre **recuento de caracteres**
(`.chars().count()`) y sobre presencia de la subcadena original, no sobre
longitud en bytes: la regresión con la cadena medida —«acción íntegra ñandú»,
20 caracteres— falla hoy con 24 y pasa mañana con 20, que es exactamente la
medición del smoke reproducida en el módulo. Las fronteras se cubren una por
una en vez de en un caso grande: multibyte antes de una referencia, multibyte
después, `@@` entre acentos, ruta acentuada que resuelve, puntuación no ASCII
que cierra el token. Un caso grande que fallara no diría cuál de las cinco
propiedades se rompió.

### D4 — Elegibilidad fast-forward: por criterio, con tripwire declarado

El criterio del motor (`fast_forward_eligible`): ninguna capability nueva y
deltas sin MODIFIED ni REMOVED. Esta change cumple ambos y conviene decir por
qué no es un atajo. `repo-context` ya vive en `.meltemi/specs/` y es la dueña
del `@` y su expansión, así que no hay capability nueva. Y los dos requisitos
son ADDED de verdad: «Expansión determinista de referencias», el requisito
vigente, promete qué se inyecta por cada referencia (contenido cercado,
listado, límites, marca de truncado, señal de inexistente) y **no dice una
palabra sobre el texto que las rodea** ni sobre el alfabeto del token. Nada de
lo que promete deja de ser cierto tras esta change; lo que faltaba era decir
que el resto del prompt también es un compromiso. Reescribirlo para meterle la
integridad dentro habría sido un MODIFIED cosmético que además obligaría a
reproducir sus tres escenarios enteros — más riesgo de perder uno que valor
ganado.

Tripwire explícito, en la disciplina del precedente: si al implementar
apareciera que la corrección obliga a enmendar «Expansión determinista de
referencias» (por ejemplo, si el conteo de `bytes` de `RefExpansion` tuviera
que cambiar de unidad), esa enmienda sale a su propia change o esta pasa a
spec-full. Se declara ahora, no se descubre después.

### D5 — El barrido de `as char` se hace una vez y se escribe

`as char` sobre un `u8` es la firma exacta del defecto, y una corrección
puntual no vale nada si el patrón vive en otros diez sitios. El barrido
(`grep -rn 'as char' core/ tui/ proto/ desktop/`) devuelve **un solo hallazgo
en código propio**: la línea que esta change arregla. El único otro resultado
del árbol es dato de fixture de una gramática dentro de
`desktop/ui/node_modules/`, que no es código nuestro ni se compila con el
workspace. Queda escrito aquí para que el próximo lector no tenga que repetir
la búsqueda ni fiarse de la memoria de nadie; ampliarla a otros patrones de
doble codificación (conversiones `from_utf8_lossy` sobre datos parciales,
`OsStr` a `String`) sería otra change con su propia evidencia, y no se insinúa
que estén sanos por no haberlos mirado.
