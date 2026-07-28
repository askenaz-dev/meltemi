# adaptadores-propios-acp — design

## Context

El nivel 2 de la flota pilota Claude Code y Codex a través de adaptadores
ACP de terceros declarados en el registro. Ese suelo se movió: Zed archivó
su `codex-acp` en Rust el 2026-07-22 («Development has moved to
agentclientprotocol/codex-acp»), los adaptadores canónicos viven hoy en
TypeScript bajo la org neutral `agentclientprotocol`, y el adaptador de
Claude envuelve el Agent SDK que los términos de Anthropic (feb-2026)
nombran como no autorizado para OAuth de suscripciones de consumo. El
research (scratchpad de adaptadores, verificado 2026-07-27 contra GitHub,
npm y la documentación de cada proveedor) estableció la arquitectura; el
mantenedor dio la directiva: «no quiero ACP (adapters) de terceros. Revisa
los que tenemos y construimos los propios». Los adaptadores propios dejan
de ser una opción a evaluar: son los puntos de pilotaje que Meltemi
distribuye.

Esta change revierte el fuera-de-alcance que `niveles-integracion-conformidad`
dejó registrado — proposal: «Mantener adaptadores propios: se consumen
adaptadores abiertos existentes»; design: «mantener adaptadores propios (se
consumen los abiertos existentes, declarados en datos)» — y lo hace por
escrito, con fechas y fuentes (D1).

Interacción con changes vecinas: `pulido-pre-anuncio` (pendiente,
fast-forward) refresca los `adapter-install` de terceros a las
distribuciones vigentes bajo `@agentclientprotocol` — debe aterrizar antes,
por honestidad con el usuario de hoy (D11). `motor-propio-byok` (proposal
redactada, sin implementar) describe la detección de directorio hermano
para binarios `bundled`; como no existe en el código, esta change la
implementa como mecanismo genérico y el motor la hereda (D8).

## Goals / Non-Goals

**Goals:** dos adaptadores ACP propios en Rust, en este monorepo, que
pilotan exclusivamente los binarios oficiales `claude` y `codex` con la
auth que cada CLI gestiona; los adaptadores viajan empaquetados en los
instaladores y son la capa de pilotaje por defecto del registro para las
dos entradas de nivel 2; detección genérica de capas `bundled`; permisos
relevados al proxy vigente con compuerta dura y pérdidas visibles;
conformidad por versión; notas legales veraces; guía reescrita en
lockstep; CI sin red ni agentes reales.

**Non-Goals:** forkear los adaptadores TypeScript; toda vía basada en el
Agent SDK o en canales no documentados; prohibir los adaptadores de
terceros (siguen pilotables por configuración); adaptadores para otros
agentes; sandbox propio (change aparte); auto-actualización o descarga de
adaptadores (el daemon jamás descarga nada); dialectos alternativos de los
CLIs pilotados.

## Decisions

### D1 — Revertir la decisión registrada, por escrito

`niveles-integracion-conformidad` excluyó mantener adaptadores propios
cuando el ecosistema ofrecía adaptadores abiertos vivos y en Rust. Los
hechos que la vencieron, con fecha: (1) 2026-07-22, Zed archiva
`codex-acp` en Rust; las implementaciones canónicas quedan en TypeScript
bajo `agentclientprotocol` — consumirlas exige un runtime Node en la
distribución, contra el rumbo de un solo lenguaje de sistemas, y la única
vía Rust del ecosistema es embeber el runtime del proveedor como librería
(patrón que choca con §2, ver D6). (2) feb-2026, los términos de Anthropic
nombran al Agent SDK como no autorizado para OAuth de suscripción; el
adaptador canónico de Claude envuelve ese SDK y ningún fork escapa, porque
la zona gris vive en la superficie de auth, no en el mantenedor. (3) El
camino seguro que `docs/research/integracion-agentes.md` nombró desde el
principio — pilotar el binario oficial con la sesión ya iniciada — no lo
toma ningún adaptador del ecosistema; solo un adaptador propio lo toma.
(4) 2026-07-27, la directiva del mantenedor. Una decisión registrada se
revierte con otra decisión registrada: esta.

### D2 — Un crate, dos binarios, cero dependencias nuevas

`core/meltemi-adapters`: una librería de puente compartida (lado ACP,
supervisión del subproceso proveedor, framing NDJSON, apagado limpio) y
dos binarios, `meltemi-claude-acp` y `meltemi-codex-acp`. Un solo crate
porque los dos adaptadores comparten más de la mitad de su esqueleto y sus
tests; dos binarios porque el registro declara un punto de pilotaje por
entrada y la detección resuelve binarios, no flags. Los nombres llevan el
prefijo `meltemi-` y difieren de los binarios de terceros
(`claude-agent-acp`, `codex-acp`): jamás una colisión de PATH ni una
detección ambigua. Dependencias: tokio, serde/serde_json y el crate
oficial `agent-client-protocol` — las tres ya pineadas en el workspace.
**Ningún adaptador enlaza pila HTTP/TLS**: a diferencia de
`meltemi-engine` (que necesita rustls para hablar con el modelo), aquí
toda la red vive en el CLI oficial. La propiedad es verificable por
cargo-deny igual que la de meltemid, y el shim MCP de permisos (D5) se
implementa a mano sobre serde: JSON-RPC por stdio no justifica una
dependencia nueva (§10).

### D3 — Una capability `own-adapters`, sin nombres de terceros en specs

El borrador de propuesta nombraba dos capabilities (`adapter-claude-code`,
`adapter-codex`). Se corrige: la verdad viva de `.meltemi/specs/` no
contiene un solo nombre de producto de terceros (verificado por grep), y
el propio registro declara la regla — «third-party names live here, never
in specs». Los slugs de capability son directorios de la verdad viva. Una
sola capability `own-adapters` (espejo de `own-engine` en
`motor-propio-byok`) con requisitos por dialecto, en términos neutrales:
«dialecto de sesión headless de eventos JSON» y «dialecto de servidor
JSON-RPC». Los nombres concretos (claude, codex, stream-json, app-server,
flags) viven en el registro, en este design y en la guía — datos factuales
de interoperabilidad, no verdad normativa.

### D4 — Dialecto de sesión headless: el binario oficial `claude`, jamás el SDK, jamás `--bare`

`meltemi-claude-acp` lanza el `claude` oficial con la sesión que el
usuario ya inició: `-p --input-format stream-json --output-format
stream-json --include-partial-messages`. Qué compra: deltas de tokens y
transcripts de subagentes (casi-paridad de streaming con nivel 1),
`--resume`/`--fork-session` headless con ámbito de directorio de proyecto
y worktrees (calza con el modelo de Meltemi), `--mcp-config` para la
proyección de perfiles MCP existente. Qué se prohíbe: el Agent SDK (zona
gris nombrada por los términos del proveedor) y `--bare` (salta el OAuth y
exige `ANTHROPIC_API_KEY` — exactamente lo que §2 y el principio BYOK no
quieren como default).

**Riesgo pineado**: la documentación del proveedor anuncia que `--bare`
será el default de `-p` en una versión futura. Si ese flip llega y el
adaptador no lo maneja, el OAuth muere en silencio. Mitigación: el evento
`system/init` del cable trae un arreglo `capabilities` que existe para
detección de features; el adaptador detecta la superficie efectiva en el
handshake y, si el modo con sesión iniciada no está disponible (o el CLI
indica modo de clave de API), **rehúsa con diagnóstico y remedio** — nunca
degrada a inyectar una clave. La marca «docs del proveedor llaman a
`claude -p` "the Agent SDK via the CLI"» es marketing sobre el mismo
binario oficial; la frontera real es: binario oficial sí, librería SDK no.

**Enmienda (2026-07-28, tarea 5.2 — la corrida manual contra el CLI real)**:
dos afirmaciones de arriba no sobrevivieron al binario. Fuente: `claude`
2.1.167 en Windows 11 (26200), lanzado con exactamente los argumentos de
sesión del adaptador; procedimiento y salidas en `docs/conformidad-manual.md`.

1. **No existe arreglo `capabilities` en el evento inicial.** El evento real
   trae `apiKeySource`, `claude_code_version`, `permissionMode`, `model`,
   `tools`, `slash_commands`, `agents`, `skills` y `mcp_servers`; ningún
   `capabilities`. La detección de features no puede colgar de él, y la
   mitigación del flip de `--bare` descansa entonces sobre `apiKeySource`
   —que **sí** existe y vale `"none"` bajo la sesión iniciada, con lo que el
   nombre provisional de la tarea 1.3 queda anclado— más el rehúso ante
   cualquier otra fuente anunciada. El código ya trataba `capabilities` como
   información y nunca como requisito («capabilities are read, never
   demanded»), así que la corrección es de esta decisión, no del adaptador.
2. **El CLI no anuncia la sesión hasta recibir su primera entrada.** No es
   lentitud: lanzado sin entrada, no emite nada en 60 segundos; escrita una
   línea de entrada, el evento inicial llega en el acto. El handshake del
   adaptador lo espera *antes* de enviar nada, de modo que contra el CLI real
   toda sesión agota el plazo y rehúsa con `provider_handshake_failed` —
   nivel verificado 0. El fixture emitía el evento inicial antes del primer
   `await-input` y por eso ninguna prueba lo veía: un fixture solo prueba lo
   que se le pidió parecerse. La corrección es la tarea 5.3, y con ella el
   fixture pasa a emitirlo donde el CLI lo emite.

**Enmienda (2026-07-28, tarea 5.3 — la corrección y lo que exigió decidir)**:
el handshake de este dialecto **ocurre en el primer turno**, después de
escribir el mensaje del usuario y antes de mapear un solo evento de ese turno;
la guarda de superficie corre ahí, con la misma conducta de siempre — rehúso
diagnosticado ante cualquier fuente de credenciales que no sea la sesión
iniciada, jamás degradación silenciosa. Lo que abrir la sesión ya no necesita
es que el CLI hable, y eso obligó a una decisión que este design no tenía:

3. **La identidad de la sesión se dicta, no se aprende.** El adaptador acuña un
   UUID y lo entrega en `--session-id` (existe en 2.1.167 y el CLI lo respeta
   al pie de la letra: el archivo de sesión que deja lleva ese nombre). Antes
   la identidad salía del evento inicial, que es precisamente lo que ataba el
   handshake al `session/new`; dictarla desata los dos y conserva intacta la
   reanudación de la tarea 3.5, porque el id que el daemon recuerda sigue
   siendo el del CLI — ahora por construcción y no por haberlo escuchado. Un
   `--resume` nombra la sesión que continúa y nunca viaja junto a
   `--session-id`: dos identidades en un lanzamiento son una pregunta que el
   CLI no debería tener que responder.

4. **`capabilities` deja de leerse.** El punto 1 de la enmienda anterior lo
   declaró inexistente y el código lo seguía leyendo «por si acaso»; un campo
   que ningún CLI emite no describe nada, así que sale del tipo del cable, de
   la superficie y de la procedencia. La mitigación del flip de `--bare`
   descansa entera sobre `apiKeySource`, que sí existe.

5. **La procedencia se parte en dos actualizaciones, porque se conoce en dos
   momentos.** Qué binario se lanzó, en qué versión y con qué id — hechos del
   `session/new`, registrados ahí. Qué credencial, qué modelo y qué modo de
   permisos anunció el CLI — hechos del primer turno, registrados ahí. Fingir
   que lo segundo se sabía al lanzar es exactamente el error que esta tarea
   deshizo, y una sola actualización lo habría reintroducido en el log.

Y una lección de método que la change se lleva puesta: **el orden es parte de
un cable**. El guión del proveedor simulado ahora anuncia la sesión detrás del
primer `await-input` —con un marcador `once`, porque el CLI lo dice una vez por
proceso y no una por turno— y una prueba corre ese cable sin entrada y exige
silencio. Un fixture cortés en el punto exacto donde el binario no lo es no es
media prueba: es una prueba de un proveedor que no existe.

**Qué significa cancelar en este dialecto**: ver D12, punto 2. En resumen: el
único paro que su superficie documenta es el fin de la entrada, ese fin llega de
verdad, y por tanto **termina la sesión y no solo el turno**. Un turno posterior
se rehúsa con `session_finished` y el remedio es abrir una sesión nueva — sin
pérdida, porque la identidad dictada del punto 3 hace que las anteriores se
reanuden por id.

### D5 — Permisos del dialecto headless: prompt-tool + hooks, pérdidas declaradas

Dos canales documentados, en capas:

1. **`--permission-prompt-tool`** apunta a una herramienta de un servidor
   MCP mínimo por stdio que el CLI lanza según el `--mcp-config`
   proyectado. El servidor es el mismo binario `meltemi-claude-acp` en un
   modo shim, conectado a su proceso padre por un canal privado; cada
   petición del CLI se convierte en `session/request_permission` de la
   sesión ACP del adaptador, que meltemid proxya a la bandeja humana por
   el flujo vigente. **El daemon no gana transporte alguno** y el shim no
   abre socket alguno.

   **Enmienda (2026-07-27, tarea 3.3)**: este design decía «canal privado
   heredado al lanzar (pipe anónimo)». No es implementable, y el hecho que
   lo vence es de arquitectura, no de plataforma: **el shim no es hijo del
   adaptador**. Lo lanza el CLI pilotado —un runtime que Meltemi no
   controla— como uno de sus servidores MCP, de modo que el shim es un
   nieto y ningún descriptor del adaptador sobrevive ese salto; en Windows
   no sobrevive de ninguna forma. Lo reemplaza un **directorio de
   rendezvous por sesión**, creado por el adaptador dentro del directorio
   temporal del usuario (`0700` donde la plataforma tiene modos; en Windows
   el temporal por usuario ya es exclusivo del usuario por ACL heredada):
   el shim escribe su pregunta como archivo y espera la respuesta, ambos
   con escritura atómica por renombrado dentro del mismo directorio. La
   sustitución conserva **todas** las propiedades por las que el pipe
   estaba ahí —privado al usuario, sin puerto, sin listener, sin
   transporte nuevo en el daemon— y su costo es un sondeo de 20 ms, pagado
   donde nadie puede notarlo: detrás de un humano decidiendo. Que el
   directorio desaparezca es además la señal de vida del adaptador: un shim
   que no lo encuentra deniega en el acto en vez de esperar su plazo. Se
   descartó por escrito la alternativa de un socket local (named pipe en
   Windows, UDS en el resto): cumpliría igual, pero el estándar de la casa
   para un endpoint local es ACL explícita de usuario —lo que el daemon
   hace con `windows-sys`— y eso significaba código inseguro específico de
   plataforma en un crate cuyo design (D2) declara cero dependencias
   nuevas; el rendezvous alcanza la misma garantía con la biblioteca
   estándar.
2. **Hooks `PreToolUse`** (inyectados vía `--settings`) como compuerta
   dura: deniegan toda llamada no aprobada incluso si el CLI corre en un
   modo permisivo (`bypassPermissions`). El orden de evaluación del
   proveedor está documentado (hooks → deny → ask → mode → allow → prompt
   tool), y los hooks son el eslabón que ninguna configuración del CLI
   puede saltar.

Pérdidas, por escrito y visibles en sesión: el prompt-tool solo se
consulta cuando ninguna regla estática decide; `AskUserQuestion` y las
herramientas `requiresUserInteraction` se auto-deniegan en modo no
interactivo — el adaptador muestra la denegación con su motivo, no la
esconde ni la aprueba por su cuenta; el contrato del prompt-tool está
infradocumentado upstream (anthropics/claude-code#1175) y puede moverse —
la detección de features de D4 es el amortiguador, y los fixtures de cable
(D10) congelan el contrato observado por versión. El canal `canUseTool`
del SDK existe en el mismo cable pero no está documentado: §6 lo excluye.
En Windows no hay sandbox nativo del CLI (solo WSL2): la compuerta son los
hooks más el worktree aislado, y `sandbox-propio` sigue siendo la change
que cierra ese hueco.

Y una cosa más que este canal es y que este punto no decía: es también
donde se escribe la configuración del CLI, secretos ya resueltos incluidos.
Cuánto vive eso —y por qué no vive lo que este design creía— está en D13.

### D6 — Dialecto de servidor JSON-RPC: `codex app-server`, jamás el embed de librería

`meltemi-codex-acp` lanza el `codex` oficial en modo `app-server`:
JSON-RPC 2.0 bidireccional con delimitación por líneas sobre stdio,
documentado por el proveedor como «the interface Codex uses to power rich
interfaces such as the Codex VS Code extension» — publicado explícitamente
para terceros. Las primitivas de conversación del servidor
(hilo/turno/ítem) se mapean a la sesión ACP; las aprobaciones que el
servidor solicita se relevan a `session/request_permission`.

Se rechaza el patrón de las dos implementaciones Rust existentes (la de
Zed, archivada; la comunitaria `cola-io`): ambas embeben `codex-core` y
crates hermanos como dependencias de librería, con lo que el adaptador
mismo hace red y lee el almacén de auth de Codex. Apache-2.0 de punta a
punta y aún así incompatible con §2 («solo binarios oficiales, con la
autenticación que cada agente gestiona»). Spawn del binario oficial, sin
excepción.

**Conformidad por versión**: `codex app-server generate-json-schema`
vuelca el esquema exacto del binario instalado. Los tipos del adaptador se
validan contra fixtures de ese esquema en CI (la disciplina que `proto/`
ya practica) y el handshake detecta el desfase adaptador↔CLI y rehúsa con
remedio — nunca se asume compatibilidad.

### D7 — La prueba que §6 exige

Del lado Meltemi el estándar es ACP, hablado con el crate oficial ya
pineado en el workspace. Del lado proveedor **no existe estándar abierto
que cubra el pilotaje programático de estos agentes**: ACP no lo cubre
(los CLIs no lo hablan — ese es el problema entero), MCP es herramientas,
LSP es inteligencia de código. Las superficies elegidas son la superficie
programática oficial y documentada de cada proveedor: stream-json
(documentación de headless/CLI del proveedor) y app-server (README y
generadores de esquema del proveedor). El adaptador es exactamente la
pieza que §6 bendice: traducción entre el estándar abierto y la superficie
oficial del dueño. Regla de gobernanza heredada de `motor-propio-byok`:
ninguna capacidad puede colgar de un canal no documentado del proveedor ni
de un canal privado adaptador↔daemon — todo lo que exceda ACP base sería
una extensión ACP abierta y documentada.

### D8 — Flip del registro y detección `bundled` genérica

Las filas `claude-code` y `codex-cli` cambian su capa adaptador:
`bin = "meltemi-claude-acp"` / `bin = "meltemi-codex-acp"`, con
`bundled = true` y sin `adapter-install` (la capa viaja en los
instaladores de Meltemi). Las capas `cli-bin`, `cli-candidate-paths` y
`cli-install` no se tocan: el CLI oficial se sigue detectando e instalando
aparte, y la semántica de dos capas de `flota-deteccion-guia` queda
intacta — solo cambia qué binario es la capa adaptador y de dónde sale.

La detección `bundled` — sondear el directorio del ejecutable del daemon
en ejecución — la describe la proposal de `motor-propio-byok`, pero esa
change no tiene design ni código: **se implementa aquí como mecanismo
genérico del registro** (cualquier capa con `bundled = true` lo usa; nada
cuelga de un id concreto) y el motor lo hereda cuando llegue.
Precedencia: PATH, luego `candidate-paths`, luego el directorio hermano
del daemon — coherente con la filosofía vigente de que la intención del
usuario pisa el default (override de entorno > `command` > id), y no es
degradación silenciosa: el catálogo reporta la ruta absoluta y la fuente
del hallazgo, el log de sesión registra el binario efectivo, y el skew
adaptador↔daemon lo detecta el handshake ACP. `proto/` gana campos
aditivos en la capa (procedencia empaquetada del hallazgo); paridad ×3
heredada por `fleet/list`.

Los adaptadores de terceros siguen pilotables por configuración (`custom`
o `command` literal), sin trato distinto. El registro deja de
recomendarlos; no los prohíbe.

**Precisión (2026-07-28, revisión adversarial)**: «genérico» incluye la
forma de entrada que este registro todavía no tiene y que el motor propio
sí tendrá — **una sola capa, y empaquetada**, sin CLI de proveedor debajo.
En esa forma el comando de instalación no se declara en `adapter-install`
sino en `cli-install`, de modo que la guarda de parseo, escrita mirando
solo la forma de dos capas, dejaba pasar la contradicción: una capa
empaquetada con el comando de un tercero pegado. Dónde vive la regla queda
fijado, porque estaba viviendo en el sitio equivocado: **en la capa**. El
campo `install` es del contrato y una superficie puede leerlo directo de
`fleet/list`, así que una capa empaquetada no lo lleva, se declare donde se
declare; el remedio y el comando ofrecido lo consultan por una sola
función, para que las dos salidas no puedan discrepar sobre una capa
construida a mano; y la guarda de parseo pasa a mirar el campo que
corresponde a la forma de **esa** entrada, para seguir rechazando la
contradicción en el origen en vez de corregirla en silencio.

### D9 — Estatus legal: gris sigue gris, con la nota reescrita con verdad

La nota de la entrada de Claude se reescribe para describir la
arquitectura nueva: la capa de pilotaje ya no envuelve el Agent SDK — es
el binario oficial con la sesión que el usuario inició, exactamente el
camino seguro que la nota vigente señala. Pero el estatus **se queda en
`grey`**: «tolerado» afirmaría que el proveedor tolera la orquestación de
`claude -p` por terceros, y no existe evidencia publicada de eso — el
research lo dice sin rodeos («Anthropic has published no OpenAI-style
blessing»). Subir el estatus con la evidencia actual sería maquillaje al
servicio propio, justo lo que el requisito «sin maquillaje» prohíbe. La
nota nueva dice las dos cosas: esta vía elimina la exposición del SDK y no
cuenta con postura publicada del proveedor. Si esa postura llega, la nota
se actualiza con fuente. Codex permanece `tolerated` con nota actualizada:
app-server fue publicado para terceros y la dependencia de supply chain
sobre un repo archivado desaparece. (Si el mantenedor prefiere
`tolerated`-con-nota para Claude — defendible según el research — es un
cambio de un literal y de esta decisión, no de la arquitectura.)

### D10 — Tests: fixtures de cable de proveedor, e2e real, conformidad manual

CI jamás corre agentes reales ni toca red; entonces el cable del proveedor
se simula, no el adaptador. Crate nuevo `core/mock-provider` (patrón
mock-agent, cero dependencias nuevas) con dos binarios:

- `mock-claude-wire`: emite stream-json guionado — `system/init` con
  `capabilities`, deltas parciales, tool calls que disparan el prompt-tool
  y los hooks, resultado final — y acepta entrada stream-json; guiones por
  archivo/variable de entorno, como mock-agent.
- `mock-codex-wire`: servidor JSON-RPC NDJSON guionado — handshake,
  conversación hilo/turno/ítem, petición de aprobación — más un volcado de
  esquema fixture para el test de conformidad.

Tres anillos: (1) unit/integración del puente y de cada mapeo contra los
mocks en memoria o por stdio; (2) e2e de workspace donde **meltemid pilota
el binario real del adaptador** y este pilota el mock wire — el mismo
anillo que hoy corre contra mock-agent; (3) conformidad contra CLIs reales:
manual, por opt-in, con resultado persistido con fecha y versión, y los
escenarios que solo un CLI real ejerce se marcan vía `sdd/verify-mark` con
nota — la disciplina que `niveles-integracion-conformidad` estableció y
`procedencia-de-release` ya practica.

**Enmienda (2026-07-28, tarea 5.2)**: el tercer anillo se materializa en
`core/meltemid/tests/conformance_real.rs`, `#[ignore]` y además silencioso
sin `MELTEMI_CONFORMANCE_REAL=1` — dos cerrojos, porque uno solo es el que
se olvida. Persiste en el almacén real del usuario, que es lo que hace que
`fleet/list` reporte el nivel verificado de una corrida de verdad. Regla que
la corrida trajo consigo: **un criterio que no se pudo ejercer no se
reporta**, ni aprobado ni fallido; `conformance::verified_level` se niega a
otorgar un nivel cuyos criterios declarados no estén todos presentes, de
modo que una corrida incompleta produce un resultado incompleto en lugar de
uno halagador. Y `verifiedLevel: 0` es un resultado, no un fallo de la
corrida: es lo que la primera corrida devolvió para el dialecto headless, y
es la razón por la que existe la tarea 5.3.

### D11 — Orden con `pulido-pre-anuncio`

`pulido-pre-anuncio` aterriza primero: refresca los `adapter-install` de
terceros a las distribuciones vigentes bajo `@agentclientprotocol`, porque
el usuario de hoy merece comandos que instalen proyectos vivos. Esta
change reemplaza después la capa adaptador de esas mismas filas y retira
esos `adapter-install`. No hay conflicto: el requisito «Vigencia de las
rutas de instalación de la instantánea» que pulido añade sigue aplicando a
todo comando que sobreviva (los `cli-install`), y la revisión de la
instantánea que esta change hace documenta su propia verificación con
fuente y fecha, como ese requisito exige. Si por calendario esta change
llegara antes, absorbe el refresco de datos trivialmente; el orden
declarado es pulido → esta.

**Verificación de la instantánea de esta change (tarea 4.2, 2026-07-28)**:
`pulido-pre-anuncio` se archivó el 2026-07-27 dejando la instantánea en
`version = "2026-07-27"`. Esta revisión sube a `2026-07-28` y **retira** los
dos `adapter-install` de terceros en vez de re-verificarlos: la capa de
pilotaje de ambas entradas de nivel 2 pasa a ser un binario que Meltemi
construye y empaqueta, de modo que ninguna ruta de terceros sobrevive a la
revisión — y una ruta que no se declara no puede envejecer. Los únicos
comandos de instalación que quedan son los dos `cli-install` de los CLIs
oficiales, **idénticos** a los de la instantánea del 2026-07-27, cuya
verificación contra la fuente de distribución (registro npm, 2026-07-27)
consta en `.meltemi/changes/archive/2026-07-27-pulido-pre-anuncio/.verify.jsonl`.
Ningún comando nuevo se introduce, así que no hay nada que citar de memoria.
La misma nota vive en la cabecera del propio archivo de registro, donde la
lee quien edite la instantánea la próxima vez.

### D12 — Cerrar es soltar, y el turno se arma donde se acepta

Añadida el 2026-07-28 tras la revisión adversarial de la change, antes de
la puerta del mantenedor. Dos defectos del mismo tema —el ciclo de vida
del turno y del proceso— y dos decisiones que quedan escritas porque
cambian conducta observable.

1. **Cerrar la entrada del proveedor es *soltar* el stream, jamás
   `AsyncWrite::shutdown`.** Contra la entrada de un proceso hijo, tokio
   implementa `poll_shutdown` como `Poll::Ready(Ok(()))` en las tres
   plataformas (1.52.3: `process/unix/mod.rs`, y en Windows delegando en
   `io/blocking.rs`): no cierra nada. Solo soltar el descriptor cierra la
   tubería. Consecuencias que la change traía y nadie veía: todo apagado
   limpio esperaba la gracia entera y terminaba matando al CLI
   —`ShutdownOutcome::Exited` era inalcanzable contra un hijo real—, y en
   el dialecto headless, donde cerrar la entrada **es** la cancelación, una
   cancelación no enviaba señal alguna: el turno seguía hasta agotar su
   gracia y la sesión anotaba que el CLI había ignorado algo que nunca se
   le dijo. `FrameWriter::close` suelta el stream, es idempotente y un
   frame escrito después se rehúsa con tubería rota en vez de fingir que
   viajó.

   Por qué ninguna prueba lo vio: todas conversaban por `tokio::io::duplex`,
   que sí honra `shutdown` señalando fin de flujo. Un doble más educado que
   aquello que sustituye — exactamente la clase de fallo que la tarea 5.3 ya
   pagó una vez con un guión demasiado cortés. Por eso la corrección trae
   una prueba contra un **proceso hijo real**
   (`core/meltemi-adapters/tests/process_lifecycle.rs`), junto a las de
   memoria y no en su lugar.

2. **Una cancelación en el dialecto headless termina la sesión, no solo el
   turno.** Es lo que su superficie permite: el único paro documentado es
   el fin de la entrada, y ahora que ese fin llega de verdad, el CLI no
   puede recibir otro turno. Se elige eso antes que una cancelación que no
   ocurre — el humano apretó parar. La sesión sigue existiendo como objeto
   ACP (se lee, se cierra, se reanuda por id), pero un turno posterior se
   rehúsa con `session_finished` y su remedio dice la verdad: abrir una
   sesión nueva, cuyos turnos previos no se pierden porque este dialecto
   reanuda por id (D4, punto 3). Un `provider_turn_failed` invitando a
   «reintentar» habría sido mentira sobre cuál de las dos cosas pasó.

3. **El control de turno del dialecto se rearma donde el prompt se acepta,
   en el bucle de despacho ACP, y no dentro del turno.** El bucle es
   serial: `session/prompt` y `session/cancel` se atienden ahí, enteros,
   antes de la tarea que corre el turno. Rearmar dentro del turno dejaba
   una ventana —angosta y suficiente— en la que una cancelación quedaba
   registrada y luego borrada por el turno al que iba dirigida: al servidor
   no le llegaba nada, el turno corría completo —comandos y cambios de
   archivo incluidos, cada uno aprobado por el proxy— y la sesión
   respondía `cancelled` igual. El trabajo ocurría y al humano se le decía
   que no. `ProviderSession::begin_turn` existe para eso y por eso es
   síncrono.

4. **Un turno que la cancelación rompe se responde `cancelled`, no como
   avería.** Corolario de (2): en una superficie cuyo único paro es cerrar
   la entrada, cancelar es precisamente lo que hace que el turno deje de
   funcionar. Responder con ese error diría que el paro rompió algo cuando
   hizo lo único que se le pidió, y ACP exige `cancelled` para un prompt
   cancelado en cualquier caso. Las palabras del proveedor no se pierden:
   van al stderr del adaptador, que el daemon ya recoge al log de sesión.

Y una corrección de fixture que la misma revisión destapó, del mismo tema:
los dos e2e de cancelación cancelaban en cuanto veían *cualquier*
`agent_update`, y la procedencia de la sesión pasó a ser uno de ellos —
emitido en `session/new`, antes de que exista el prompt. Cancelar contra
una sesión sin turno se responde «no hay nada que interrumpir», que es
correcto y no prueba nada: el cable guionado sostiene su turno para siempre
y la prueba muere de su propio plazo. Ahora esperan el `prompt_sent` y un
trozo de mensaje del proveedor, que es lo que sus propios comentarios ya
decían que esperaban.

### D13 — El canal privado no sobrevive al olvido

Añadida el 2026-07-28, misma revisión adversarial que D12, defecto
distinto. El canal de sesión de D5 no guarda solo preguntas y respuestas:
guarda también la configuración con la que se lanza el CLI, y ahí van los
valores de entorno que el daemon **ya resolvió** para los servidores MCP
de la sesión —secretos del usuario, en claro— junto a `settings.json`, que
es la compuerta dura. `Rendezvous::close` lo retira al cerrarse la sesión
en orden, y ese cierre no es el camino que corre: el daemon termina un
adaptador matándolo, y un proceso matado no ejecuta limpieza alguna —ni
`Drop`, ni `kill_on_drop`, nada, en ninguna plataforma. El resultado era un
directorio por sesión, con sus secretos resueltos, acumulándose en el
temporal del usuario mientras durase la máquina. Y el comentario del código
afirmaba lo contrario —«este archivo es la única copia y se va cuando se va
la sesión»—, que es la mitad que no se tolera: una afirmación falsa en el
código envejece peor que el defecto que describe mal.

**Lo elegido: barrer al abrir.** Antes de crear el canal de una sesión, el
adaptador retira los canales que ninguna sesión puede estar usando. Corre
ahí porque es el único momento en que este proceso está con certeza vivo y
con certeza a punto de escribir secretos resueltos en el disco; no hay un
«último momento» en un camino que termina en un kill.

La regla es **antigüedad**, medida sobre el directorio y todo su contenido,
con umbral de un día, y jamás sobre canales del propio proceso. Tres
razones que la sostienen:

- Es señal de vida real y no un sustituto del instante de arranque: cada
  llamada a herramienta de una sesión gobernada crea y borra un archivo en
  su canal, así que una sesión viva lo toca constantemente y una que no lo
  ha tocado en un día no ha corrido una herramienta en un día.
- Preguntar al sistema si el proceso dueño sigue vivo pide una dependencia
  que este crate no tiene (§10) y, peor, un pid: los pid se reciclan, así
  que la respuesta puede ser una mentira con forma de certeza.
- El peor caso queda nombrado y es asimétrico: barrer un canal que seguía
  vivo cuesta una **denegación** y jamás una aprobación —el shim lee la
  ausencia como «nadie puede decidir esto» y dice no—, pero una denegación
  sigue siendo una sesión perturbada, y por eso el umbral es ancho.

**Lo descartado, con su motivo.** (a) Que el daemon cierre los adaptadores
en orden en vez de matarlos: es mejor arquitectura y no es el remedio de
esta change. Exige rehacer el camino cliente ACP del daemon —el crate
oficial lanza el hijo y lo mata al soltar la conexión— y además la mitad
del adaptador, porque su conexión stdio no termina cuando el par cierra la
suya (verificado contra este binario y contra `mock-agent`, D10); es
cirugía en el camino que usan **todos** los agentes, a las puertas de una
release, y aun conseguida seguiría existiendo el kill para un adaptador
colgado, de modo que el barrido haría falta igual. (b) No escribir el
secreto: la superficie documentada del CLI toma la configuración MCP por
archivo, pasarla por línea de comandos la haría visible en el listado de
procesos de la máquina —peor, no mejor—, y borrarla en cuanto el CLI
arranca sería apostar sobre cuándo la relee; con `settings.json` esa
apuesta es la compuerta dura, y la compuerta dura no se apuesta.

**Lo que el barrido no arregla, dicho aquí y no descubierto después**: el
canal de la última sesión de la máquina sobrevive hasta que se abra otra.
Acotar eso del todo exige (a) o un árbol de procesos que muera entero, que
es de `sandbox-propio` y ya está anotado ahí. La delta de `own-adapters`
recoge la regla en el requisito de permisos, con su escenario, y el
comentario del código dice ahora lo que de verdad pasa.
