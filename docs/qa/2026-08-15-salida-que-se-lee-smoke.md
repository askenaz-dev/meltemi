# Comprobación conducida — `salida-que-se-lee`

**Fecha**: 2026-08-15 · **Plataforma**: Windows 11 · **Binario**:
`target/release/meltemi.exe`, contra el propio repositorio.

## Antes y después

Antes:

```
36 capabilit(y/ies) in the living truth
  acp-session  10 req  19 scenario(s)
  artifact-format  7 req  14 scenario(s)
```

Después:

```
Living truth
  36 capabilities · 284 requirements · 630 scenarios

  acp-session             10 req  19 scenarios
  artifact-format          7 req  14 scenarios
```

Y el listado de cambios, con la cifra que antes exigía leer todas las filas:

```
Changes
  71 total · 19 active · 52 archived · none awaiting you

  active    PDST  tasks 4/4    review 0/2   verify 7/7      acceso-remoto-en-dos-vias
  active    PDST  tasks 11/11  review 0/4   verify 11/11    avisos-de-escritorio
```

## Qué se midió

| Comprobación | Medido |
| --- | --- |
| La salida a un pipe no lleva color | **0 bytes de escape** en la salida capturada |
| `--yaml` emite un documento | `specs --yaml` produce YAML de bloque, con toda cadena entrecomillada |
| Las columnas se alinean | los nombres arrancan en la misma columna con contadores de 3 y de 6 caracteres |
| Sin color no se pierde información | la salida pintada, quitados los escapes, es **idéntica** a la monocroma |
| El error en YAML es un documento | lleva el código de la taxonomía, sin prosa humana y sin tocar stderr |

Las tres últimas se ejercitan en la suite, no solo aquí: la comparación
pintada-contra-monocroma es un test, de modo que el color no puede empezar a
cargar significado más adelante sin que la suite lo note.

## Lo que no se midió

**Que el terminal pinte de verdad.** Capturar la salida para inspeccionarla la
convierte en un pipe, y en un pipe el color se apaga por diseño — así que la
propia medición destruye lo que quiere medir. Lo que sí está probado es la
decisión (`paints()` con sus cuatro señales) y que el pintado produce escapes y
solo escapes. Que `is_terminal()` diga la verdad en la terminal de Windows
queda a la vista del mantenedor, y es de las cosas que se confirman mirando.

Levantar una pseudo-terminal (ConPTY) para automatizarlo sería su propia
change; se anota en vez de fingir que se hizo.

## Hallazgos de la implementación

1. **Padear después de pintar alinea los escapes, no las letras.** Las
   secuencias ANSI no ocupan ancho, así que `{:width$}` sobre una cadena ya
   pintada cuenta caracteres invisibles. Se padea antes y se pinta después, y el
   comentario lo dice en los dos sitios donde ocurre.
2. **La última columna no se padea.** La primera versión padeaba el nombre, que
   es la última columna: el resultado eran espacios finales invisibles que
   ningún terminal aprovecha. El test exige que cada fila sea igual a su propio
   `trim_end`.
3. **El filtro del test atrapaba el resumen.** Buscar las filas por
   `contains("active")` incluía la línea «19 active» del resumen. Afinado a
   `starts_with("  active")`.
