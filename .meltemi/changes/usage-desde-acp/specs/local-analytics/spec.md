# local-analytics — delta

## ADDED Requirements

### Requirement: Uso que el protocolo transporta
La contabilidad SHALL leer el uso que el agente reporta por ACP (`usage_update`)
desde el log de cada sesión y SHALL presentarlo como cifras del agente, aparte de
los contadores de consumo: por sesión, el último reporte —el contexto en uso, el
tamaño de su ventana y el costo acumulado que declaró—; por celda y en los
totales, cuántas sesiones reportaron, el reporte de contexto más lleno y, por
moneda, la suma de los últimos costos de cada sesión con cuántas sesiones lo
aportaron. El contexto reportado MUST NOT presentarse como consumo, varios
reportes de una misma sesión MUST NOT sumarse, monedas distintas MUST NOT sumarse
ni convertirse, y un costo no reportado SHALL quedar ausente y MUST NOT
presentarse como cero. La declaración de honestidad SHALL decir que el contexto y
el costo son los que el agente reportó, sin cálculo ni conversión de Meltemi.

#### Scenario: El contexto y el costo reportados se leen del protocolo
- **WHEN** el agente de una sesión reportó uso por el protocolo
- **THEN** su celda SHALL presentar el contexto y el costo que el agente reportó
- **AND** los contadores de consumo de esa sesión SHALL seguir ausentes

#### Scenario: El costo acumulado no se suma dos veces
- **WHEN** el agente reporta su costo acumulado varias veces en una misma sesión
- **THEN** SHALL contar solo el último reporte de esa sesión

#### Scenario: Contexto sin costo deja el costo ausente
- **WHEN** el agente reporta contexto sin costo
- **THEN** el contexto SHALL presentarse
- **AND** el costo SHALL quedar ausente, nunca en cero

#### Scenario: Monedas distintas no se mezclan
- **WHEN** sesiones de una misma celda reportan costos en monedas distintas
- **THEN** SHALL presentarse una suma por moneda
- **AND** MUST NOT convertirse ni sumarse entre sí

#### Scenario: Las superficies muestran lo reportado como cifra del agente
- **WHEN** la CLI o la GUI presentan una celda con uso reportado
- **THEN** SHALL mostrar el contexto y el costo rotulados como reportados por el agente
- **AND** la declaración de honestidad SHALL decirlo junto a los números

## MODIFIED Requirements

### Requirement: Frontera explícita entre medido y no reportado
La respuesta SHALL distinguir lo medido de lo no reportado con un motivo
explícito y estable por sesión sin dato — el protocolo no transporta uso, el
protocolo transportó contexto y costo pero no consumo, el nivel no ejecuta
proceso, o la salida estructurada no declaró contadores — y MUST NOT sumar ambos
conjuntos en una misma cifra. Todo total de tokens SHALL declarar su cobertura,
con las sesiones medidas y las sesiones sin dato, y las superficies SHALL
rotular el dato ausente con símbolo y palabra; un dato ausente MUST NOT
renderizarse como cero.

#### Scenario: Sesión ACP marcada como no reportada por el protocolo
- **WHEN** la contabilidad incluye una sesión pilotada por ACP cuyo agente no reportó uso
- **THEN** sus tokens SHALL presentarse como no reportados por el protocolo
- **AND** la celda SHALL conservar íntegras sus métricas de actividad

#### Scenario: Sesión ACP con uso reportado no se declara sin uso
- **WHEN** la contabilidad incluye una sesión pilotada por ACP cuyo agente reportó uso por el protocolo
- **THEN** sus tokens de consumo SHALL presentarse ausentes con el motivo de que el protocolo transportó contexto y costo pero no consumo
- **AND** MUST NOT declararse que el protocolo no transporta uso

#### Scenario: Cobertura declarada junto al total
- **WHEN** un total de tokens se calcula sobre parte de la actividad del período
- **THEN** SHALL declarar cuántas sesiones lo respaldan y cuántas quedaron sin dato

#### Scenario: Medido y no reportado nunca comparten cifra
- **WHEN** un período combina sesiones medidas y sesiones sin dato
- **THEN** la cifra medida SHALL sumar solo las sesiones medidas
- **AND** las sesiones sin dato SHALL aparecer con su motivo, jamás como cero
