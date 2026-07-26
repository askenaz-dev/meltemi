# espera-humana

> Primera de los tres prerrequisitos de daemon del Agent Boss
> (`enmienda-agent-boss`, meltemi.md v1.4). Vía rápida con design completo:
> los cuatro artefactos de una vez; gate del mantenedor dado el 2026-07-26
> ("ok, adelante" sobre el alcance nombrado en el plan).

## Why

La auditoría del Agent Boss (2026-07-26) verificó que la cola de permisos ya
tiene la arquitectura correcta — vive en el daemon, sobrevive reconexiones,
cualquier cliente decide, la primera resolución gana — y que dos detalles de
la capa de escalado la sabotean exactamente en el caso que más importa: el
operador que no está sentado frente a la máquina.

1. **La denegación instantánea por caída de la conexión dueña.** El push
   `permission/request` viaja a la conexión que inició la sesión; si esa
   conexión murió — un parpadeo del túnel SSH basta —, el fallo del push
   resuelve la petición entera como denegada (`acp.rs:508-512`). La cola
   global, que existe precisamente para sobrevivir reconexiones, nunca llega
   a ejercer: el push la mata primero. Peor: esa denegación le gana por
   first-wins a cualquier otro cliente conectado que iba a aprobar.
2. **El plazo fijo de 120/30 segundos, no configurable.** Cuatro constantes
   hard-coded (`propose.rs:35`, `sdd_flow.rs:31`, `server.rs:1323`,
   `server.rs:1676`); al vencer, default-deny. Nadie contesta desde un
   almuerzo en 120 segundos; el agente recibe un "no" que nadie pronunció.

La constitución no pide nada de esto. §3 dice: "Sin cliente conectado, toda
petición de permiso se deniega" — habla de la ausencia de clientes, no de un
cronómetro ni de la salud de una conexión en particular. Esta change alinea
la implementación con la letra: mientras haya un cliente conectado que pueda
decidir, la petición espera al humano; cuando no queda ninguno (tras una
gracia breve de reconexión que absorbe los parpadeos), la denegación
constitucional aplica — explícita y auditada, nunca por accidente de
transporte.

## What Changes

- **La cola es la única fuente de resolución.** El push vivo
  `permission/request` queda como lo que siempre debió ser: un aviso rápido.
  Su respuesta afirmativa decide (como hoy); su fallo — conexión muerta, sin
  clientes, sin respuesta — **no resuelve nada**. La petición sigue en la
  cola para quien conecte.
- **Política de espera configurable** (`[permissions]` en `config.toml`,
  usuario y proyecto, con diagnósticos de higiene como el resto de la
  config):
  - `wait`: `"while-connected"` (default nuevo para flujos interactivos —
    propose, ciclo SDD, direct) o un entero de segundos para quien prefiera
    cota. Con cota, el vencimiento sigue el camino de hoy: expira visible,
    notifica `permission/timeout`, deniega auditado como `timeout`.
  - `implement-wait`: cota en segundos para turnos autónomos de
    `sdd/implement` (default 30, el valor actual — un pipeline autónomo no
    debe colgarse en silencio; subirla o pasarla a `"while-connected"` es
    decisión del usuario).
  - `no-client-grace`: segundos que una petición sobrevive sin ningún
    cliente conectado antes de la denegación constitucional (default 30 —
    absorbe el parpadeo del túnel sin diluir §3).
- **Registro de clientes conectados** en el daemon (contador + watch): el
  escalado observa cuántos clientes inicializados hay para aplicar la
  política y la gracia. No es un RPC nuevo ni un cambio de contrato: es el
  hecho que el daemon ya conocía y no consultaba.
- **Plazo honesto en el contrato**: `pendingPermission.expiresInSeconds`
  pasa a opcional — una petición bajo `while-connected` no tiene plazo y el
  contrato deja de inventarle uno. TUI y GUI muestran "esperando tu
  decisión" cuando no hay plazo (paridad ×3; la CLI `--json` transporta la
  ausencia tal cual).

## Capabilities

### Modified Capabilities

- `permission-rules`: + requisito «Espera humana» (política, gracia,
  denegación constitucional auditada); ~ «Cola de pendientes consultable»
  (el plazo es opcional: existe cuando la política impone cota).

## Impact

- `proto/`: `PendingPermission.expires_in_seconds` → `Option<i64>`;
  `permission.schema.json` lo saca de `required`; conformance actualizada.
  Cambio aditivo-compatible: los lectores tratan ausencia como "sin plazo".
- `core/meltemid`: `config.rs` (+`[permissions]` con diagnósticos),
  `server.rs` (registro de clientes en el ciclo de conexión; los cuatro
  flujos pasan la política en vez de la constante), `acp.rs` (escalado por
  política; el push ya no resuelve), `pending.rs` (plazo opcional).
- Superficies: TUI (`render.rs`) y GUI (`Permissions.svelte`, `stores.ts`,
  mensajes ES/EN) renderizan la espera sin plazo.
- Tests: unit en `pending.rs` y `config.rs`; e2e en `e2e_permisos.rs` — la
  caída de la conexión dueña no resuelve (otro cliente decide después), la
  espera sin plazo se declara sin plazo, la cota configurada vence auditada,
  y sin clientes la gracia deniega constitucional.
- Sin dependencias nuevas. Los defaults de `implement` no cambian; el
  default interactivo sí (120 s → esperar al humano mientras haya cliente),
  que es exactamente lo que la enmienda ratificó.

## Fuera de alcance

- Estado `waiting_permission` y `gatePending` descubrible: change
  `sesion-esperando`, siguiente del plan.
- Stream de eventos para clientes tardíos y RPC largos asíncronos: change
  `eventos-para-tardios`.
- El aviso de espera opt-in (`remote-access`): la puerta quedó abierta por
  la enmienda; su change llegará con la fase 3 o antes si se pide.
- Cualquier persistencia de la cola a disco (hoy es en-memoria y muere con
  el daemon; una petición no debe sobrevivir a su agente, que muere con él).
