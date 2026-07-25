## Context

`flota-multiproveedor` ya dejó el motor listo: `[[fleet.profile]]` redirige el
contexto de autenticación del binario oficial, `resolve_fleet_agent` elige
perfil → id de catálogo → agente configurado, y la resolución (binario efectivo,
fuente, perfil, nivel) queda como evento `agent_resolved` en el JSONL. El daemon
es uno por usuario, el índice de sesiones guarda `project_root` por sesión y
`session/list` sin filtro ya recorre todas las claves de proyecto del directorio
de datos. Lo que falta es superficie: la GUI se ancla al cwd (comando Tauri
`project_root`), la TUI lista sesiones planas, y el perfil que corrió una sesión
no viaja en el contrato. El árbol de proyectos con sesiones etiquetadas por
agente y suscripción ya es cierto en disco y falso en pantalla.

## Goals / Non-Goals

**Goals:** registro de proyectos conocidos, persistido y reconstruible;
agregación del listado global por proyecto; agente resuelto y suscripción
(nombre de perfil) legibles por contrato; árbol Proyecto → Sesiones en la GUI con
ámbito conmutable; equivalente agrupado en la TUI; guía canónica de dos cuentas
del mismo proveedor.
**Non-Goals:** cuota o costo por suscripción (§2/§9 — es
`analitica-consumo-local`); credenciales, jamás (§2 — los perfiles solo
redirigen contexto); workspaces multi-repo de equipo (fase 3); recorrer el disco
del usuario en busca de repositorios.

## Decisions

### D1 — `project-registry` como capacidad propia, no un injerto en el histórico
El registro de proyectos tiene ciclo de vida propio —persistencia, recencia,
raíces que desaparecen, reconstrucción— y método propio; `session-history`
seguirá siendo dueña de la verdad *por sesión* y solo gana los campos de
resolución. Alternativas rechazadas: meterlo todo en `session-history` (mezcla
dos ciclos de vida distintos, el índice por proyecto y el catálogo por usuario, y
deja el método nuevo sin capacidad natural que lo cobije); colgarlo de
`fleet-catalog` (los perfiles son flota; los proyectos no lo son en absoluto).

### D2 — `projects/index.jsonl` apend-only, reconstruible desde las sesiones
El registro vive en `<data_dir>/projects/index.jsonl`, una línea JSON por alta
(`{"v":1,"projectKey","root","firstSeenAt","lastSeenAt"}`) con fold last-wins por
clave conservando la primera vez vista. Se elige archivo persistido y no cálculo
al vuelo porque `project_key` es un SHA-256 truncado: `all_project_keys` enumera
claves pero **no puede invertirlas** a rutas, y hoy la raíz solo vive dentro de
los registros de sesión. De ahí la invariante heredada: el registro MUST poder
reconstruirse desde `projects/<key>/sessions/` —los registros de sesión son la
fuente de verdad, igual que en `sesiones-reanudables`— y el archivo solo añade
recencia y memoria de un proyecto cuyo historial se vació. Convive sin colisión
con los directorios hermanos porque `all_project_keys` solo acepta entradas con
subdirectorio `sessions/`. Mismo patrón JSONL que el índice de sesiones y el log
de ediciones: cero dependencias y cero formatos nuevos (§6/§10). Rechazado: un
documento TOML/JSON reescrito en sitio (lectura-modificación-escritura, peor ante
caídas) y SQLite (dependencia injustificable, §10).

### D3 — Alimentado por el uso real, nunca por exploración del disco
El alta ocurre exactamente en los dos momentos en que el usuario ya apuntó
Meltemi a ese repositorio: al arrancar una sesión sobre esa raíz (junto al
registro de inicio que ya escribe el índice) y al resolver el contexto de
proyecto por contrato. El daemon MUST NOT recorrer el disco buscando repos ni
inferir proyectos de ninguna otra fuente: nada aparece en el árbol que el usuario
no haya usado. Consecuencia aceptada: un proyecto sin uso no se lista, y para
estrenarlo el conmutador de la GUI abre una carpeta local; el registro cuenta
historia, no adivina intenciones.

### D4 — Contrato aditivo: `project/list` y dos campos opcionales por sesión
Método nuevo `project/list` (`ProjectListParams`/`ProjectListResult`/
`ProjectInfo` en camelCase, `proto/schemas/v1/project-list.schema.json`) con
clave, raíz, existencia en disco, última actividad y contadores de sesiones
activas y totales. `SessionInfo` y `SessionRecord` ganan dos campos **opcionales**
con default: `agentId` (id del catálogo cuando la resolución lo nombró) y
`profile` (nombre del perfil). Se añaden los dos y no solo el perfil porque sin
el id del catálogo la superficie tendría que *adivinar* el agente desde la ruta
del binario, justo donde nada debe ser ambiguo; ambos ya están en `ResolvedAgent`
al lanzar. Nombre `project/list` y no `context/project*` para no confundir la
resolución de contexto de un repo con el catálogo de repos vistos.

### D5 — La suscripción es el nombre del perfil, jamás la credencial
Las superficies etiquetan la suscripción con el nombre del perfil y nada más.
Ningún campo nuevo del contrato, del índice o del registro transporta la
sobrecapa `env` del perfil ni material de autenticación (§2): el binario se
autentica solo dentro del contexto que el perfil selecciona, y el lint de higiene
vigente sigue rehusando secretos en claro en la configuración. `fleet/list` ya
publica los perfiles como filas con su agente subyacente: el lanzador, el drawer
y el árbol leen de ahí, sin fuente paralela.

### D6 — El cwd deja de ser jaula: ámbito de proyecto conmutable por superficie
GUI: el comando `project_root` pasa a ser solo el ámbito **inicial**; el proyecto
activo es estado de la superficie, lo fija el conmutador de la cabecera del
sidebar (la casilla que `gui-clase-mundial` ya reserva para "el proyecto activo
arriba, conmutable") y se persiste en `desktop-ui.json` junto al resto del estado
de UI; toda invocación con ámbito de proyecto inyecta el activo en lugar del cwd.
El lanzador de "Nueva sesión" gana el selector de proyecto sin métodos nuevos
(compone los RPC existentes, como manda su design). TUI: la vista Sesiones agrupa
por proyecto y el ámbito se conmuta desde la paleta, con el cwd como ámbito
inicial. Rechazado: una ventana por proyecto (multiplica conexiones al daemon,
rompe el modelo de un daemon por usuario y parte la bandeja de permisos, que es
global por diseño).

### D7 — Un árbol, una llamada: agregación en el cliente
`session/list` sin filtro ya es global y cada `SessionInfo` declara su
`projectRoot`: el árbol se construye agrupando **una** respuesta, unida a
`project/list` para los proyectos sin sesión viva, el orden por recencia y la
marca de raíz ausente. Rechazado: anidar las sesiones dentro de `project/list`
(duplicaría la carga de sesión en dos métodos y crearía dos fuentes de verdad
para la misma fila); rechazado también N llamadas filtradas por proyecto (N
viajes para datos que ya vienen en uno).

### D8 — La guía entra por `fleet-catalog`, donde viven los perfiles
La propuesta pide la sección multi-suscripción en `docs/agentes.md` sin asignarle
capacidad; su casa honesta es `fleet-catalog`, dueña de los perfiles y de su
visibilidad. La sección documenta el ejemplo canónico de dos cuentas del mismo
proveedor conviviendo en un proyecto vía redirección del contexto de
autenticación (`HOME`/`XDG_CONFIG_HOME`) y enseña `${VAR}` como única vía para
valores sensibles; jamás incluye una credencial ni pide pegarla en la
configuración de Meltemi (§2).

## Risks / Trade-offs

- **Dependencias de superficie**: el árbol se apoya en el sidebar de
  `gui-clase-mundial` y la guía en el `docs/agentes.md` de
  `flota-deteccion-guia`. Mitigación: los requisitos del árbol están escritos
  para *componer* con ese sidebar (no lo redefinen) y la sección de la guía es
  aditiva al archivo; si el orden de implementación cambia, la spec no.
- **Proyecto movido o renombrado** → la clave es un hash de la ruta canónica, así
  que un repo movido entra como proyecto nuevo y el anterior queda listado como
  ausente. Se acepta: cualquier re-vinculación sería heurística, y preferimos dos
  entradas honestas a una adivinada.
- **Rutas de Windows** → `project_key` canonicaliza antes de hashear, de modo que
  el plegado de mayúsculas/minúsculas y de rutas UNC es el del sistema de
  archivos; se prueba en las tres plataformas.
- **Registro apend-only que solo crece** → líneas mínimas y fold last-wins; no se
  poda nada automáticamente porque borrar historia del usuario en silencio sería
  peor que un archivo de unos kilobytes.
- **Árbol largo con muchos proyectos** → orden por recencia y `limit` en el
  método; el colapso visual por nodo se decide al maquetar.

## Migration Plan

Aditivo por completo. Sin `projects/index.jsonl`, la primera consulta lo
reconstruye desde el índice de sesiones (D2). Las líneas antiguas de
`SessionRecord` sin `agentId`/`profile` deserializan con sus defaults y se
completan al reconstruir desde `agent_resolved`. Una superficie sin preferencia
de proyecto activo se comporta como hoy: el cwd. Reversión: retirar el método y
los dos campos opcionales; el JSONL queda como dato inerte.

## Open Questions

- ¿Debe existir un "olvidar este proyecto" explícito, o basta el orden por
  recencia? Sería un método aparte; se decide con uso real, no antes.
- ¿El árbol colapsa por omisión los proyectos sin sesiones vivas? La spec solo
  exige que sean alcanzables; el criterio se fija al maquetar contra el suelo de
  ancho de la GUI.
