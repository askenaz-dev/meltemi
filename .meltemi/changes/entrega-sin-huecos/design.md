# Design — entrega-sin-huecos

## Context

La release sale sola desde `release-con-el-push`: un commit de versión nueva en
`main` abre el camino, `release.yml` corre los gates en las tres plataformas,
empaqueta, junta todo bajo un `SHA256SUMS`, lo atesta y publica; después avisa
a meltemi-web, que en su despliegue escribe la versión del IDE en sus páginas
(`ide-release.mjs`). Lo que queda a mano está antes y después de ese camino.
Antes, elegir la versión y escribir el changelog. Después, decir en el sitio
qué está publicado. Y fuera del camino queda lo que nunca entró: el APK y las
firmas de plataforma.

Las restricciones vienen escritas: la constitución (§3, efectos irreversibles
con aprobación explícita; §9, sin telemetría; §10, dependencias justificadas),
`docs/release.md` (el aviso a la web lleva su propio token; ninguna clave de
firma en CI, regla que esta tanda enmienda) y la decisión 0004 de meltemi-web
(toda configuración de infraestructura es Terraform).

## Decisions

### D1 — Automatizar hasta el merge, y ni un paso más

Cada paso que hoy hace el mantenedor pasa a hacerlo CI, salvo uno: el merge de
la PR de release. Ese merge es la aprobación explícita que §3 pide para un
efecto irreversible (una release es inmutable), y es la forma más barata de
tenerla: el mantenedor ve el número, el porqué y las notas antes de publicar.
No se agrega revisor al entorno `release` ni un paso de aprobación en el
workflow; la PR ya es ese paso.

### D2 — La regla del hito, escrita ya

`docs/versionado.md` gana «A tested milestone bumps the minor»: un hito que una
persona probó de punta a punta sube el minor aunque nada rompa. Se escribe en
esta paraguas, antes que la herramienta que la aplica, porque la próxima
release (la 0.3.0) puede salir a mano antes de que `version-que-propone-ci`
exista, y tiene que salir conforme a la política. Qué cuenta como hito —un
informe de `docs/prueba-de-uno.md` cuyo veredicto dice que sale— lo fija
`version-que-propone-ci`, que es quien lo calcula.

### D3 — 1.0.0: los criterios, y por qué hoy no

`meltemi.md` §10 define el **Hito v1.0** con tres criterios: paridad de núcleo
verificada por CI, instalable en macOS, Windows y Linux, y funcionamiento
headless por SSH. Los tres se cumplen hoy: la matriz de paridad es gate de CI,
la release publica núcleo e instalador en las tres plataformas, y `meltemi
bridge` por SSH es un canal completo.

Pero 1.0.0, en SemVer, es una promesa de contrato estable, y la misma fase del
documento fundacional lista entregables que no existen: el motor propio
(`motor-propio-byok`, solo propuesta), el sandbox propio, los hooks y los
plugins sobre el SDK (solo nombres). Además faltan dos cosas de calidad de
entrega: los instaladores de Windows y macOS avisan porque no están firmados,
y nadie corrió todavía la prueba de un desarrollador.

Por eso la lista de preparación que calcula `version-que-propone-ci` tiene dos
grupos:

- **gates** (todos en verde para proponer 1.0.0): los tres criterios del Hito
  v1.0; MSI, DMG y APK firmados en la última release; un informe de la prueba
  de un desarrollador que dice «sale» sobre la versión anterior; y ninguna
  ruptura del contrato en las dos últimas releases;
- **informativos**: los entregables de fase 2 que no están en el texto del
  hito (motor propio, sandbox, hooks, plugins sobre el SDK), con el estado de
  sus changes.

Que los informativos no sean gate es una lectura del documento fundacional: el
hito los deja fuera de sus criterios, la fase los incluye. **Confirmarla es del
mantenedor** (tarea 3.3); si dice que sí son gate, la herramienta los mueve de
grupo y no cambia nada más. Con los datos de hoy, la PR propondría 0.3.0 y
mostraría pendientes la firma y la prueba.

### D4 — Una GitHub App para todo el camino de entrega

El mantenedor ya tiene una GitHub App. Instalada en este repositorio y en
meltemi-web —y en el tap y el bucket de `distribucion-y-actualizacion` cuando
existan—, da a cada job un token efímero con solo los permisos de ese job, vía
`actions/create-github-app-token` (pineada por commit). Reemplaza cuatro cosas:

- el token del workflow, que no puede abrir PRs (el Terraform deja apagado
  `can_approve_pull_request_reviews`) y cuyas PRs no disparan CI;
- el `GH_ADMIN_TOKEN` que hoy sirve de respaldo del aviso a la web, por
  elección del mantenedor del 2026-10-23, mientras falta el token acotado;
- el token de publicación de manifiestos que pedía `distribucion-y-actualizacion`;
- cualquier PAT personal.

La clave privada de la App es un secreto de entorno (`release-pr` o `release`,
limitados a `main`) declarado en el Terraform de meltemi-web
(`ide-delivery-infra`). No es una clave de firma de artefactos: firma pedidos de
token, y rotarla no invalida nada publicado.

### D5 — La enmienda de firma, acotada

La enmienda de `docs/release.md` la escribe `firma-de-plataforma`, junto con la
infraestructura que la hace verdad: escribir hoy cómo se custodian claves que
todavía no existen sería describir lo que no hay. Lo que la enmienda conserva y
lo que cambia, dicho aquí para que las demás changes lo den por sentado:

- **conserva**: la firma de la release es la atestación del workflow, sin clave
  de largo plazo, y verifica igual que hoy;
- **cambia**: las firmas de plataforma se admiten, bajo custodia escrita (qué
  clave, dónde vive, quién la rota, cómo se revoca), en un entorno `signing`
  limitado a `main` o en un servicio de firma en la nube, usadas solo por jobs
  que firman lo ya construido.

### D6 — Lo que va a meltemi-web

Tres changes de allá, en inglés como su método:

- `ide-state-as-it-is`: las filas de `/ide/` y `/es/ide/` al día, con un test
  que las fija, como `state-as-it-is` hizo con la portada;
- `ide-state-from-release`: el despliegue pinta esas filas desde
  `capabilities.json`, como `ide-release.mjs` ya hace con la versión;
- `ide-delivery-infra`: los entornos `signing` y `release-pr` del IDE, sus
  secretos contados sobre variables (presentes solo cuando existen), y la App
  como credencial del aviso.

### D7 — Lo que esta paraguas no hace

No implementa ninguna de sus changes ni toca workflows. Sus tareas hechas son la
regla del hito, el inventario y las propuestas; las demás se cierran cuando su
change se archiva. Se archiva cuando la tanda termina, con la confirmación de
D3 del mantenedor.

## Risks / Trade-offs

- Una PR de release que se acumula sin mergear es una release que no sale. La
  cadencia de `docs/release.md` (nunca más de un mes) sigue valiendo; la PR
  abierta es el recordatorio.
- La lista de 1.0 puede envejecer si cambia el documento fundacional; vive en
  código con tests, y un cambio de `meltemi.md` §10 la obliga a cambiar.
