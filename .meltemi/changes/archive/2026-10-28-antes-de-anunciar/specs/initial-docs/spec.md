# initial-docs — delta

## ADDED Requirements

### Requirement: Quickstart desde los instaladores
El quickstart SHALL empezar en los assets de la release publicada —instalador
o archivo del núcleo, con su verificación— y llevar a una sesión gobernada con
el agente que el usuario ya tiene, en terminal y en el escritorio, antes del
ciclo de specs; SHALL NOT pedir compilar ni escribir a mano la configuración
del agente. Compilar desde la fuente SHALL documentarse en el README y en la
guía de contribución.

#### Scenario: El quickstart no pide compilar
- **WHEN** corre la verificación de la documentación
- **THEN** SHALL fallar si el quickstart pide `cargo build` o un `config.toml` escrito a mano
- **AND** SHALL fallar si no enlaza los assets de la release y su verificación

### Requirement: Preguntas frecuentes y resolución de problemas
La documentación SHALL tener una página de preguntas frecuentes y resolución de
problemas que responda, enlazando la guía que lo desarrolla: un agente que la
flota no puede lanzar, el CLI que deniega sin terminal, el túnel hacia un
named pipe, el turno que termina sin decir nada, un agente que edita sin
preguntar, el aviso del instalador sin firma de plataforma y dónde están los
datos y los registros. El README y el quickstart SHALL enlazarla.

#### Scenario: Cada pregunta tiene su respuesta
- **WHEN** corre la verificación de la documentación
- **THEN** SHALL fallar si falta una de las preguntas declaradas o el enlace desde el README
