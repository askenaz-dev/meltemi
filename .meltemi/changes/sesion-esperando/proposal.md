# sesion-esperando

> Segundo de los tres prerrequisitos de daemon del Agent Boss
> (`enmienda-agent-boss`, meltemi.md v1.4), tras `espera-humana`. Vía rápida
> con design completo; gate del mantenedor dado el 2026-07-26.

## Why

`espera-humana` consiguió que una petición **espere** de verdad al humano en
vez de denegarse sola. Eso vuelve urgente la pregunta que la auditoría del
Agent Boss dejó sin responder: **¿cómo sabe el jefe remoto que algo lo
espera?** Hoy no puede saberlo, por dos huecos verificados:

1. **`waiting_permission` es superficie muerta.** El estado existe en el
   contrato (`SessionState`), la TUI lo pinta (`render.rs:931`) y la GUI le
   dedica color, glifo y filtro (`Sidebar.svelte`, `StatusBadge.svelte`,
   `StatusBar.svelte`) — pero el daemon **jamás lo fija**: las únicas
   llamadas a `set_state` son `Active` y `Ended`. Una sesión bloqueada
   esperando tu decisión se lista exactamente igual que una trabajando a
   toda máquina. Las superficies llevan meses listas para un dato que nunca
   llegó.
2. **Los gates SDD pendientes son indescubribles.** `gate_pending` vive en
   `.cycle-state.json` y solo lo supo quien invocó el verbo, en el
   `SddResult` de su propia llamada. `change/list` — el listado que existe
   precisamente para dar el estado agregado de cada change — reporta
   artefactos, tareas, review y verify, pero no dice que la propuesta lleva
   dos horas esperando tu aprobación. Un cliente que conecta después no
   tiene forma de enterarse por RPC alguno.

El resultado es un plano de control que sabe lo que espera y no lo cuenta.
Esta change lo cuenta, sin inventar RPC nuevos: el estado que ya está en el
contrato pasa a fijarse, y el listado que ya existe pasa a incluir el dato
que ya está en disco.

## What Changes

- **El daemon fija `waiting_permission`**: al escalar una petición al humano,
  la sesión pasa a `waiting_permission`; al resolverse — por decisión, por
  cota vencida o por la denegación constitucional — vuelve a `active`. Con
  varias peticiones simultáneas de la misma sesión, la espera se cuenta:
  vuelve a `active` cuando **ninguna** queda pendiente, nunca antes. El
  estado viaja por `status` y `session/list` como cualquier otro; ninguna
  superficie necesita cambio, porque las tres ya lo saben pintar.
- **`change/list` declara el gate pendiente**: `ChangeInfo` gana
  `gatePending` (booleano) y `gateArtifact` (qué artefacto espera: proposal,
  specs, design, tasks o constitution), leídos del `.cycle-state.json` que el
  ciclo ya persiste. Campo aditivo, sin RPC nuevo, sin escribir nada: el
  listado sigue siendo de solo lectura.
- **Las superficies lo muestran** (paridad ×3): la tabla de changes de la GUI
  y el listado de la TUI marcan la change cuya autoría espera decisión, con
  el artefacto nombrado. Una change sin ciclo activo — la mayoría — no
  declara nada, ni un `false` disfrazado de estado.

## Capabilities

### Modified Capabilities

- `acp-session`: + requisito «Sesión bloqueada por una decisión humana» (el
  estado se fija mientras la petición escala y se restituye al resolverse).
- `method-navigation`: ~ «Listado de changes con estado agregado» (el estado
  agregado incluye el gate pendiente y su artefacto).

## Impact

- `proto/`: `ChangeInfo` + `gate_pending`/`gate_artifact`;
  `change.schema.json` los declara (`gatePending` requerido —
  siempre computable; `gateArtifact` opcional, ausente cuando no hay gate);
  conformance cubre con y sin gate.
- `core/meltemid`: `session.rs` (contador de esperas por sesión y las dos
  transiciones), `acp.rs` (marca y desmarca alrededor del escalado),
  `navigate.rs` (lee `CycleState` al agregar), y el cableado de la registry
  hasta el handler ACP.
- Superficies: `tui/src/run.rs` (columna del listado) y
  `desktop/ui/src/lib/views/Project.svelte` + `stores.ts` + mensajes ES/EN.
- Tests: unit de la registry (anidamiento de esperas), unit de `navigate`
  (gate leído del estado del ciclo, ausencia honesta sin ciclo), y e2e que
  observa `session/list` en `waiting_permission` mientras la petición espera
  y en `active` tras decidir.
- Sin dependencias nuevas. Sin métodos RPC nuevos.

## Fuera de alcance

- `eventos-para-tardios` (stream de eventos para clientes que no iniciaron la
  sesión, y formas asíncronas de los RPC que bloquean el turno entero): el
  tercer prerrequisito, change propia.
- Un RPC agregador tipo «qué me espera» que cruce permisos y gates en una
  sola llamada: con `permission/pending` y `change/list` completos, la
  superficie ya puede componerlo; un método nuevo exigiría probar que ningún
  cliente puede hacerlo por sí mismo.
- Notificar el gate pendiente (push): la puerta del aviso de espera la abrió
  `enmienda-agent-boss` y su mecanismo es change propia.
