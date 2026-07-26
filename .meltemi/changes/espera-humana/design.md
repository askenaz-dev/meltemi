# Design — espera-humana

## Context

`pending.rs` ya es la arquitectura deseada (cola global del daemon, first-wins,
broadcast de cambios). Los defectos viven aguas arriba: `spawn_live_push`
resuelve la cola cuando el push falla, y `escalate` corre contra un `sleep`
fijo. Este design corrige la capa de escalado sin tocar la semántica de
resolución que las superficies ya conocen.

## Decisions

### D1 — La cola resuelve; el push solo avisa
`spawn_live_push` conserva dos comportamientos y pierde uno: una respuesta
bien formada decide (camino rápido de hoy); una respuesta malformada deniega
(el cliente contestó — mal, pero contestó — y un intento explícito no debe
quedar en el limbo); un **fallo de transporte no hace nada** — se registra en
el log operacional y la petición sigue pendiente. La denegación por ausencia
de clientes deja de ser un efecto colateral del transporte y pasa a ser una
decisión explícita del escalado (D3), auditada como `default_deny`.

### D2 — Política de espera por flujo, con defaults deliberados
`WaitPolicy` = `WhileConnected` | `Bounded(segundos)`. Los flujos interactivos
(propose, explore/constitution/propose/plan/gate, direct) toman
`[permissions].wait`, default `while-connected`: el cambio ratificado por la
enmienda. `sdd/implement` toma `implement-wait`, default `Bounded(30)` — el
valor vigente — porque un pipeline autónomo detenido en silencio es peor que
una denegación auditada; quien vigila desde el celular la sube o la pasa a
`while-connected` a sabiendas. La cota vencida conserva el camino actual
completo: `expire` visible, notificación `permission/timeout`, resolución
`timeout` denegada.

### D3 — Registro de clientes y gracia de reconexión
El daemon cuenta conexiones **inicializadas** (el handshake `initialize` es lo
que convierte un socket en un cliente) en un `watch<usize>`. El escalado
espera sobre tres futuros: la resolución de la cola, la cota de la política
(si existe), y el observador de clientes: cuando el contador llega a 0 arranca
la gracia (`no-client-grace`, default 30 s); si expira con el contador aún en
0, la petición se resuelve `default_deny` — la letra de §3 — y así queda en
el log de sesión. Si un cliente conecta durante la gracia, la espera continúa
como si nada. La gracia reconcilia §3 con la realidad de un túnel móvil: un
parpadeo no es "sin cliente", es "cliente reconectando".

### D4 — El plazo del contrato se vuelve honesto
`expiresInSeconds` era obligatorio porque toda petición tenía cronómetro. Bajo
`while-connected` no lo hay, y el contrato no inventa centinelas (`i64::MAX`
sería una estimación disfrazada): el campo pasa a opcional y su ausencia
significa "sin plazo — esperando decisión humana". `waiting_seconds` y
`expired` no cambian. Compatibilidad: es relajación de `required` — los
lectores existentes que asuman presencia se actualizan en esta misma change
(TUI, GUI); externos leerán ausencia como lo que es.

### D5 — Paridad ×3 sin RPC nuevos
Nada de esto añade métodos: `permission/pending`, `permission/decide` y las
notificaciones existentes bastan. TUI y GUI ganan un render ("esperando tu
decisión") y la config es la misma para las tres superficies. El registro de
clientes es interno al daemon.

### D6 — Sin dependencias nuevas
`tokio::sync::watch` ya viene con el runtime pineado. `config.rs` parsea
`[permissions]` con el patrón de diagnósticos existente (valor inválido →
warning con remedio + default, jamás pánico).
