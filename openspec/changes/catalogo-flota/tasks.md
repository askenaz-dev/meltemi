## 1. Registro y contrato

- [ ] 1.1 Definir el formato TOML de la instantánea del registro (versión, entradas: id, nombre, binarios por SO, rutas candidatas, args ACP, nivel declarado) y poblarla desde el research interno; embeberla con `include_str!` y cabecera SPDX _(Req: Catálogo desde instantánea empaquetada; design D1/D6)_
- [ ] 1.2 Override del registro: `MELTEMI_FLEET_REGISTRY` y clave de config equivalente, con validación y versión reportada del sustituto _(Req: Catálogo desde instantánea — escenario de registro sustituido)_
- [ ] 1.3 Tipos del contrato en `proto/`: `methods::FLEET_LIST`, `FleetListParams { projectRoot? }`, `FleetAgent`, `FleetListResult { registryVersion, agents }`; código de error 2001 `agent_not_detected` _(Req: Consulta fleet/list; Selección por id)_

## 2. Daemon: detección y selección

- [ ] 2.1 Detección pasiva multiplataforma: resolución en PATH + rutas candidatas, con PATHEXT acotado en Windows (`.exe`/`.cmd`/`.bat`), sin lanzar procesos _(Req: Detección local pasiva)_
- [ ] 2.2 Agentes `custom` en config (usuario y proyecto) integrados al catálogo y a la detección _(Req: Agentes personalizados del usuario)_
- [ ] 2.3 Handler `fleet/list`: catálogo + detección fresca por consulta + marcado del configurado vía `projectRoot` _(Req: Consulta fleet/list)_
- [ ] 2.4 Selección por `[agent] id` con precedencia env > command > id; resolución a binario detectado + args ACP; error 2001 con remedy sin lanzar procesos _(Req: Selección de agente por id de catálogo)_

## 3. CLI

- [ ] 3.1 Gramática: `fleet` como subcomando operativo; mapeo `fleet`→`initialize`+`fleet/list`; salida humana y `--json` (un objeto); códigos de la taxonomía _(Modified: cli-contract — Gramática; Mapeo comando↔RPC. Req: Subcomando fleet)_

## 4. TUI

- [ ] 4.1 `Command::FleetList`/`Update::Fleet` en el actor de conexión; solicitud al entrar a la vista 4 _(design D5)_
- [ ] 4.2 Vista Flota poblada: glifo+palabra de detección, etiqueta de nivel, marcador de configurado; cero detectados conserva pista BYO-agent; accesibilidad baseline (ASCII/NO_COLOR) _(Req: Vista Flota poblada)_
- [ ] 4.3 Registrar `fleet` en la paleta (obligación viva de `tui-shell`: registro de método nuevo)

## 5. Tests y calidad

- [ ] 5.1 Unit: parseo de la instantánea y del override; detección con PATH fixture (presente/ausente/PATHEXT Windows); precedencia env>command>id _(escenarios de Catálogo, Detección y Selección)_
- [ ] 5.2 E2e contra daemon efímero con registro fixture apuntando a `mock-agent`: `fleet/list` marca configurado y detectado; sesión por id lanza el mock; id no detectado → 2001 sin subprocesos _(escenarios de Consulta y Selección; repos fixture temporales, jamás este repo)_
- [ ] 5.3 E2e CLI: `meltemi fleet` humano y `--json` (un objeto, stderr limpio); render de la vista Flota con `TestBackend` (detectado/no detectado sin color, ASCII) _(Req: Subcomando fleet; Vista Flota poblada)_
- [ ] 5.4 `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo test` verdes en el workspace
