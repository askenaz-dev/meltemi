## ADDED Requirements

### Requirement: Guía de perfiles multi-suscripción
La guía de agentes SHALL documentar los perfiles de lanzamiento como suscripciones
con nombre, con el ejemplo canónico de dos cuentas del mismo proveedor conviviendo
en un proyecto mediante la redirección del contexto de autenticación del binario
oficial, y SHALL enseñar la referencia `${VAR}` como única vía para valores
sensibles. La guía MUST NOT incluir credencial alguna ni instruir a pegarla en la
configuración de Meltemi (constitución §2).

#### Scenario: Ejemplo canónico de dos cuentas del mismo agente
- **WHEN** el lector busca cómo usar dos suscripciones del mismo proveedor
- **THEN** la guía SHALL mostrar dos perfiles nombrados sobre el mismo agente del catálogo
- **AND** SHALL explicar que cada uno solo selecciona el contexto donde el binario se autentica

#### Scenario: La guía no pide credenciales
- **WHEN** la guía documenta la sobrecapa de entorno de un perfil
- **THEN** SHALL usar referencias `${VAR}` resueltas al lanzar
- **AND** SHALL NOT mostrar ni pedir material secreto en la configuración
