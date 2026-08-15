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
dock; sonidos propios.

## Decisions

### D1 — Se extiende lo que existe; el plugin no entra todavía

La propuesta pedía el plugin oficial de notificaciones de Tauri. **Se aplaza,
con su condición escrita**, y esta change extiende `request_attention` a los
tres disparadores.

El motivo es que el plugin no es gratis y lo que compra ya está a medias
resuelto: trae una dependencia nueva, permiso del SO que pedir, y tres riesgos
por plataforma que la propia propuesta enumera —bundle firmado en macOS,
identidad de aplicación en Windows, daemon DBus en Linux—. Lo que compra a
cambio es que el aviso **diga texto** y **persista** en el centro de
notificaciones. Y esa segunda propiedad es justamente la que roza §2: el centro
del SO guarda fuera del log gobernado.

Con lo que ya existe, quien vuelve al ordenador ve el icono reclamando atención
y el título diciendo por qué. Eso responde el caso que motiva la change —«dejé
un turno largo y volví a mi editor»— sin dependencia y sin riesgo por
plataforma.

**Condición para la fase 2, escrita para que no se relitigue por gusto**: si el
uso demuestra que el parpadeo se pierde —porque la ventana está minimizada y el
título no se lee, o porque el usuario dejó el escritorio—, el plugin entra en su
propia change con su justificación §10, su medición del presupuesto de
instalador y sus tres verificaciones manuales por plataforma.

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

`desktop/` (extender el mando existente y sus llamadas) y `tui/` (campana
opt-in). **Cero dependencias nuevas** — el cambio que la propuesta preveía como
mayor riesgo desaparece. Cero contrato, cero daemon.
