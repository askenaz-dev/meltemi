# Diseño

## Composición

**Tesis visual:** superficie tranquila y densa, con una franja de categorías y
secciones abiertas, usando los tokens existentes y sin tarjetas decorativas.

**Plan de contenido:** navegación persistente arriba; sólo una categoría visible
debajo; sus títulos y controles conservan el orden actual. Apariencia contiene
tema e idioma; Herramientas, editor y avisos; Proyecto, configuración y permisos;
Conexiones, máquinas e identidad de malla; Información, diagnóstico y privacidad.
No hay controles de tipografía que justifiquen una pestaña de Texto.

**Interacción:** selección y foco se desplazan juntos mediante el `TabStrip`
existente; su desplazamiento mínimo revela una pestaña fuera del ancho; hover y
anillo de foco ofrecen respuesta inmediata sin animaciones decorativas.

## Decisiones

1. Reutilizar `TabStrip`: conserva un único patrón de flechas, Home/End, orden de
   tabulación y botones de desplazamiento. Los identificadores llevan el prefijo
   `settings-` y las pestañas no se pueden cerrar.
2. Ocultar paneles con `hidden`, sin desmontarlos. La plantilla sin guardar, los
   resultados de diagnóstico y los de pruebas de conexión permanecen en memoria;
   cambiar categoría no llama a guardar ni ejecuta acciones.
   La sincronización de la plantilla observa sólo su valor persistido; actualizar
   tema o idioma en el mismo store no debe reemplazar una edición pendiente.
3. Apariencia es la categoría inicial. La categoría seleccionada es estado local
   de la vista; no se amplía el contrato de persistencia. La declaración completa
   de privacidad permanece accesible en Información, en vez de al final de todas
   las secciones.
4. La franja no se desplaza verticalmente con el contenido. Cada panel mantiene
   su propio scroll. Controles flexibles y destinos táctiles de 44 px evitan
   desbordamientos en anchos estrechos. Se reutilizan ambos temas y forced-colors.
   Los nombres de categorías se leen completos al desplazarse. La medición del
   `TabStrip` se difiere al siguiente frame desde ResizeObserver: mostrar sus
   botones de scroll no debe redimensionar el elemento durante la notificación.
5. Verificación: checks y pruebas UI existentes, build de ambas entradas, motor
   de specs y un navegador con puente Tauri simulado sobre un fixture temporal.
   Comprobar teclado, asociación ARIA, borrador, diagnóstico, ES/EN, ambos temas,
   ancho estrecho y capturas; no hace falta arrancar daemon ni agentes reales.
