# Design — barra-de-estado-agentica

## Context

Verificado el 2026-08-10. `StatusBar.svelte` no recibe props: se auto-suscribe
a `conn`, `pending` y `sessions` (`:3-5`), y `App.svelte:545` la monta una vez
fuera de `<main>`. Muestra conexión + versión + endpoint a la izquierda
(`:17-26`) y «N en curso» + permisos a la derecha (`:28-33`), con el endpoint
como único truncado (40ch, `:50-56`).

Lo que el shell **ya tiene** sin pedir nada nuevo: `activeProject`, `sessions`
(ya acotado al proyecto), `allSessions`, `projects`, `pending` y `fleet`
(`stores.ts:147-155`). Falta exactamente lo que esta change quiere añadir.

**Dos hallazgos que corrigen la propuesta**, y conviene decirlos antes que
nada:

1. **«Esperando gate» no es un estado de sesión.** El contrato tiene
   `starting | active | waiting_permission | ended | interrupted`
   (`session-list.schema.json:9-12`) y nada más. El gate pertenece a la
   **change** (`gatePending`/`gateArtifact` en `change.schema.json:39-40`), no
   a la sesión. La propuesta pedía un desglose de sesiones en tres, y el
   tercero no existe ni debe inventarse.
2. **`analytics/usage` no filtra por sesión.** Sus params son `projectRoot`,
   `since`, `until`, `granularity`, `agent`, `profile`, `limit`
   (`analytics.schema.json:8-35`) y la celda agrega por proyecto/agente/periodo
   (`:120-147`). «El consumo de las sesiones activas» no es derivable hoy sin
   un campo aditivo — que la propia propuesta marcaba como condicional.

Y una frontera que ya existe y hay que respetar: `coverage` con
`unreportedReason.kind` (`:96-119`), donde los niveles 1 y 2 —casi toda la
flota— caen en `protocol_carries_no_usage` porque **ACP no transporta usage**
(`analytics.rs:300-309`).

Además: no hay store global de changes (`Project.svelte:21` lo guarda local), y
`LiveData` de la TUI no conoce change, gate ni consumo (`live.rs:227-259`).

## Goals / Non-Goals

**Goals**: que la barra responda «¿sobre qué trabajo y qué espera de mí?» sin
cambiar de vista; que lo que diga del consumo sea verdad o calle.

**Non-Goals**: señales de editor (Ln/Col, encoding, EOL) — el rumbo excluye ser
editor de propósito general; configurar qué segmentos se ven; la extensión ACP
de usage.

## Decisions

### D1 — Los segmentos, y su orden

`proyecto · change + gate` a la izquierda tras la conexión; `consumo`,
`sesiones`, `permisos` a la derecha. El orden va del contexto (dónde estoy) a
la demanda (qué espera de mí), que es el orden en que se lee una barra.

El proyecto usa `projectName(root)` (`tree.ts:39-42`), que ya calcula el
segmento final, con la ruta completa en el `title` — el patrón que el sidebar y
el Home ya usan.

### D2 — El gate es de la change, y el desglose de sesiones tiene dos estados, no tres

Se corrige la propuesta con lo que el contrato dice:

- **Sesiones**: «N en curso» se desglosa en **activas** y **esperando
  permiso**, que son los dos estados que existen y que significan cosas
  distintas para el humano (una trabaja, la otra le espera).
- **Gate**: segmento propio, de la change, con la forma «change · gate:
  artefacto». No se disfraza de estado de sesión.

Cuál es la change «activa»: `change/list` no lo declara. Criterio escrito: **si
alguna tiene `gatePending`, esa es la que la barra nombra** —es la accionable—;
si ninguna, la barra dice cuántas hay activas. Un gate esperando es lo único
que la barra necesita empujar hacia adelante.

### D3 — El consumo es el del proyecto en el día, no el de las sesiones activas

La propuesta pedía el consumo de las sesiones activas; el contrato no lo
permite y esta change **no añade un campo para conseguirlo**. En su lugar,
el segmento muestra el consumo **medido del proyecto en el día**, que
`analytics/usage` sí devuelve tal cual (`granularity: day`, `projectRoot`).

Es además lo que sirve al propósito: administrar cuotas es una pregunta por
día, no por sesión. Y cuando el consumo no está medido, la barra **no muestra
un cero**: calla o dice «no reportado» con la razón estable que `coverage` ya
entrega. Un cero sería una afirmación falsa sobre un agente que simplemente no
lo cuenta.

### D4 — Un store para las changes, con la guarda que ya existe

El fetch sube de `Project.svelte` a `stores.ts` como store propio, conservando
la guarda `isMeltemiProject(root)` (`Project.svelte:39`): un proyecto sin
`.meltemi/` no pregunta por changes. La vista Proyecto pasa a consumir el
store en vez de su copia local — una fuente, dos lectores.

### D5 — Los segmentos llevan a su vista

Clic (y foco de teclado con nombre accesible) navega: proyecto→Proyecto,
change→Proyecto, sesiones→Sesiones, permisos→bandeja, consumo→Uso. Una barra
que dice algo y no lleva a ello obliga a recordar dónde estaba.

### D6 — Prioridad al encoger, declarada

Se cae primero el endpoint (ya truncado hoy), luego la versión, luego el
consumo, luego el proyecto. **Conexión y permisos no se caen nunca**: es la
misma jerarquía que el suelo de tamaño de la TUI ya aplica
(`render.rs:162-188`), no una nueva.

### D7 — La TUI gana la change y su gate; el consumo se queda en su vista

`LiveData` gana change+gate y el header los muestra. **El consumo no entra al
header del terminal**: ya lleva versión, uptime, sesiones y bandeja, y una
sexta cifra lo vuelve ilegible en 80 columnas. El terminal tiene su vista de
uso, que es donde esa pregunta se responde con detalle. Es una decisión de
cromo, no de capacidad: ninguna superficie pierde acceso a nada.

### D8 — Los guardianes actuales se respetan al pie de la letra

`scenarios_shell.rs:140-158` exige que `StatusBar.svelte` contenga literalmente
`$conn.endpoint`, las tres palabras de conexión y la coexistencia de
`aria-hidden="true"` con `$t(`. Los segmentos nuevos **se añaden sin mover esas
expresiones**: nada de extraer el endpoint a una derivada, que rompería un test
sin cambiar una sola conducta.

## Risks / Trade-offs

- **Densidad**: la barra es una línea y gana cuatro segmentos. La prioridad de
  D6 es la mitigación y el smoke la mide en la ventana mínima (900 px).
- **Refresco del consumo**: `Usage.svelte` refetch por cambio de control y no
  hace polling. La barra necesita política propia; el design la fija en
  «al cambiar de proyecto y al terminar una sesión», no un temporizador —
  un contador que se refresca solo es un contador que gasta batería.
- El desglose «live» está duplicado en tres sitios (`StatusBar.svelte:9-13`,
  `tree.ts:27`, `Sidebar.svelte`). **No se unifica aquí**: un test pinea el
  criterio dentro del sidebar (`scenarios_shell.rs:114-137`) y unificarlo es
  una limpieza con su propio riesgo. Queda anotado.

## Migration / Rollout

`desktop/ui` y `tui/`; el daemon no gana capacidad y el contrato no se mueve.
Sin migración.
