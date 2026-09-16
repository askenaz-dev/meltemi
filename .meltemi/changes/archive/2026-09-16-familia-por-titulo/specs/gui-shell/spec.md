## ADDED Requirements

### Requirement: El título del schema declara la ligadura del formulario
La generación de formularios tipados de la paleta SHALL ligar cada método al
schema cuyo `title` declara ese método, y el nombre del archivo del schema NO
SHALL ser llave de esa ligadura. El contrato MAY nombrar un archivo de schema
por la familia a la que sirve o por los verbos que declara, y el gate que
vigila la correspondencia método→schema MUST NOT rehusar una ligadura correcta
por la forma del nombre del archivo, de modo que un schema que sirve a varios
verbos de una familia quepa en el contrato sin renombrarse.

#### Scenario: Un schema nombrado por sus verbos pasa el gate
- **WHERE** un archivo de schema lleva el nombre de los verbos que declara en lugar del de su familia
- **WHEN** el gate revisa la ligadura de un método servido por ese schema
- **THEN** SHALL aceptarla

#### Scenario: Los dos métodos del taller se ligan a su schema
- **WHEN** el gate revisa `change/workspace` y `change/land`
- **THEN** ambos SHALL aceptarse contra el schema que los declara en su `title`
