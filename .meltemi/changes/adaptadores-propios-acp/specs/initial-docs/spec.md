# initial-docs — delta

## ADDED Requirements

### Requirement: Guía de la capa empaquetada y de la vía de terceros
La guía de agentes SHALL explicar que la capa adaptador de las entradas con
adaptador propio viaja en los instaladores de Meltemi — sin comando de
instalación de terceros — y qué hacer cuando falta: reinstalar o reparar la
instalación de Meltemi. La guía SHALL documentar además la receta para
seguir usando un adaptador de terceros por configuración del usuario,
presentada como vía disponible y no recomendada, y SHALL decir que el
estatus y la nota legal que Meltemi muestra describen la vía que Meltemi
distribuye y no esa otra, remitiendo al lector a la licencia del adaptador y
a los términos del proveedor, que son los que sí la gobiernan. La guía MUST
NOT prestar a una vía de terceros el estatus ni la nota de la entrada. La
guía MUST NOT presentar la ruta propia como bendecida por proveedor alguno:
el estatus y la nota del registro se muestran tal cual, y toda actualización
de postura MUST citar fuente.

#### Scenario: La capa empaquetada explicada sin comando de terceros
- **WHEN** el lector consulta la sección de una entrada con adaptador propio
- **THEN** la guía SHALL decir que la capa adaptador viaja con los instaladores de Meltemi
- **AND** el remedio documentado ante su ausencia SHALL ser reinstalar o reparar Meltemi, no un comando de terceros

#### Scenario: Receta de adaptador de terceros por configuración
- **WHEN** el lector busca cómo usar un adaptador de terceros en lugar del propio
- **THEN** la guía SHALL mostrar la declaración por configuración del usuario con un ejemplo válido
- **AND** SHALL decir que el estatus y la nota del registro no cubren esa vía y remitir a los términos que sí la gobiernan, sin recomendarla

#### Scenario: Sin bendición inventada
- **WHEN** corre la verificación de coherencia entre registro y guía
- **THEN** el estatus y la nota legal de cada entrada SHALL coincidir con los declarados en el registro
- **AND** la guía SHALL NOT afirmar aprobación de proveedor que el registro no declare
