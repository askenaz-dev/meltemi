# Tasks — piso-del-aviso-sin-carrera

Vía rápida, sin deltas. Un commit atómico por tarea, con referencia
`(piso-del-aviso-sin-carrera N.M)` y sin trailers de co-autoría.

## 1. El test

- [x] 1.1 `past_the_floor_the_next_wait_is_told_again`: «dentro» antes de
  esperar al grabador (design D1) y «fuera» medido desde `last_sent` (design
  D2) — gates: el test rojo con un grabador de 300 ms y el de antes, verde con
  el nuevo; `cargo test -p meltemi-client` y `cargo fmt --check`
  <!-- 2026-10-28: con `sleep 0.3` en el grabador de Unix, puesto solo en
  local, el test de antes falla con «inside» en `notices.rs:622`, el mismo
  mensaje que el CI de macOS de `ee0aa7b` (run 40743781500). Con el nuevo,
  los trece tests de `notices::` pasan con ese grabador lento. Grabador
  restaurado; suite de `meltemi-client` 52/52 y `fmt --check` limpio. Bajo
  carga en Linux el de antes no fallaba (0 de 320 corridas en paralelo): el
  grabador lento es la reproducción, porque es lo que macOS hace. -->

## 2. Cierre

- [ ] 2.1 `meltemi validate piso-del-aviso-sin-carrera` limpio; clippy `-D
  warnings`; el CI de la PR de la tanda verde en las tres plataformas; entrada
  en `docs/plan-de-cambios.md`; y la change al histórico fechado a mano, porque
  `meltemi archive` rehúsa una change sin deltas (precedente:
  `grabador-de-avisos-sin-carrera` 2.2)
