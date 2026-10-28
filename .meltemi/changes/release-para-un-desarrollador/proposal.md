# release-para-un-desarrollador

> Vía completa (proposal → design → specs → tasks): una change paraguas que
> ordena lo que falta para publicar la primera release que **una persona
> prueba de punta a punta** —el mantenedor, en su máquina, con los agentes
> que ya paga— y que trae lo único que ninguna change existente cubre: el
> guion de esa prueba, su informe y la regla de que una release así lo
> publica. Nace de la revisión del 2026-10-28 del estado del IDE frente a lo
> que el sitio declara del cliente de escritorio («en desarrollo»).

## Why

El sitio dice del cliente de escritorio «en desarrollo: los instaladores
salen con cada release; la carcasa se pule». La revisión del 2026-10-28 lo
confirma y lo precisa. **No hay ninguna spec del escritorio pendiente de
implementar**: el MSI, el DMG y el `.deb` salen con cada release bajo el
mismo `SHA256SUMS` y la misma atestación que el núcleo, la matriz de paridad
es gate de CI y las diez vistas de la GUI tienen su smoke publicado en
`docs/qa/`. Lo que hace honesto el «en desarrollo» son tres cosas, y las
tres tienen dueño: el MSI y el DMG **salen sin firma de plataforma** porque
Authenticode y la notarización exigen certificados comprados
(`docs/release.md`, «Platform signing»: deuda declarada, el instalador avisa
en el primer arranque); el transcript pinta las llamadas a herramienta como
píldoras «título · estado» sin resultado ni diff (`herramientas-a-la-vista`,
propuesta, 15º del orden vigente); y `primer-arranque-del-home` tiene una
tarea abierta, el smoke 3.2, que espera un fixture con flota vacía.

El inventario del método, hecho sobre `main` @ ee0aa7b:

| Qué | Cuánto |
|---|---|
| Changes activas | 58 |
| · terminadas y aterrizadas en `main`, **sin archivar** | 28 |
| · con una tarea abierta (`primer-arranque-del-home` 3.2) | 1 |
| · solo propuesta, sin turno | 29 |
| Changes archivadas | 66 |
| Specs vivas | 36 |
| Commits en `main` después de la v0.2.0 (2026-10-23) | 53, ninguno toca `proto/` ni la gramática del CLI |

Las 28 sin archivar no son descuido: `meltemi archive` arrastra la siguiente
cabecera `## …` del delta al bloque del requisito que pliega
(`split_requirements`, `core/meltemid/src/archive.rs`), y el archivo de
`release-con-el-push` del 2026-10-24 tuvo que repararse a mano; el defecto
quedó anotado «para una change propia» y no la tiene. Importa aquí porque
`antes-de-anunciar` genera el `CHANGELOG.md` **del archivo**, y
`meltemi-release notes` lee de ahí las notas de la release: sin archivar, la
próxima release sale con las notas que genera GitHub, como salió la 0.2.0.

Lo que una persona encontraría hoy al probar, medido —no supuesto— por la
corrida integrada del 2026-09-26 y la aceptación del hito v0.1 con Claude
Code 2.1.280, OpenCode 1.18.32 y Codex 0.156.1:

1. **Ninguna regla de permisos gobierna a Claude Code ni a Codex**: sus
   adaptadores no envían `rawInput` ni `locations`, una regla por comando o
   por ruta nunca casa, y cada petición escala al cliente. En la aceptación
   del hito se decidieron a mano **86 aprobaciones y 32 denegaciones**;
   recordar una decisión solo funciona con el agente simulado. Lo cubre
   `adaptadores-con-rawinput`, propuesta y sin turno.
2. **OpenCode edita sin pedir permiso**: con su configuración por omisión no
   pregunta por ACP y la sesión queda registrada pero no gobernada. Anotado
   en `docs/agentes.md` como «la primera cosa que arreglar», sin change.
3. **`meltemi log` no encuentra el registro de una sesión de calle**: la
   sesión de `dispatch` se indexa con el worktree como raíz y su registro
   vive bajo la clave del proyecto principal; página vacía con código `0`.
   Y los worktrees de tarea no se excluyen del git del árbol principal.
   Anotado, sin change.
4. **Codex no se reanuda** (`codex-reanudar-y-mcp`, propuesta) y los
   adaptadores de Claude Code y Codex **no reportan uso**
   (`extension-acp-usage`, propuesta).
5. **El quickstart empieza en `cargo build`** y en un `config.toml` a mano,
   cuando los instaladores atestados y `meltemi session … --agent` existen;
   no hay FAQ, no hay `CHANGELOG.md`, no hay plantillas de issue por tipo.
   Lo cubre `antes-de-anunciar`, cuyo gate (la página de privacidad de
   meltemi-web) se cumplió el 2026-10-24.
6. **No hay `meltemi doctor`**: la primera pregunta de cualquier problema
   —qué versiones hablan, por qué endpoint, qué agentes y en qué versión—
   sigue repartida entre `status`, `fleet` y el log. Lo cubre
   `meltemi-doctor`, propuesta.
7. **`main` está en rojo en macOS** desde el merge de `enfasis-con-color`
   (2026-10-28): `notices::tests::past_the_floor_the_next_wait_is_told_again`
   usa un piso de 200 ms y afirma «inside» después de esperar a que el
   primer aviso se grabe; en un runner lento el primer aviso tarda más que
   el piso y la segunda espera ya está fuera. La PR pasó, Linux y Windows
   pasaron: es una carrera del test, con el mismo patrón que
   `grabador-de-avisos-sin-carrera` y `seguir-y-cancelar-sin-carrera`
   corrigieron. Sin change.

Nada de esto es una feature nueva. Es el orden en que lo ya propuesto tiene
que aterrizar para que una persona pueda instalar desde los assets de la
release, trabajar una tarde con sus agentes y decir qué pasó —y lo que ese
«decir qué pasó» necesita y hoy no existe: un guion que se pueda seguir sin
leer el repositorio y un informe que acompañe a la release.

## What Changes

- **La tanda, declarada en `docs/plan-de-cambios.md`.** Cinco changes ya
  propuestas aterrizan antes de la release, en este orden y con su vía:
  `adaptadores-con-rawinput` (rápida: sin ella el proxy no gobierna a los
  dos agentes del mantenedor), `antes-de-anunciar` (rápida: quickstart desde
  los instaladores, FAQ, plantillas, `CHANGELOG.md` en el formato que lee
  `meltemi-release notes`), `meltemi-doctor` (rápida), la tarea 3.2 de
  `primer-arranque-del-home` (smoke con flota vacía) y, como deseable que no
  bloquea, `herramientas-a-la-vista` (completa: el transcript del escritorio
  deja de pintar píldoras, que es lo que el sitio llama «pulir la carcasa»).
  `regla-precisa-en-la-bandeja` y `auditar-reglas-y-procedencia` entran si
  caben; `codex-reanudar-y-mcp` y `extension-acp-usage` no son gate: el
  guion declara sus huecos como desviaciones conocidas.
- **Cuatro changes nuevas, nombradas aquí y abiertas como propuesta propia**
  (el rumbo de estructura: lo que surge se anota, no se cuela):
  `piso-del-aviso-sin-carrera` (rápida, sin deltas: el test del piso mide el
  tiempo en vez de suponerlo, y `main` vuelve a verde en macOS; **va
  primero**, porque una tanda que empieza en rojo no se puede medir);
  `archivo-sin-cabeceras-arrastradas` (rápida: `split_requirements` deja de
  arrastrar la cabecera siguiente, las cabeceras toleradas de `fleet-catalog`,
  `remote-access` y `session-history` se limpian, y las 28 changes terminadas
  se archivan con el motor, no a mano; es prerrequisito del `CHANGELOG.md`);
  `opencode-que-pregunta` (rápida: Meltemi lanza OpenCode de modo que pida
  permiso por ACP o, si su configuración no lo admite, la flota lo declara
  como «no gobernado» con palabras y el guion lo excluye); y
  `registro-de-la-sesion-de-calle` (rápida: `session/log` encuentra el
  registro de una sesión de `dispatch`, y los worktrees de tarea quedan
  excluidos del git del árbol principal).
- **El guion de la prueba de un desarrollador**, `docs/prueba-de-uno.md`:
  instalar desde los assets de la release (el MSI con el aviso de primer
  arranque **esperado y escrito**, el archivo del núcleo, la verificación con
  `gh attestation verify`), `meltemi doctor` y `meltemi fleet`, vincular una
  suscripción, una sesión libre en la terminal y otra en el escritorio con la
  bandeja de permisos decidiendo por regla, el ciclo entero sobre un fixture
  —`propose`, `review` con una reelaboración, `assign` de dos agentes,
  `dispatch`, `race`, `verify`, `archive`—, los verbos remotos por SSH como
  pierna opcional, y desinstalar comprobando qué queda en disco. Cada paso con
  lo que se espera ver, dónde queda registrado y qué desviación conocida
  puede aparecer. Se escribe para Windows primero (la máquina del
  mantenedor), con las diferencias de macOS y Linux en el mismo documento, y
  la docs CI verifica sus pasos scriptables contra los binarios, como hace
  con el quickstart.
- **El informe de la corrida**, `docs/qa/<fecha>-prueba-de-uno.md`, con la
  plantilla de `docs/hito-v01.md`: versión y commit, plataforma, agentes y
  versiones, resultado por paso, desviaciones, veredicto. El guion lo
  escribe el mantenedor; el agente prepara la plantilla y no inventa filas.
- **La regla en `docs/release.md`**: una release que marca un hito probado
  **enlaza su informe desde las notas de la release**, junto a las changes
  que archiva. La versión la decide y la escribe el mantenedor con
  `set-version`; el design registra la recomendación —**0.3.0**, por el mismo
  motivo por el que la 0.2.0 marcó la primera release por push— y la política
  de versionado no cambia: lo que la tanda trae es aditivo.
- **La frase del sitio cambia cuando esto aterrice**, no antes: la fila
  «Cliente de escritorio» de la tabla de estado de meltemi-web pasa a decir
  por qué (instaladores sin firma de plataforma hasta que haya certificados)
  en vez de «se pule la carcasa». Es una change de meltemi-web, enlazada
  desde las tareas de esta.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `initial-docs`: + requisito «Guion de la prueba de un desarrollador»: un
  documento que una persona sigue desde los assets de una release sin leer el
  repositorio, con lo esperado por paso y sus desviaciones conocidas; sus
  pasos scriptables se verifican en la docs CI.
- `release-distribution`: + requisito «Una release que marca un hito probado
  publica su informe»: las notas enlazan el informe de la corrida y las
  changes que archiva; la versión la escribe el mantenedor.

## Impact

- Archivos: `docs/prueba-de-uno.md` (nuevo), `docs/qa/<fecha>-prueba-de-uno.md`
  (nuevo, lo escribe la corrida), `docs/release.md`, `docs/hito-v01.md`
  (enlace), `README.md` y `LEEME.md` (fila del índice), `docs/plan-de-cambios.md`
  (la tanda y el pendiente del arquitecto). Esta change **no toca código de
  producto**: cada cosa que lo toca tiene su change en la tanda.
- Cero dependencias nuevas; cero RPC.
- Lo que solo el mantenedor puede hacer, y qué bloquea cada cosa: ratificar la
  enmienda 1c (sin ella `lanzador-conversacional` no archiva y queda fuera del
  `CHANGELOG.md`), ratificar las de `abrir-con-lo-tuyo` y `docs-en-el-sitio`
  (no bloquean esta release), reservar los crates (no bloquea), instalar
  ollama (solo la pierna local, opcional), escribir la versión con
  `set-version` (es la publicación) y correr el guion.
- Riesgo: que la paraguas crezca. Regla escrita: aquí no se implementa nada
  que otra change posea; si una change de la tanda se atasca, la release sale
  con su hueco declarado como desviación en el guion, no se espera.

## Fuera de alcance

- Los certificados de firma de plataforma: deuda declarada, sigue declarada.
- La distribución por gestores de paquetes (`distribucion-y-actualizacion`),
  las docs en el sitio (`docs-en-el-sitio`), el compañero móvil (cerrado) y el
  motor propio: no son condición para que una persona pruebe.
- Anunciar: esta release es para un desarrollador, no para el mundo; el
  anuncio sigue siendo de `antes-de-anunciar` con su gate.
- Un `.rpm` o un AppImage: fuera de la familia Debian, el escritorio se
  compila desde la fuente, como ya dice el README.
