# Tasks — adaptadores-con-rawinput

Vía rápida. Un commit atómico por tarea, con referencia
`(adaptadores-con-rawinput N.M)` y sin trailers de co-autoría.

## 1. Los adaptadores

- [x] 1.1 Claude: `request_for` lleva `rawInput` y `locations` (design D1) —
  escenario «La petición lleva la entrada y la ruta»; gates: tests del adaptador
  <!-- 2026-10-28: `the_request_carries_the_input_and_the_path_the_rules_read`
  pinea `rawInput` sin editar, la ruta de `Write` en `locations`, el comando de
  `Bash` en `rawInput.command` y la forma serializada que lee `RequestFacts`.
  `path_in` de `mapping.rs` pasa a `pub(crate)`: la petición y la llamada sacan
  la ruta de la misma función. Suite de `meltemi-adapters` verde (155 + los de
  integración). -->
- [ ] 1.2 Codex: el `Mapper` recuerda los hechos de cada ítem y `request_for` los
  lleva cuando la aprobación concede lo que el ítem es (design D2, D3) —
  escenarios «La aprobación de un comando lleva el comando del ítem», «Lo que
  concede otra cosa no hereda el comando» y «Un cambio de varios archivos no
  presenta una sola ruta»; gates: tests del adaptador

## 2. El motor de reglas

- [ ] 2.1 Un prefijo `allow` no casa con lo encadenado; `deny` sí (design D4) —
  escenarios «Lo encadenado tras el prefijo escala al humano» y «Deny sigue
  alcanzando lo encadenado»; gates: tests de `permissions.rs`, del aprobador
  del CLI y `docs/referencia-cli.md`

## 3. De punta a punta y cierre

- [ ] 3.1 e2e con el adaptador de Claude real y el proveedor simulado: una
  regla `allow` de `execute` con prefijo de comando decide sin escalar —
  escenario «La petición lleva la entrada y la ruta»; gates: el e2e
- [ ] 3.2 `docs/agentes.md` dice que las reglas por comando y por ruta gobiernan
  a los dos; `meltemi validate adaptadores-con-rawinput` limpio y `meltemi
  verify` con los seis escenarios enlazados; fmt, clippy `-D warnings` y las
  suites de `meltemi-adapters`, `meltemid` (permisos y e2e de adaptadores) y
  `tui` (aprobador)
