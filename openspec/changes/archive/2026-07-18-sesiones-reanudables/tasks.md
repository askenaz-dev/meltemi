## 1. Persistencia

- [x] 1.1 Índice apend-only de metadatos por proyecto + reconstrucción desde logs _(Req: Metadatos persistentes)_
- [x] 1.2 Marcado `interrupted` al arrancar para sesiones sin fin registrado _(Req: Sesiones interrumpidas)_

## 2. Contrato

- [x] 2.1 `proto/`: `SESSION_LIST` (filtros/orden/límite) y `SESSION_LOG` (paginado por líneas) + tipos _(Req: Listado histórico; Lectura del registro)_
- [x] 2.2 Handlers en `meltemid` (los clientes jamás leen disco del daemon)

## 3. Reanudación

- [x] 3.1 Capturar la capacidad de carga de sesiones del handshake y persistir el id de sesión del agente en metadatos
- [x] 3.2 Acción reanudar: sesión nueva con carga de la anterior; vínculo en metadatos; advertencia de estado cambiado; sin capacidad → no ofrecida _(Req: Reanudación negociada)_

## 4. Superficies

- [x] 4.1 TUI: filtro/etiqueta de históricas en la tabla; drill-in de finalizada con transcript paginado desde `session/log` _(Req: Histórico en superficies)_
- [x] 4.2 CLI `sessions` (humano + `--json`) con gramática y mapeo del delta acumulativo; paleta actualizada _(Modified: cli-contract)_

## 5. Tests y calidad

- [x] 5.1 Unit: reconstrucción de índice; marcado interrupted; paginado de log
- [x] 5.2 E2e contra daemon efímero: caída simulada → interrumpida listada; reanudar con mock-agent que anuncia carga; degradación sin capacidad; CLI sessions --json
- [x] 5.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
