## Context

`meltemi` (crate `tui/`) es un cliente fino sobre `meltemid` vía JSON-RPC. La
regla de despacho de `cli-contract` ya enруta la invocación desnuda con TTY al
modo interactivo, hoy un stub. Esta change construye ese modo: el **shell** de la
TUI. El daemon ya expone todo lo que el shell debe superficiar — `status`
(sesiones y estados), `session/event` (streaming), `permission/request` +
`permission/timeout` (bandeja y vencimientos), `session/cancel` (cancelación
ACP), `shutdown` (apagado limpio) y un registro JSONL por sesión inspeccionable
tras finalizar. La constitución (§11) exige diseñar los textos de usuario para
i18n (ES/EN) desde el inicio.

Alcance: el **shell**, no el interior de cada feature. La UX rica de la bandeja
de permisos (#9), la revisión de specs (#15), el catálogo de flota (#7) y los
worktrees (#16) son changes propias; aquí se reservan sus casas, indicadores y
teclas. Restricciones heredadas: Windows de primera clase (realidad de conhost),
operación por SSH (latencia, 80x24, reflow), binario < 25 MB, arranque < 1 s, sin
telemetría, dependencias mínimas y pineadas.

Esta arquitectura es la síntesis de un panel de diseño de cinco propuestas
independientes evaluadas por tres jueces y endurecida por un crítico de
completitud; las decisiones y sus alternativas se resumen abajo.

## Goals / Non-Goals

**Goals:**
- Un modelo de información y de navegación por teclado coherente y verificable.
- Estados vacíos y onboarding que nunca dejan una pantalla muda ni un callejón.
- Una línea base de accesibilidad que TODA vista (presente y futura) hereda.
- Paridad de núcleo: toda capacidad del daemon alcanzable desde la TUI.
- Casas limpias para #7/#9/#15/#16 sin construir su interior.

**Non-Goals:**
- El interior de cada feature (bandeja, review, flota, worktrees).
- El ciclo de autoría SDD completo (#14) y el editor de specs enriquecido (GUI).
- El visor profundo de sesiones finalizadas (#8): solo se reserva su entrada.

## Decisions

### D1 — Arquitectura "Cabina de mando": chrome persistente + 4 vistas + drill-in + overlays
Un chrome no enfocable (cabecera + pie) enmarca cuatro vistas de primer nivel
(1 Sesiones, 2 Proyecto, 3 Permisos, 4 Flota), un solo nivel de *drill-in*
(Sesiones→Sesión) y una capa de overlays (paleta `:`, ayuda `?`, confirmaciones).
- **Por qué**: ganó el panel en coherencia y ajuste a personas; es el modelo
  probado por herramientas de terminal maduras (una vista a pantalla completa,
  conmutación instantánea), óptimo para SSH (redibujo acotado) y para lectores de
  pantalla (orden de lectura lineal cabecera→cuerpo→pie).
- **Alternativas**: un dashboard multipanel simultáneo (redunda lista/detalle y
  compite por el ancho en 80 columnas); una navegación puramente modal (más
  potente pero con más carga cognitiva y peor descubribilidad).

### D2 — Contrato de consistencia del teclado y conjunto de teclas robusto
Un **único keymap** (dato central) honrado por toda vista: misma tecla = misma
categoría de acción (Enter drill-in, Esc atrás/cerrar, Tab foco de panel, `/`
filtro, `:` paleta, `?` ayuda). **Split**: dígitos 1–4 conmutan vistas desde
cualquier parte; las letras actúan en la vista enfocada. Conjunto **robusto**:
solo letras, dígitos, Esc, Enter, Tab y flechas — sin Alt/Meta (los come SSH),
sin F1–F12 (los interceptan emuladores), sin Ctrl capturados por el TTY. Esc se
desambigua de secuencias CSI por timeout corto y siempre tiene alternativa
`q`/Backspace. Ratón nunca requerido.
- **Por qué**: la realidad de conhost + SSH hace frágiles esas teclas; el keymap
  como dato permite un lint de consistencia y hace la navegación testeable sin
  terminal.

### D3 — Prioridad de señales fijada por el shell; indicadores críticos irrenunciables
El shell fija un orden de prioridad: **daemon-caído > permiso-pendiente >
error/fin-inesperado > streaming**. El estado de conexión y el contador de la
bandeja de permisos son **lo último que se sacrifica** y jamás se descartan al
comprimir el chrome bajo presión de espacio. La desconexión es **ruidosa** porque
el daemon deniega por defecto sin cliente; los vencimientos de permiso ocurridos en
ausencia se superficializan, no se borran en silencio.
- **Por qué**: perder una petición de permiso urgente o una caída silenciosa es
  el peor fallo de un plano de control de seguridad; el shell lo previene por
  construcción. La degradación es automática por espacio (no hay colapso manual de
  chrome en esta change), lo que evita cualquier vía de ocultar los indicadores
  críticos.

### D4 — Accesibilidad como invariante transversal, verificable por lint
Tres reglas que TODA vista hereda: (a) **nunca solo color** — cada estado se
codifica con glifo/forma + etiqueta (el color es decorativo); (b) **`NO_COLOR`**
(cualquier valor no vacío), `--no-color` y `TERM=dumb` → render monocromo sin
pintar fondo (respeta el tema del terminal y ahorra bytes por SSH); (c)
**fallback ASCII** — tabla única de glifos con **gemelo ASCII para cada símbolo**,
regla dura verificable por lint ("ningún glifo Unicode sin gemelo"), sin emoji.
Más: reflow en SIGWINCH sin scroll horizontal del texto esencial, reduce-motion
por defecto sobre SSH, y el **modo CLI scriptable (`--json`) como ruta accesible
garantizada** de último recurso. Toda cadena visible pasa por una tabla de
mensajes ES/EN (constitución §11).
- **Por qué**: el soporte real de lectores de pantalla en terminales es limitado;
  la garantía honesta es la degradación textual + la ruta scriptable.

### D5 — Conexión asíncrona: chrome inmediato, arranque en segundo plano
El shell dibuja el chrome al instante y conecta/arranca el daemon de forma
asíncrona, distinguiendo el estado transitorio `conectando/arrancando…` del fallo
`inalcanzable` (código 10 de `cli-contract`), con reconexión por backoff que
sobrevive a reinicios del daemon y caídas de SSH.
- **Por qué**: presupuesto de arranque < 1 s; nunca una pantalla muda ni un
  cuelgue esperando la conexión.

### D6 — Framework de TUI: inmediato + backend de terminal multiplataforma
Se adopta un framework de TUI de modo inmediato con un backend de terminal
multiplataforma (perfil `ratatui` + `crossterm`), pineado y justificado; es la
**primera dependencia de UI** del proyecto.
- **Por qué**: modo inmediato encaja con un cliente que refleja el estado del
  daemon; el backend soporta conhost de Windows sin dependencia de ncurses;
  comparte el 100% de los tipos con el núcleo Rust sin FFI; binario pequeño
  (presupuesto < 25 MB); ecosistema maduro. Auditado por cargo-deny.
- **Alternativas**: un framework basado en callbacks (más pesado, peor encaje con
  estado externo); TUI a mano sobre secuencias ANSI (coste desproporcionado y
  frágil en Windows).

### D7 — Paridad de núcleo por la paleta como catch-all
La paleta `:` espeja la gramática CLI y expone **toda** capacidad del daemon por
tecleo, aunque aún no tenga vista o tecla dedicada; **todo método RPC nuevo del
daemon debe registrarse** en su autocompletado. Es también la superficie más
amable para lectores de pantalla (lista lineal filtrable).
- **Por qué**: hace la paridad de núcleo verificable y da hogar inmediato a
  capacidades antes de que exista su UI.

### D8 — Reserva global de teclas de acción transversal
El keymap reserva **desde ya** solo las teclas transversales con significado
definido en esta change: `a` (saltar a la bandeja de permisos) y `x` (cancelar la
sesión activa, con confirmación). El arranque del daemon no toma una letra
reservada (ver Open Questions). No se reservan letras sin significado definido.
- **Por qué**: barato ahora, evita romper el contrato de consistencia cuando
  aterricen esas changes; reservar letras sin significado sería a su vez ambiguo e
  inlintable.

### D9 — Testabilidad y entrega por olas
El shell contract completo se especifica de una vez (spec-first), pero la
implementación se entrega en olas: (1) chrome + navegación + esqueleto de
accesibilidad; (2) vistas cableadas al daemon (Sesiones/Sesión, estados vacíos);
(3) endurecimiento (pérdida de daemon en vivo, suelo de tamaño, reconciliación de
vencimientos, control de ciclo de vida, confirmaciones). Palancas de test sin
terminal: keymap como dato (lint de consistencia y de reserva), tabla de glifos
con lint "sin gemelo ASCII → fallo", reductores de estado de vista puros, y
snapshots de accesibilidad (`NO_COLOR`/ASCII/80x24) por plataforma.

## Risks / Trade-offs

- **Superficie grande para una sola change** → olas de implementación explícitas
  (D9); la accesibilidad y la navegación core van primero. Si se prefiere, las
  vistas de endurecimiento podrían separarse a un follow-up, pero se mantienen
  aquí por ser paridad/seguridad y baratas de especificar.
- **Primera dependencia de UI** → framework pineado, uso fino, auditado por
  cargo-deny; la lógica pesada sigue en el daemon.
- **Heterogeneidad de terminales** (conhost, SSH, lectores de pantalla) →
  conjunto de teclas robusto, fallback ASCII, ruta scriptable garantizada, y
  snapshots por plataforma en CI.
- **Complejidad de la prioridad de señales** → orden fijado en el shell y
  cubierto por escenarios; los indicadores críticos son irrenunciables por
  construcción.
- **Colisión de teclas con changes futuras** → reserva global desde ya (D8).

## Migration Plan

Aditivo sobre el crate `tui/`: nuevos módulos del shell; la rama `Interactive` de
`dispatch` deja de imprimir el aviso diferido y lanza el shell (delta MODIFIED de
`cli-contract`). Reversión: revertir la rama `Interactive` al stub y no compilar
los módulos del shell; el modo scriptable no se ve afectado.

## Open Questions

- **Arranque del daemon desde la TUI**: ¿acción explícita de arrancar (con una
  tecla no reservada por decidir, o desde la paleta; "no arrancar procesos por
  sorpresa") o intento automático al entrar (que asume el onboarding)? A decidir en
  la ola 1. No usará `s` (que no está reservada, pero conviene evitar la confusión
  histórica).
- **Ámbito multi-repo**: con sesiones de varios repos, ¿el header fija el ámbito
  del cwd o ofrece un selector de proyecto activo? La columna worktree/rama lo
  mitiga, no lo cierra.
- **Compositor**: Enter-para-enviar vs Enter-para-nueva-línea frente a
  Enter=drill-in global; resolver la tecla de envío del prompt multilínea.
- **Umbrales de reflow**: ancho exacto del colapso a "modo lineal" y orden de
  prioridad de columnas por tabla; fijar con snapshots por breakpoint y plataforma.
- **Fluidez vs ancho de banda**: ¿techo de FPS y nivel de reduce-motion
  configurables o autodetectados por transporte (local vs SSH)?
- **Primer uso multiplataforma**: rutas del directorio de datos en Windows y
  degradación a onboarding no persistente si el directorio no es escribible.
- **Monocromo**: garantizar que foco (borde/título inverso) y selección (gutter
  `>` inverso) sigan distinguibles cuando coexisten sin color.
