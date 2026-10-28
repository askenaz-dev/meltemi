# Design — redirigir-turno

## Context

Verificado el 2026-08-10. La carrera que esta change quiere abrir está **cerrada
a propósito**, y el código dice dónde:

- `InstructionQueue` guarda `items`, `accepting` y `cancelled` **bajo el mismo
  lock** (`session.rs:28-40`). Ese lock es el punto de atomicidad que la change
  necesita: no hay que inventarlo.
- `enqueue` escribe `InstructionQueued` **con el lock tomado** (`:71-76`) —
  log-before-enqueue, ya resuelto.
- El borde del turno rompe con una **doble condición**:
  `cancelled.load(…) || matches!(status, TurnStatus::Cancelled)`
  (`acp.rs:307-319`). Hoy toda cancelación mata el bucle, incluida la que esta
  change quiere que solo corte el turno.
- `SessionRegistry::cancel` **señala primero y marca la cola después**
  (`session.rs:259-277`). La interrupción necesita exactamente el orden
  inverso.

Y tres hallazgos que la propuesta no conocía:

1. **Nada resuelve hoy un permiso en vuelo al cancelar.** La espera hace
   `select!` de tres ramas —resolución, vencimiento, deny constitucional— y
   **no hay rama de cancelación** (`acp.rs:496-516`). `PendingQueue::drop_request`
   existe, está documentado «session cancelled» y **no tiene un solo llamador
   en el repositorio** (`pending.rs:250-256`). Es el hueco exacto que la
   propuesta prometía cubrir.
2. **El mock no honra `CancelNotification`**: `--turn-delay-ms` mantiene el
   turno abierto y una cancelación no lo acorta. Un e2e de «interrumpir y
   relevar» necesita que el mock aprenda a reaccionar, o probaría el reloj en
   vez de la interrupción.
3. **Hay un guardián que esta change debe enmendar**:
   `a_cancelled_queue_dispatches_nothing_even_with_items` (`session.rs:416-435`)
   pinea que una instrucción ya encolada **no** se despacha tras
   `mark_cancelled`. Es exactamente la conducta que `interrupt` invierte —para
   su propia instrucción y solo para ella.

## Goals / Non-Goals

**Goals**: el gesto del medio —interrumpir lo que el agente hace y redirigirlo
sin perder la sesión—, atómico, con el log diciendo quién interrumpió.

**Non-Goals**: inyectar texto sin cortar el turno (ACP no lo transporta);
cambiar `session/cancel`; colas con prioridad; reanudación automática.

## Decisions

### D1 — Encolar y señalar bajo el mismo lock, en ese orden

`interrupt` no es «cancelar y luego dirigir»: es una operación que toma el lock
de la cola, **encola el relevo** y **marca la interrupción**, y lo suelta. Que
ambos flags vivan ya bajo ese lock es lo que hace posible no dejar ventana: el
borde del turno no puede observar la cola vacía entre las dos mitades.

Invierte el orden de `cancel` (señalar→marcar) por la razón que lo hace
correcto: en una cancelación no queda nada que despachar, así que el orden da
igual; en una interrupción **sí queda**, y señalar antes de encolar es
precisamente la ventana que el bucle aprovecharía para romper.

### D2 — El borde distingue por bandera, no por el estado del turno

`TurnStatus::Cancelled` seguirá llegando igual: es lo que el agente reporta
cuando drena. Lo que cambia es que la cola sepa **por qué**. Una bandera
turn-scoped —`interrupted`, junto a `cancelled` y bajo el mismo lock— hace que
el borde lea:

- `cancelled` → lo de hoy: cerrar la cola, romper el bucle.
- `interrupted` con relevo encolado → **consumir el relevo y seguir**,
  limpiando la bandera para el turno siguiente.
- `Cancelled` espontáneo, sin bandera nuestra → romper, como hoy. **Esa
  prudencia no se toca**: un agente que se cancela solo no es una invitación a
  seguir mandándole trabajo.

### D3 — El permiso en vuelo se resuelve como cancelado, con el llamador que faltaba

La espera de permisos gana su rama de cancelación y llama a `drop_request`, que
lleva escrito desde el principio para qué era y nunca se usó. El desenlace es
`PermissionOutcome::Cancelled`, que ya **cuenta como denegación** en el ledger
(`pending.rs:194-221`): interrumpir no crea un agujero en la auditoría.

### D4 — El log distingue quién paró

Requisito innegociable: un lector del registro debe poder separar «el agente se
detuvo» de «el humano lo interrumpió». La forma —evento propio o carga sobre
los existentes— la decide la implementación, pero sin esa distinción el
histórico miente sobre quién tomó la decisión.

### D5 — El guardián se enmienda, y su intención se conserva

`a_cancelled_queue_dispatches_nothing_even_with_items` **sigue siendo cierto
para una cancelación**. Lo que se añade es su gemelo: una cola interrumpida
**sí** despacha el relevo que la interrupción encoló, y solo ese. Se escriben
los dos juntos para que la diferencia quede a la vista de quien lea.

### D6 — El mock aprende a reaccionar, detrás de su bandera

Como `--think`, una bandera nueva —el mock honra `CancelNotification` cortando
su espera— **apagada por defecto**, para no cambiar lo que leen los e2e de
cancelación que ya existen (`e2e_control_remoto.rs:380`,
`e2e_adaptadores_cancel.rs:35`). Sin eso, un e2e de interrupción mediría el
reloj y no la interrupción.

## Risks / Trade-offs

- **Es una carrera lo que se abre.** El design enumera las tres que importan y
  cada una lleva su test: interrupción que llega cuando el turno ya terminaba
  solo; dos interrupciones seguidas; interrupción contra `session/cancel`
  simultáneo. La última tiene respuesta escrita: **cancelar gana** — es el
  gesto más fuerte y el único irreversible.
- Un agente que no honre la cancelación de turno no drenará: el resultado lo
  dirá y **no se simulará éxito**. La spec de `own-adapters` ya exige que la
  cancelación llegue y el turno diga la verdad.

## Migration / Rollout

Campo aditivo en `session/direct` (params y schema), bandera nueva en la cola,
una rama en la espera de permisos, el mock con su bandera, y las dos
superficies. El contrato crece; nada existente cambia de forma.
