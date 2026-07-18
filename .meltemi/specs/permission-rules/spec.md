# permission-rules Specification

## Purpose
TBD - created by archiving change proxy-permisos. Update Purpose after archive.
## Requirements
### Requirement: Motor de reglas y precedencia
El daemon SHALL evaluar cada petición de permiso contra las reglas persistentes
antes de escalarla al humano: efecto `allow` o `deny` por herramienta, prefijo de
comando y prefijo de ruta, con ámbitos de proyecto y globales. La precedencia
MUST ser: reglas de proyecto sobre globales, y `deny` sobre `allow` en empate;
sin regla aplicable, la petición SHALL escalar al humano (`ask`). Una regla MUST
NOT conceder nada distinto de las opciones que el agente ofreció.

#### Scenario: Regla permite sin escalar
- **WHEN** llega una petición cuyo comando coincide con una regla `allow` del proyecto
- **THEN** el daemon SHALL responder al agente con la opción de permitir
- **AND** SHALL NOT escalar la petición al cliente

#### Scenario: Deny gana el empate
- **WHERE** una petición coincide con una regla `allow` global y una `deny` de proyecto
- **THEN** el daemon SHALL denegar

#### Scenario: Sin regla se pregunta
- **WHEN** ninguna regla aplica a la petición
- **THEN** el daemon SHALL escalarla como pendiente al humano

### Requirement: Persistencia de reglas por ámbito
Las reglas SHALL persistir en TOML: globales en el directorio de configuración
del usuario y de proyecto en `.meltemi/`. El daemon MUST recargarlas al abrir
sesión y validar su forma, reportando reglas malformadas como diagnóstico sin
descartar el resto.

#### Scenario: Regla malformada no derriba el motor
- **WHEN** el archivo de reglas contiene una entrada inválida
- **THEN** el daemon SHALL reportar la entrada con su ubicación
- **AND** SHALL aplicar las reglas válidas restantes

### Requirement: Cola de pendientes consultable
El daemon SHALL mantener las peticiones de permiso pendientes como cola de primera
clase consultable mediante `permission/pending` (id, sesión, herramienta, resumen,
opciones, plazo), y MUST notificar los cambios de la cola a todos los clientes
conectados. Una petición vencida MUST permanecer visible como vencida en la
consulta inmediata, nunca descartada en silencio.

#### Scenario: Bandeja sobrevive la reconexión
- **WHEN** un cliente se reconecta mientras existen peticiones pendientes
- **THEN** `permission/pending` SHALL devolver la cola vigente
- **AND** el indicador del cliente SHALL reflejar el conteo real

#### Scenario: Cambio notificado
- **WHEN** una petición entra o se resuelve
- **THEN** el daemon SHALL emitir la notificación de cambio a los clientes conectados

### Requirement: Decisión por id y reconciliación
El daemon SHALL aceptar decisiones mediante `permission/decide` (id + opción
elegida) además de la respuesta al push existente; la primera resolución MUST
ganar y la vía perdedora MUST recibir una respuesta explícita de "ya resuelta"
sin aplicar decisión alguna.

#### Scenario: Decide tras reconexión
- **WHEN** un cliente reconectado resuelve por `permission/decide` una petición pendiente
- **THEN** el agente SHALL recibir la opción elegida
- **AND** la cola SHALL reflejar la resolución

#### Scenario: Doble resolución reconciliada
- **IF** llegan dos decisiones para la misma petición por vías distintas
- **THEN** el daemon SHALL aplicar solo la primera
- **AND** SHALL responder "ya resuelta" a la segunda

### Requirement: Bandeja interactiva con creación de reglas in situ
La vista de Permisos SHALL listar la cola con edad y plazo (escalado textual al
acercarse el vencimiento), permitir decidir por petición y ofrecer la creación de
una regla persistente en el mismo gesto, proponiendo la regla más específica
posible y exigiendo confirmación explícita. La línea base de accesibilidad del
shell MUST regir toda la vista.

#### Scenario: Aprobar y crear regla
- **WHEN** el usuario aprueba una petición eligiendo "permitir siempre"
- **THEN** el shell SHALL confirmar la regla propuesta (la más específica) antes de persistirla
- **AND** la decisión SHALL aplicarse a la petición en curso

#### Scenario: Sugerencia anti-fatiga
- **WHILE** el humano aprueba repetidamente peticiones idénticas
- **THEN** el daemon SHALL adjuntar a la siguiente petición la sugerencia de regla equivalente

### Requirement: Auditoría con procedencia de la decisión
El registro JSONL de sesión SHALL enriquecer cada decisión de permiso con su
procedencia — humano, regla (con ámbito y contenido) o vencimiento — de modo que
toda concesión sea rastreable a quién o qué la tomó.

#### Scenario: Decisión por regla auditada
- **WHEN** una regla resuelve una petición
- **THEN** el evento de decisión SHALL registrar la regla aplicada y su ámbito

