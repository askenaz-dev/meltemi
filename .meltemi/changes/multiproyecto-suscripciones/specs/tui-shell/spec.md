## ADDED Requirements

### Requirement: Sesiones agrupadas por proyecto con ámbito conmutable
La vista Sesiones SHALL agrupar las sesiones por proyecto con encabezado de grupo,
y cada fila SHALL mostrar su agente y el nombre de su suscripción cuando la
sesión resolvió por perfil; el filtro `/` vigente SHALL admitir además reducir por
proyecto, y el ámbito de proyecto SHALL ser conmutable desde la paleta sin salir
del shell, con el cwd como ámbito inicial. La agrupación MUST honrar la línea base
de accesibilidad (glifo o forma más palabra, gemelo ASCII, `NO_COLOR`) y el
degradado de columnas vigente, sin ocultar datos en silencio.

#### Scenario: Sesiones agrupadas por proyecto
- **WHEN** el usuario abre la vista Sesiones con sesiones en dos proyectos
- **THEN** la tabla SHALL presentarlas bajo un encabezado por proyecto
- **AND** cada fila SHALL indicar su agente y su suscripción

#### Scenario: Filtro por proyecto reduce a un grupo
- **WHEN** el usuario pulsa `/` y teclea parte de la raíz de un proyecto
- **THEN** la vista SHALL reducirse a las sesiones de ese proyecto

#### Scenario: Suscripción legible sin color ni Unicode
- **WHERE** están activos `NO_COLOR` o el modo ASCII
- **THEN** el proyecto y la suscripción de cada sesión SHALL seguir legibles como texto
- **AND** ninguna distinción SHALL depender del color

#### Scenario: Ámbito de proyecto conmutado desde la paleta
- **WHEN** el usuario conmuta el proyecto de ámbito desde la paleta
- **THEN** las consultas con ámbito de proyecto SHALL usar esa raíz
- **AND** el chrome SHALL reflejar el ámbito vigente
