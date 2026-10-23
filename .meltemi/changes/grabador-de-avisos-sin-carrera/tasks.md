# Tasks — grabador-de-avisos-sin-carrera

## 1. El grabador

- [ ] 1.1 Test del fixture en `core/meltemi-client/src/notices.rs`: dos
  lanzamientos simultáneos del grabador dejan dos líneas, veinte rondas
  (design D2); comprobar que falla con el `append.cmd` de hoy — gate: el test
  rojo en Windows con el script de antes
- [ ] 1.2 `write_appender` de Windows: el candado con `mkdir` alrededor del
  `>>` (design D1) — gates: `cargo test -p meltemi-client notices` verde, y el
  test del aviso de prueba repetido cincuenta veces sin un rojo

## 2. Cierre

- [ ] 2.1 `meltemi validate grabador-de-avisos-sin-carrera` limpio; `cargo fmt
  --check`, clippy `-D warnings` en Windows y como Unix (`--target
  x86_64-linux-android`), y la suite de `meltemi-client`; la rama aterriza en
  `main` con `meltemi land grabador-de-avisos-sin-carrera confirm`, se empuja,
  y el CI de `main` queda verde en las tres plataformas
- [ ] 2.2 `meltemi archive grabador-de-avisos-sin-carrera` y la entrada en
  `docs/plan-de-cambios.md`
