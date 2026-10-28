# Design — piso-del-aviso-sin-carrera

## Context

`Emitter::waiting` lanza el comando del aviso y fija `last` al volver de
`send`, sin esperar al comando (design D4 de `aviso-de-espera-opt-in`). El
piso se mide desde ese instante. Los tests observan el aviso con un grabador,
un script que añade una línea por aviso a un archivo, y `wait_for` sondea ese
archivo cada 50 ms.

## Goals / Non-Goals

**Goals**: que el test del piso compruebe las dos mitades del piso sin depender
de cuánto tarda el sistema en arrancar el grabador.

**Non-Goals**: cambiar el emisor; arreglar otros tests frágiles.

## Decisions

### D1 — «Dentro» se pregunta antes de esperar al grabador

Las dos llamadas a `waiting` van seguidas. Entre ellas solo pasa lo que tarda
la primera en volver de `send`, y el piso empieza **después** de esa vuelta,
así que la segunda cae dentro del piso por muchos milisegundos que tarde el
grabador. La espera del grabador va después, y solo confirma que el primer
aviso llegó.

### D2 — «Fuera» se mide desde `last_sent`

El test duerme hasta `last_sent() + piso + 50 ms`, calculado, en vez de dormir
350 ms fijos. Así el margen no depende de cuánto tardó la primera espera del
grabador.

*Alternativas.*
- Subir el piso a varios segundos. Agranda el margen pero sigue suponiendo
  que el grabador tarda menos que el piso, y alarga el test sin arreglarlo.
- Medir el tiempo transcurrido y saltarse la aserción si se pasó. Un test que
  se salta su aserción bajo carga no vigila nada, justo donde hace falta.

## Risks / Trade-offs

- Que el hilo del test se quede sin CPU durante todo el piso entre las dos
  llamadas seguidas. Son dos sentencias consecutivas contra un piso de 200 ms;
  no se cubre con más código.

## Verification

Con un grabador que tarda 300 ms, puesto solo en local: el test de antes falla
siempre con «inside» y el nuevo pasa, igual que los otros doce del módulo.
