## 1. Rediseño del shell (arquitectura visual)

- [x] 1.1 Alinear tokens y reglas al design system normativo (`design-system/`): formalizar `--panel`/`--hair`/`--text-faint`, densidad 32/8/16, radios, sombra única, reglas duras; actualizar `docs/ux/design-system.md` y `app.css` _(Req: Densidad y profundidad; design D8, D11)_
- [x] 1.2 Shell de tres zonas: sidebar (proyecto, vistas con contadores, Ajustes), barra superior (contexto + buscador Ctrl+K + acción primaria), barra de estado (conexión, versión, endpoint, resumen) — keymap intacto _(Req: Arquitectura visual; design D8)_
- [x] 1.3 Tablas densas con jerarquía, hover/selección visibles y categorías como pills/dots (Flota, Sesiones, Proyecto) _(Req: Densidad y profundidad)_
- [x] 1.4 Avatares de inicial con color estable por hash + badges de estado unificados _(Req: Identidad de entidades; design D9)_
- [x] 1.5 Panel de detalle (drawer) para Flota y Sesiones con acciones; Esc lo cierra primero _(Req: Panel de detalle)_

## 2. Fundaciones de experiencia

- [x] 2.1 Generador `gen-method-forms.mjs` desde `proto/schemas/v1` → `src/lib/generated/method-forms.ts` + gate de frescura en CI _(Req: Paleta — formularios; design D1)_
- [x] 2.2 Comandos `ui_state_load`/`ui_state_save` (`<data_dir>/desktop-ui.json`) + aplicación de tema `data-theme` sin destello _(Req: Tema y estado persistentes; design D3)_
- [x] 2.3 Iconografía de línea propia (SVG inline, trazo 1.5) + mark de `brand/` en chrome y vacíos + fallback forced-colors del wordmark _(Req: Identidad; design D5)_

## 3. Paleta de clase mundial

- [x] 3.1 Difusa por subsecuencia con bonus de prefijo/segmento + tests unitarios ("wapp" → worktree/apply-edit) _(Req: Paleta — difusa; design D2)_
- [x] 3.2 Grupos por dominio + recientes/frecencia persistidos + hints de teclado en filas _(Req: Paleta — grupos y recientes)_
- [x] 3.3 Formularios tipados con `required` marcados + conmutador a JSON crudo + fallback honesto sin schema _(Req: Paleta — formularios)_

## 4. Acciones primarias y vistas

- [x] 4.1 Lanzador "Nueva sesión" como acción primaria (agente/perfil + modo: explorar/proponer/despachar/dirigir, solo RPCs existentes); `propose` a una tecla en paleta y Proyecto _(Req: La sesión como acción primaria; design D12)_
- [x] 4.2 Estados vacíos accionables: inicializar constitución (Proyecto), refrescar detección (Flota) _(Req: La sesión como acción primaria)_
- [x] 4.5 Bandeja de permisos según el UI kit: estados normal/por vencer/vencida, regla sugerida, y sin animación de layout _(Req: Densidad y profundidad; design D11)_
- [x] 4.3 Sesiones: filtro `/` + orden por columna + chips de resumen por estado _(Req: Sesiones filtrables)_
- [x] 4.4 Sesiones: tiempos relativos localizados (Intl) con absoluto accesible + acciones por fila (cancelar, dirigir) _(Req: Sesiones filtrables)_

## 5. Transcript y avisos

- [x] 5.1 Render por tipo de evento (glifo+tono; desconocidos a neutro) + expansión in situ + timestamps conmutables _(Req: Transcript de primera clase; design D7)_
- [x] 5.2 Copiar línea/todo + búsqueda local con resaltado navegable _(Req: Transcript de primera clase)_
- [x] 5.3 Avisos: timestamps relativos + tope con colapso e historial; banner con "reintentar ahora" y "copiar diagnóstico" _(Req: Avisos con memoria acotada)_

## 6. Editor sin pérdidas

- [x] 6.1 Guardia de sucios (guardar/descartar/cancelar, no destructivo por defecto) en cierre de pestaña y navegación _(Req: Ninguna pérdida silenciosa)_
- [x] 6.2 Retención de `onCloseRequested` con sucios abiertos hasta la decisión _(Req: Ninguna pérdida silenciosa)_
- [x] 6.3 Quick-open Ctrl+P con la difusa de 3.1 + recientes por proyecto persistidos _(Req: Ninguna pérdida silenciosa; design D6)_

## 7. Ajustes, atención y ventana

- [x] 7.1 Vista Ajustes: tema (claro/oscuro/sistema), idioma, plantilla "Abrir con…" persistida (env como override), declaración sin-cuentas/red/telemetría _(Req: Superficie de Ajustes; design D10)_
- [x] 7.2 Ajustes — sección proyecto: configuración efectiva en solo-lectura + salto a editarla en el editor trazable _(Req: Superficie de Ajustes)_
- [x] 7.3 `request_user_attention` + título con contador al llegar/vencer permisos sin foco; limpieza al enfocar; nota por SO en `docs/plataformas.md` _(Req: Atención del sistema; design D4)_
- [x] 7.4 Persistencia/restauración de geometría y última vista con regla de visibilidad multi-monitor _(Req: Tema y estado persistentes)_

## 8. Calidad

- [x] 8.1 i18n ES/EN de toda cadena nueva + lint limpio; a11y del shell nuevo y overlays (foco, Esc, roles, forced-colors) _(constitución §11; Req a11y vigente)_
- [x] 8.2 Tests: difusa, generador de formularios (snapshot), guardia de sucios, color estable por hash, regla de visibilidad de ventana _(spec: escenarios)_
- [ ] 8.3 Gates completos + re-medición de presupuestos §12 en QA (instalador/arranque/RAM sin regresión) _(constitución §7; Req presupuestos vigente)_
