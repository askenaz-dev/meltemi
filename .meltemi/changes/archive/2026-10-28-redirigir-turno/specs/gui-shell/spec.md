# gui-shell — delta

## ADDED Requirements

### Requirement: Interrumpir y enviar desde el compositor

Con la sesión trabajando y texto escrito, el compositor SHALL ofrecer
interrumpir el turno y enviar la instrucción como su relevo, junto al envío que
la encola. Sin texto NO SHALL ofrecerse: no hay nada con lo que relevar. La
acción SHALL distinguirse de detener la sesión, que conserva su confirmación.

#### Scenario: Interrumpir y enviar se ofrece con texto y sesión trabajando

- **WHEN** la sesión trabaja y hay texto en el compositor
- **THEN** SHALL ofrecerse interrumpir y enviar, junto al envío que encola

#### Scenario: Sin texto no hay nada que relevar

- **WHEN** el compositor está vacío
- **THEN** NO SHALL ofrecerse interrumpir y enviar
