# session-history — delta

## ADDED Requirements

### Requirement: El despacho deja registro de primera clase

Toda sesión abierta por un despacho de competidor SHALL asentar registro en
el índice de sesiones al abrir y al concluir, con su nivel de integración
real y su procedencia (id de catálogo y perfil cuando aplique), de modo que
el listado histórico la muestre completa sin depender de la reconstrucción
desde los logs. La reconstrucción desde logs SHALL seguir operando como red
de seguridad y MUST recuperar la procedencia desde el evento de resolución
registrado.

#### Scenario: Sesión de despacho listada completa

- **WHEN** un despacho de competidor concluye
- **THEN** el listado de sesiones SHALL mostrar esa sesión con su nivel
  real y su procedencia
- **AND** sin necesidad de reconstruirla desde los logs

#### Scenario: La red de seguridad recupera la procedencia

- **IF** el índice de sesiones falta o está dañado
- **WHEN** el listado se reconstruye desde los logs
- **THEN** la procedencia SHALL recuperarse del evento de resolución de la
  sesión
