# local-analytics — delta

## ADDED Requirements

### Requirement: Uso que el agente reporta por el protocolo
La contabilidad SHALL leer el uso que el agente reporta por ACP (`usage_update`)
desde el log de cada sesión y SHALL presentarlo como cifras del agente, aparte de
los contadores de consumo. La unidad SHALL ser la conversación del agente —cada
proceso del agente dentro del log de una sesión—: de cada una, el último costo
acumulado que el agente reportó y su reporte de contexto más lleno. Por celda,
SHALL presentar cuántas sesiones reportaron, el reporte de contexto más lleno
rotulado como máximo y, por moneda, la suma de los costos de sus conversaciones
con cuántas sesiones la aportaron; los totales SHALL sumar las sesiones y los
costos por moneda de las celdas devueltas. Una conversación que otra sesión de la
consulta reanudó SHALL contar su costo una sola vez, en la sesión que la
reanudó. El contexto reportado MUST NOT presentarse como consumo, dos reportes
de una misma conversación MUST NOT sumarse, monedas distintas MUST NOT sumarse ni
convertirse, y un costo no reportado SHALL quedar ausente y MUST NOT
presentarse como cero. La declaración de honestidad SHALL decir que el contexto y
el costo son los que cada agente reportó y que Meltemi solo suma el costo por
moneda, sin estimar, convertir ni poner precio.

#### Scenario: El contexto y el costo reportados se leen del protocolo
- **WHEN** el agente de una sesión reportó uso por el protocolo
- **THEN** su celda SHALL presentar el contexto más lleno y el costo que el agente reportó
- **AND** los contadores de consumo de esa sesión SHALL seguir ausentes

#### Scenario: El costo acumulado de una conversación cuenta una vez
- **WHEN** el agente reporta su costo acumulado varias veces en una misma conversación
- **THEN** SHALL contar solo el último costo reportado de esa conversación

#### Scenario: Cada conversación de una sesión aporta su costo
- **WHEN** una sesión lanza un agente por tarea, como `implement`
- **THEN** el costo de la sesión SHALL sumar el último costo de cada conversación

#### Scenario: Un reporte sin costo no borra el costo anterior
- **WHEN** el agente reporta un costo y después un reporte sin costo en la misma conversación
- **THEN** SHALL contar el último costo reportado

#### Scenario: Contexto sin costo deja el costo ausente
- **WHEN** el agente reporta contexto sin costo
- **THEN** el contexto SHALL presentarse
- **AND** el costo SHALL quedar ausente, nunca en cero

#### Scenario: Monedas distintas no se mezclan
- **WHEN** sesiones de una misma celda reportan costos en monedas distintas
- **THEN** SHALL presentarse una suma por moneda con sus sesiones
- **AND** MUST NOT convertirse ni sumarse entre sí

#### Scenario: Una conversación reanudada no se cuenta dos veces
- **WHEN** la consulta incluye una sesión y la sesión que la reanudó
- **THEN** el costo de esa conversación SHALL contarse una sola vez, en la sesión que la reanudó

#### Scenario: Las superficies muestran lo reportado como cifra del agente
- **WHEN** la CLI o la GUI presentan una celda con uso reportado
- **THEN** SHALL mostrar el contexto máximo y el costo rotulados como reportados por el agente
- **AND** la declaración de honestidad SHALL decirlo junto a los números

## MODIFIED Requirements

### Requirement: Frontera explícita entre medido y no reportado
La respuesta SHALL distinguir lo medido de lo no reportado con un motivo
explícito y estable por sesión sin dato — el agente no reportó uso por el
protocolo, el protocolo transportó contexto y costo pero no consumo, el nivel no
ejecuta proceso, o la salida estructurada no declaró contadores — y MUST NOT
sumar ambos conjuntos en una misma cifra. Todo total de tokens SHALL declarar su
cobertura, con las sesiones medidas y las sesiones sin dato, y las superficies
SHALL rotular el dato ausente con símbolo y palabra; un dato ausente MUST NOT
renderizarse como cero.

#### Scenario: Sesión ACP marcada como no reportada por el protocolo
- **WHEN** la contabilidad incluye una sesión pilotada por ACP cuyo agente no reportó uso
- **THEN** sus tokens SHALL presentarse como no reportados por el protocolo
- **AND** la celda SHALL conservar íntegras sus métricas de actividad

#### Scenario: Sesión ACP con uso reportado no se declara sin uso
- **WHEN** la contabilidad incluye una sesión pilotada por ACP cuyo agente reportó uso por el protocolo
- **THEN** sus tokens de consumo SHALL presentarse ausentes con el motivo de que el protocolo transportó contexto y costo pero no consumo
- **AND** MUST NOT declararse que el agente no reportó uso

#### Scenario: Cobertura declarada junto al total
- **WHEN** un total de tokens se calcula sobre parte de la actividad del período
- **THEN** SHALL declarar cuántas sesiones lo respaldan y cuántas quedaron sin dato

#### Scenario: Medido y no reportado nunca comparten cifra
- **WHEN** un período combina sesiones medidas y sesiones sin dato
- **THEN** la cifra medida SHALL sumar solo las sesiones medidas
- **AND** las sesiones sin dato SHALL aparecer con su motivo, jamás como cero
