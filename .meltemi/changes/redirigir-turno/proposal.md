# redirigir-turno

> Vía completa (proposal → design → specs → tasks). Toca la semántica de
> sesión del daemon, el contrato `proto/` y las dos superficies: nace deber de
> paridad §4 y se sirve en la misma change. El design carga las decisiones
> finas (bandera turn-scoped vs. cancel de sesión, evento del log, resultado
> del verbo) antes de escribir una línea.

## Why

Meltemi tiene hoy dos de los tres gestos sobre una sesión en marcha. Se puede
**detener** (`session/cancel`: el turno drena a `Cancelled`, la cola se
cierra, la sesión termina) y se puede **complementar** (`session/direct`: la
instrucción se encola y se despacha al borde del turno). Falta el del medio,
el que Claude Code hace con Esc: **interrumpir lo que el agente está haciendo
y redirigirlo sin perder la sesión** — «no sigas por ahí; haz esto».

No es un hueco casual: es una carrera cerrada a propósito. El registro marca
la cola cancelada al cancelar (`session.rs`, nota de diseño) precisamente
para que una instrucción tardía no se despache a un turno que no va a correr;
y el bucle ACP rompe incondicionalmente cuando el turno termina en
`Cancelled` (`acp.rs`), porque hoy un turno cancelado solo puede significar
que la sesión muere. Imitar la interrupción desde un cliente —cancelar y
luego dirigir— es exactamente la carrera que ese código cierra. La forma
correcta no es esquivar la decisión sino darle un verbo atómico.

El protocolo ya lo permite: la cancelación de ACP es **por turno** — la
notificación cancela el prompt en vuelo y la sesión del agente sobrevive
lista para el siguiente. Es Meltemi quien hoy elige terminar. Y la spec
`own-adapters` ya exige que la cancelación llegue y el turno diga la verdad
en los adaptadores propios, así que el mismo cable sirve sin tocarlos.

## What Changes

- **`session/direct` gana `interrupt` opcional** (booleano, ausente = falso).
  Con la sesión en turno, el daemon **encola primero** la instrucción y
  **después** señala la cancelación del turno en vuelo: una operación
  atómica, sin ventana en la que el borde del turno pueda ver la cola vacía.
  Sobre una sesión terminada, la bandera no aplica (el design fija si se
  ignora con resultado honesto o se rehúsa).
- **El borde del turno aprende a distinguir**: cancel de **sesión** (lo de
  hoy: cola cerrada, bucle roto) de interrupción de **turno** pedida por
  Meltemi con relevo encolado — en ese caso el turno drena a `Cancelled` y el
  bucle despacha la siguiente instrucción en vez de romper. Un `Cancelled`
  espontáneo del agente, sin interrupción nuestra, sigue rompiendo: esa
  prudencia vigente no se toca.
- **Interrumpir mientras espera permiso también funciona**: la petición
  pendiente se resuelve como cancelada (el desenlace existe en el proxy y en
  ACP para exactamente esto) y queda registrada; el ledger de decisiones no
  gana agujeros.
- **El log dice quién interrumpió**: el design decide la forma (evento
  `turn_interrupted` o carga sobre los existentes), con un requisito
  innegociable — un lector del log debe poder distinguir «el agente se
  detuvo» de «el humano lo interrumpió».
- **La GUI lo ofrece donde se escribe**: con la sesión trabajando y texto en
  el compositor, junto a «Encolar» (lo de hoy) aparece «Interrumpir y
  enviar». Sin texto, no hay nada que relevar y no se ofrece. La referencia
  que el mantenedor entregó (2026-08-09: el menú Stop and Send / Add to
  Queue / Steer with Message de un producto vecino, con Enter y ⌥Enter como
  atajos) entra al design como forma candidata — un solo punto de envío con
  destino elegible; «Detener y enviar» como tercer destino se evalúa allí,
  componiendo con el ■ Detener de `compositor-que-trabaja`, no duplicándolo.
- **La TUI gana el mismo gesto** sobre su flujo de dirección existente
  (paridad §4 servida en la change, no prometida); el design fija la tecla.
- **Conformidad**: el schema `session-direct` versiona el parámetro y el
  resultado; el test de conformidad de `meltemi-proto` lo cubre; el
  mock-agent gana el escenario e2e (turno largo interrumpido con relevo) para
  que CI lo ejercite sin agentes reales.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «Interrupción con relevo» — atomicidad del
  encolar-y-señalar, la distinción sesión/turno en el borde, la resolución de
  permisos pendientes, la verdad del log. Si el design encuentra que el
  requisito vigente «Dirección de una sesión existente» debe enmendarse en
  vez de extenderse, lo declara y el delta se vuelve MODIFIED con su texto
  completo.
- `gui-shell`: + requisito «Interrumpir y enviar desde el compositor».
- `tui-shell`: + requisito «Interrumpir y enviar desde la conversación».

## Impact

- Archivos: `core/meltemid/src/session.rs` (bandera turn-scoped y atomicidad
  de cola), `core/meltemid/src/acp.rs` (el borde del turno),
  `core/meltemid/src/server.rs` (el verbo), `proto/schemas/v1/
  session-direct.schema.json` + `meltemi-proto`, `core/mock-agent`
  (escenario), `tui/`, `desktop/ui` (`SessionDetail`).
- Cero dependencias nuevas. Los adaptadores propios no se tocan: reciben la
  misma `CancelNotification` de siempre y su spec ya exige drenarla con la
  verdad.
- **El riesgo real es la carrera** que esta change convierte en semántica:
  interrupción que llega cuando el turno ya terminaba solo, doble
  interrupción, interrupción contra cancel de sesión simultáneo. El design
  las enumera y cada una queda pineada por un test del registro — es
  exactamente la clase de sutileza por la que esta change es vía completa.
- Las sesiones nivel 1 (ACP nativo) y nivel 2 (adaptadores) comparten el
  comportamiento; si un agente concreto no honra la cancelación de turno,
  el turno no drena y el resultado lo dice — no se simula éxito.

## Fuera de alcance

- **Inyección a mitad de turno** (que el agente vea texto nuevo sin cortar
  el turno): ACP no lo transporta; inventar un canal propio exigiría la
  prueba escrita del §6 y no la tenemos.
- **Cambiar `session/cancel`**: detener-del-todo queda exactamente como está,
  confirmación incluida (decisión del mantenedor en `compositor-que-trabaja`).
- **Reanudación automática tras un cancel** («deshacer el detener»): el
  resume manual existe y basta.
- **Colas con prioridad o reordenables**: la cola sigue siendo FIFO; el
  relevo interrumpido no salta posiciones, vacía el turno y el borde
  despacha en orden.
