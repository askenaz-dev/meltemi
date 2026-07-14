## 1. Andamiaje y reutilización del núcleo

- [ ] 1.1 Crear el crate `tui/` (binario `meltemi`) con cabecera SPDX y añadirlo como miembro del workspace en el `Cargo.toml` raíz
- [ ] 1.2 Exponer el transporte local, el arranque bajo demanda (`connect_or_start`) y el peer RPC de `meltemid` como librería reutilizable, sin duplicar la lógica de socket (design D6)

## 2. Gramática y despacho

- [ ] 2.1 Implementar el parser de argumentos propio, sin dependencia de terceros: subcomandos operativos, subcomandos reservados, flags globales `--json`/`--help`(`-h`)/`--version`(`-V`) y `--` como fin de flags _(Req: Gramática de subcomandos y reserva)_
- [ ] 2.2 Implementar la regla de despacho por subcomando y por TTY con `std::io::IsTerminal` sobre stdout _(Req: Regla de despacho CLI↔TUI)_
- [ ] 2.3 Implementar el arranque interactivo diferido (aviso por stderr, salida con éxito) y los subcomandos locales `version` y `help` sin tocar el daemon _(Req: Regla de despacho CLI↔TUI; Mapeo comando↔método RPC)_

## 3. Disciplina de salida y códigos

- [ ] 3.1 Implementar la taxonomía de códigos de salida (0/1/2/10/11/12/13) como tipo central y mapear cada desenlace a su código _(Req: Taxonomía de códigos de salida)_
- [ ] 3.2 Implementar la disciplina stdout/stderr y el envoltorio `--json` (exactamente un objeto en stdout en éxito y en error; stderr libre de JSON) _(Req: Disciplina de flujos stdout/stderr; Salida legible por máquina con --json)_

## 4. Subcomandos respaldados por RPC

- [ ] 4.1 Cablear `status` → `initialize` + `status`, con presentación humana y variante `--json` _(Req: Mapeo comando↔método RPC; Arranque del daemon bajo demanda)_
- [ ] 4.2 Cablear `propose <idea>` → `initialize` + `propose`, sin la disciplina de gates de #14 _(Req: Mapeo comando↔método RPC)_
- [ ] 4.3 Cablear `stop` → `initialize` + `shutdown` _(Req: Mapeo comando↔método RPC)_
- [ ] 4.4 Traducir los errores de transporte y de contrato a los códigos `10` (inalcanzable) y `11` (contrato) _(Req: Taxonomía de códigos de salida; Arranque del daemon bajo demanda)_

## 5. Tests y calidad

- [ ] 5.1 Tests de gramática y despacho: subcomando desconocido → `2`, subcomando reservado no es `2`, invocación desnuda sin TTY → `2`, con subcomando siempre scriptable _(escenarios de Gramática y Regla de despacho)_
- [ ] 5.2 Tests de disciplina de salida y `--json`: la salida útil solo en stdout, el progreso solo en stderr, un objeto JSON en éxito y en error con su código _(escenarios de Disciplina de flujos y --json)_
- [ ] 5.3 Test e2e del mapeo RPC contra un daemon efímero en un repo fixture temporal (nunca la raíz del repo): `status` y `stop`; `propose` mínimo con `mock-agent` _(escenarios de Mapeo comando↔método RPC y Arranque bajo demanda)_
- [ ] 5.4 `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo test` verdes en todo el workspace
