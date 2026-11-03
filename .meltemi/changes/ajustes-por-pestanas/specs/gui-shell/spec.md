## MODIFIED Requirements

### Requirement: Superficie de Ajustes
La superficie SHALL ofrecer Ajustes con casa propia, alcanzable desde el
sidebar, con: tema (claro/oscuro/sistema), idioma (ES/EN), plantilla de
"Abrir con…" (persistida y usada por el deep-link), el visor de la
configuración efectiva del proyecto (`.meltemi/config.toml` y permisos) con
salto directo a editarla en el editor trazable, y la declaración explícita de
que no hay cuentas, red ni telemetría, accesible en Información. Los ajustes
MUST persistir en el directorio de datos del usuario.

#### Scenario: La plantilla de Abrir con se configura y persiste
- **WHEN** el usuario define la plantilla de "Abrir con…" en Ajustes y reinicia
- **THEN** el deep-link SHALL usar esa plantilla sin depender de variables de entorno

#### Scenario: Ver y editar la configuración efectiva
- **WHEN** el usuario abre la categoría Proyecto en Ajustes
- **THEN** SHALL ver la configuración efectiva y poder saltar a editarla en el editor (guardado trazable)

#### Scenario: La promesa de privacidad es visible
- **WHEN** el usuario abre Información en Ajustes
- **THEN** la declaración sin cuentas, sin red y sin telemetría SHALL estar visible

## ADDED Requirements

### Requirement: Categorías de Ajustes accesibles
Ajustes SHALL organizar los controles existentes en Apariencia, Herramientas,
Proyecto, Conexiones e Información mediante pestañas con etiquetas ES/EN.
Cada pestaña SHALL controlar un panel identificado y etiquetado; sólo el panel
seleccionado SHALL estar visible y disponible para el foco. Cambiar de categoría
MUST NOT descartar entradas sin guardar, resultados ni ejecutar acciones.

#### Scenario: Entrar y recorrer categorías con teclado
- **WHEN** el usuario abre Ajustes y recorre las pestañas con flechas, Home o End
- **THEN** Apariencia SHALL estar seleccionada inicialmente y selección y foco SHALL moverse juntos
- **AND** sólo una pestaña SHALL pertenecer al orden de Tab
- **AND** cada panel SHALL estar asociado a su pestaña mediante ARIA

#### Scenario: El borrador y los resultados sobreviven al cambio de categoría
- **WHEN** el usuario escribe una plantilla sin guardarla o genera un diagnóstico y visita otra categoría
- **THEN** la entrada y el resultado SHALL seguir presentes al volver
- **AND** cambiar de categoría MUST NOT guardar la plantilla ni repetir el diagnóstico

#### Scenario: Cambiar tema o idioma conserva la plantilla pendiente
- **WHEN** el usuario deja una plantilla sin guardar y cambia el tema o el idioma en Apariencia
- **THEN** la plantilla pendiente SHALL conservarse al volver a Herramientas
- **AND** el valor persistido MUST NOT cambiar hasta que el usuario pulse Guardar

#### Scenario: Navegación en ancho estrecho
- **WHEN** el ancho disponible no permite mostrar todas las pestañas
- **THEN** la franja SHALL permitir alcanzar cada categoría mediante scroll y teclado
- **AND** la pestaña seleccionada SHALL quedar visible y el contenido MUST NOT desbordar horizontalmente la vista
- **AND** los controles SHALL ofrecer destinos táctiles de al menos 44 px en ancho pequeño
