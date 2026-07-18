## ADDED Requirements

### Requirement: Semántica operativa por nivel
El daemon SHALL soportar los cuatro niveles de integración con semántica
declarada: nivel 1 ACP nativo por stdio; nivel 2 ACP a través de un binario
adaptador declarado en el registro; nivel 3 ejecución headless con salida
estructurada del agente; nivel 4 integración por artefactos vía proyección de
contexto. Cada sesión MUST declarar su nivel y las capacidades ausentes de ese
nivel MUST ser visibles, no simuladas.

#### Scenario: Sesión declara su nivel
- **WHEN** se abre una sesión con un agente de nivel 2
- **THEN** la sesión SHALL reportar nivel 2
- **AND** el flujo de permisos SHALL operar igual que en nivel 1

#### Scenario: Capacidad ausente visible
- **WHERE** una sesión corre en nivel 3
- **THEN** la superficie SHALL mostrar que no existe canal de permisos rico
- **AND** SHALL NOT presentar aprobaciones simuladas

### Requirement: Lanzamiento por adaptador en nivel 2
El daemon SHALL lanzar los agentes de nivel 2 a través del adaptador ACP
declarado en su entrada del registro, sometido a la misma detección pasiva, el
mismo handshake y el mismo passthrough de permisos que un agente nativo.

#### Scenario: Adaptador como puente transparente
- **WHEN** se abre una sesión con un agente de nivel 2 cuyo adaptador está detectado
- **THEN** el daemon SHALL lanzar el adaptador con los argumentos declarados
- **AND** la sesión SHALL comportarse como una sesión ACP (streaming, permisos, cancelación)

#### Scenario: Adaptador no detectado
- **IF** el adaptador declarado no está detectado
- **THEN** el daemon SHALL responder el error de agente no detectado con remedio
- **AND** SHALL NOT lanzar ningún proceso

### Requirement: Guardarraíles obligatorios del nivel 3
El daemon SHALL rehusar lanzar una ejecución de nivel 3 si no puede garantizar
sus guardarraíles: directorio de trabajo acotado, controles nativos del agente
configurados desde la entrada del registro, y las denegaciones del motor de
reglas aplicadas como configuración previa. La salida estructurada del agente
SHALL mapearse al subconjunto común de eventos de sesión y lo no mapeable MUST
conservarse crudo en el log.

#### Scenario: Sin guardarraíles no se lanza
- **IF** el directorio acotado no puede prepararse para una tarea de nivel 3
- **THEN** el daemon SHALL rehusar el lanzamiento con diagnóstico
- **AND** SHALL NOT ejecutar el agente

#### Scenario: Salida estructurada mapeada
- **WHEN** un agente de nivel 3 emite su salida JSON de progreso
- **THEN** la sesión SHALL reflejar los eventos mapeados en streaming
- **AND** el log SHALL conservar la salida original

### Requirement: Integración por artefactos en nivel 4
El daemon SHALL soportar el nivel 4 sin subproceso: la proyección de contexto es
el vehículo de instrucciones y el trabajo del agente externo se registra como
sesión de tipo externo, trazable aunque no pilotada. Las superficies MUST
presentar el nivel 4 como lo que es: integración de solo-contexto.

#### Scenario: Proyección como única vía
- **WHEN** un proyecto usa un agente de nivel 4
- **THEN** la proyección SHALL incluir el destino declarado por su entrada
- **AND** ninguna sesión pilotada SHALL abrirse para ese agente

### Requirement: Suite de conformidad por nivel
El proyecto SHALL mantener una suite de conformidad ejecutable con criterios
pasa/no-pasa por nivel (streaming, cancelación, permisos, sesión, salida
estructurada, proyección), que en CI MUST correr exclusivamente contra agentes
simulados y sin red; la ejecución contra agentes reales MUST ser manual y por
opt-in explícito. El resultado por agente SHALL persistirse con fecha y versión.

#### Scenario: Conformidad en CI con simulados
- **WHEN** la suite corre en CI
- **THEN** SHALL ejecutar los criterios de cada nivel contra los agentes simulados
- **AND** SHALL NOT contactar red ni agentes reales

#### Scenario: Resultado persistido
- **WHEN** una corrida de conformidad concluye para un agente
- **THEN** el resultado por criterio SHALL persistirse con fecha y versión del agente

### Requirement: Nivel verificado en el catálogo
El catálogo SHALL reportar, además del nivel declarado, el nivel verificado por
la última corrida de conformidad persistida (con su fecha), y las superficies
MUST distinguir visualmente declarado de verificado.

#### Scenario: Declarado no es verificado
- **WHEN** un agente declara nivel 1 sin corrida de conformidad registrada
- **THEN** `fleet/list` SHALL reportar nivel declarado 1 y verificado ausente
- **AND** la vista Flota SHALL mostrar la distinción con etiqueta textual
