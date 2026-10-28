# gui-shell — delta

## ADDED Requirements

### Requirement: El compositor dice que se trabaja

Mientras un agente trabaja, el compositor de la superficie SHALL mostrarlo con
un indicador ambiental en su propio marco, de modo que la señal esté donde el
usuario va a escribir lo siguiente. El indicador SHALL animar únicamente
transform u opacidad y NO SHALL animar disposición alguna. SHALL apagarse
cuando la sesión espera una decisión del usuario: esperar no es trabajar, y un
indicador en marcha mientras el agente está detenido mentiría sobre quién debe
actuar. SHALL existir un solo indicador ambiental por vista.

#### Scenario: El compositor se enciende mientras el agente trabaja

- **WHEN** una sesión está iniciándose o activa
- **THEN** el compositor SHALL mostrar su indicador de trabajo
- **AND** el indicador NO SHALL animar la disposición de nada

#### Scenario: La luz se apaga cuando la sesión espera una decisión

- **WHILE** una sesión espera una decisión de permiso
- **THEN** el compositor NO SHALL mostrar el indicador de trabajo

#### Scenario: Sin movimiento, el estado se sigue diciendo

- **WHEN** el usuario ha pedido movimiento reducido
- **THEN** el indicador SHALL retirarse por completo, sin quedar detenido a la
  vista
- **AND** el estado SHALL seguir dicho por el marco y por el texto de la fila
  del compositor

### Requirement: Detener está al alcance de donde se escribe

Mientras una sesión está viva, el compositor SHALL ofrecer detenerla junto al
envío. La acción SHALL pedir la misma confirmación explícita que el acceso del
encabezado y SHALL ejecutar el mismo verbo; el acceso del encabezado SHALL
conservarse. Detener SHALL ofrecerse también mientras la sesión espera una
decisión.

#### Scenario: Detener desde el compositor

- **WHEN** una sesión está viva
- **THEN** el compositor SHALL ofrecer detenerla junto al envío
- **AND** activarlo SHALL pedir confirmación explícita antes de detenerla

#### Scenario: Un verbo, dos accesos

- **WHEN** se detiene desde el compositor y desde el encabezado
- **THEN** ambos SHALL ejecutar el mismo verbo con la misma confirmación
