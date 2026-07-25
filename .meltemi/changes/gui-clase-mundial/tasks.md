## 1. Fundaciones de experiencia

- [ ] 1.1 Generador `gen-method-forms.mjs` desde `proto/schemas/v1` → `src/lib/generated/method-forms.ts` + gate de frescura en CI _(Req: Paleta — formularios; design D1)_
- [ ] 1.2 Comandos `ui_state_load`/`ui_state_save` (`<data_dir>/desktop-ui.json`) + aplicación de tema `data-theme` sin destello _(Req: Tema y estado persistentes; design D3)_
- [ ] 1.3 Iconografía de línea propia (SVG inline, trazo 1.5) + mark de `brand/` en chrome y vacíos + fallback forced-colors del wordmark; extensión de `docs/ux/design-system.md` _(Req: Identidad; design D5)_

## 2. Paleta de clase mundial

- [ ] 2.1 Difusa por subsecuencia con bonus de prefijo/segmento + tests unitarios ("wapp" → worktree/apply-edit) _(Req: Paleta — difusa; design D2)_
- [ ] 2.2 Grupos por dominio + recientes/frecencia persistidos + hints de teclado en filas _(Req: Paleta — grupos y recientes)_
- [ ] 2.3 Formularios tipados con `required` marcados + conmutador a JSON crudo + fallback honesto sin schema _(Req: Paleta — formularios)_

## 3. Acciones primarias y vistas

- [ ] 3.1 "Proponer un cambio" en el chrome (botón + atajo) abriendo el formulario de `propose` _(Req: Acciones primarias)_
- [ ] 3.2 Estados vacíos accionables: inicializar constitución (Proyecto), refrescar detección (Flota) _(Req: Acciones primarias)_
- [ ] 3.3 Sesiones: filtro `/` + orden por columna + chips de resumen por estado _(Req: Sesiones filtrables)_
- [ ] 3.4 Sesiones: tiempos relativos localizados (Intl) con absoluto accesible + acciones por fila (cancelar, dirigir) _(Req: Sesiones filtrables)_

## 4. Transcript y avisos

- [ ] 4.1 Render por tipo de evento (glifo+tono; desconocidos a neutro) + expansión in situ + timestamps conmutables _(Req: Transcript de primera clase; design D7)_
- [ ] 4.2 Copiar línea/todo + búsqueda local con resaltado navegable _(Req: Transcript de primera clase)_
- [ ] 4.3 Avisos: timestamps relativos + tope con colapso e historial; banner con "reintentar ahora" y "copiar diagnóstico" _(Req: Avisos con memoria acotada)_

## 5. Editor sin pérdidas

- [ ] 5.1 Guardia de sucios (guardar/descartar/cancelar, no destructivo por defecto) en cierre de pestaña y navegación _(Req: Ninguna pérdida silenciosa)_
- [ ] 5.2 Retención de `onCloseRequested` con sucios abiertos hasta la decisión _(Req: Ninguna pérdida silenciosa)_
- [ ] 5.3 Quick-open Ctrl+P con la difusa de 2.1 + recientes por proyecto persistidos _(Req: Ninguna pérdida silenciosa; design D6)_

## 6. Atención y persistencia de ventana

- [ ] 6.1 `request_user_attention` + título con contador al llegar/vencer permisos sin foco; limpieza al enfocar; nota por SO en `docs/plataformas.md` _(Req: Atención del sistema; design D4)_
- [ ] 6.2 Persistencia/restauración de geometría y última vista con regla de visibilidad multi-monitor + chip visible del atajo de paleta _(Req: Tema y estado persistentes)_
- [ ] 6.3 Selector de tema claro/oscuro/sistema (persistido vía 1.2) _(Req: Tema y estado persistentes)_

## 7. Calidad

- [ ] 7.1 i18n ES/EN de toda cadena nueva + lint limpio; a11y de los overlays nuevos (foco atrapado, Esc, roles) _(constitución §11; Req a11y vigente)_
- [ ] 7.2 Tests: difusa, generador de formularios (snapshot de un método), guardia de sucios (lógica), estado de ventana (regla de visibilidad) _(spec: escenarios)_
- [ ] 7.3 Gates completos + re-medición de presupuestos §12 en QA (instalador/arranque/RAM sin regresión) _(constitución §7; Req presupuestos vigente)_
