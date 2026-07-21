## Context

Fase 1 cerró con CLI y TUI en paridad sobre ~22 áreas de método del contrato
(`proto/schemas/v1/`) y 30 changes archivadas. La GUI es la segunda superficie
prometida (constitución §4; meltemi.md D4 §7.4) y su design tiene una deuda
nominal que saldar: la spec `edit-surface` remite aquí la política de
concurrencia humano↔agente. Los insumos declarados en el plan
(`docs/ux/design-system.md`, `docs/paridad-nucleo.md`) no existen: esta change
los produce.

## Goals / Non-Goals

**Goals:** superficie de escritorio con paridad verificable por CI; política de
concurrencia humano↔agente resuelta; edición utilitaria in situ trazable vía
daemon; presupuestos de huella del fundacional cumplidos; design system y
matriz de paridad como artefactos vivos.
**Non-Goals:** motor propio, sandbox, hooks, plugins, mobile (changes/fases
propias); empaquetar servidores LSP; reemplazar el editor del usuario (la cerca
`edit-surface` sigue intacta); transporte de red en cliente o daemon — jamás.

## Decisions

### D1 — Toda la conexión en el backend Rust; la webview solo pinta
El proceso Rust de Tauri reutiliza el cliente JSON-RPC del proyecto — los
módulos `rpc`/`transport`/`bootstrap`/`paths` que vivían dentro de `meltemid`
(y que la TUI consumía re-exportados), extraídos al crate compartido
`core/meltemi-client`, refactor sin cambio de contrato; `meltemid` los
re-exporta para sus internos — y habla con el daemon por el socket local,
igual que la TUI. La
webview consume exclusivamente comandos/eventos del IPC de Tauri: no abre
sockets, no hace fetch remoto; CSP estricta sin orígenes remotos y capacidades
Tauri mínimas declaradas (deny-by-default, coherente con constitución §3 y el
modelo de seguridad de Tauri). Un solo dueño de la conexión da reconexión,
backoff y prioridad de señales idénticas a las de la TUI sin duplicar lógica.

### D2 — Frontend: Svelte 5 + TypeScript, pineados
Framework compilado sin runtime pesado: huella mínima al servicio de los
presupuestos, buen soporte cross-webview (WebView2 / WKWebView / WebKitGTK) y
a11y e i18n maduros. Alternativas evaluadas: React (árbol de dependencias y
huella mayores sin ganancia para una superficie fina), Leptos/WASM (un solo
lenguaje en todo el producto — tentador — pero payload WASM y tooling de a11y
e i18n en webview aún verdes; se reevalúa si su ecosistema madura). Deps del
frontend pineadas por lockfile y auditadas en CI como las de Rust (constitución
§10); Vite como bundler (default de Tauri, sin config exótica).

### D3 — Paridad verificable: registro tipado + matriz como gate de CI
Como la paleta de la TUI, la GUI mantiene un registro tipado método RPC →
acción de paleta. `docs/paridad-nucleo.md` es la matriz viva capacidad → RPC →
CLI/TUI → GUI, y un check de CI (script sobre `proto/schemas/v1/`) falla si un
método del contrato carece de entrada en el registro de la TUI o en el de la
GUI. Convierte la constitución §4 de promesa en gate, exactamente lo que el
hito v1.0 exige ("paridad de núcleo verificada por CI").

### D4 — Concurrencia humano↔agente: bloqueo suave, jamás bloqueo duro
El daemon ya conoce las sesiones activas y su worktree; se añade el estado
observable "turno en vuelo". Política en tres niveles: (a) **turno en vuelo** →
el guardado exige confirmación reforzada que advierte el riesgo de conflicto;
(b) **sesión activa sin turno en vuelo** → advertencia y confirmación simple;
(c) **worktree libre** → guardado sin fricción. En todos los casos la escritura
pasa por el daemon y queda como `human_edit`; el daemon antepone al siguiente
turno del agente una nota con los archivos editados por el humano desde su
último turno — viaja en el prompt del turno, porque ACP no define notificación
push de ediciones externas y no inventamos protocolo (constitución §6). Nunca
bloqueo duro: el humano siempre decide; la honestidad la dan el registro y la
nota, y el riesgo residual lo acota el checkpoint por tarea existente.

### D5 — Edición in situ: un método, una traza
`worktree/apply-edit` (aditivo en `proto/`): ruta + contenido o hunk + sesión
asociada opcional → validación de que la ruta cae dentro del worktree →
escritura + evento `human_edit` (archivo, sesión, marca temporal) en el JSONL.
Toda superficie escribe por aquí (requisito vigente de `edit-surface`); paridad
de poder desde el día uno: subcomando CLI y registro en la paleta TUI. La
inteligencia LSP es experiencia de superficie, no capacidad del daemon: la GUI
habla LSP con servidores que el usuario ya tiene instalados o configura
(BYO-LSP), degradando a resaltado sintáctico si faltan — no se empaquetan
servidores (presupuesto de instalador y §10).

### D6 — Design system e i18n compartidos
`docs/ux/design-system.md` deriva tokens de brand V2 (tipografía, color,
espaciado, densidad) y fija la regla transversal que la TUI ya honra: todo
estado se codifica con símbolo + palabra, nunca solo color. Catálogo de
mensajes ES/EN como fuente única con lint que rehúsa cadenas hardcodeadas en la
webview; el formato converge con la tabla de mensajes de la TUI donde sea
práctico. Los tokens quedan pensados para reutilizarse en el compañero móvil
(fase 3) sin rediseño.

### D7 — Distribución: bundler de Tauri dentro del pipeline existente
MSI en Windows, DMG en macOS, AppImage + deb en Linux, firmados con la custodia
documentada de `release-distribution` y sus mismos gates. El runtime de webview
del SO se aprovecha (WKWebView, WebKitGTK) o se bootstrapea (WebView2), nunca
se embebe: así el instalador respeta < 15 MB, y el pipeline mide el tamaño como
gate bloqueante.

## Risks / Trade-offs

- **Webviews distintos por SO** → riesgo ya asumido en meltemi.md §11: CI en
  las tres plataformas, CSS conservador fijado por el design system, polyfills
  puntuales.
- **E2e de GUI frágiles** → la lógica vive en el backend Rust (testeable como
  la TUI, con mock-agent y fixtures); tauri-driver solo para smoke donde exista
  driver; donde no (macOS), verificación manual documentada por escenario.
- **RAM < 80 MB depende del webview del SO** → se mide y publica por release en
  QA, no como test bloqueante; el gate duro de CI es el tamaño del instalador.
- **La nota-al-siguiente-turno no interrumpe un turno en vuelo** → aceptado:
  interrumpir sería inventar semántica ACP; mitigado por la confirmación
  reforzada y los checkpoints por tarea.
- **Registro de paleta GUI desactualizado rompería la paridad en silencio** →
  por eso el check D3 es bloqueante en CI, no advertencia.

## Migration Plan

Aditivo por completo. El workspace suma `desktop/` y el crate cliente
compartido (la TUI migra a consumirlo sin cambio de contrato observable);
`proto/` solo suma un método. Reversión: retirar el miembro del workspace y el
método aditivo; el daemon y las superficies existentes no pierden nada.

## Open Questions

- Catálogo de mensajes ES/EN: ¿crate compartido o JSON generado desde una
  fuente única? Se decide en la tarea de i18n con la fricción real en la mano.
- Firma de MSI/DMG: ¿la custodia ya dispone de certificados de firma de código
  por plataforma? Si no, el gate queda documentado como pendiente de
  infraestructura, no de código.
