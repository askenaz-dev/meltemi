# gui-shell — delta

## ADDED Requirements

### Requirement: Lienzo completo del shell
La columna central del shell SHALL repartir el alto de la ventana entre sus
barras (banner de daemon, barra superior, avisos) a su altura natural y la
vista enrutada, que SHALL ocupar todo el resto hasta la barra de estado
anclada al borde inferior, con independencia de cuántas barras condicionales
estén presentes. Ninguna fila de los árboles o listas del shell SHALL
renderizarse por debajo de su altura de línea.

#### Scenario: La vista ocupa el alto disponible
- **WHEN** el shell se renderiza con el daemon conectado y sin avisos activos
- **THEN** la vista enrutada SHALL extenderse hasta la barra de estado
- **AND** la barra de estado SHALL quedar anclada al borde inferior de la ventana

#### Scenario: Filas del árbol sin recorte
- **WHEN** el árbol del proyecto renderiza más filas de las que caben en su panel
- **THEN** cada fila SHALL conservar al menos su altura de línea
- **AND** el excedente SHALL desplazarse con scroll, nunca comprimirse
