# auditar-reglas-y-procedencia

> Vía completa: toca el log de sesión y la respuesta de `permission/decide`.
> Nace de los límites que `aprobador-en-el-cli` declaró al cerrar (design D1 y
> D5).

## Why

Tres huecos de auditoría quedaron escritos:

- **Una regla creada no deja rastro.** `permission/decide {persistRule}` la
  escribe en `permissions.toml`, pero ningún log de sesión lo registra: una
  concesión futura por esa regla no se puede rastrear hasta quién la creó,
  cuándo y desde qué superficie, y ahora puede crearla también un terminal.
- **Un fallo al escribirla no se dice.** Si la persistencia falla, el daemon lo
  anota en su propio registro y responde `applied` igual
  (`server.rs:2786-2795`): la superficie le dice al humano que recordó algo que
  no quedó escrito.
- **La denegación de un cliente sin terminal parece una persona.** El CLI sin
  terminal responde el push con el rechazo del agente, y el log lo anota
  `decidedBy: client`, igual que una tecla pulsada.

## What Changes

- Evento `permission_rule_persisted` en el log de la sesión de la petición: la
  regla, el archivo, el `requestId` y el cliente que lo pidió.
- `PermissionDecideResult` gana si la regla se escribió (aditivo), y cada
  superficie dice la verdad cuando no.
- La respuesta del push puede declararse desatendida (aditivo); el daemon la
  anota con una procedencia propia, distinta de la de una persona.

## Capabilities

### Modified Capabilities

- `permission-rules`: auditoría de las reglas creadas y procedencia de un
  cliente sin humano.

## Impact

- `proto/`, `core/meltemid/src/{server.rs,acp.rs,pending.rs}`, el aprobador del
  CLI y las dos bandejas.
