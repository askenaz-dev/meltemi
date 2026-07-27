# gui-shell — delta

## ADDED Requirements

### Requirement: Alineación global de los controles con icono
El skin compartido de botones SHALL disponer icono y etiqueta en una sola
línea, con el icono centrado verticalmente respecto del texto y separación
uniforme, provisto por la regla global del skin: un componente MUST NOT
necesitar re-declarar la alineación en su hoja local para que un botón con
icono y etiqueta se renderice correcto. Las acciones de un estado vacío SHALL
renderizarse cada una a su altura natural y MUST NOT estirarse para igualar
la altura de otra, tampoco cuando la fila envuelve.

#### Scenario: Icono y etiqueta en una línea
- **WHEN** un botón compone un icono y una etiqueta sin reglas de layout locales
- **THEN** ambos SHALL renderizarse en una sola línea
- **AND** el icono SHALL quedar centrado verticalmente respecto del texto

#### Scenario: Par de acciones del estado vacío a altura pareja
- **WHEN** un estado vacío ofrece dos acciones y la fila las envuelve
- **THEN** cada acción SHALL conservar su altura natural
- **AND** ninguna SHALL estirarse para igualar la altura de la otra

### Requirement: Etiquetas de acción sin atajo incrustado
Las cadenas del catálogo de mensajes MUST NOT incrustar la pista de un atajo
de teclado como texto plano dentro de una etiqueta de acción — un número
entre paréntesis se lee como contador vivo que no es; el atajo SHALL
mostrarse únicamente en su afordancia dedicada del chrome (`kbd`).

#### Scenario: La acción de flota sin falso contador
- **WHEN** el estado vacío de Sesiones ofrece la acción de ir a la Flota
- **THEN** su etiqueta SHALL leerse sin número entre paréntesis
- **AND** SHALL NOT sugerir un recuento de agentes

#### Scenario: El atajo conserva su afordancia
- **WHEN** el usuario observa el sidebar con el estado vacío de Sesiones en pantalla
- **THEN** el ítem Flota SHALL seguir mostrando su atajo como `kbd`
