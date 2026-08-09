# sesion-que-espera

> Vía completa (proposal → design → specs → tasks). Toca el corazón del ciclo
> de vida de sesión del daemon (el borde del turno), la forma del arranque en
> el contrato y las dos superficies. El design carga las decisiones gruesas
> —desacople del RPC, política de idle, frontera con el resume— antes de una
> línea. Aporta además, para `session/start`, la evidencia análoga a la que
> `eventos-para-tardios` exigió antes de dar forma asíncrona a los RPC que
> bloquean el turno entero.

## Why

El mantenedor lo mostró con una captura: terminó el turno y el compositor
quedó muerto — «session ended and not resumable». Quiere lo que Claude Code,
Codex y Copilot dan por sentado: **seguir iterando en el mismo chat**, con la
sesión esperando su próximo mensaje.

La causa está localizada, no es un misterio. El bucle de sesión corre turnos
«mientras la cola tenga trabajo»: en el borde del turno, `take_or_close()`
encuentra la cola vacía, la cierra atómicamente («once the loop finds the
queue empty at a turn boundary … it stops accepting»,
`core/meltemid/src/session.rs:22-26, 88-101`) y el bucle rompe
(`core/meltemid/src/acp.rs:313-319`). Al salir, el crate ACP **mata el
subproceso del agente al drop** (`acp.rs:7-8`) y `finalize_ok` emite
`session_ended` y deregistra la sesión
(`core/meltemid/src/session_finalize.rs:65-100`). No existe ningún estado
«esperando instrucciones»: la única ventana para encolar es mientras el turno
está en vuelo.

Lo que hoy se parece a persistencia es **auto-resume**: `session/direct`
sobre una sesión terminada crea una sesión **nueva** encadenada por
`resumed_from` + `session/load` de ACP (`core/meltemid/src/server.rs:1974,
2027-2132`), y la GUI ya lo ofrece con el botón «Reanudar»
(`SessionDetail.svelte:788-793`). Tiene dos costes que la captura hizo
visibles: exige que el agente declare `loadSession` (`session_index.rs:71-73`;
si no, error 2004 duro — mock-agent solo lo declara con `--load-session`,
`core/mock-agent/src/main.rs:34,45`) y relanza el subproceso entero por cada
mensaje.

Y hay una razón estructural que explica por qué nadie lo arregló de pasada:
**toda la sesión vive dentro de una sola petición RPC** — `session/start` no
responde hasta que la sesión completa termina
(`core/meltemid/src/free_session.rs:259-317`; despacho en
`server.rs:313-315`). Una sesión que espera indefinidamente no puede vivir
dentro de una request que debe responder. `eventos-para-tardios` ya puso los
eventos en streaming y difirió las formas asíncronas de los RPC que bloquean
el turno entero (`sdd/gate`, `sdd/review-decide`, `worktree/dispatch`) con su
razón escrita — «entra como change propia si la evidencia de uso lo pide, con
su prueba escrita». `session/start` no estaba en esa lista, pero comparte
exactamente la forma bloqueante: esta change aporta esa evidencia para él, y
trae su prueba.

## What Changes

- **El borde del turno aprende a esperar**: con la cola vacía, la sesión ya
  no cierra — queda en espera (señal, sin busy-wait) con la conexión ACP y el
  subproceso vivos; encolar despierta el bucle. `session/cancel` conserva su
  semántica de terminar de verdad, y la interrupción con relevo de
  `redirigir-turno` compone sin cambios: ambas operan sobre el mismo borde
  que esta change reforma, por eso van adyacentes en el orden.
- **El arranque se desacopla de la vida de la sesión**: forma exacta en el
  design — candidatas: parámetro aditivo de `session/start` que responde
  temprano con el id y deja el stream de eventos como verdad (los clientes ya
  viven de él), o verbo nuevo. Regla innegociable: **aditivo**; quien hoy
  espera el resultado final sigue recibiéndolo igual.
- **Política de idle explícita**: cuánto tiempo y cuántas sesiones pueden
  esperar con subproceso vivo, configurable con defaults conservadores. Al
  vencer, finalize honesto con `reason` que dice *idle*, jamás `completed`
  fingido — y desde ahí aplica el resume de siempre. Hoy **nada** gobierna
  esa acumulación porque el estado no existe; nace gobernado.
- **El finalize se difiere al fin verdadero**: cancel, idle vencido, error, o
  apagado del daemon con drenado ordenado (el kill de huérfanos existente no
  se debilita).
- **Las superficies dicen el estado nuevo**: pestaña/lista/tablero muestran
  «esperando instrucciones» con glifo+palabra (no es *trabajando* — el anillo
  de `compositor-que-trabaja` NO gira en espera, su regla de luz honesta se
  extiende a este estado); el compositor queda vivo en GUI y TUI; CLI por
  `session/direct` como hoy. Paridad ×3 servida en la change.
- **El resume no muere**: sesiones terminadas (histórico, pre-change, idle
  vencido) conservan el camino actual. La spec distingue por primera vez
  *viva-esperando* de *terminada-resumable*, y la GUI deja de necesitar el
  rótulo «Reanudar» para la conversación normal.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + la espera en el borde del turno, la política de idle con
  su finalize honesto, y el arranque desacoplado. Si el requisito vigente del
  ciclo de vida pinea textualmente el cierre en cola vacía, ese delta se
  declara MODIFIED con su texto completo — el design lo verifica contra la
  verdad viva antes de escribir.
- `gui-shell` / `tui-shell`: + el estado «esperando instrucciones» y el
  compositor vivo tras el turno.
- `cli-contract`: + solo si el design elige flag o verbo nuevo para el
  arranque desacoplado.

## Impact

- Archivos: `core/meltemid/src/{session.rs, acp.rs, free_session.rs,
  session_finalize.rs, server.rs}`, `proto/` (parámetro/campos aditivos),
  `tui/`, `desktop/ui` (`SessionDetail`, tira), matriz de paridad, mock-agent
  (escenario e2e de espera + despertar + idle).
- Cero dependencias nuevas.
- **Riesgo mayor, dicho**: subprocesos idle acumulados son RAM y procesos
  reales del proveedor; la política de idle es la mitigación y el QA mide el
  reposo con N sesiones esperando. Segundo riesgo: el crate ACP mata al drop
  — mantener vivo es mantener la `connection` en scope, no pelear contra el
  crate; el design lo pinea con un test.
- Interacción declarada con `redirigir-turno`: mismo borde, cambios
  compuestos; el orden de abordaje las pone adyacentes para no reabrir ese
  código dos veces.

## Fuera de alcance

- **Inyección a mitad de turno sin interrumpir**: ACP no la transporta; ya
  está rechazada por escrito en `redirigir-turno`.
- **Sobrevivir reinicios del daemon** con el subproceso vivo: el resume
  existente es la respuesta a esa caída; no se promete lo imposible.
- **Pool o precalentamiento de agentes**: optimización futura con evidencia.
- **Cambiar la semántica de `session/cancel`**: detener sigue deteniendo,
  con su confirmación.
