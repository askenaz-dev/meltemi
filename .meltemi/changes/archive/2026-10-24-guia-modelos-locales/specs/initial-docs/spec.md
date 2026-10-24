# initial-docs — delta

## ADDED Requirements

### Requirement: Guía de modelos locales verificada y fechada
El repositorio SHALL incluir una guía de modelos locales y autohospedados,
enlazada desde la guía de agentes, con recetas para un proveedor local, un
endpoint único para varios proveedores y un agregador de modelos, cada una con
la configuración del agente y el perfil de lanzamiento de Meltemi que la
selecciona. La guía SHALL declarar la fecha y las versiones con que se verificó
cada receta y si la verificación fue de forma o en vivo, y MUST NOT citar de
memoria. Ninguna receta MUST contener una clave en claro: toda clave SHALL
llegar por referencia al entorno, y una verificación del pipeline MUST fallar si
un bloque de la guía no parsea o lleva una clave literal. La guía y el backlog
MUST NOT presentar como utilizable hoy un nivel de integración que el daemon no
lanza.

#### Scenario: Tres recetas con su perfil
- **WHEN** el lector busca cómo correr un modelo local o autohospedado bajo Meltemi
- **THEN** SHALL encontrar las tres recetas con la configuración del agente y el perfil de Meltemi de cada una

#### Scenario: Ninguna clave en claro
- **WHEN** corre la verificación de la guía de modelos locales
- **THEN** cada bloque de configuración SHALL parsear
- **AND** la verificación SHALL fallar si un bloque lleva una clave literal en lugar de una referencia al entorno

#### Scenario: La guía declara con qué se verificó
- **WHEN** el lector abre la guía
- **THEN** SHALL leer la fecha, la versión del agente y si cada receta se verificó en forma o en vivo

#### Scenario: Lo que no se lanza no se promete
- **WHEN** la guía o el backlog mencionan una vía para modelos locales
- **THEN** MUST NOT presentar como utilizable un agente de un nivel que el daemon no lanza
