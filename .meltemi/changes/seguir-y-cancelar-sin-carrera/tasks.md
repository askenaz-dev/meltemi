# Tasks — seguir-y-cancelar-sin-carrera

## 1. El test

- [x] 1.1 La sesión cancelada se espera fuera del registro con `until`
  (design D1) — gate: `cargo test -p meltemi --test session_verbs` verde
  <!-- 2026-10-24: verde en 2,8 s; fmt limpio. -->
- [ ] 1.2 El helper que sigue una sesión real: primer turno callado, sesión
  entre turnos, `watch` suscrito y entonces la instrucción; usado por las dos
  fases que siguen una sesión real; `Watching` lee su stderr en vivo (design
  D2, D3) — gates: el test verde, y bajo carga, con el binario del test lanzado
  dieciséis veces en paralelo, ninguna corrida roja donde antes las había

## 2. Cierre

- [ ] 2.1 `meltemi validate seguir-y-cancelar-sin-carrera` limpio; `cargo fmt
  --check`, clippy `-D warnings` en Windows y como Unix (`--target
  x86_64-linux-android`), y la suite completa del workspace; la rama llega a
  `main` con `git merge --no-ff` (no tiene taller para `meltemi land`), se
  empuja, y el CI de `main` queda verde en las tres plataformas
- [ ] 2.2 Al histórico fechado a mano, porque `meltemi archive` rehúsa una
  change sin deltas, y la entrada en `docs/plan-de-cambios.md`
