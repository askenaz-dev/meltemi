# gui-shell — delta

## ADDED Requirements

### Requirement: Vincular suscripciones desde la Flota

La ficha del agente en la vista de Flota SHALL ofrecer vincular una
suscripción cuando la entrada declara su variable de contexto, pidiendo
únicamente el nombre del vínculo; al crearse, el gesto de login compuesto
SHALL quedar visible con su acción de copiar. Los vínculos SHALL poder
desvincularse desde la misma ficha, y la superficie SHALL decir que
desvincular no borra el contexto de autenticación. Una entrada sin variable
declarada MUST NOT ofrecer el flujo y SHALL señalar la vía manual.

#### Scenario: Vincular desde la ficha del agente

- **WHEN** el usuario vincula una suscripción desde la ficha de un agente con
  variable declarada
- **THEN** la Flota SHALL listar la fila nueva del perfil sin recargar la
  aplicación
- **AND** el formulario SHALL haber pedido solo el nombre

#### Scenario: El gesto de login queda a un clic de copiar

- **WHEN** el vínculo se crea
- **THEN** la ficha SHALL mostrar el gesto de autenticación compuesto
- **AND** SHALL ofrecer copiarlo con la acción de copia existente

#### Scenario: Desvincular dice lo que no borra

- **WHEN** el usuario desvincula desde la ficha
- **THEN** la superficie SHALL declarar que el directorio de contexto queda
  intacto
- **AND** la fila del perfil SHALL desaparecer de la Flota
