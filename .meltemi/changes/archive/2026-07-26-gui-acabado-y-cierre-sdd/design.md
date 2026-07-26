# Design — gui-acabado-y-cierre-sdd

## Context

Cuatro defectos encontrados conduciendo la app real (no leyendo su código):
tres de presentación en el cliente de escritorio y uno de ciclo de vida de
sesión en el daemon. Este design registra las decisiones para que las
correcciones no se «arreglen» en otra dirección más adelante.

## Decisions

### D1 — Columna flex, no rejilla reparada
La rejilla podía repararse fijando `grid-row: 4` en la vista, pero seguiría
acoplada al número de barras: la próxima barra condicional rompería el layout
en silencio otra vez. La columna flex expresa el invariante real — «cada barra
a su altura natural, la vista se queda con el resto» — y es indiferente a
cuántos hijos condicionales se rendericen. El test de cableado prohíbe
explícitamente `grid-template-rows` en el shell para dejar la decisión
apuntalada.

### D2 — `.git` se filtra en el walker, no en los consumidores
La ruta de búsqueda (`fsops::tree_search`) ya excluía `.git` por su cuenta;
filtrarlo también en la GUI habría duplicado la regla en cada consumidor.
Excluirlo en `build_map` con `filter_entry` corta el subárbol entero en la
fuente: ningún consumidor lo ve, no consume presupuesto de truncado y la TUI
hereda la corrección sin cambio propio (paridad §4). `.hidden(false)` se
mantiene: `.meltemi/` es contexto y debe listarse.

### D3 — `run_turn` pasa por el finalizador compartido
`session_finalize` existe precisamente para que todo turno único comparta la
misma cola: eventos terminales, registro de fin con metadatos de reanudación,
baja del registro vivo. `propose` y `session/direct` ya lo usan; `run_turn`
era el único camino que daba de baja sin cerrar. Se replica la forma exacta de
`propose.rs` — registro de inicio en el índice al arrancar (una caída real
debe seguir listando como interrumpida: ese es el significado del estado) y
`finalize_ok`/`finalize_err` al terminar. No se introduce un finalizador
nuevo ni se toca el contrato.

### D4 — Sin dependencias nuevas, sin RPC nuevos
Las cuatro correcciones son locales a archivos existentes. El contrato
`proto/` no se mueve; ningún esquema cambia; cargo-deny no ve entradas nuevas.

### D5 — La verificación mira la ventana
Los tests de la superficie del escritorio verifican cableado leyendo el código
(convención de `surface.rs`), y ninguno de estos cuatro defectos era visible
así. La verificación de esta change incluye un smoke conducido por CDP sobre
el binario real (medidas de layout, contenido del árbol, estado de sesión tras
un ciclo completo) publicado en `docs/qa/`. Automatizarlo como gate de CI
queda apuntado fuera de alcance.
