<!-- SPDX-License-Identifier: Apache-2.0 -->
# Verificación — ajustes por pestañas

**Fecha:** 2026-11-03. **Fuente:** `28d9f20`, rama `codex/visual-polish`, sobre
`origin/main` `8e1db46`. Windows, Node 24.15.0, Edge 156.0.4314.58.

## Alcance y montaje

La vista `Settings.svelte` real y sus estilos se montaron con Vite sobre un
fixture temporal de navegador. `mockIPC` de Tauri respondió en memoria: proyecto
`C:/fixture/harbour`, dos máquinas ficticias, identidad de ejemplo, configuración,
avisos y diagnóstico. Se registraron las llamadas para distinguir conservar un
resultado de volver a ejecutar su acción. No se inició un daemon ni un agente,
no se usaron credenciales y no se modificaron datos personales.

Playwright Core y la detección de Edge se reutilizaron desde el checkout local
de Diagrams; no se añadió dependencia al IDE. El conductor temporal quedó en
`C:/tmp/meltemi-settings-browser.mjs`; reporte y nueve capturas en
`C:/tmp/meltemi-settings-visual/`. Como en los smokes anteriores, las capturas no
se versionan. Las capturas muestran el componente con el puente simulado, no
una ejecución del binario Tauri completo.

## Escenarios comprobados

| Escenario | Resultado |
| --- | --- |
| Entrar y recorrer categorías con teclado | Cinco pestañas ES/EN, Apariencia inicial, flechas con vuelta, Home/End; foco y selección coinciden. Una pestaña en el orden de Tab y asociación recíproca `aria-controls`/`aria-labelledby`. |
| El borrador y los resultados sobreviven al cambio de categoría | La plantilla pendiente, el diagnóstico y el resultado de conexión siguen al volver; no aumentan las llamadas de diagnóstico o prueba. Sólo un panel visible. |
| Cambiar tema o idioma conserva la plantilla pendiente | Cambiar a oscuro e inglés conserva la edición en Herramientas; el valor persistido sigue siendo el anterior hasta pulsar Guardar. |
| La plantilla de Abrir con se configura y persiste | Guardar llama al mecanismo existente con el texto editado. La persistencia nativa no se reimplementó; su conexión sigue cubierta por `scenarios_shell.rs`. No se midió un reinicio nativo aquí. |
| Ver y editar la configuración efectiva | Proyecto muestra los dos archivos del fixture y Editar recibe `.meltemi/config.toml`. |
| La promesa de privacidad es visible | Información muestra los cuatro compromisos existentes junto al diagnóstico. |
| Navegación en ancho estrecho | A 320×640 las cinco categorías son alcanzables, la activa queda visible con nombre completo, tabs de al menos 44 px y ningún panel desborda horizontalmente. |
| Conservación de acciones | El cambio de máquina pasa por `onGuarded` antes del callback existente; las pruebas y la edición sólo responden a sus controles. |
| Temas y acceso | Capturas a 1100×720 en claro/ES y oscuro/EN; a 320×640 en cada categoría. Forced-colors y reduced-motion conservan foco, borde activo y navegación. |
| Errores de navegador | Ningún `pageerror` ni evento `window.error` en la pasada final. |

Las capturas de Apariencia, Herramientas y Conexiones se revisaron visualmente:
jerarquía de títulos, legibilidad, controles y nombres completos. No se añadió
una categoría de tipografía porque la vista no tiene esos ajustes. El compañero
móvil actual no enruta a Settings; su entrada sigue compilando y este cambio no
añade una pantalla móvil nueva.

## Checks locales

- `npm run check`: cero errores y advertencias Svelte.
- `npm run lint:i18n` y `npm run check:forms`: limpios.
- `npm test`: 115/115.
- `npm run build`: ambas entradas, escritorio y móvil. Sigue el aviso de tamaño
  del bundle principal; no se cambió la estrategia de empaquetado.
- `npm audit --audit-level=high`: pasa; dos avisos bajos existentes de
  `deepmerge`/plugin Svelte, sin cambiar dependencias.
- `desktop/tests/scenarios_shell.rs`: 128/128. Compilado directamente con
  `rustc --test`, `CARGO_MANIFEST_DIR` apuntando a este `desktop/` y `serde_json`
  del target local; no es una compilación completa de Tauri.
- `cargo test --locked -p meltemi-spec --test dogfood`: 2/2, sobre este worktree.
- `cargo fmt --all --check` y `git diff --check`: limpios.

## Hallazgos resueltos

1. El efecto de la plantilla dependía del objeto completo de preferencias:
   cambiar tema o idioma reemplazaba una edición pendiente. Ahora observa sólo
   el valor persistido de la plantilla.
2. Los botones de overflow cambiaban el ancho durante una notificación de
   ResizeObserver. La medición del `TabStrip` compartido se difiere un frame y
   se cancela al desmontar; el aviso de bucle dejó de aparecer.
3. La prueba de skeleton dependía de la indentación exacta del markup. Ahora
   normaliza espacios conservando la aserción de que el skeleton pertenece a
   la rama de carga.

Queda pendiente la CI completa Linux/macOS/Windows antes de merge. No se
ejecutaron publicaciones ni se cambió la versión del producto.
