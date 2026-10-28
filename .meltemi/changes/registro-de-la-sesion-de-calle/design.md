# Design — registro-de-la-sesion-de-calle

## Context

`worktree/dispatch` (`core/meltemid/src/server.rs`) abre el registro con
`project_key(root)` —el proyecto principal— y escribe en el índice un registro
con `project_root` igual al worktree, la calle. `session/log` calcula la clave
con la raíz que recibe, así que con la de la calle busca en un directorio que
no existe. `find_session_record`, que localiza una sesión por raíz para
reanudarla o dirigirla, tiene el mismo hueco, y `resume_with_instruction` abre
el registro de la sesión reanudada con la clave de `record.project_root`, que
para una calle es otra vez el worktree.

`worktrees::create` (asignación y despacho) no llama a `ensure_excluded`; solo
`mount` (el taller) lo hace.

## Decisions

### D1 — La forma de la ruta dice de qué proyecto es una calle

Una función `worktrees::owning_project(path)` devuelve `<raíz>` cuando la ruta
termina exactamente en `.meltemi/worktrees/<change>/<calle>`, y nada en otro
caso. Por la forma, no por el registro de worktrees gestionados: una calle ya
limpiada sale del registro, y su sesión sigue en el índice y su registro en
disco. Leer bajo la clave del proyecto principal no da más acceso que pasar esa
raíz directamente, que cualquier cliente ya puede hacer.

### D2 — Una sola clave de registro, en los tres sitios

`worktrees::log_key(root)` es `project_key(owning_project(root) o root)`.
La usan `session/log`, `find_session_record` y la reanudación. La reanudación
sigue corriendo el agente en la calle y guardando la calle como raíz en el
índice; solo cambia dónde se escribe, y el proyecto que se toca en el registro
de proyectos es el principal, no la calle.

### D3 — Excluir al crear cualquier worktree gestionado

`create` llama a `ensure_excluded` después de registrar el worktree, como
`mount`. Es idempotente: una sola línea en `.git/info/exclude`, nunca dos.

## Risks / Trade-offs

- Un usuario que tenga un repositorio propio bajo una ruta con esa forma
  exacta, fuera de un worktree de Meltemi, vería sus sesiones agrupadas con el
  repositorio de arriba. La forma incluye `.meltemi/worktrees`, que es la raíz
  gestionada de Meltemi; el caso no es realista.
