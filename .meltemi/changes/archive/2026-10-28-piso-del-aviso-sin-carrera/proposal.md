# piso-del-aviso-sin-carrera

> Vía rápida: los artefactos de una vez y sin deltas, porque no cambia ningún
> requisito ni el comportamiento del producto, solo un test. 0º de la tanda de
> `release-para-un-desarrollador` (2026-10-28): una tanda que empieza con
> `main` en rojo no se puede medir. Mismo patrón que
> `grabador-de-avisos-sin-carrera` y `seguir-y-cancelar-sin-carrera`.

## Why

`notices::tests::past_the_floor_the_next_wait_is_told_again` falló en el CI de
macOS del merge de `enfasis-con-color` (`ee0aa7b`, run 40743781500) con
«inside» en `core/meltemi-client/src/notices.rs:622`. Linux y Windows pasaron,
y la PR que llevó ese merge también: el commit no tocaba los avisos.

La causa está en el test, no en el producto. El piso de 200 ms empieza cuando
`Emitter::waiting` envía el aviso, no cuando el grabador lo escribe. El test
esperaba a que el grabador escribiera y **después** preguntaba si la segunda
espera caía dentro del piso. En un runner lento, arrancar `sh` y escribir la
línea tarda más que el piso, la segunda espera ya está fuera y sale un aviso.

Reproducido el 2026-10-28 de forma determinista: con un grabador que tarda
300 ms en escribir (puesto solo en local), el test falla siempre con el mismo
«inside» del CI. Bajo carga en Linux no aparece (0 de 320 corridas en paralelo),
que es por lo que solo macOS lo enseñó.

## What Changes

- El test pregunta «dentro del piso» enseguida después del primer aviso, antes
  de esperar al grabador.
- «Pasado el piso» se mide desde `Emitter::last_sent()` más el piso, en vez de
  dormir 350 ms y suponer que bastan.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- Ninguna: el emisor no cambia, y no se toca ningún requisito ni escenario.

## Impact

- `core/meltemi-client/src/notices.rs`, solo su módulo de tests.
- Cero dependencias, cero RPC, cero cambios fuera de los tests.

## Fuera de alcance

- Cambiar el emisor, el piso o el momento en que empieza: es lo diseñado
  (`aviso-de-espera-opt-in`, design D4).
- La fragilidad bajo carga de `e2e_agente_por_sesion.rs` que anotó
  `aviso-de-espera-opt-in`.
