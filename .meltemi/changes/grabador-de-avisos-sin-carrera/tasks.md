# Tasks — grabador-de-avisos-sin-carrera

## 1. El grabador

- [x] 1.1 `write_appender` de Windows en `core/meltemi-client/src/notices.rs`:
  el candado con `mkdir` alrededor del `>>` (design D1) — gates: `cargo test
  -p meltemi-client notices` verde, y el test del aviso de prueba repetido
  cincuenta veces sin un rojo
  <!-- 2026-10-23: suite de `meltemi-client` 51/51, fmt y clippy `-D
  warnings` limpios. En una máquina tranquila el gate pedido no distingue:
  el módulo `notices` pasó 20 de 20 veces también sin el candado. Bajo
  carga, como en el CI, sí: el binario de tests lanzado dieciséis veces en
  paralelo, solo con `the_test_notice_ignores_the_floor_and_does_not_spend_it`,
  falló 12 de 480 corridas con el script de antes y 0 de 480 con el
  candado. -->
- [ ] 1.2 Test del fixture, solo en Windows: con el candado sostenido por el
  test el grabador no escribe, y escribe al soltarlo (design D2) — gates: el
  test rojo con el `append.cmd` de antes y verde con el de 1.1

## 2. Cierre

- [ ] 2.1 `meltemi validate grabador-de-avisos-sin-carrera` limpio; `cargo fmt
  --check`, clippy `-D warnings` en Windows y como Unix (`--target
  x86_64-linux-android`), y la suite de `meltemi-client`; la rama aterriza en
  `main` con `meltemi land grabador-de-avisos-sin-carrera confirm`, se empuja,
  y el CI de `main` queda verde en las tres plataformas
- [ ] 2.2 `meltemi archive grabador-de-avisos-sin-carrera` y la entrada en
  `docs/plan-de-cambios.md`
