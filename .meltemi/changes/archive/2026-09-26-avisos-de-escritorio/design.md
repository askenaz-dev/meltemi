# Design — avisos-de-escritorio

## Context

Verificado el 2026-08-10, y corrige un hecho de la propuesta.

La propuesta afirma que «ninguna superficie toca una API de avisos del SO».
**No es exacto**: `desktop/src/lib.rs:118-142` implementa `request_attention`,
que pide atención al sistema —parpadeo de barra de tareas en Windows, rebote de
dock en macOS, *urgency hint* en Linux— por la API del núcleo de Tauri, **sin
plugin y sin dependencia**. Y ya trae escrita la mitad difícil:

- **La regla de foco existe y funciona**: `pending > 0 && !focused` (`:135`).
- **Se limpia sola**: `None` retira la petición en las plataformas que la
  mantienen (`:140`).
- **El título lleva el motivo**, y lo compone la superficie porque el catálogo
  de mensajes es suyo (§11, `:129-133`).
- Lo llaman `stores.ts:385` y `App.svelte:198`, ya sobre la cola de permisos.

Lo que falta, entonces, no es «el último metro» entero: es que ese mecanismo
**solo lo disparan los permisos**. Un gate esperando y una sesión que termina o
falla no piden atención, y son los otros dos momentos en que el humano hace
falta.

## Goals / Non-Goals

**Goals**: que los tres momentos en que se espera a una persona pidan su
atención, no solo uno; que el motivo se lea sin volver a la ventana; y que nada
del contenido del turno salga del log gobernado.

**Non-Goals**: el aviso remoto del Agent Boss; avisos de progreso; badges de
dock; sonidos propios; historial o centro de avisos propio.

## Decisions

### D1 — El plugin entra ahora, sobre lo que ya existe

Decisión del mantenedor (2026-08-10): «vamos con el plugin desde ya, no dejemos
cosas a medias». Un primer borrador de este design proponía aplazarlo y
extender solo `request_attention`; se descarta. Los dos mecanismos **no
compiten, se complementan**, y esa es la razón por la que la decisión sale
bien:

- `request_user_attention` **reclama** (parpadeo, rebote, *urgency hint*) y no
  dice nada. Se ve si vuelves a la máquina.
- La notificación **dice qué pasó** y sobrevive en el centro del sistema. Se ve
  aunque no vuelvas.

La change hace las dos: pide atención **y** notifica, con la misma regla de
foco y el mismo contenido mínimo.

**Justificación §10 de la dependencia**: `tauri-plugin-notification`, pineada
igual que el resto del stack y confinada al cliente GUI — el mismo patrón que
`tauri-plugin-dialog` (`=2.7.2`), que `lanzador-conversacional` introdujo para
el selector de carpeta. El daemon no la enlaza: sigue sin red y sin
dependencias nuevas.

**Los tres riesgos por plataforma, declarados y no descubiertos**:

- **macOS**: una app sin bundle firmado puede no mostrar avisos en desarrollo.
  Se prueba sobre el bundle, y la guía lo dice.
- **Windows**: el toast exige identidad de aplicación, que da el MSI. El
  ejecutable suelto **se declara** como caso sin aviso, no se finge.
- **Linux**: depende de un daemon DBus de notificaciones presente. Su ausencia
  **degrada a silencio declarado en Ajustes**, jamás a error fatal.

En los tres, la petición de atención sigue funcionando: por eso el plugin se
suma a lo que hay en vez de sustituirlo. Si el sistema no entrega el aviso, la
superficie lo dice con su remedio y **nunca finge que avisó**.

**El permiso del SO se pide en el primer aviso real**, no al arrancar: pedir
permiso antes de tener nada que decir es la forma más rápida de que lo
denieguen.

### D2 — Tres disparadores, una transición cada uno

- **Permiso en espera de decisión humana** — ya implementado.
- **Gate SDD esperando** — la barra de estado ya conoce el gate desde
  `barra-de-estado-agentica`; el mismo store lo dispara.
- **Sesión terminada o interrumpida** — la transición, no el estado.

Se avisa en la **transición**, nunca en el repintado, y una ráfaga de la misma
sesión colapsa en una sola petición con recuento: el título ya lleva número, y
pedir atención dos veces por lo mismo es pedirla en vano.

### D3 — El título dice el motivo y nada del turno

El título lleva **qué espera y cuánto**, jamás el texto del prompt ni de la
respuesta. Un título de ventana lo lee cualquiera que pase por detrás y lo
guarda el gestor de ventanas fuera del log; el contenido del trabajo no sale de
ahí. Esta regla es la que la fase 2 heredará intacta.

### D4 — La TUI: campana opt-in, y solo eso

Campana del terminal (`\a`) tras los mismos disparadores, **desactivada por
defecto** y activable por configuración: una campana que suena sin pedirla es
la razón por la que la gente desactiva las campanas. La guía dice qué
emuladores la honran y no se promete lo que el emulador no dé.

## Risks / Trade-offs

- **El parpadeo no dice tanto como un toast.** Es el trade que D1 asume y
  declara, con la condición de revisión escrita.
- El título de ventana es un canal estrecho: cabe el motivo y el número, nada
  más. Es también su virtud (D3).

## Migration / Rollout

`desktop/` (el plugin, el mando existente y sus llamadas, la sección de
Ajustes) y `tui/` (campana opt-in). **Una dependencia nueva**, pineada y
confinada al cliente GUI; pasa por cargo-deny y **el gate de tamaño de
instalador se re-mide, no se supone**. Cero contrato, cero daemon.
