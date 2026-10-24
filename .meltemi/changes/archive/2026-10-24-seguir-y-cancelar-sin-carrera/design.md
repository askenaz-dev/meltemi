# Design — seguir-y-cancelar-sin-carrera

## Context

`following_reading_and_cancelling_end_to_end` recorre en fases los verbos de
`verbos-del-agent-boss` contra un daemon en proceso y el agente simulado. Dos
de sus preguntas dependen de quién llega antes, y el test no lo controla:

- tras el cierre, si la sesión ya salió del registro que responde `status`;
- al seguir, si `watch` ya está suscrito cuando el agente pide permiso.

Las dos tienen respuesta segura con solo esperar lo correcto.

## Goals / Non-Goals

**Goals**: que el test no dependa del reparto de la CPU, y que siga probando
lo mismo: que `watch` emite en vivo, que `cancel` cierra la sesión y que el
registro de `watch` es la cola del de `log`.

**Non-Goals**: cambiar el daemon, `watch` o el simulado; acelerar el test.

## Decisions

### D1 — La sesión cancelada se espera fuera del registro

El test sustituye la pregunta única por el `until` acotado de treinta segundos
que ya usa la fase «Dejar de seguir no cancela la sesión»: «the cancelled
session is gone». Lo que prueba no cambia, porque la sesión sale del registro
o el test falla en rojo al vencer el plazo.

*Alternativa descartada.* Sacar la sesión del registro antes de publicar su
cierre, en el daemon. Rompería el orden del que depende «watch termina con la
sesión»: `watch` mira si su sesión sigue viva, y podría verla ida antes de que
le llegue el cierre.

### D2 — El permiso se pide después de que `watch` se suscriba

Un helper común a las dos fases que siguen una sesión real hace, en orden:

1. arranca la sesión desligada con el simulado en `--silent-turn-at 0`, de
   modo que su primer turno termina sin pedir nada;
2. espera a que la sesión quede `waiting_instruction`;
3. lanza `watch` y espera la línea «following session … Ctrl+C stops
   following» en su stderr. `watch` la escribe después de declarar su interés
   y de ver la sesión viva;
4. le da a la sesión una instrucción por `session/direct`, y ese segundo turno
   pide el permiso.

La sesión registra un `turn_empty` antes de que empiece el seguimiento, y no
estorba. Lo que `watch` imprime sigue siendo la cola del registro, que es lo
que el test compara.

*Alternativa descartada.* Retrasar el permiso con `--turn-delay-ms`. Solo
agranda el margen: con carga suficiente, `watch` vuelve a llegar tarde.

### D3 — `Watching` lee el stderr mientras `watch` corre

Hoy el test lee el stderr de `watch` de una vez, al final. Para esperar la
línea de D2, una tarea lo lee línea a línea y las manda por un canal.
`Watching` ofrece esperar una línea y, al final, todo lo escrito, con un plazo
para no colgarse si el proceso no cerrara su stderr.

## Risks / Trade-offs

- **El primer turno callado es un escenario distinto del de antes.** El turno
  que se sigue es ahora el segundo, no el primero. Lo que el escenario exige
  es seguir una sesión viva, y el segundo turno lo es igual.
- **El test tarda lo mismo**: el turno callado no espera nada, y las esperas
  nuevas terminan en cuanto su condición se cumple.
