## 1. Registro de proyectos en el daemon

- [x] 1.1 Módulo del registro sobre `projects/index.jsonl` apend-only (clave, raíz, primera y última vez vista) con fold last-wins y reconstrucción desde el índice de sesiones cuando falta o está dañado _(Req: Registro de proyectos conocidos persistido y reconstruible; design D2)_
- [x] 1.2 Marca de ausencia: la raíz que ya no existe en disco se conserva y se reporta ausente, jamás se borra _(Req: Registro de proyectos conocidos persistido y reconstruible; design D2)_
- [x] 1.3 Alta en los dos puntos de uso real (arranque de sesión, junto al registro de inicio del índice; resolución del contexto de proyecto), sin recorrer el disco del usuario _(Req: Registro alimentado por el uso real; design D3)_

## 2. Contrato aditivo

- [x] 2.1 `proto/`: constante `project/list`, tipos `ProjectListParams`/`ProjectListResult`/`ProjectInfo` en camelCase y `proto/schemas/v1/project-list.schema.json` _(Req: Consulta project/list con paridad de superficies; design D4)_
- [x] 2.2 `SessionInfo` y `session-list.schema.json`: campos opcionales `agentId` y `profile`, sin transportar nunca la sobrecapa de entorno _(Req: Agente y suscripción resueltos en los metadatos de sesión; design D5)_
- [x] 2.3 Casos de conformidad para los tipos nuevos y para el listado de sesiones con los campos aditivos _(Req: Consulta project/list con paridad de superficies; design D4)_

## 3. Suscripción y agente en los metadatos

- [x] 3.1 `SessionRecord` gana `agent_id` y `profile` con defaults compatibles; los escriben el registro de inicio y el de fin, y el fold los preserva _(Req: Agente y suscripción resueltos en los metadatos de sesión; design D4)_
- [x] 3.2 Reconstrucción desde el log: el evento de resolución repuebla agente y perfil cuando el índice falta _(Req: Agente y suscripción resueltos en los metadatos de sesión; design D2)_
- [x] 3.3 Handler de `session/list`: propaga agente y perfil, y el listado sin filtro queda declarado global con la raíz de cada sesión para agregar en el cliente _(Req: Listado histórico por contrato; design D7)_

## 4. Handler y paridad de núcleo

- [x] 4.1 Handler `project/list`: registro + contadores de sesiones activas y totales + orden por recencia + inclusión de raíces ausentes _(Req: Consulta project/list con paridad de superficies; design D2)_
- [x] 4.2 Subcomando CLI `projects` (con `--json`) y regeneración de la referencia CLI _(Req: Consulta project/list con paridad de superficies; design D4)_
- [x] 4.3 Entrada en la paleta de la TUI, en el registro tipado de la GUI y fila en `docs/paridad-nucleo.md` (gate bloqueante de paridad) _(Req: Consulta project/list con paridad de superficies; design D4)_

## 5. Árbol y ámbito en la GUI

- [x] 5.1 Proyecto activo como estado de la superficie: el cwd pasa a ser solo el ámbito inicial, el activo se persiste en el estado de UI del directorio de datos y toda llamada con ámbito lo inyecta _(Req: Ámbito de proyecto conmutable y persistente; design D6)_
- [x] 5.2 Conmutador de proyecto en la cabecera del sidebar sobre `project/list`, con raíz ausente marcada y su remedio _(Req: Ámbito de proyecto conmutable y persistente; design D6)_
- [x] 5.3 Árbol Proyecto → Sesiones agregado en cliente desde un `session/list` global unido a `project/list`, con avatar de agente, pill de suscripción, densidad 32/8/16 y sin animación de layout _(Req: Árbol Proyecto → Sesiones en el sidebar; design D7)_
- [x] 5.4 Selector de proyecto en el lanzador de "Nueva sesión", con el activo preseleccionado y perfiles ofrecidos por nombre de suscripción, sin métodos nuevos _(Req: Selector de proyecto en el lanzador de sesión; design D6)_
- [x] 5.5 Suscripción visible en las filas de la vista Sesiones y en el drawer de detalle, con la misma identidad de agente en todas _(Req: Árbol Proyecto → Sesiones en el sidebar; design D5)_

## 6. Sesiones agrupadas en la TUI

- [x] 6.1 Vista Sesiones agrupada por proyecto con encabezado de grupo y agente · suscripción por fila, honrando gemelo ASCII, `NO_COLOR` y el degradado de columnas _(Req: Sesiones agrupadas por proyecto con ámbito conmutable; design D6)_
- [x] 6.2 Filtro por proyecto sobre `/` y conmutación del ámbito de proyecto desde la paleta _(Req: Sesiones agrupadas por proyecto con ámbito conmutable; design D6)_

## 7. Guía e internacionalización

- [x] 7.1 Sección de perfiles multi-suscripción en la guía de agentes con el ejemplo canónico de dos cuentas del mismo proveedor y `${VAR}` como única vía _(Req: Guía de perfiles multi-suscripción; design D8)_
- [x] 7.2 Cadenas nuevas de GUI y TUI enrutadas por los catálogos ES/EN, sin texto hardcodeado _(Req: Árbol Proyecto → Sesiones en el sidebar; design D6)_

## 8. Tests y calidad

- [x] 8.1 Unit: fold del registro (last-wins con primera vez preservada, sin duplicados), reconstrucción desde el índice, raíz ausente conservada, orden por recencia _(Req: Registro de proyectos conocidos persistido y reconstruible)_
- [x] 8.2 Unit: alta solo en los puntos de uso real; ninguna consulta pobla proyectos no usados _(Req: Registro alimentado por el uso real)_
- [x] 8.3 Unit: `SessionRecord` con y sin los campos nuevos (compatibilidad de defaults) y repoblado desde el evento de resolución; ningún campo lleva entorno del perfil _(Req: Agente y suscripción resueltos en los metadatos de sesión)_
- [x] 8.4 E2e sobre dos fixtures-proyecto temporales con `mock-agent` y dos perfiles distinguibles: sesiones en ambos → `project/list` reporta los dos con sus contadores y `session/list` sin filtro trae ambos con raíz y suscripción; mover una raíz la deja ausente pero listada _(Req: Consulta project/list con paridad de superficies)_
- [x] 8.5 Smoke de la GUI: el árbol agrupa dos proyectos y distingue dos suscripciones del mismo agente sin animar el layout _(Req: Árbol Proyecto → Sesiones en el sidebar)_
- [x] 8.6 Smoke de la TUI: agrupación por proyecto, filtro por proyecto y ámbito conmutado desde la paleta, en ASCII y `NO_COLOR` _(Req: Sesiones agrupadas por proyecto con ámbito conmutable)_
- [x] 8.7 E2e del listado global: una sola consulta sin filtro agrega el árbol de ambos proyectos _(Req: Listado histórico por contrato)_
- [x] 8.8 Verificación documentada del ámbito persistente y del lanzador multiproyecto por escenario, más `cargo clippy -- -D warnings`, `fmt --check`, gate de paridad y tests verdes en las tres plataformas _(Req: Ámbito de proyecto conmutable y persistente)_
