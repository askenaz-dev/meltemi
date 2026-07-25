## Context

La flota (catálogo embebido + detección pasiva) funciona exactamente como se
especificó y, aun así, miente en la práctica: el mantenedor tiene instalados los
CLI oficiales de dos proveedores y la vista Flota los muestra "no detectados".
La causa es que en las entradas de nivel 2 `bin` nombra **el adaptador ACP** —
el único punto de entrada que `meltemid` puede pilotar hoy (`fleet.rs`
`detect()`/`resolve_agent_command`) — de modo que la detección responde sobre
una capa distinta de la que el usuario instaló. El "no" es técnicamente
honesto y comunicativamente inútil: no dice qué falta ni cómo resolverlo.
Se suma un segundo hueco real: el sondeo de Windows está acotado a
`.exe/.cmd/.bat` (`WINDOWS_EXTS`) y los shims de npm/nvm también existen como
`.ps1`, que hoy no se ven en absoluto. Y no hay ninguna guía por agente: para
un producto BYO-agent, la primera pantalla de flota **es** el onboarding, y el
repositorio no ofrece a GitHub ni una página que diga el comando exacto.

## Goals / Non-Goals

**Goals:** detección de dos capas (CLI oficial del proveedor + adaptador ACP)
con estados compuestos honestos; remedio accionable por capa con el comando
exacto en las tres superficies; sondeo de Windows que reconozca los shims de
script como evidencia de instalación; estatus legal de la ruta de integración
mostrado sin maquillaje; guía de agentes verificada contra el registro para que
no pueda envejecer en silencio.
**Non-Goals:** instalar adaptadores por el usuario — Meltemi muestra el
comando, nunca lo ejecuta (constitución §3: sin efectos externos silenciosos);
cambiar los niveles declarados o la política de niveles (eso es de la suite de
conformidad); método RPC nuevo; el sitio web (`sitio-web-producto` reutiliza la
guía).

## Decisions

### D1 — Las dos capas se declaran en el registro; `bin` no cambia de significado
El registro suma claves opcionales por entrada: `cli-bin` (+
`cli-candidate-paths`, `cli-install`) para la capa del CLI oficial del
proveedor, `adapter-install` para la capa del adaptador, y `legal-status` /
`legal-note` (D6). `bin` conserva exactamente su semántica vigente — lo que la
detección resuelve y el lanzamiento ejecuta — porque cualquier otra cosa
reescribiría el contrato de `resolve_agent_command` y de la política de niveles
sin necesidad. De ahí la regla de composición: WHERE la entrada declara
adaptador, sus capas son `cli` (desde `cli-bin`) y `adapter` (desde `bin`);
en otro caso hay una sola capa `cli` (desde `bin`). Los datos de terceros
—nombres de producto, comandos de instalación, notas legales— viven en el
registro, jamás en las specs, siguiendo la convención ya vigente del snapshot.
Alternativa rechazada: invertir `bin` para que nombre el CLI oficial y añadir
`adapter-bin`; habría hecho que el catálogo declarase "detectado" un agente que
el daemon no puede pilotar, exactamente la deshonestidad opuesta a la que
motiva esta change.

### D2 — Cinco estados compuestos, y `detected` sigue significando "pilotable"
La detección resuelve cada capa por separado y compone un estado único por
entrada: `ready` (el punto de pilotaje está, y el CLI oficial también cuando la
entrada declara dos capas), `adapter_missing` (el CLI oficial está, el
adaptador no: el bug del mantenedor), `cli_missing` (el adaptador está, el CLI
oficial no), `not_detected` (ninguna capa) y `not_launchable` (hay evidencia de
instalación pero ningún objetivo ejecutable, D4). El campo `detected` MANTIENE
su significado actual —el punto de entrada que el daemon puede pilotar— para
que `fleet/list`, el error 2001 y la selección por id sigan coherentes: un
agente cuyo CLI existe pero cuyo adaptador falta se reporta `detected: false`
con estado `adapter_missing` y remedio. Cambiar `detected` a "hay algo
instalado" habría hecho que la superficie prometiera un lanzamiento que el
daemon rehúsa un segundo después. Estados en inglés porque son identificadores
del contrato; su traducción vive en los catálogos de mensajes.

### D3 — Campos aditivos: las capas como lista, no como pares de banderas
`FleetAgent` suma `layers[]` (por capa: `kind` `cli`|`adapter`, nombre del
binario declarado, `detected`, `binaryPath` cuando existe, `evidenceOnly`
cuando el hallazgo es solo evidencia, `install` con el comando), más
`installState`, `remedy`, `remedyCommand`, `legalStatus` y `legalNote`. Una
lista, y no `cliDetected`/`adapterDetected`, porque las entradas de una sola
capa quedan descritas por el mismo mecanismo, la capa `headless` de nivel 3
cabe mañana sin otro campo, y "instalado pero no lanzable" es un atributo de la
capa concreta, no del par. Todo es aditivo y opcional: `fleet/list` no gana
requisitos y ningún cliente previo se rompe. No hay método RPC nuevo, así que
la matriz de paridad no gana filas — la paridad de esta change es de **render**
(D8), no de superficie de contrato.

### D4 — `.ps1` es evidencia de instalación, jamás objetivo de lanzamiento
El sondeo de Windows se parte en dos conjuntos: el de **lanzamiento**
(`.exe`, `.cmd`, `.bat`, el actual) y el de **evidencia**, que lo extiende con
`.ps1`. Un hallazgo solo en el conjunto de evidencia marca la capa como
instalada con `evidenceOnly` y compone `not_launchable` con su remedio; nunca
se devuelve como ruta a ejecutar. Razón: `CreateProcess` no ejecuta un `.ps1`
directamente; lanzarlo exigiría interponer un intérprete (`powershell -File`)
en el canal stdio de ACP, quedar a merced de la política de ejecución del
sistema y reabrir el problema de citado de argumentos — tres riesgos nuevos
para pilotar un binario oficial que en su instalación normal también deja un
shim ejecutable. Alternativa rechazada: añadir `.ps1` a `WINDOWS_EXTS` sin
distinguir; habría producido rutas que el daemon intenta lanzar y el sistema
rechaza, sustituyendo un "no detectado" honesto por un fallo opaco al abrir
sesión. Alternativa rechazada: leer `PATHEXT` completo del entorno; el conjunto
acotado mantiene la detección barata, determinista y testeable en las tres
plataformas.

### D5 — El remedio es dato del registro: se muestra, nunca se ejecuta
El comando de instalación por capa vive en el registro (D1) y viaja en
`remedyCommand`; la frase del remedio se compone del estado y la capa que
falta. Una sola fuente sirve a la vez a las superficies y a la guía (D7), así
que no puede haber dos verdades. Meltemi MUST NOT ejecutarlo: mostrar el
comando es información, correr un instalador es un efecto externo irreversible
que el usuario no pidió (constitución §3, y alcance excluido de la proposal).
Por coherencia, el rehúso 2001 de un id cuya capa de pilotaje falta nombra la
capa ausente y su comando en `remedy`, en vez del texto genérico de hoy: el
mismo diagnóstico, en el momento en que más importa.

### D6 — Estatus legal declarado y mostrado sin maquillaje
El registro declara por entrada `legal-status` (`sanctioned`|`tolerated`|
`grey`) y `legal-note` (una frase corta, tomada del research interno de
integración), y las superficies la muestran junto al remedio, tal cual, WHERE
la entrada la declara. Es la regla derivada que el propio research ya fija y
que la constitución §2 exige: cuando la ruta de integración de un proveedor
está en zona gris para las suscripciones de consumo, la nota lo dice y señala
el camino seguro —pilotar el binario oficial— en el mismo lugar donde el
usuario está a punto de instalar algo. Alternativa rechazada: omitirla y dejar
el juicio al usuario "para no sesgar"; ocultar una restricción conocida del
proveedor mientras se le ofrece el comando que la roza es precisamente lo que
§2 prohíbe.

### D7 — La guía se verifica contra el registro; no se genera
`docs/agentes.md` es prosa con juicio —qué significa cada nivel, notas
legales, síntomas de detección por sistema operativo— y eso no lo escribe un
generador. Pero sus **hechos** (entradas, nivel, binarios de cada capa,
comandos de instalación) salen del registro, y un test en el espíritu del gate
de frescura ya vigente para la referencia CLI verifica la biyección: cada
entrada del registro tiene su sección y cada sección nombra una entrada que
existe, con el nivel y los binarios que el registro declara; además los
ejemplos de configuración de perfiles deben parsear como configuración válida.
Alternativa rechazada: generar la guía entera (pierde exactamente la parte que
la hace útil); alternativa rechazada: guía manual revisada a ojo (es el modo de
fallo que originó esta change). Idioma: inglés, siguiendo el precedente del
README como primer contacto público del repositorio; los artefactos del método
siguen en español (§11) y el espejo en español queda como pregunta abierta.
Detalle real que la implementación debe respetar: el lint de documentación
prohíbe nombres de productos de terceros **en el README**, así que el enlace se
redacta sin nombrarlos, y la guía se suma a la lista de documentos cuyo enlazado
interno se verifica.

### D8 — Paridad de render en las tres superficies, sin método nuevo
El mismo estado compuesto, la misma capa faltante, el mismo comando y la misma
nota legal se renderizan en la CLI (`fleet` humano y `--json`), en la vista
Flota de la TUI (glifo + palabra, nunca color solo) y en el drawer de detalle
de la GUI que la superficie de escritorio ya tiene. En la GUI el render sigue
el design system normativo de `design-system/`: nivel como pill, detección como
dot + palabra, filas de 32 px, celdas de 8 px, radios 4/8, hairlines de 1 px y
un único nivel de sombra; el comando se ofrece copiable y ni la bandeja de
permisos ni los banners de señal animan layout. Las etiquetas pasan por los
catálogos ES/EN; el comando del remedio viaja como dato y MUST NOT traducirse.

## Risks / Trade-offs

- **Los comandos de instalación envejecen** (paquetes que se renombran) → son
  dato en un único lugar, la guía se verifica contra ese lugar y la corrección
  es un cambio de datos sin tocar código; y como nunca se ejecutan, un comando
  obsoleto informa mal, no rompe nada.
- **`ready` puede sugerir "autenticado"** → no lo afirma: la autenticación es
  del binario oficial y un fallo se muestra tal cual (§2, postura vigente); el
  estado habla de presencia de capas, y así se etiqueta en las superficies.
- **Más superficie de datos de terceros en el registro** → aceptado y acotado a
  datos factuales de interoperabilidad, que es dónde el proyecto ya decidió que
  vivan; las specs siguen sin nombrar productos.
- **Registros sustituidos por el usuario sin las claves nuevas** → todas son
  opcionales: un override vigente sigue parseando y compone una sola capa.
- **Windows como camino distinto** → el conjunto de evidencia solo existe ahí;
  se cubre con tests `cfg(windows)` sobre directorios fixture y no altera el
  comportamiento en macOS/Linux (§7, Windows primera clase).

## Migration Plan

Aditivo por completo: claves de registro opcionales, campos de `fleet/list`
opcionales, `detected` y el error 2001 con su significado intacto. Las entradas
de una sola capa reportan exactamente lo que reportan hoy más su capa única.
Reversión: retirar los campos aditivos, las claves del registro y la guía; la
detección vuelve al comportamiento previo sin migración de datos, porque no hay
estado persistido nuevo.

## Open Questions

- ¿Espejo en español de la guía (`docs/agentes-es.md`) o queda cubierto por
  `i18n-superficies`? Se decide cuando exista el catálogo de mensajes común.
- Para una entrada de dos capas en estado `adapter_missing`, ¿tiene sentido
  ofrecer la ruta headless del CLI oficial como plan B? Toca la política de
  niveles, que pertenece a la suite de conformidad: queda fuera hasta que esa
  change tenga evidencia.
- ¿Comandos de instalación por gestor de paquetes o por sistema operativo? Hoy
  uno por capa; se abre a tabla por SO si la fricción real lo pide.
