# gui-shell — delta

## ADDED Requirements

### Requirement: La identidad de la malla se ve donde se ven los ajustes

La superficie de escritorio SHALL mostrar en Ajustes una sección de identidad
con lo que el daemon lee de la malla del usuario: quién es, el proveedor o
nombre de la malla, este equipo, y la lista de sus equipos con la presencia de
cada uno declarada con símbolo y palabra. Esa sección SHALL ofrecer el gesto de
vínculo como texto copiable, NO SHALL ejecutarlo ni abrir el navegador, y SHALL
decir de quién es la identidad —de la malla del usuario, leída y no guardada—.

Con la identidad configurada, la barra lateral SHALL mostrar en su zona
inferior —por encima de Ajustes, que SHALL seguir siendo la última entrada— el
nombre de login con la cuenta de equipos, o «Enlazar este equipo» cuando este
equipo aún no la tiene; sin configurar, el pie NO SHALL cambiar. Esa fila NO
SHALL participar del reparto entre la navegación y el árbol, y con la barra
plegada a riel SHALL conservar su etiqueta accesible y su foco.

La declaración de privacidad «sin cuentas» SHALL conservarse tal cual, y el
onboarding NO SHALL mencionar ni exigir la identidad.

#### Scenario: Ajustes muestra quién soy y mis equipos

- **WHEN** hay identidad en la malla y se abre Ajustes
- **THEN** la sección SHALL mostrar el nombre de login, el proveedor, este
  equipo y la lista de equipos con su presencia en símbolo y palabra
- **AND** SHALL ofrecer el gesto de vínculo con un control de copiar

#### Scenario: Sin enlazar, la barra ofrece enlazar y no exige nada

- **WHERE** hay identidad configurada y el daemon responde que este equipo no
  la tiene
- **THEN** el pie de la barra SHALL mostrar «Enlazar este equipo» que lleva a
  Ajustes
- **AND** ninguna otra parte de la superficie SHALL cambiar ni pedir nada

#### Scenario: Sin configurar, el cromo no pide nada

- **WHERE** no hay tabla de identidad en la configuración
- **THEN** el pie de la barra NO SHALL cambiar
- **AND** Ajustes SHALL seguir siendo la única puerta a la identidad

#### Scenario: Plegada, la identidad sigue alcanzable

- **WHEN** la barra está plegada a riel con identidad configurada
- **THEN** la fila SHALL conservar su etiqueta accesible y su foco
- **AND** Ajustes SHALL seguir siendo la última entrada

#### Scenario: La promesa de privacidad sigue siendo cierta

- **WHEN** se lee la sección de identidad junto a la de privacidad
- **THEN** la declaración «sin cuentas» SHALL seguir presente sin cambios
- **AND** la sección de identidad SHALL decir que la identidad es de la malla
  del usuario y que Meltemi la lee, no la guarda
