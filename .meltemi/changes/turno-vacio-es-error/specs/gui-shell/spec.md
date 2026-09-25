# gui-shell — delta

## ADDED Requirements

### Requirement: El turno vacío se dice con glifo, palabra y remedio
WHEN el log de una sesión registra un turno vacío, la conversación de esa
sesión SHALL mostrarlo en su lugar como un estado propio, con glifo y palabra,
nunca solo con color, y con el remedio en condicional: probar el agente por su
cuenta y revisar su proveedor y sus credenciales. El texto SHALL existir en
español y en inglés.

#### Scenario: El turno vacío se lee en la conversación
- **WHEN** la conversación pliega un log con un `turn_empty`
- **THEN** SHALL mostrar un aviso de turno vacío con glifo, la palabra y el remedio
- **AND** el aviso NO SHALL presentarse como una línea de sistema neutra

#### Scenario: Un turno con salida no muestra el aviso
- **WHEN** la conversación pliega un turno con mensajes o herramientas
- **THEN** NO SHALL mostrar el aviso de turno vacío
