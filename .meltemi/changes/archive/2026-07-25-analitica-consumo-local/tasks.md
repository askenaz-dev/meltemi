## 1. Evento de uso y captura del nivel 3

- [x] 1.1 `proto/`: tipo de evento de uso del log de sesión (contadores opcionales, origen, modelo) + rama en `session-event.schema.json` + casos de conformidad _(Req session-history: Evento local de uso en ejecuciones headless; design D3)_
- [x] 1.2 Captura en la costura de mapeo del nivel 3 (`levels::map_headless_line`): reconocer las claves de uso de la salida oficial y persistirlas con su origen; contador no declarado queda ausente; jamás credenciales ni identidad de cuenta _(Req session-history: Evento local de uso en ejecuciones headless; design D3)_
- [x] 1.3 `core/mock-headless`: emitir una línea de uso del subconjunto oficial para ejercitar la captura sin agente real ni red _(Req: Tokens únicamente medidos, jamás estimados; design D3)_

## 2. Agregador local

- [x] 2.1 Módulo agregador en `meltemid`: prefiltro por período con el índice de sesiones, plegado de los logs JSONL y celdas proyecto × agente × perfil × período con cubeta de no atribuido _(Req: Contabilidad local agregada sobre los registros de sesión; design D1, D2)_
- [x] 2.2 Métricas de actividad del conjunto cerrado: sesiones, cerradas y sin fin, segundos activos solo de sesiones cerradas, prompts, turnos por motivo de fin, permisos pedidos/aprobados/denegados/vencidos, ediciones humanas (log de sesión + log de ediciones del proyecto), commits, checkpoints y errores _(Req: Contabilidad local agregada sobre los registros de sesión; design D2)_
- [x] 2.3 Tokens medidos y cobertura: suma solo de eventos de uso, contadores ausentes ausentes, motivo estable por sesión sin dato _(Req: Tokens únicamente medidos, jamás estimados; design D3, D4)_
- [x] 2.4 Frontera medido/no reportado en la respuesta: totales solo sobre lo medido, cobertura declarada, ninguna cifra mixta _(Req: Frontera explícita entre medido y no reportado; design D4)_
- [x] 2.5 Declaración de honestidad estructurada de claves estables (qué se mide, de qué registro, qué no es visible, que nada sale de la máquina, que nada es estimado) _(Req: Declaración de honestidad junto a los números; design D6)_

## 3. Método y paridad de núcleo

- [x] 3.1 `proto/`: constante del método `analytics/usage` + tipos camelCase de params y result + `analytics.schema.json` + casos de conformidad _(Req: Método de agregación de consumo por contrato; design D5)_
- [x] 3.2 Handler del daemon: filtros de proyecto/agente/perfil, rango, granularidad y límite; sin proyecto agrega todos los del directorio de datos; vacío honesto; parámetro inválido rehúsa con remedio _(Req: Método de agregación de consumo por contrato; design D5)_
- [x] 3.3 CLI: subcomando `usage` (tabla legible y `--json` de un objeto) + regeneración de `docs/referencia-cli.md` _(Req: Paridad de superficies de la contabilidad; design D5)_
- [x] 3.4 TUI: entrada del método nuevo en la paleta _(Req: Paridad de superficies de la contabilidad; design D5)_
- [x] 3.5 GUI: entrada en el registro tipado + fila en `docs/paridad-nucleo.md` con el gate de `tui/tests/parity.rs` verde _(Req: Paridad de superficies de la contabilidad; design D5)_

## 4. Panel de contabilidad en la GUI

- [x] 4.1 Vista de contabilidad en la navegación del sidebar, con selector de período y filtros de proyecto, agente y perfil _(Req: Panel de contabilidad con la densidad del design system; design D7)_
- [x] 4.2 Tablas densas del design system (32/8/16, radios 4/8, hairline, numerales tabulares), sin dependencias de gráficos y sin animar layout ni desplazar bandeja o banners _(Req: Panel de contabilidad con la densidad del design system; design D7)_
- [x] 4.3 Dato no reportado con símbolo + palabra y su motivo, jamás como cero _(Req: Frontera explícita entre medido y no reportado; design D4)_
- [x] 4.4 Declaración de honestidad renderizada junto a las cifras desde el catálogo ES/EN, en el panel y en la salida CLI _(Req: Declaración de honestidad junto a los números; design D6)_
- [x] 4.5 Accesibilidad del panel: operación completa por teclado con foco visible sobre filtros, período y celdas _(Req: Panel de contabilidad con la densidad del design system; design D7)_

## 5. Tests y calidad

- [x] 5.1 Unit: plegado de celdas y cubeta de no atribuido, duración solo de sesiones cerradas, agregación sin índice, ausencia que no se vuelve cero, motivos de no reportado, claves estables de la declaración _(Req: Contabilidad local agregada sobre los registros de sesión; design D1, D2, D4)_
- [x] 5.2 E2e sobre fixtures temporales: logs sintéticos multiproyecto y multiperfil más una corrida de `mock-headless` con línea de uso — la sesión ACP reporta sin datos de tokens y la headless agrega los medidos; sin red ni agentes reales _(Req: Tokens únicamente medidos, jamás estimados; design D3, D4)_
- [x] 5.3 Contrato y paridad: casos de conformidad del método y del evento verdes, gate de paridad verde y referencia CLI fresca _(Req: Paridad de superficies de la contabilidad; design D5)_
- [x] 5.4 `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las tres plataformas _(Req: cobertura de todos los requisitos; constitución §7)_
