# Design — rama-por-change

## Context

Verificado el 2026-08-16, en el código y en el uso real:

- Las sesiones concurrentes ya trabajan así **a mano**: `git worktree list`
  muestra `agentic-ide-modos` y `agentic-ide-preguntas` como worktrees hermanos
  con la rama del nombre de su change. El flujo existe; lo que falta es que
  Meltemi lo hospede.
- `core/meltemid/src/worktrees.rs` ya tiene la maquinaria: nomenclatura
  estable (`.meltemi/worktrees/<change>/<task>-<agente>`, rama
  `meltemi/<c>/<t>-<a>`), registro append-only propio
  (`.meltemi/worktrees/registry.jsonl`), y la regla de no tocar lo ajeno con
  su confirmación para limpiar con cambios sin commitear.
- Los métodos del contrato se agrupan por sustantivo: `worktree/*` para las
  carreras, `change/*` para el ciclo de la change (`change/list`,
  `change/show`). El taller es del ciclo de la change; su mecánica es un
  worktree.
- La paridad §4 tiene su vía pavimentada: el registro obligatorio de métodos
  (paleta TUI + `registry.ts` GUI + `docs/paridad-nucleo.md`) con
  `tui/tests/parity.rs` como gate bloqueante. Un método nuevo sin sus tres
  entradas no compila el gate.
- Ninguna change activa tiene deltas sobre `worktree-orchestration` (revisado
  contra las 17 activas): ADDED aquí no colisiona con nadie.
- Los worktrees de carrera dentro de `.meltemi/worktrees/` **no están
  excluidos del estado de git** del árbol principal — no ha dolido porque las
  carreras reales corrieron en fixtures. El taller de change sí viviría ahí a
  diario, así que esta change salda esa deuda de paso.

## Goals / Non-Goals

**Goals**: que «dame el taller de esta change» y «aterriza esta change» sean
verbos de Meltemi con la misma disciplina que el resto — idempotencia,
propiedad, previsualización, confirmación y rehúso honesto.

**Non-Goals**: resolver conflictos; borrar ramas; una vista GUI dedicada;
tocar el eje tarea×agente de las carreras; reescribir historia ya en `main`.

## Decisions

### D1 — El taller es otro eje, no otra carrera

La orquestación existente responde «¿cómo compiten N agentes sobre esta
tarea?». El taller responde «¿dónde vive esta change mientras está abierta?».
Compartir capability (`worktree-orchestration`) es correcto porque la
mecánica y las reglas de propiedad son las mismas; separar los requisitos es
correcto porque los ejes son ortogonales — un taller puede contener carreras
dentro (los worktrees de tarea se crean desde la base que el taller fija).

### D2 — La rama lleva el nombre de la change, sin prefijo

Las carreras usan el namespace `meltemi/…` porque sus ramas son mecánica
interna que nadie mergea a mano. La rama del taller es lo contrario: **es la
rama humana de la change**, la que el mantenedor pidió ver, la que se fusiona
a `main` y la que las sesiones ya crean con ese nombre exacto. La propiedad no
la da el prefijo sino el registro: el daemon anota lo que creó y rehúsa tocar
una rama homónima que no esté en su registro, con remedio («esa rama ya
existe y no la creé yo; renómbrala o retírala»).

### D3 — El worktree vive dentro del proyecto, no como hermano

Las sesiones humanas usan directorios hermanos (`../repo-<change>`) porque es
cómodo en el explorador. El daemon no: **solo escribe dentro del proyecto que
gobierna**, y un directorio hermano es escribir en el padre del repositorio,
fuera de su ámbito. El taller va en `.meltemi/worktrees/<change>/workspace` —
la raíz gestionada que ya existe, con su ciclo de vida y su registro. Quien
quiera un hermano puede seguir haciéndolo a mano; el taller gestionado es del
daemon y de sus reglas.

Consecuencia que esta change salda: la raíz gestionada se excluye del estado
de git **por vía local** — una entrada en `.git/info/exclude`, que no se
versiona y no toca el `.gitignore` del usuario. Escribir en el `.gitignore`
versionado sería colar un cambio de árbol en cada proyecto que use talleres;
`info/exclude` es exactamente el mecanismo de git para exclusiones de máquina.

### D4 — `change/workspace` es «dame», no «crea»

Idempotente: si el taller existe y es gestionado, lo devuelve con su ruta y su
rama; si no existe, lo crea desde **la punta de la rama por defecto** (no
desde HEAD, que depende de dónde esté parado quien pregunta) y lo devuelve
igual. El resultado declara si fue creación o reencuentro. Así el verbo sirve
de entrada única: una sesión que arranca no necesita saber si es la primera.

### D5 — `change/land` previsualiza sin `confirm` y jamás resuelve conflictos

La casa ya tiene el patrón (`commit`, `revert`): sin `confirm`, la
previsualización — qué commits aterrizarían y qué archivos tocan; con
`confirm`, el merge a la rama por defecto con `--no-ff`, para que la forma de
la change quede visible en el grafo. Tres rehúsos honestos, cada uno con su
remedio: taller con cambios sin commitear (commitea o descarta), merge con
conflictos (resuélvelo en tu git — el daemon **aborta** el merge y deja la
rama por defecto intacta), y rama por defecto que avanzó de forma que el
merge no aplica limpio (trae los cambios al taller primero).

La fusión con conflictos se rehúsa en vez de dejarse a medio aplicar: un
`merge --abort` inmediato es la diferencia entre un rehúso y un árbol roto.

### D6 — El taller sin aterrizar no se pierde en silencio

`worktree/remove` ya exige confirmación con cambios sin commitear. El taller
añade la mitad que faltaba: **commits sin aterrizar** — trabajo commiteado en
la rama que la rama por defecto no alcanza. Retirarlo pide la misma
confirmación explícita, y el mensaje dice cuántos commits se quedarían solo en
la rama. La rama en sí nunca se borra: retirar el taller retira el worktree.

### D7 — Paridad por el registro, y la vista queda fuera

Método nuevo del daemon → deber §4. Se paga por la vía que el proyecto diseñó
para esto: entrada en la paleta TUI, entrada en `registry.ts` de la GUI (con
`gen:forms` regenerado — el gate `check:forms` lo caza), y fila en
`docs/paridad-nucleo.md`. `tui/tests/parity.rs` es bloqueante, así que el
olvido no compila. La vista dedicada (un panel de talleres) queda anotada como
candidata futura: la paleta ya hace ambos métodos invocables desde ambas
superficies.

## Risks / Trade-offs

- **Un worktree dentro del árbol principal** significa que un `git clean -fdx`
  del usuario en la raíz se lo lleva. Es el mismo riesgo que las carreras ya
  aceptan, y el registro permite reconstruir qué había. Se documenta.
- **`--no-ff` genera commits de merge** que algunos prefieren evitar. Es
  deliberado: la forma de la change en el grafo es trazabilidad §8, no ruido.
  Si alguien quiere fast-forward, el remedio es su git.
- **La rama por defecto se detecta**, no se asume `main`: `init.defaultBranch`
  varía. Se resuelve del `HEAD` remoto o del repositorio, y el e2e lo cubre
  con un fixture cuyo default no es `main`.
- **Dos sesiones pidiendo el mismo taller a la vez**: la creación es
  idempotente y el registro es append-only con fold posterior, la misma
  estrategia que ya usan las carreras. El segundo llega y reencuentra.

## Migration Plan

Aditivo: nadie que no invoque los verbos nota nada. Los talleres creados a
mano por las sesiones siguen siendo ajenos al daemon (no están en su registro)
y por tanto intocables, que es exactamente el comportamiento correcto.

## Open Questions

- ¿Debe `land` ofrecer borrar la rama tras aterrizar? Fuera de alcance hoy;
  si el uso acumula ramas muertas, es una change de limpieza con su propia
  confirmación.
