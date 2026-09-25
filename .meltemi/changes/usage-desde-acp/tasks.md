# Tasks — usage-desde-acp

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y el simulado

- [x] 1.1 Contrato: `AgentReport` (sesiones, contexto más lleno, costo por
  moneda con sus sesiones) en la celda y `AgentReportTotals` (sesiones y costo
  por moneda) en los totales; `AgentReportCost`; el motivo
  `protocol_carries_context_not_consumption`; la clave de declaración
  `agent-reported-context-and-cost`; schema y casos de conformidad; la celda y
  los totales dejan de derivar `Eq`; para que el workspace compile, la CLI gana
  la línea de la clave nueva y los literales de celda llevan el campo vacío
  (design D1, D4, D5, D6) — gates: `cargo build --workspace`, suite de
  `meltemi-proto`

- [ ] 1.2 El simulado reporta uso: `--usage` (contexto creciente y costo
  acumulado del proceso en fracciones binarias exactas al cerrar cada turno),
  `--usage-no-cost` y `--usage-currency <código>`, apagados por omisión y con
  sus unit tests (design D9) — gates: `cargo build -p mock-agent`, suite de
  `mock-agent`

## 2. El plegado

- [ ] 2.1 `facts_of_log` parte el log en conversaciones en cada `agent_process`
  y de cada una toma el último costo reportado —sin que un reporte sin costo lo
  borre— y el reporte de contexto más lleno; una sesión que otra de la consulta
  reanudó no aporta costo; la celda y los totales agregan por moneda; el motivo
  de los tokens pasa a `protocol_carries_context_not_consumption` para esas
  sesiones; el patrón de `agent_resolved` deja de filtrar por modelo; unit tests
  del plegado con logs e índices fixture (design D2, D3, D4, D5, D8) —
  escenarios «El costo acumulado de una conversación cuenta una vez», «Un
  reporte sin costo no borra el costo anterior», «Una conversación reanudada no
  se cuenta dos veces», «Sesión ACP con uso reportado no se declara sin uso»,
  «Cobertura declarada junto al total», «Medido y no reportado nunca comparten
  cifra» — gates: suite de `meltemid`

- [ ] 2.2 E2e con el simulado a través del daemon: una sesión libre con
  `--usage`, otra con `--usage-no-cost`, otra con `--usage-currency EUR`, un
  `implement` de dos tareas con `--usage` y una sesión sin reporte; la
  contabilidad dice lo reportado, suma por conversación y por moneda, y conserva
  `protocol_carries_no_usage` para la que no reportó (design D2, D4, D5) —
  escenarios «El contexto y el costo reportados se leen del protocolo», «Cada
  conversación de una sesión aporta su costo», «Contexto sin costo deja el costo
  ausente», «Monedas distintas no se mezclan», «Sesión ACP marcada como no
  reportada por el protocolo», «Sesión ACP no fabrica evento de uso» — gates:
  suite de `meltemid`

## 3. Superficies y documentación

- [ ] 3.1 La CLI muestra el contexto máx. y el costo por moneda bajo cada celda
  y el costo en los totales; la vista Uso de la GUI gana las columnas «Contexto
  máx.» y «Costo (del agente)» con la marca de ausente, los tipos y el texto
  ES/EN del motivo nuevo, del motivo reescrito («el agente no reportó uso por el
  protocolo») y de la declaración; la paleta de la GUI y la de la TUI dejan de
  decir «solo lo medido»; pines en `core/meltemid/tests/scenarios_analytics.rs`
  (design D6, D7) — escenario «Las superficies muestran lo reportado como cifra
  del agente» — gates: suite de `meltemi` (tui), `-p meltemid --test
  scenarios_analytics`, suite de `meltemi-desktop`, `svelte-check`, `node
  --test` y `lint:i18n` de `desktop/ui` (con la unión temporal a los
  `node_modules` de `main`, retirada al terminar)

- [ ] 3.2 Documentación: la ayuda de `usage` dice que el contexto y el costo son
  los que el agente reporta, con `docs/referencia-cli.md` regenerada, y
  `docs/paridad-nucleo.md` gana `agentReport`, el motivo y la clave nuevos por
  superficie — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate usage-desde-acp` limpio, `meltemi verify
  usage-desde-acp` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  usage-desde-acp` y `cargo clean` del taller
