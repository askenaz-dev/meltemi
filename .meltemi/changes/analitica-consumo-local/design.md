## Context

El registro ya existe: cada sesión escribe un log JSONL apend-only por proyecto
(`session_log`) y el índice de sesiones (`session_index`) guarda sus marcas de
inicio y fin por `project_key`. Los eventos que ese log ya contiene son hechos
completos y auditables — inicio, prompts, expansiones, actualizaciones del
agente, permisos pedidos y decididos, fin de turno con su motivo, checkpoints,
commits por tarea, resolución de agente/perfil, ediciones humanas, fin y
errores —, de modo que la contabilidad de actividad no necesita instrumentar
nada nuevo: necesita plegar lo que ya está escrito.

Los tokens son otra cosa. ACP v1.2 no transporta uso, así que para una sesión
pilotada por ACP no existe cifra honesta. Los modos headless oficiales del
nivel 3 sí la emiten en su salida estructurada (`claude -p --output-format
stream-json`, `codex exec --json`), y esa salida ES la interfaz oficial del
agente: leerla es juego limpio (§2), a diferencia de la cuota o la facturación
de la cuenta, que no se toca jamás. `flota-multiproveedor` dejó esto declarado
como futuro condicionado a demanda ("solo cabe contabilidad local de lo que
Meltemi despachó"); se pidió.

Dos hechos del código acotan la change y se declaran de frente: (1) hoy el
daemon rehúsa lanzar sesiones de nivel 3 (`server.rs` responde 2000 ante
`Launch::Headless`), por lo que la costura de captura es el punto de mapeo
`levels::map_headless_line` y quien la ejercita hoy es la suite de
conformidad con `mock-headless`; (2) el panel vive en la navegación del
sidebar que introduce `gui-clase-mundial`, y el design system del mantenedor
(`design-system/`) es la fuente visual normativa.

## Goals / Non-Goals

**Goals:** contabilidad local agregada sobre los registros que ya existen, por
proyecto × agente × perfil × período; captura de tokens únicamente donde la
interfaz oficial los emite; frontera explícita entre medido y no reportado;
declaración de honestidad como requisito de primera clase; método aditivo con
paridad verificada en las tres superficies.
**Non-Goals:** cuota, saldo o facturación de la cuenta del proveedor — jamás
(§2); estimación de tokens por conteo propio de texto (números inventados);
traducir tokens a dinero (exige listas de precios y el plan de la cuenta: no
hay cifra honesta); telemetría o exportación de métricas fuera de la máquina —
jamás (§9); presupuestos y alertas de gasto (fast-follow si la contabilidad
demuestra demanda).

## Decisions

### D1 — Plegado de los registros locales, sin almacén nuevo
La agregación se computa bajo demanda plegando el índice de sesiones y los logs
JSONL del proyecto; el índice actúa de prefiltro (sus marcas de inicio y fin
deciden qué logs toca abrir para el rango pedido) y los logs aportan los
hechos. Alternativas rechazadas: (a) una base de datos embebida — dependencia
nueva sin justificación (§10) y, peor, un segundo almacén que puede discrepar
del log, cuando `session-history` ya zanjó que los logs son la fuente de
verdad; (b) contadores incrementales mantenidos en el momento de escribir —
una cifra que se desincroniza tras una caída a mitad de turno es una mentira
silenciosa. El costo se acota con el prefiltro por período y un límite de
celdas en la respuesta, no con una caché que haya que invalidar.

### D2 — Celdas de proyecto × agente × perfil × período, con cubeta explícita
La unidad de respuesta es la **celda**: `(projectKey, agent, profile, período)`
con su bloque de actividad. El agente de la celda es el binario efectivo que
corrió, tomado del evento de resolución (`agent_resolved.binary`) y, en su
defecto, del programa de `session_started.agentCommand`: se cuenta lo que
realmente se ejecutó, no lo que la configuración prometía. El perfil viene del
mismo evento cuando la resolución fue por perfil; la celda arrastra además el
nivel de integración como atributo (lo necesita el motivo de "sin dato" de
D4), pero el nivel no es dimensión.

El conjunto de métricas de actividad es **cerrado y declarado**: sesiones,
sesiones cerradas y sesiones sin fin registrado, segundos activos (solo de
sesiones con ambas marcas: una sesión abierta no se extrapola), prompts,
turnos por motivo de fin, permisos pedidos/aprobados/denegados/vencidos,
ediciones humanas, commits por tarea, checkpoints y errores. Sumar un hecho
más (p. ej. bytes de contexto inyectados por `refs_expanded`) es un delta
futuro, no una extensión silenciosa.

WHERE un hecho no se puede atribuir, va a una cubeta explícita de **no
atribuido** y no se reparte: el caso real es la edición humana aplicada sin
sesión activa, que `gui-tauri-paridad` D5 registra en el log de ediciones del
proyecto (`.meltemi/edits/events.jsonl`); se cuenta cuando la raíz del
proyecto es alcanzable, y en su propia cubeta. Rechazado: prorratear los
hechos sin dueño entre las celdas atribuidas — inventaría atribución.

Consecuencia asumida: la dimensión de agente se lee como binario, no como
nombre del registro de flota, porque el evento de resolución no lleva el id del
catálogo; la superficie resuelve el nombre legible cruzando con `fleet/list`.

### D3 — Tokens medidos en su propio evento, capturados en la costura del nivel 3
Se añade un tipo de evento de sesión propio (`usage_reported`) con los
contadores que el stream declara, el origen del dato y el modelo cuando el
stream lo nombra. La captura vive en `levels::map_headless_line`, la costura
única por la que pasa toda línea de salida estructurada de nivel 3: reconoce
las claves de uso documentadas de cada modo oficial y las persiste; el resto
de la línea sigue conservándose crudo como hoy. Un contador que el stream no
declara **queda ausente** (campo opcional sin valor), nunca cero: cero y
"desconocido" no pueden colisionar. El evento no lleva credenciales,
cabeceras, cookies ni identificadores de cuenta (§2).

Alternativas rechazadas: consultar APIs de cuota o facturación del proveedor
(prohibido, §2, y además no es lo que Meltemi despachó); leer la configuración
o el estado de sesión del binario para deducir uso (§2); estimar tokens
contando el texto del prompt (número inventado; si algún día se ofrece será
opt-in y rotulado como estimado, según la propia proposal); meter el uso dentro
de `agent_update` (quedaría indistinguible de la prosa del agente y no
agregable sin heurísticas).

### D4 — "No reportado" es un valor de primera clase, jamás un cero
Cada celda lleva `tokens` ausente o medido, más una **cobertura**: sesiones
medidas, sesiones sin dato y el motivo de cada ausencia, con tres valores
estables — el protocolo no transporta uso (sesión ACP), el nivel no ejecuta
proceso (nivel 4), el stream no declaró contadores (corrida de nivel 3 sin
línea de uso). Medido y no reportado nunca comparten cifra: los totales de
tokens suman solo lo medido y declaran sobre cuánta actividad se calcularon.
Las superficies rotulan la ausencia con símbolo + palabra (regla transversal
del design system: el color nunca es el único portador de significado), no con
un `0` que se leería como consumo nulo.

### D5 — `analytics/usage`: un método aditivo, la agregación en el daemon
Método nuevo `analytics/usage` (constante + tipos `Params`/`Result` camelCase
en `meltemi-proto`, `analytics.schema.json` en `proto/schemas/v1/` y casos de
conformidad). Params: raíz de proyecto opcional (ausente = todos los proyectos
con registros en el directorio de datos, que el daemon ya enumera), rango
`since`/`until`, granularidad `day|week|month|total`, filtros de agente y
perfil, y límite de celdas. Result: celdas, totales con su cobertura y la
declaración de D6. Un parámetro inválido (rango invertido, granularidad
desconocida) **rehúsa** con el error de parámetros del contrato; no degrada en
silencio a un período por defecto. Toda la agregación ocurre en el daemon:
ningún cliente lee su disco, y así la contabilidad funciona igual por túnel
SSH que en local.

Paridad §4 desde el día uno: subcomando scriptable `usage` (tabla legible y
`--json` de un objeto, con la referencia CLI regenerada), entrada en la paleta
de la TUI, entrada en el registro tipado de la GUI y fila en
`docs/paridad-nucleo.md` — el gate de `tui/tests/parity.rs` es bloqueante.

### D6 — La declaración de honestidad: dato estructurado del daemon, texto de la superficie
El daemon devuelve la declaración como estructura de claves estables (de qué
registros sale la medición, qué no es visible — cuota, saldo y facturación de
la cuenta —, que nada sale de la máquina y que ninguna cifra es estimada) y
cada superficie la renderiza desde su catálogo de mensajes ES/EN. Rechazado:
que el daemon devuelva prosa localizada — rompería el catálogo único (§11) y
haría del daemon un traductor. La declaración se muestra junto a los números,
sin interacción previa: una nota al pie escondida detrás de un clic no cumple
§9, la cumple una frase visible al lado de la cifra.

### D7 — Panel tabular, sin librería de gráficos
El panel es tabla densa del design system: filas de 32 px, celdas de 8 px,
paneles de 16 px, radios 4/8 sin pills de control, hairlines de 1 px y un solo
nivel de sombra reservado a overlays, con numerales tabulares para que las
cifras no salten al refrescar. Sin dependencia de gráficos (§10): el UI Kit del
mantenedor no define patrón de chart alguno e inventar uno traería peso al
instalador y una segunda gramática visual. Refresco sin animar layout, y la
bandeja de permisos y los banners de señal no se mueven jamás por culpa del
panel (regla dura absoluta del design system).

## Risks / Trade-offs

- **Costo de plegar muchos logs grandes** → prefiltro por período con el
  índice, lectura solo de los logs que solapan el rango y límite de celdas;
  medido en el e2e con logs sintéticos, no prometido.
- **Hoy el nivel 3 no se despacha como sesión** → la captura queda en la
  costura de mapeo y se ejercita con `mock-headless`; en un sistema que solo
  corre ACP el panel dirá "sin datos de tokens" para todo. Es honesto, no está
  roto, y la spec lo dice con esas palabras.
- **Contadores heterogéneos entre proveedores** (nombres y desglose distintos,
  y sujetos a cambio) → se persiste solo lo que el stream declara, con su
  origen; si un modo cambia de forma, el contador se ve ausente y la suite de
  conformidad lo detecta. Nunca se sintetiza un campo faltante.
- **Un panel de números invita a leerlos como cuota** → por eso la declaración
  de honestidad es requisito con escenario, no nota al pie.
- **Dimensión de agente por binario, no por id del catálogo** → nombre legible
  resuelto por la superficie contra `fleet/list`; el alta del id en el evento
  de resolución queda como pregunta abierta.
- **Windows** → las claves de proyecto y de árbol ya están normalizadas
  (`project_key`, `tree_key`); el plegado y sus tests corren en las tres
  plataformas.

## Migration Plan

Aditivo por completo: un método nuevo, un tipo de evento nuevo y una vista
nueva. El envelope del evento de sesión conserva su versión: los lectores
existentes saltan las líneas que no parsean (así pliega ya el índice), de modo
que un binario anterior leyendo un log nuevo ignora el evento de uso sin
corromper nada. Los logs previos agregan actividad y declaran sus tokens como
no reportados: no hay backfill — inventar historia de uso sería exactamente lo
que esta change prohíbe. Reversión: retirar el método, su entrada en los
registros y la vista; los logs quedan tal cual, con o sin eventos de uso.

## Open Questions

- ¿Añadir el id del catálogo al evento de resolución de agente para que la
  dimensión muestre el nombre del registro sin cruzar con `fleet/list`? Delta
  menor de `fleet-catalog`, fuera de esta change.
- Nombres exactos de las claves de uso de cada modo oficial: se fijan con la
  salida real en la mano en la tarea de captura; hasta entonces el mapeo
  reconoce las claves documentadas y deja constancia del origen.
- ¿Presupuestos y alertas? Solo si la contabilidad demuestra demanda, como
  declara la propia proposal.
