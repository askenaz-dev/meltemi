## Context

`gui-tauri-paridad` dejó la superficie de escritorio correcta y verificada
(28/28 tareas, paridad como gate, presupuestos medidos). La auditoría de la
superficie con vara de clase mundial encontró una pérdida de datos silenciosa
en el editor, una marca invisible en alto contraste, el trabajo primario sin
acción de primera clase y una paleta funcional pero sin memoria ni formularios.
Esta change es solo experiencia: cero contrato, cero daemon — por eso la matriz
de paridad no cambia y el riesgo es acotado.

## Goals / Non-Goals

**Goals:** que la primera impresión sea la de una herramienta de escritorio
moderna (la vara del mantenedor: Lens, GitKraken) — arquitectura visual de
sidebar + barra de estado, densidad y profundidad reales, identidad de
entidades y superficie de Ajustes; ninguna pérdida silenciosa de trabajo
humano; el flujo `propose` a un clic/tecla; paleta con difusa, grupos,
recientes y formularios tipados desde los schemas; sesiones y transcript al
nivel de las mejores herramientas; tema/ventana persistentes; atención del
SO ante permisos sin foco; presupuestos §12 intactos.
**Non-Goals:** plugin de notificaciones del SO; capacidades nuevas del daemon;
librerías de iconos o de fuzzy; cambiar la cerca `edit-surface`; teclas
configurables (FUERA de la cerca).

## Decisions

### D1 — Formularios tipados generados del contrato, con frescura en CI
Un generador de build (`desktop/ui/scripts/gen-method-forms.mjs`, Node puro,
sin red) lee `proto/schemas/v1/*.schema.json`, resuelve el `$defs` de
`<verbo>Params` por método del registro y emite
`src/lib/generated/method-forms.ts`: campos con nombre, tipo, `required`,
enums y defaults. La paleta renderiza el formulario tipado (marcando los
required) y conserva "JSON crudo" como modo avanzado conmutable. Un gate de
CI regenera y compara (mismo patrón que la frescura de `referencia-cli.md`):
si el contrato cambia y el formulario no, el build falla — los formularios no
pueden mentir. Los métodos sin `Params` en schema caen honestos al modo JSON.

### D2 — Difusa propia, grupos y recientes: memoria local, cero dependencias
Coincidencia por subsecuencia con bonus de prefijo de palabra y de segmento
(`wapp` → `worktree/apply-edit`), implementada en ~40 líneas (el universo son
~40 entradas; una librería sería §10 injustificable). Grupos por dominio del
método (sesión, SDD, worktree, checkpoint, spec, sistema) con encabezados; los
usados recientemente suben primero (frecencia simple: recencia + conteo).

### D3 — Persistencia de UI en el directorio de datos, no en el webview
Un comando Tauri (`ui_state_load`/`ui_state_save`) lee y escribe
`<data_dir>/desktop-ui.json`: tema (claro/oscuro/sistema), geometría y estado
de ventana, última vista, recientes del editor y frecencia de paleta. En el
directorio de datos del usuario —como el flag de onboarding— porque el
almacenamiento del webview es efímero ante reinstalaciones del runtime. El
tema se aplica con `data-theme` en `<html>` y variables del design system; el
selector vive en el onboarding/ayuda junto al de idioma.

### D4 — Atención de permisos con la API de ventana del núcleo
Cuando llega `permission/request`/`permission/changed` con pendientes > 0 y la
ventana no tiene foco: `Window::request_user_attention` (parpadeo de taskbar
en Windows, bounce de dock en macOS, urgencia en Linux — API del núcleo Tauri,
sin plugins ni red) + título `Meltemi (N permisos)`. Al recuperar el foco, la
atención se limpia y el título vuelve. Nada suena ni sale de la máquina; el
comportamiento exacto por SO se documenta en `docs/plataformas.md`.

### D5 — Marca real con fallback honesto
El mark de `brand/meltemi-mark-*.svg` se inyecta inline (asset local) en el
chrome junto al wordmark y, en gris de marca, en los estados vacíos. El
gradiente del wordmark queda detrás de `@media (forced-colors: none)`; en
forced-colors el texto SHALL usar `CanvasText` (color del sistema) — la marca
jamás desaparece. Los glifos de estado vacío dejan el emoji de plataforma
(⛵) por iconos de línea propios (SVG inline, trazo 1.5, geometría del mark),
extendiendo `docs/ux/design-system.md` con la sección de iconografía.

### D6 — Guardia de sucios como superficie modal de primera clase
Un solo componente de decisión (guardar / descartar / cancelar, foco en la
opción no destructiva, Esc cancela) se dispara en: cierre de pestaña sucia,
navegación que desmonta el editor (Esc, dígitos, breadcrumb) y
`onCloseRequested` de la ventana con sucios abiertos. Guardar reutiliza el
flujo `apply-edit` vigente (con su política de bloqueo suave intacta).
Quick-open (Ctrl+P) filtra el árbol ya cargado con la misma difusa de D2;
recientes por proyecto se persisten vía D3.

### D7 — Transcript por tipo de evento, sin inventar datos
Cada `type` del log conocido recibe glifo + tono del design system
(prompt_sent ▸ acento, agent_update · texto, permission_* ● warn, human_edit ✎
info, error ▲ danger, …); los desconocidos caen a neutro con su nombre crudo
(honesto ante eventos futuros). Línea expandible para el texto completo del
payload, timestamps conmutables, copiar línea/todo (clipboard de la webview,
local) y búsqueda en lo cargado. Sin virtualización: los logs se paginan ya
por `session/log`; si un transcript enorme lo exige, se decide con evidencia.

### D8 — Arquitectura de tres zonas, keymap intacto
El shell pasa de tabs superiores a la arquitectura canónica de app de
escritorio: sidebar (proyecto arriba, vistas con contadores, Ajustes abajo),
barra superior (contexto + buscador `Ctrl+K` visible + acción primaria) y
barra de estado (conexión, versión, endpoint, resumen). Referencia visual
committeada: `docs/ux/mockups/shell-clase-mundial.html` — no es spec, es
dirección; la spec manda. El design system gana la escala de elevación
explícita (página `--bg`, superficie `--surface`, flotante `--surface-2` +
sombra) y densidad de tabla (filas 32–36 px, celda 8 px) que las vistas
consumen. El keymap y la paleta no cambian: el sidebar los hace visibles.

### D9 — Identidad de entidades: color estable por hash
Avatar de iniciales por agente con tono derivado determinísticamente del id
(hash → índice en una paleta fija de 8 tonos del design system, AA sobre su
texto): mismo agente, mismo color, siempre, sin estado ni configuración.
Badges de estado y pills de nivel comparten tokens entre vistas — una sola
fuente en el design system extendido (`docs/ux/design-system.md` crece con
secciones de elevación, densidad, avatares y pills).

### D10 — Ajustes: la casa de lo persistente
Vista propia (ítem inferior del sidebar): apariencia (tema D3), idioma,
plantilla "Abrir con…" —que pasa de variable de entorno a ajuste persistido
en `desktop-ui.json`, leída por el comando Rust del deep-link con la env var
como override—, sección de proyecto (configuración efectiva en solo-lectura
con salto al editor trazable) y la declaración sin-cuentas/sin-red/
sin-telemetría (constitución §9 hecha visible). Resuelve la pregunta abierta
del design original: el selector de tema vive aquí, no en el onboarding (el
onboarding conserva idioma, que sí es decisión de primer arranque).

## Risks / Trade-offs

- **Deriva schema↔formulario** → el gate de frescura D1 la hace imposible en
  silencio; el modo JSON crudo sigue siempre disponible como escape.
- **`request_user_attention` difiere por SO** (informativo vs crítico, dock
  vs taskbar) → se acepta y documenta; es señal, no contrato.
- **Crecer la paleta puede tentar a un launcher genérico** → la cerca sigue:
  solo métodos del contrato y navegación de vistas, nada de comandos de
  sistema.
- **Persistencia de ventana con multi-monitor** → restaurar solo si la
  geometría sigue visible; si no, defaults (regla explícita en tasks).
- **Presupuestos §12** → iconos SVG inline y difusa propia no mueven el
  instalador (<15 MB holgado); se re-mide en el QA de release igualmente.

## Migration Plan

Solo superficie y aditivo: sin cambios de contrato ni de daemon. El archivo
`desktop-ui.json` ausente ⇒ defaults actuales (primera ejecución idéntica a
hoy). Reversión: retirar los componentes nuevos; nada persiste fuera del
directorio de datos del usuario.

## Open Questions

- Umbral de frecencia de la paleta (¿cuántos recientes arriba?): se fija en
  implementación con uso real; la spec solo exige que los recientes suban.
- ¿El sidebar colapsa a riel de iconos bajo un ancho umbral? El mockup no lo
  exige; se decide al maquetar contra el suelo de 900 px.
