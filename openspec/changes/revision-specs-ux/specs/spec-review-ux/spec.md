## ADDED Requirements

### Requirement: Diff de deltas por requisito
La superficie de revisión SHALL renderizar los deltas agrupados por operación y
requisito, mostrando para MODIFIED la comparación por statement y por escenario
alineada por nombre; la operación MUST comunicarse con palabra y glifo bajo la
línea base de accesibilidad, nunca solo con color.

#### Scenario: MODIFIED alineado por escenario
- **WHEN** el usuario revisa un delta MODIFIED de un requisito
- **THEN** la vista SHALL mostrar el antes y el después por escenario alineados por nombre
- **AND** los escenarios añadidos o retirados SHALL marcarse con palabra y glifo

#### Scenario: Legible sin color
- **WHERE** rige NO_COLOR o ASCII
- **THEN** las operaciones SHALL distinguirse por etiqueta textual y glifo gemelo

### Requirement: Checklist de revisión persistente
El verbo `review` SHALL recorrer los requisitos del delta como checklist con
estados aprobado, comentado o rechazado, persistidos en la change y reanudables;
cerrar la revisión MUST exigir todos los ítems decididos, y los diagnósticos del
motor SHALL anclarse al requisito que los produce.

#### Scenario: Review reanudable
- **WHEN** el usuario abandona una review a medias y vuelve
- **THEN** la checklist SHALL conservar los estados decididos
- **AND** SHALL continuar en el primer ítem pendiente

#### Scenario: Cierre exige decisión total
- **IF** quedan ítems sin decidir
- **THEN** el cierre de la review SHALL rehusarse indicando cuáles faltan

### Requirement: Comentario convertido en instrucción
Un comentario de revisión SHALL despacharse al agente autor como instrucción de
reelaboración del artefacto de specs citando el requisito comentado, reabriendo
su gate; el vínculo comentario→reelaboración MUST quedar registrado en la change.

#### Scenario: Comentario reabre el gate
- **WHEN** el usuario comenta un requisito en la review
- **THEN** el agente SHALL recibir la instrucción con el requisito citado
- **AND** el gate de specs SHALL reabrirse para la nueva entrega

### Requirement: Verbo review operativo
El subcomando `review` SHALL ser operativo: en la TUI como flujo interactivo de
la vista Proyecto y en modo scriptable por pasos con `--json` reportando el
estado de la checklist, sin quedar jamás a la espera interactiva sin TTY.

#### Scenario: Estado de la checklist por JSON
- **WHEN** se invoca `meltemi review --json` sobre una change en revisión
- **THEN** el binario SHALL emitir un objeto con los ítems y sus estados
