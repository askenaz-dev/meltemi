# Design — primer-arranque-del-home

## Context

Verificado en `Home.svelte` el 2026-08-10. `launchable` filtra la flota por
`detected || source === "profile"`, y con ese conjunto vacío la cara del chip
sigue diciendo «agente del proyecto» en tono neutro — prometiendo un default
que el envío va a rehusar. En la **misma fila**, el chip de proyecto ya
advierte con tono `warn` cuando falta la carpeta: la asimetría está a la vista
y es lo que esta change corrige.

Dentro del menú vacío la pista ya es honesta («sin agentes detectados: revisa
la Flota») pero **nombra la vista sin abrirla**, y `Home.svelte` no tenía forma
de navegar: no recibía ningún prop para hacerlo.

## Goals / Non-Goals

**Goals**: que el compositor diga lo que la flota tiene antes de que un envío
falle; que la pista que nombra un lugar pueda ir a él; y un reconocimiento
discreto de lo que sí se detectó.

**Non-Goals**: wizard, checklist ni tour (la coreografía que la propuesta
rechaza por escrito); tocar la detección o el catálogo; los demás estados
vacíos de la GUI.

## Decisions

### D1 — La cara del chip advierte, con el mismo trato que el de proyecto

`noFleet` es `$fleet.length > 0 && launchable.length === 0`: **la flota
respondió y no hay nada lanzable**. La primera mitad importa — sin ella, el
chip advertiría durante el instante en que la flota aún no ha contestado, y una
advertencia que aparece y se va sola es ruido, no señal.

Con `noFleet`, la cara dice «sin agentes detectados» y el ítem del default
**deja de ofrecerse**: no se elige lo que no puede resolverse.

### D2 — La pista que nombra un lugar va a él

El menú vacío gana un ítem que abre la vista de flota, a través de un
`onOpenFleet` que el shell cablea con su propia navegación. Un párrafo que
nombra un sitio sin llevar a él deja el gesto de encontrarlo al lector.

### D3 — El reconocimiento se dice una vez, y se recuerda

En la primera llegada con algo lanzable, la cara del chip muestra el recuento.
El «ya se dijo» se **persiste** en el estado de UI: sin eso, el saludo volvería
con cada ventana y dejaría de ser un saludo para volverse una insignia.

## Risks / Trade-offs

Lo único delicado es que el reconocimiento no se vuelva molestia; por eso es
una vez, en la cara de un chip, y persistido. Sin globos y sin tour.

## Migration / Rollout

Solo `desktop/ui`. Cero contrato, cero daemon, cero dependencias.
