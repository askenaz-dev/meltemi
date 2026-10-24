# seguir-y-cancelar-sin-carrera

> Vía rápida: los artefactos de una vez y sin deltas, porque no cambia ningún
> requisito ni el comportamiento del producto, solo el orden en que un test
> espera. Gate del mantenedor dado el 2026-10-24 («continua», tras
> `grabador-de-avisos-sin-carrera`, que dejó este test nombrado como el
> siguiente frágil de Windows). Nace del CI de `release-con-el-push`.

## Why

`following_reading_and_cancelling_end_to_end`, en `tui/tests/session_verbs.rs`,
falla a ratos. Tiene dos causas, y las dos son del test: pregunta algo antes de
que la respuesta pueda ser cierta.

1. **El cierre.** Tras `cancel`, `watch` imprime `session_ended` y termina, y el
   test pregunta en el acto si la sesión sigue en `status`. El daemon cierra en
   este orden: escribe `session_ended`, anota el fin en el índice y saca la
   sesión del registro vivo (`session_finalize.rs`). Entre lo primero y lo
   último hay una ventana, y `follow.rs` la nombra como esperada. Dentro de esa
   ventana la sesión todavía figura, y así falló el CI de `8037fbc` (run
   39991046157, intento 1): «the cancelled session is gone». El orden del
   daemon es el correcto. Si la sesión saliera del registro antes de publicar
   su cierre, `watch`, que también termina cuando su sesión deja de estar viva,
   podría terminar sin imprimir el cierre como última línea, y eso lo exige un
   escenario de `cli-contract`.
2. **El permiso.** El test arranca la sesión con su primera instrucción y
   después lanza `watch`. `watch` emite lo que ocurre desde que se suscribe, no
   lo anterior (design D6 de `verbos-del-agent-boss`). El agente simulado pide
   permiso en su primer turno, y bajo carga lo pide antes de que `watch` se
   suscriba: el test espera treinta segundos un registro que ya pasó. Con el
   binario del test lanzado dieciséis veces en paralelo en esta máquina,
   fallaron así 3 de 160 corridas. La fase «Dejar de seguir no cancela la
   sesión» tiene la misma carrera.

## What Changes

- El test espera, con el `until` acotado que ya usa otra fase, a que la sesión
  cancelada salga del registro, en vez de preguntarlo una sola vez.
- Las dos fases que siguen una sesión real la arrancan con un primer turno
  callado (`--silent-turn-at 0` del simulado) y esperan a que quede entre
  turnos. Luego lanzan `watch` y esperan su línea «following session», que
  `watch` escribe cuando ya está suscrito. Solo entonces le dan la instrucción
  que pide permiso, así que el permiso llega siempre después de la
  suscripción.
- `Watching` lee el stderr de `watch` línea a línea mientras corre, para poder
  esperar esa línea, y al final lo entrega entero como antes.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- Ninguna: ni el daemon ni `watch` cambian, y no se toca ningún requisito ni
  escenario.

## Impact

- `tui/tests/session_verbs.rs`, nada más.
- Cero dependencias, cero RPC, cero cambios fuera de los tests.

## Fuera de alcance

- Cambiar el orden del cierre en el daemon: por lo que dice el punto 1 de Why.
- Que `watch` repita los registros anteriores a su suscripción: para eso está
  `log`.
