# tui-shell — delta

## ADDED Requirements

### Requirement: La identidad y las máquinas se leen desde el terminal

La paleta del terminal SHALL ofrecer los verbos de identidad —consultar quién
soy y mis equipos, y componer el gesto de vínculo— con descripción en ambos
idiomas, y SHALL presentar la respuesta con la presencia de cada equipo en
símbolo con gemelo ASCII y palabra. Sin cliente de malla, el terminal SHALL
mostrar el diagnóstico y el remedio del daemon sin cambiar de vista. El estado
vacío sin daemon NO SHALL mencionar la identidad.

#### Scenario: Identidad desde la paleta

- **WHEN** se invoca el verbo de identidad desde la paleta con la malla
  enlazada
- **THEN** el terminal SHALL mostrar quién soy, este equipo y la lista con
  presencia en símbolo y palabra

#### Scenario: Sin cliente de malla, el terminal dice el remedio

- **WHERE** el daemon rehúsa por no haber cliente de malla
- **THEN** el terminal SHALL mostrar el diagnóstico y el remedio tal cual
- **AND** SHALL seguir en la misma vista
