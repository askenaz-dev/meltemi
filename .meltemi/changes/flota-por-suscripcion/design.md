# Design — flota-por-suscripcion

## Context

Verificado el 2026-08-09:

- `fleet/list` ya devuelve las suscripciones como filas de fuente `profile` con
  `underlyingAgent` (`fleet-catalog`: «cada perfil declarado SHALL aparecer con
  su fuente, su agente subyacente y su detección»). El daemon cumple.
- `Fleet.svelte:133-171` pinta `{#each $fleet as agent}` plano: nombre, fuente,
  nivel, detección y configurado. **`underlyingAgent` no aparece en la tabla**;
  solo en el cajón (`:206-208`).
- `tui/src/run.rs:1568` imprime `(profile → <agente>)` en cada perfil. La
  terminal lo dice; la gráfica no.
- `Fleet.svelte:150-152` pinta el nivel como `{levelLabel(agent)}` más `" ✓"`
  cuando hay nivel verificado. `integration-levels:100-103` exige «la vista
  Flota SHALL mostrar la distinción con etiqueta textual». Un `✓` no es una
  etiqueta textual, y ese escenario no tiene hoy ningún test que lo lleve.

## Goals / Non-Goals

**Goals**: que la tabla diga de qué agente es cada suscripción y cuántas tiene
cada agente; que ninguna suscripción desaparezca por tener un agente
desconocido; que el nivel se lea con palabras.

**Non-Goals**: tocar el daemon, el contrato o la TUI; mover acciones a la fila;
agrupar por proveedor.

## Decisions

### D1 — El agrupamiento es una función pura sobre lo que ya llega

`fleet-groups.ts` toma la lista de `fleet/list` y devuelve filas ordenadas con
una profundidad: agente, luego sus suscripciones, luego el siguiente agente.
Nada se pide al daemon y nada se guarda: es una vista de lo que ya está.

Que sea puro tiene una razón concreta además de la costumbre: el orden y el caso
huérfano son las dos cosas que se rompen en silencio, y ambas se prueban con
`node --test` en vez de mirarlas.

**Orden**: los agentes conservan el orden en que llegan —el catálogo ya lo
decide, y reordenar aquí sería inventar una segunda opinión—; las suscripciones
de cada agente van por nombre, que es lo único estable que el usuario controla.

### D2 — La huérfana se lista, marcada, al final

Una suscripción cuyo `underlyingAgent` no está en el catálogo —o que no lo
declara— no se cuelga de nadie. Se lista en un grupo final propio, con el id que
declara a la vista. Descartarla convertiría una configuración a mano en una
desaparición sin diagnóstico, que es justo lo que esta superficie evita en el
árbol de proyectos con las raíces ausentes.

### D3 — La relación se dice con palabras; la sangría solo acompaña

Cada fila de suscripción lleva, en texto, «suscripción de <agente>». La sangría
y el filo son decoración: quien lee con lector de pantalla o quien copia la
tabla recibe la misma información. Es la misma regla que el estado con símbolo y
palabra.

Alternativa descartada: `<table>` con `rowgroup` por agente. Un grupo de filas
con encabezado propio en HTML obliga a `tbody` por agente, y entonces el
encabezado del grupo no es una fila de datos comparable con las demás —la
detección y el nivel del agente dejarían de leerse en las mismas columnas—. Se
prefiere una sola secuencia de filas con la relación escrita.

### D4 — El nivel se dice con palabras, y el test se enlaza al escenario vivo

`N2 ✓` pasa a `N2 · verificado` y `N1` a `N1 · declarado`. El requisito que lo
exige ya existe en `integration-levels`; esta change no lo duplica en su delta,
lo **cumple**, y marca su test con el nombre del escenario vivo para que la
verdad viva vuelva a tener quien la sostenga.

Se conserva el `✓` **junto** a la palabra: quien ya lo leía como «verificado» no
pierde su atajo visual, y quien no, ahora tiene la palabra. Lo que la spec
prohíbe es el glifo *solo*.

## Risks / Trade-offs

- **La tabla crece**: con muchas suscripciones, más filas. Aceptado — son las
  filas que el usuario creó, y el recuento en el agente permite leer el total
  sin contarlas.
- **La sangría en una tabla es frágil** si alguien reordena columnas. Por eso la
  relación va en texto y la sangría es lo prescindible.
- **`levelLabel` cambia de forma**, y algún test podría pinear la vieja. Se
  revisa la suite y se ajusta lo que corresponda, sin aflojar ninguna guardia.

## Migration Plan

Aditivo y reversible: un módulo puro, una tabla que ordena distinto y dos
palabras. Nada persiste, nada del contrato se mueve, ningún archivo Rust de
producto cambia.

## Open Questions

- ¿Debería el agente poder plegar sus suscripciones? No hasta que alguien tenga
  tantas que estorben; plegar por defecto escondería justo lo que esta change
  destapa.
