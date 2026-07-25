## ADDED Requirements

### Requirement: Guía de agentes verificada contra el registro
El repositorio SHALL incluir una guía de agentes enlazada desde el README que,
por cada entrada del registro de flota, explique qué instala el usuario, cómo se
detecta cada capa, el nivel de integración y qué significa, la configuración de
perfiles para varias suscripciones con ejemplos completos, y la solución de
problemas de detección por sistema operativo. El registro SHALL ser la única
fuente de los hechos de la guía y una verificación del pipeline MUST fallar
cuando guía y registro divergan: entrada sin sección, sección sin entrada, o
nivel y binarios distintos de los declarados. El enlace desde el README MUST
respetar la regla vigente de no nombrar productos de terceros fuera de datos
factuales de interoperabilidad.

#### Scenario: Cada entrada del registro tiene su sección con nivel y binarios
- **WHEN** corre la verificación de la guía de agentes
- **THEN** cada entrada del registro SHALL tener su sección en la guía
- **AND** la sección SHALL declarar el mismo nivel y los mismos binarios por capa que el registro

#### Scenario: Entrada nueva o renombrada falla la verificación
- **WHEN** el registro gana o renombra una entrada sin actualizar la guía
- **THEN** la verificación SHALL fallar señalando la entrada y la sección ausente

#### Scenario: Ejemplos de configuración de perfiles válidos
- **WHEN** la verificación revisa los ejemplos de perfiles de la guía
- **THEN** cada ejemplo SHALL parsear como configuración válida del proyecto
- **AND** SHALL NOT contener material secreto en claro

#### Scenario: Solución de problemas de detección por sistema operativo
- **WHEN** un usuario cuyo agente no se detecta consulta la guía
- **THEN** SHALL encontrar los síntomas y los remedios por sistema operativo, incluidos los shims de script en Windows

#### Scenario: Enlace desde el README sin nombrar productos
- **WHEN** corre el lint de documentación sobre el README
- **THEN** el enlace a la guía SHALL resolver a un archivo existente
- **AND** el README SHALL seguir sin nombrar productos de terceros
