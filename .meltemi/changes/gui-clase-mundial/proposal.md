## Why

La GUI de `gui-tauri-paridad` es correcta —paridad verificada, contraste AA
medido en ambos temas, teclado completo, presupuestos cumplidos— pero correcta
no es clase mundial. La auditoría de la superficie (recorrido real del shell,
contraste computado, revisión de flujos contra el estándar de las mejores
herramientas de escritorio) encontró la distancia exacta:

1. **Una pérdida de datos silenciosa**: cerrar una pestaña sucia del editor,
   navegar fuera o pulsar Esc descarta cambios sin preguntar. Inaceptable en
   cualquier liga.
2. **La marca desaparece en alto contraste**: el wordmark usa gradiente con
   `color: transparent`; en forced-colors queda invisible. Y la app no usa la
   marca real (`brand/`) en ninguna superficie — se siente genérica.
3. **El trabajo nº 1 no tiene botón**: proponer un cambio —el corazón del
   producto— solo existe tecleando en la paleta. Los estados vacíos de
   Proyecto/Permisos/Flota aconsejan en texto pero no ofrecen la acción.
4. **La paleta es funcional, no memorable**: coincidencia por subcadena en
   orden del contrato, sin difusa, sin grupos, sin recientes, y el único modo
   de parámetros es JSON crudo — teniendo los schemas tipados del contrato en
   `proto/schemas/v1` para generar formularios.
5. **El plano de control no reclama atención**: un permiso pendiente con la
   ventana sin foco no produce ninguna señal del SO; el agente queda bloqueado
   esperando a un humano que no se enteró.
6. **Fricciones de diario**: sesiones sin filtro/orden/tiempos relativos ni
   acciones por fila; transcript plano (tipo+texto truncado, sin expandir,
   copiar ni buscar); tema solo-SO sin selector; ventana que olvida tamaño y
   posición; avisos que crecen sin tope.

Esta change lleva la superficie de "cumple la spec" a "la herramienta que
eliges por gusto", sin tocar el daemon ni el contrato: todo es experiencia de
superficie dentro de la cerca `edit-surface` y la constitución.

## What Changes

- **Editor sin pérdida silenciosa**: guardia de pestañas sucias (guardar /
  descartar / cancelar) en cierre de pestaña, navegación y cierre de ventana;
  quick-open (Ctrl+P) sobre el árbol; archivos recientes por proyecto.
- **Identidad en toda condición**: la marca real de `brand/` en el chrome y
  los estados vacíos, con fallback sólido en forced-colors; iconografía de
  línea propia y coherente (fuera emoji de plataforma).
- **Paleta de clase mundial**: coincidencia difusa por subsecuencia, grupos
  por dominio, recientes primero (persistidos), hints de teclado visibles, y
  **formularios tipados generados en build desde los schemas del contrato**
  con el JSON crudo como modo avanzado; frescura verificada en CI.
- **Acciones primarias**: "Proponer un cambio" como acción de primera clase
  en el chrome; todo estado vacío ofrece su siguiente paso ejecutable.
- **Sesiones vivas**: filtro `/`, orden por columna, tiempos relativos
  localizados (absoluto accesible), chips de resumen por estado y acciones por
  fila (cancelar con confirmación, dirigir).
- **Transcript de primera**: render por tipo de evento (glifo + tono), texto
  expandible, timestamps conmutables, copiar línea/todo y búsqueda local.
- **Tema y ventana persistentes**: selector claro/oscuro/sistema y estado de
  ventana (tamaño/posición/última vista) persistidos en el directorio de
  datos; chip visible `Ctrl+K` en el chrome.
- **Atención de permisos sin foco**: solicitud de atención al SO (API de
  ventana, local) y contador en el título cuando llega un permiso con la
  ventana desenfocada.
- **Avisos con memoria acotada**: timestamps, tope visible con colapso e
  historial; el banner de daemon caído gana "reintentar ahora" y "copiar
  diagnóstico".

## Capabilities

### New Capabilities
- _Ninguna._

### Modified Capabilities
- `gui-shell`: + requisitos de identidad accesible, paleta difusa con
  formularios tipados, acciones primarias, sesiones filtrables, transcript
  rico, guardia de edición sucia, persistencia de tema/ventana, atención de
  permisos y avisos acotados. **Depende de que `gui-tauri-paridad` esté
  archivada** (sus deltas crean `gui-shell` en la verdad viva).

## Impact

- Solo superficie: `desktop/ui/` (paleta, vistas, componentes, i18n, tokens),
  `desktop/src/` (comandos de persistencia local, atención de ventana),
  `desktop/ui/scripts/` (generador de formularios desde `proto/schemas/v1` +
  gate de frescura en CI). **Cero cambios en `proto/` y `meltemid`**: la
  matriz de paridad no se toca.
- Sin dependencias nuevas: difusa propia (~40 ítems), iconos SVG inline,
  atención vía API de ventana del núcleo Tauri, persistencia en JSON del
  directorio de datos.

## Fuera de alcance

- Notificaciones nativas del SO (plugin nuevo = dependencia nueva §10): la
  atención de ventana cubre el caso; se reevalúa con evidencia de uso.
- Todo lo FUERA de la cerca `edit-surface` (plugins, depurador, keybindings
  estilo IDE): quick-open y recientes son navegación utilitaria del bucle
  agéntico, no autoría sostenida.
- Cambios de daemon/contrato (drag-resize de paneles multi-proyecto, RPC de
  lectura): nada aquí lo exige.
- Rediseño del design system: se extiende `docs/ux/design-system.md`
  (iconografía, marca), no se reemplaza.
