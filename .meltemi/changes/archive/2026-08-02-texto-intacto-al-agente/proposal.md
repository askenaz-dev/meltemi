# texto-intacto-al-agente

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre `repo-context`, que ya vive
> en la verdad viva, y ninguna capability nueva. La elegibilidad no es un
> tecnicismo cómodo: el requisito vigente de expansión describe qué se inyecta
> por cada referencia y nunca prometió nada sobre el texto que las rodea, así
> que no hay requisito que enmendar — hay superficie normativa que faltaba
> (design D4, con su tripwire).

## Why

El smoke conducido del 2026-07-31 (`lanzador-conversacional` 9.5) midió lo que
ningún test veía: **el prompt llega al agente con los acentos rotos**. «acción
íntegra ñandú» entra con 20 caracteres y el registro de la sesión guarda 24 —
«acciÃ³n Ã­ntegra Ã±andÃº». Se aisló con un cliente JSON-RPC pelado sobre la
tubería con nombre, sin GUI y sin shell en el camino, hasta una línea de
`core/meltemid/src/repo_map.rs`:

```rust
out.push(bytes[i] as char);
```

`expand_refs` recorre el prompt **como bytes** y empuja cada byte al resultado
convertido a `char`. En Rust ese `as` no decodifica nada: mapea el byte a su
code point homónimo, es decir a Latin-1. Un carácter de dos bytes como «ó»
(`0xC3 0xB3`) sale como dos caracteres, «Ã» y «³», que al re-serializarse en
UTF-8 ocupan cuatro bytes. El texto queda doble-codificado antes de salir del
daemon.

Tres hechos fijan la gravedad. Primero, **no es un defecto de pintado**: lo que
se corrompe es el prompt, y el prompt es lo que el agente recibe y lo que el
log JSONL guarda como reconstruible. Segundo, **alcanza a todos los caminos de
prompt**: las dos únicas puertas del daemon hacia un turno —
`free_session.rs:248` y `propose.rs:204` — pasan por `expand_refs` sin
excepción, así que no hay superficie sana. Tercero, **el mantenedor escribe en
español**: en la práctica casi ninguna frase suya llega entera. Es anterior a
la change que lo destapó, y solo se destapó ahora porque el compositor se
volvió la puerta de entrada y la transcripción le devuelve al usuario su propia
frase.

De la misma raíz cuelga un segundo defecto que la corrección obliga a mirar de
frente: el token de una referencia se lee con `is_ref_char`, que solo admite
`is_ascii_alphanumeric()` y `/ . - _`. Un archivo con acentos o eñes —
ordinarios en esta máquina — **no se puede referenciar**: `@informé.md` corta
en la «é» y busca un `informe` que no existe, señalando «no encontrado» sobre
una ruta que el usuario nunca escribió. Arreglar el paso del texto sin arreglar
el alfabeto del token dejaría la mitad del problema en pie, con el agravante de
que la mitad superviviente produce un diagnóstico falso.

## What Changes

- **`expand_refs` deja de caminar el texto carácter a carácter.** El tramo
  literal entre dos referencias se copia de una sola vez como rebanada `&str`
  —`out.push_str(&text[i..at])`— buscando el siguiente `@` con `find`. Los
  índices de byte se conservan, porque la lógica del token los necesita para
  rebanar, y todos los puntos donde el índice avanza caen en frontera de
  carácter por construcción (design D1). El resultado es a la vez correcto y
  más rápido que empujar byte por byte: desaparece la conversión, no se
  sustituye por otra.
- **El token de una referencia admite cualquier alfabeto.** `is_ref_char` pasa
  de operar sobre `u8` a operar sobre `char`, con `is_alphanumeric()` en lugar
  de `is_ascii_alphanumeric()`, y el escaneo del token usa `char_indices`. Es
  una ampliación estricta sobre ASCII (para un `char` ASCII ambas pruebas
  coinciden), así que ninguna referencia que hoy resuelve deja de resolver. La
  elección de `is_alphanumeric` sobre «cualquier byte no ASCII» es deliberada
  y se argumenta en el design D2: la puntuación española y tipográfica («¿»,
  «—», «…») **no** es alfanumérica, así que sigue cerrando el token en vez de
  ser tragada por él.
- **Dos requisitos nuevos en `repo-context`** que dejan escrito lo que hasta
  hoy solo era conducta accidental: el texto no referenciado viaja intacto
  carácter por carácter (con `@@` literal y el `@` suelto como texto, que
  tampoco estaban en la verdad viva pese a existir en el código), y las rutas
  fuera de ASCII se referencian como cualquier otra sin que la puntuación se
  cuele en el token.
- **Tests por escenario en `core/meltemid/src/repo_map.rs`**, incluida la
  regresión con la cadena exacta que se midió («acción íntegra ñandú»)
  afirmando el recuento de caracteres, y los casos de frontera: carácter
  multibyte antes y después de una referencia, `@@` junto a acentos, ruta
  acentuada que resuelve, puntuación no ASCII que cierra el token.

## Capabilities

### New Capabilities

Ninguna. El `@` y su expansión ya son de `repo-context`, que vive en
`.meltemi/specs/`; esta change no inventa superficie, escribe la que faltaba
en la que hay.

### Modified Capabilities

- `repo-context`: + requisito «Integridad del texto que rodea a las
  referencias» (el texto no referenciado llega tal cual se escribió, sea cual
  sea su alfabeto; `@@` literal; `@` suelto como texto) y + requisito
  «Referencias a rutas fuera de ASCII» (el token admite letras y dígitos de
  cualquier alfabeto; la puntuación no ASCII lo cierra).

## Impact

- Superficies: ninguna cambia de forma. La corrección vive entera en el daemon
  y fluye por igual a la CLI, la TUI y la GUI — paridad heredada, porque las
  tres mandan su prompt por el mismo método. El contrato `proto/` no se mueve:
  `RefExpansion` y `RepoMapResult` quedan como están.
- Dependencias: cero. `find`, `char_indices` y `char::is_alphanumeric` son
  biblioteca estándar.
- Compatibilidad: el texto que hoy sale corrupto empezará a salir correcto —
  esa es la corrección, no una regresión. Ningún log ya escrito se reescribe;
  las sesiones históricas conservan lo que se les envió, que es exactamente lo
  que un log append-only debe conservar.
- Riesgo asumido y nombrado: ampliar el token con `is_alphanumeric` hace que
  una palabra acentuada pegada a una referencia sin separador
  (`@lib.rsñandú`) se absorba dentro del token — el mismo comportamiento que
  ASCII ya tenía con `@lib.rsx`, ahora consistente para todos los alfabetos.
  Y un nombre de archivo en forma descompuesta (NFD: «e» más acento
  combinante) sigue cortando en la marca combinante, porque la biblioteca
  estándar no clasifica marcas; queda declarado en el design D2 como límite
  conocido, no escondido.
- Tests: unitarios en el módulo, sin red y sin agentes. La cadena medida entra
  como caso de regresión con nombre propio.

## Fuera de alcance

- **Auditar toda la pila por otros sitios que confundan bytes con
  caracteres**: el barrido de `as char` se hizo y en código propio no hay otro
  sitio (el único hallazgo restante es dato de test dentro de
  `desktop/ui/node_modules/`, no nuestro). Se deja escrito para que nadie lo
  repita de memoria; ampliar la búsqueda a otros patrones de doble
  codificación sería otra change con su propia evidencia.
- **Normalización Unicode de rutas** (NFC/NFD al resolver un archivo): exige
  dependencia y decisión por plataforma; el límite se declara, no se cierra
  aquí.
- **Los otros cuatro hallazgos del smoke** (nodo del proyecto olvidado,
  posición en la cola tapada por el aviso de permiso, cierre de turno como
  línea neutra, ámbito del primer `fleet/list`): cada uno con su change, como
  el rumbo de estructura manda.
- **Gramática de referencias más allá del alfabeto**: espacios en rutas,
  comillas, `~` — cambiar la forma de delimitar un token es una decisión de
  producto propia, no un arreglo de codificación.
