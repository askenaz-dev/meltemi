# Tasks — usage-desde-acp

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y el simulado

- [ ] 1.1 Contrato: `UsageReported` (sesiones, contexto usado y tamaño del
  reporte más lleno, costo por moneda y cuántas sesiones lo aportan) en la
  celda y los totales; `UsageCost`; el motivo
  `protocol_carries_context_not_consumption`; la clave de declaración
  `context-and-cost-as-the-agent-reports`; schema y casos de conformidad; la
  celda y los totales dejan de derivar `Eq` (design D1, D2, D3, D4) — gates:
  `cargo build --workspace`, suite de `meltemi-proto`

- [ ] 1.2 El simulado reporta uso: `--usage` (contexto creciente y costo
  acumulado en `USD` al cerrar cada turno) y `--usage-no-cost`, apagados por
  omisión y con sus unit tests (design D7) — gates: `cargo build -p mock-agent`,
  suite de `mock-agent`

## 2. El plegado

- [ ] 2.1 `facts_of_log` lee `usage_update` —el último reporte de cada sesión,
  sin borrar un costo anterior con un reporte sin costo—, la celda y los totales
  agregan el reporte más lleno y la suma por moneda, el motivo de los tokens
  pasa a `protocol_carries_context_not_consumption` para esas sesiones, y el
  patrón de `agent_resolved` deja de filtrar por modelo; unit tests del plegado
  y e2e con el simulado (design D2, D3, D6) — escenarios «El contexto y el
  costo reportados se leen del protocolo», «El costo acumulado no se suma dos
  veces», «Contexto sin costo deja el costo ausente», «Monedas distintas no se
  mezclan», «Sesión ACP marcada como no reportada por el protocolo», «Sesión ACP
  con uso reportado no se declara sin uso», «Cobertura declarada junto al
  total», «Medido y no reportado nunca comparten cifra» — gates: suite de
  `meltemid`

## 3. Superficies y documentación

- [ ] 3.1 La CLI muestra el contexto y el costo reportados bajo cada celda y la
  línea de la declaración nueva; la vista Uso de la GUI gana las columnas
  «Contexto» y «Costo (del agente)» con la marca de ausente y el texto ES/EN de
  la declaración (design D4, D5) — escenario «Las superficies muestran lo
  reportado como cifra del agente» — gates: suite de `meltemi` (tui), suite de
  `meltemi-desktop`, `svelte-check` y `node --test` de `desktop/ui` (con la unión
  temporal a los `node_modules` de `main`, retirada al terminar)

- [ ] 3.2 Documentación: la ayuda de `usage` dice que el contexto y el costo son
  los que el agente reporta, con `docs/referencia-cli.md` regenerada, y
  `docs/paridad-nucleo.md` gana el bloque `reported`, el motivo y la clave
  nuevos por superficie — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate usage-desde-acp` limpio, `meltemi verify
  usage-desde-acp` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  usage-desde-acp` y `cargo clean` del taller
