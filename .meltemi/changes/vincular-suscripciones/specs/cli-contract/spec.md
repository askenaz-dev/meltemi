# cli-contract — delta

## ADDED Requirements

### Requirement: Verbos de vínculo de suscripción

La gramática SHALL ofrecer `link <agente> <nombre>` y `unlink <nombre>`,
mapeados a los métodos de vínculo de suscripción del contrato; el nombre MUST
viajar al daemon tal cual se escribió, y la referencia CLI generada SHALL
enumerarlos.

#### Scenario: link crea y responde con el gesto de login

- **WHEN** se invoca `link` con un id del catálogo con variable declarada y
  un nombre válido
- **THEN** la salida SHALL confirmar el vínculo
- **AND** SHALL imprimir el gesto de autenticación compuesto

#### Scenario: unlink de un vínculo manual rehúsa con remedio

- **WHEN** se invoca `unlink` con el nombre de un perfil escrito a mano
- **THEN** el binario SHALL terminar con el código de error de contrato
- **AND** el mensaje SHALL traer el remedio del daemon
