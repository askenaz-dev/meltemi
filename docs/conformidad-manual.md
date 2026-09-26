# Conformidad contra los CLIs reales — corrida manual y opt-in

Todo lo que corre en CI es un fixture. Un fixture es una observación
congelada, y una observación congelada no dice nada sobre si el contrato que
congeló **sigue vigente**: lo único que puede responder eso es el binario
oficial que tiene instalado la persona. Esta página es ese procedimiento.

La corrida vive fuera de CI por dos razones que no se negocian: la
constitución §5 prohíbe que los tests dependan de la cuenta de un proveedor, y
§2 exige que la autenticación viva en el CLI oficial — de modo que correrla
gasta una sesión real en la cuenta de quien la corre. Nadie queda inscrito por
omisión.

## Cuándo se corre

- Antes de subir el piso o el techo de versión soportada de un adaptador.
- Cuando se re-ancla el volcado de esquema vendorizado
  (`core/mock-provider/schemas/`), que es lo que dice qué versión se soporta.
- Cuando un proveedor publica una versión mayor de su CLI.
- Cuando alguien sospecha que un fixture dejó de parecerse a la realidad. Esa
  sospecha se resuelve corriendo esto, nunca leyendo el fixture otra vez.

## Requisitos

1. El **toolchain del repo** (`rust-toolchain.toml`) y el workspace construido:

   ```
   cargo build --workspace
   ```

2. El **CLI oficial del proveedor instalado y con sesión iniciada**, con su
   propia autenticación. Meltemi no lee, no guarda y no inyecta credencial
   alguna: si el CLI no está autenticado, la corrida falla y eso es correcto.
   Verificar a mano, una vez, antes de empezar:

   | Plataforma | Comprobación |
   | --- | --- |
   | Windows (PowerShell) | `Get-Command claude, codex \| Select-Object Name, Source` |
   | macOS / Linux | `command -v claude codex` |

   Y que la flota los vea, que es la misma detección que usará la corrida:

   ```
   meltemi fleet --json
   ```

3. Nada más. La corrida **no** necesita una instalación de Meltemi: usa los
   adaptadores que este checkout acaba de construir. Esa es su única
   diferencia con lo que hace una persona instalada, donde los adaptadores
   viajan junto al daemon.

## Cómo se corre

El opt-in es explícito y doble: la variable de entorno y el flag `--ignored`.
Sin las dos, el test no hace absolutamente nada.

**Windows (PowerShell)**

```powershell
$env:MELTEMI_CONFORMANCE_REAL = "1"
$env:MELTEMI_CONFORMANCE_AGENT = "claude-code"   # opcional: una sola entrada
cargo test -p meltemid --test conformance_real -- --ignored --nocapture
```

**macOS / Linux**

```bash
MELTEMI_CONFORMANCE_REAL=1 \
MELTEMI_CONFORMANCE_AGENT=claude-code \
cargo test -p meltemid --test conformance_real -- --ignored --nocapture
```

`MELTEMI_CONFORMANCE_AGENT` acota la corrida a una entrada del catálogo
(`claude-code`, `codex-cli`); sin ella corren todas las que tengan su CLI
presente. Existe porque cada dialecto gasta un turno en la cuenta de un
proveedor distinto, y quien re-ancla uno no tiene por qué pagar el otro.

**Qué cuesta**: **dos sesiones reales por entrada**, no una. La corrida tiene
dos piernas, porque los cuatro criterios que el nivel 2 declara no caben en una
sola sesión:

1. Un turno que llega hasta el final — `streaming`, `permissions`, `session`.
2. Un turno que se para en cuanto el CLI habla dentro de él — `cancellation`.

La segunda se diseñó como la barata («cortada en sus primeras palabras») y
medida contra el CLI real **no lo es**: cuando la sesión ve la primera palabra
el proveedor ya produjo casi todo el turno, y su costo quedó a un décimo del de
la primera. Presupueste dos turnos completos por dialecto. Si la sesión no
llega a abrirse, no se gasta nada: el turno nunca se envía.

## Qué queda registrado, y dónde

El resultado se persiste como JSONL apend-only en el directorio de datos del
usuario, en `conformance/results.jsonl`:

| Plataforma | Ruta |
| --- | --- |
| Windows | `%APPDATA%\meltemi\data\conformance\results.jsonl` |
| macOS | `~/Library/Application Support/meltemi/conformance/results.jsonl` |
| Linux | `~/.local/share/meltemi/conformance/results.jsonl` |

Una línea por corrida, con su fecha y la versión del CLI que respondió:

```json
{
  "agentId": "claude-code",
  "verifiedLevel": 2,
  "agentVersion": "2.1.167",
  "runAt": "2026-07-31T13:41:21Z",
  "criteria": [
    { "level": 2, "name": "streaming", "passed": true },
    { "level": 2, "name": "session", "passed": true },
    { "level": 2, "name": "permissions", "passed": true },
    { "level": 2, "name": "cancellation", "passed": true }
  ]
}
```

Dos reglas de lectura, ambas deliberadas:

- **Un criterio que la corrida no pudo ejercer no aparece.** No se reporta
  como aprobado ni como fallido: no se reporta. El nivel solo se otorga
  cuando *todos* los criterios que ese nivel declara están presentes y
  aprobados, así que una corrida incompleta produce un resultado incompleto en
  vez de uno halagador.
- **`verifiedLevel: 0` es un resultado**, no un error de la corrida. Significa
  que contra ese binario, ese día, el nivel no quedó verificado.

Las dos reglas juntas tienen una consecuencia que conviene decir en voz alta,
porque el número solo no la cuenta: un 0 por «nada funcionó» y un 0 por «falta
un criterio por ejercer» se leen igual en el número y no significan lo mismo.
La lista de `criteria` es la que lo distingue, y por eso se persiste entera.

La segunda pierna existe precisamente por eso. Hasta el 2026-07-31 la corrida
reportaba tres criterios y el nivel 2 declara cuatro: `cancellation` no se
ejercía **nunca**, de modo que el nivel era inalcanzable por construcción y no
por lo que la corrida encontrase. Un procedimiento manual que no puede otorgar
el nivel que documenta no es un procedimiento. Regla que la pierna nueva trae:
si el turno nunca llegó a estar en vuelo, no se envió paro alguno y el criterio
**no se reporta** — un fallo ahí sería un hallazgo sobre la corrida, no sobre
el puente.

`meltemi fleet` lee la última corrida por entrada y muestra el nivel
verificado junto al declarado. Declarado ≠ verificado es información, no
vergüenza.

## Apagado entero contra un CLI instalado por npm (Windows, opt-in)

Esta comprobación no es un test: es un procedimiento de cinco minutos que
alguien corre a mano, en Windows, contra un CLI real. Existe porque lo que la
suite puede probar y lo que hay que creer no coinciden aquí. Los tests lanzan
un `.cmd` que ellos mismos escriben; lo que un instalador de npm genera es un
script que Meltemi no controla y que puede cambiar con la versión del CLI, del
gestor de paquetes o de Node. Un ensayo contra el shim real es el único que
responde por el shim real.

**Se corre**: cuando se toca el lanzamiento o el apagado de agentes, y cuando
se sube la versión de un CLI de nivel 1 que instale por npm.

**Qué hace falta**: Windows, un CLI de nivel 1 instalado con `npm i -g`, y una
sesión abierta en él.

1. Confirme que la ruta que Meltemi va a lanzar es un shim y no un ejecutable.
   Un `.cmd` o un `.bat` es un shim; un `.exe` no, y entonces esta comprobación
   no tiene nada que medir en ese agente.

   ```powershell
   (Get-Command claude).Source
   ```

2. Abra una sesión con ese agente, desde cualquier superficie, y deje que
   empiece a trabajar.

3. Con la sesión en vuelo, mire el árbol. Anote los identificadores: el del
   shim y el del proceso que cuelga de él.

   ```powershell
   Get-CimInstance Win32_Process | Where-Object { $_.Name -in 'cmd.exe','node.exe' } | Select-Object ProcessId, ParentProcessId, CommandLine | Format-List
   ```

4. Cancele la sesión.

5. Repita el paso 3. **Ninguno de los identificadores anotados debe seguir
   ahí.** Que desaparezca el shim y sobreviva el proceso de debajo es
   exactamente el defecto que el ámbito de proceso previene, y verlo aquí
   significa que el ámbito no se aplicó a esa ruta de lanzamiento.

6. La otra mitad, la que un apagado ordenado no ejercita: repita del 2 al 3, y
   en lugar de cancelar, **mate el daemon sin darle ocasión de limpiar**.

   ```powershell
   Stop-Process -Name meltemid -Force
   ```

   Vuelva al paso 3. Tampoco debe quedar nada: el sistema cierra los handles
   del proceso muerto, y el ámbito se lleva consigo lo que tenía dentro. Si
   sobrevive algo, el ámbito no se abrió o el agente no llegó a entrar en él.

**Qué se registra**: el resultado va abajo, en la corrida del día, con el CLI,
su versión, la fecha y qué se observó — nunca «verificado» a secas. Un
resultado negativo es un hallazgo, no un fallo del procedimiento.

## Qué campo pobló cada agente para sus modos

El protocolo deja anunciar los modos de sesión de dos maneras: por el campo de
modos, que llegó primero, y como una opción de configuración de categoría modo,
que llegó después. Meltemi lee las dos y da precedencia a la segunda, pero
**cuál usa cada CLI real es una medición, no una suposición** — y de eso
depende que la síntesis del campo viejo siga haciendo falta o sea código muerto
esperando a que alguien lo note.

Para cada agente de nivel 1 instalado: abra una sesión ACP y anote cuál de los
dos campos vino poblado en la respuesta de apertura — modos, opciones de
configuración, las dos, o ninguno — con la versión del CLI y la fecha. El log
de la sesión lo dice sin herramientas extra: el evento `config_options_announced`
lleva lo anunciado ya traducido, y una opción de id `mode` y categoría `mode`
que el agente no anunció como tal es la señal de que vino del campo viejo.

El resultado se anota abajo, en la corrida del día.

## Pierna de proveedor local — OpenCode contra un modelo de la propia máquina

**Estado: pendiente.** Verificada en forma el 2026-09-25 con OpenCode 1.18.32:
con la receta 1 de [`docs/local-models.md`](local-models.md), `opencode models
ollama` lista `ollama/qwen2.5-coder:7b`, y a través de Meltemi, con su perfil,
OpenCode anuncia ese modelo como vigente, `--model ollama/qwen2.5-coder:7b` se
fija y se confirma antes del primer prompt y un modelo no anunciado se rehúsa
sin prompt. Sin servidor de modelos el turno falla a los ~70 s con el «Cannot
connect to API» de OpenCode. Nunca corrida en vivo: la máquina que la escribió
no tiene servidor de modelos. Quien la corra reemplaza este párrafo por su
resultado; hasta entonces la pierna no está aprobada.

Es una pierna de **nivel 1**, a mano y por opt-in como todo lo de esta página:
el arnés automatizado (`conformance_real`) cubre los dos adaptadores propios, y
automatizar un agente nativo es de `corrida-integrada`. No gasta la cuenta de
nadie: el modelo corre en la máquina.

**Requisitos**: el servidor de modelos instalado y sirviendo, con el modelo
bajado (`winget install Ollama.Ollama`; `ollama pull qwen2.5-coder:7b`);
OpenCode 1.18.32 o posterior; y la receta 1 como el perfil `opencode-ollama` de
un repositorio fixture — nunca este.

| Criterio | Cómo se ejerce | Pasa si |
| --- | --- | --- |
| sesión | `meltemi session "resume el README" --agent opencode-ollama` en el fixture | la sesión abre, el log trae `agent_resolved` y `config_options_announced`, y cierra con `session_ended` |
| streaming | el mismo turno | llegan fragmentos del agente (`agent_update` con `agent_message_chunk`) antes del cierre, y el turno no se declara vacío |
| permisos | `meltemi session "crea NOTAS.md con una línea" --agent opencode-ollama`, en un terminal | el aprobador pregunta con las opciones del agente y la decisión queda en el log (`permission_decided`) |
| cancelación | otra sesión que pida escribir; **Ctrl+C** en la pregunta del aprobador | la sesión se cancela antes que nada, el turno cierra `cancelled` y el comando sale con `13` |

**Qué se registra**, en lugar del estado de arriba: la fecha, `opencode
--version`, la versión del servidor de modelos, el modelo, el sistema, el
resultado por criterio y los tiempos medidos —hasta el primer fragmento y el
turno completo—. Un criterio que no se pudo ejercer no se escribe, y no se
escribe como aprobado.

## Cómo se registra en el método

Los escenarios que **solo** un CLI real ejerce se marcan con nota, fuente y
fecha. El verbo vive en las paletas interactivas de la TUI y la GUI; desde un
script se llama al método directamente:

```
cargo run -q -p meltemi --example rpc -- sdd/verify-mark '{
  "projectRoot": "<ruta del repo>",
  "change": "<change>",
  "scenario": "<nombre exacto del escenario>",
  "note": "Verificado a mano contra <CLI> <versión> el <fecha>: <qué se observó>."
}'
```

La nota es el registro. Debe decir contra qué binario, en qué versión, en qué
fecha y qué se observó — nunca «verificado» a secas, y nunca de memoria.

## Formas de anuncio de modo — corrida del 2026-09-18

Windows 11 (26200), x86_64. Corrida de apertura de sesión únicamente: se envía
`initialize` y `session/new`, se lee la respuesta y se cierra la entrada.
**No se envía prompt**, así que no gasta turno de proveedor.

**Inventario del equipo**: de las seis entradas de nivel 1 del catálogo, solo
una está instalada — `opencode`. `gemini`, `copilot`, `agent` (Cursor),
`kiro-cli` y `kilo` no se encuentran en este equipo, así que no se midieron y
esta página no dice nada sobre ellas. `claude 2.1.261` y `codex-cli 0.77.0`
están instalados pero son de **nivel 2**: no responden ACP por sí mismos, sino
a través de nuestros adaptadores, y quedan fuera del alcance de esta
comprobación (design D8 acota a nivel 1).

### `opencode 1.14.33`: **las dos formas, pobladas**

| Campo | Poblado | Contenido |
| --- | --- | --- |
| `modes` | **sí** | `build` (actual) y `plan`, con descripción |
| `configOptions` | **sí** | `model` (categoría `model`, 317 valores) y `mode` (categoría `mode`, 2 valores, actual `build`) |

Tres cosas que esta medición decide, y que no se podían decidir suponiendo:

1. **La precedencia de D6 no es teórica: es el caso real.** El único agente de
   nivel 1 instalado puebla las dos formas a la vez, con los mismos dos modos
   en ambas. Fusionarlas habría producido dos selectores de modo, o uno con
   valores duplicados. La regla «gana la opción de configuración, entera» deja
   exactamente un selector, que es lo que el agente ofrece.
2. **La síntesis del campo viejo no se ejercita contra este agente**, y sigue
   haciendo falta igual: es la rama para el agente que puebla `modes` y **no**
   `configOptions`, que es el agente nacido antes de que las opciones de
   configuración existieran. Ninguno de los instalados aquí lo es. Queda
   cubierta por los tests unitarios y por el agente simulado bajo `--modes`,
   y esta página dice explícitamente que ningún binario real la ha ejercido
   todavía.
3. **`modelo-y-esfuerzo-por-sesion` D9 quedó desactualizada.** Concluyó que
   «ningún proveedor pineado anuncia opciones de sesión»; `opencode 1.14.33`
   anuncia dos, una de ellas con 317 modelos. No es un error de aquella
   conclusión sino su fecha: lo que no anunciaba entonces, anuncia ahora.

## Última corrida — 2026-09-26 (UTC)

Windows 11 Pro (26200), x86_64, desde el taller de `corrida-integrada`. Dos
fuentes, y se dice cuál es cuál: el arnés `conformance_real` para el dialecto de
Claude Code, y la [corrida integrada](qa/2026-09-26-corrida-integrada.md) —la
matriz de siete verbos de `scripts/smoke-agents.ps1`— para OpenCode, que es
nativo y el arnés no cubre.

### Dialecto de sesión headless — `claude 2.1.280`: **nivel verificado 2**

`MELTEMI_CONFORMANCE_AGENT=claude-code`, a las 00:46Z. Las cuatro comprobaciones
se ejercieron contra el binario real y las cuatro aprobaron: `streaming`,
`session`, `permissions` y `cancellation`. El paro se respondió en **10,07 s**
como `cancelled`, sin nada corriendo después: la ruta de abandono, la misma que
la corrida del 2026-07-31 midió en 10,1 s. Reemplaza, para esta versión, el
nivel verificado contra `2.1.167`.

Lo que la conformidad no ve y la corrida integrada sí: las peticiones de este
adaptador llegan sin `rawInput` ni `locations`, de modo que una regla de
permisos acotada por ruta no puede concederlas (hallazgo 2 del informe;
`adaptadores-con-rawinput`). El criterio `permissions` mide que la petición
llegue y se decida, no que una regla la reconozca.

### Nativo — `opencode 1.18.32`: observado, sin nivel verificado

Desde la corrida integrada, no desde el arnés, y por eso no se persiste como
resultado de conformidad:

| Criterio | Lo observado |
| --- | --- |
| sesión | abre, anuncia sus opciones y cierra con `session_ended`; se reanuda con `direct` |
| streaming | fragmentos del agente antes del cierre en todas las sesiones, y uso reportado por el protocolo (contexto y costo 0 USD) |
| permisos | **no pide ninguno**: edita sin preguntar, con y sin regla (hallazgo 1 del informe). El proxy no lo gobierna en su configuración por omisión |
| cancelación | `cancel` sobre una sesión viva la terminó en 9,3 s |

Con un criterio que falla, el nivel 1 no queda verificado, y la página lo dice
en vez de callar la fila.

### No medidos

Los nivel 1 del catálogo que no están instalados en esta máquina: Gemini CLI,
GitHub Copilot CLI, Cursor CLI, Kiro CLI y Kilo Code. Tampoco Aider (nivel 3,
que el daemon no lanza hoy) ni Antigravity (nivel 4). Ninguno se da por
aprobado ni por fallido: no se midieron.

## Corrida del 2026-09-25 (UTC)

Windows 11 (26200), x86_64. Solo el dialecto de servidor
(`MELTEMI_CONFORMANCE_AGENT=codex-cli`): es la corrida que exige el re-anclaje
de `codex-al-dia` al volcado de 0.156.1 (escenario «El re-anclaje exige la
corrida real»). CLI con sesión iniciada en una cuenta ChatGPT, y la
configuración del mantenedor, que arranca tres servidores MCP propios.

### Dialecto de servidor JSON-RPC — `codex-cli 0.156.1`: **nivel verificado 2**

Las cuatro comprobaciones del nivel se ejercieron contra el binario real, y las
cuatro aprobaron.

| Comprobación | Resultado | Lo observado |
| --- | --- | --- |
| `session` | aprueba | El handshake leyó 0.156.1 del primer token del user agent, `meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64) xterm-256color (meltemi-codex-acp; 0.1.1)`. La procedencia registra `providerVersion` y `providerVerifiedVersion` iguales, así que no hubo fila de distancia. |
| `streaming` | aprueba | El turno completo corrió hasta su fin y el mensaje del agente llegó en fragmentos. |
| `permissions` | aprueba | Seis aprobaciones llegaron al proxy: cinco comandos y una escritura. El decisor de la corrida concedió cada una «una vez», como lo haría un humano en la bandeja. |
| `cancellation` | aprueba | El paro se respondió en 56 ms como `cancelled`, y no quedó sesión activa. |

- **Compuerta efectiva observada**, tal como la registró la procedencia de las
  dos sesiones: `approvalPolicy` `untrusted`, `approvalsReviewer` `user` y
  `sandbox` `{"type":"readOnly","networkAccess":false}`. Es exactamente la
  compuerta pedida.
- **La escritura aprobada.** La propuesta se escribió bajo el sandbox de solo
  lectura, en `.meltemi/changes/<change>/proposal.md` del proyecto de prueba,
  tras una aprobación de cambio de archivo que el proxy concedió. Confirma, en
  una sesión pilotada por Meltemi, lo que el riesgo R5 midió el 2026-09-24 con
  un cliente mínimo.
- **Todos los comandos pidieron permiso, lecturas incluidas.** En Windows el CLI
  envuelve cada comando en `pwsh.exe -Command …`, y ninguno cayó en la lista de
  lecturas de confianza de la política `untrusted`. La fila «Codex runs some
  things without asking the permission proxy» sigue siendo cierta como cota
  superior; en esta plataforma y en esta corrida, ningún comando corrió sin
  decisión del proxy.
- Un comando leyó fuera del proyecto, en el directorio de skills del usuario
  (`~/.agents/skills/`). Lo hizo con aprobación: el sandbox de solo lectura no
  impide leer, y el proxy lo vio y decidió.
- **Consecuencia de la configuración MCP.** Las dos sesiones declararon, antes
  del prompt, la fila de lo que la compuerta no cubre y una fila por cada
  servidor MCP que la configuración del mantenedor arranca: `codex_apps`,
  `cua_repl` y `node_repl`. Ninguna llamada a esos servidores apareció en el
  turno. De haber ocurrido sin aprobación, habría corrido fuera del proxy, que
  es justo lo que esas filas declaran.
- **Tiempos**, con el reloj del log de sesión (resolución de un segundo):

  | Tramo | Duración |
  | --- | ---: |
  | Apertura: lanzamiento, handshake, hilo y compuerta | 3 s |
  | Turno completo, del prompt al cierre | 54 s |
  | Pierna del paro, del prompt a `cancelled` | 7 s |
  | Del paro a su respuesta | 56 ms |

- **El nivel se otorga con esta corrida.** `fleet/list` reporta `codex-cli`
  con nivel verificado 2 y `verifiedVersion` `0.156.1`.

### La corrida fallida de `codex-cli 0.77.0`, que esta reemplaza

La corrida del 2026-07-31, abajo, queda como la evidencia de por qué se
re-ancló. Con 0.77.0 el proveedor rechazaba cada turno con sus propias
palabras: *«The 'gpt-5.6-sol' model requires a newer version of Codex. Please
upgrade to the latest app or CLI and try again.»* `streaming` falló citando
ese motivo, y el nivel quedó en 0 (escenario «Una comprobación fallida se
registra con las palabras del proveedor»). Desde `codex-al-dia` el piso del
adaptador es 0.156.1: un CLI 0.77.0 ya no llega al turno, porque el handshake
lo rehúsa con el remedio de actualizarlo.

## Corrida del 2026-07-31

Windows 11 (26200), x86_64. Ambos CLIs presentes y con sesión iniciada. Primera
corrida con las dos piernas.

### Dialecto de sesión headless — `claude 2.1.167`: **nivel verificado 2**

Los cuatro criterios ejercidos y aprobados, por primera vez.

- `streaming`, `session` y `permissions` **aprueban** como el 2026-07-28, y por
  las mismas razones; la nota de aquella corrida sobre cuál de los dos canales
  de permiso quedó ejercido —el hook, no el prompt-tool— **sigue siendo exacta
  y sigue vigente**.
- `cancellation` **aprueba**: la segunda sesión se paró en cuanto el CLI habló
  dentro de su turno, `propose` respondió `cancelled` y no quedó sesión alguna
  activa. Es el criterio que ninguna corrida anterior había ejercido.
- **Dos observaciones distintas del mismo binario, y la diferencia importa.**
  El paro se respondió en **4,1 s** en la corrida de las 13:09Z y en **10,1 s**
  en la de las 13:41Z. La gracia del turno cancelado son 5 s y la del apagado
  otros 5: por debajo de la primera, el CLI cerró su salida por su cuenta —vio
  el fin de entrada y terminó su turno—; alrededor de la suma de las dos, no lo
  hizo y el adaptador lo terminó. El criterio mide lo mismo en los dos casos
  (el turno para y no queda nada corriendo) y las dos rutas son las que el
  escenario «Turno cancelado terminado aunque el proveedor no lo atienda»
  describe: la de abandono no es teórica, ocurre.
- **Costo real, medido en los archivos de sesión del propio CLI** (cuatro
  sesiones, las dos corridas del día):

  | Pierna | Entrada | Escritura de caché | Lectura de caché | Salida |
  | --- | ---: | ---: | ---: | ---: |
  | 13:09Z turno | 3 648 | 20 263 | 70 324 | 1 670 |
  | 13:09Z paro | 4 436 | 9 024 | 84 774 | 1 913 |
  | 13:41Z turno | 4 397 | 8 658 | 64 503 | 1 842 |
  | 13:41Z paro | 4 464 | 8 237 | 64 079 | 1 667 |

  La pierna del paro cuesta prácticamente lo mismo que la del turno completo.
  Se esperaba que fuera la barata; no lo es, y la página lo dice donde se
  presupuesta.

### Dialecto de servidor JSON-RPC — `codex-cli 0.77.0`: **nivel verificado 0**

Nivel no otorgado, y el motivo no está en el puente. Lo que la corrida sí
estableció y lo que no, en orden:

- **Un defecto real, que solo esta corrida podía encontrar**: el CLI no se podía
  lanzar. El catálogo lo daba por presente —y tenía razón— y el adaptador
  rehusaba con «`codex` could not be launched (program not found)». La causa es
  de Windows: `npm i -g` deja `codex.cmd` y ningún `codex.exe`, y
  `CreateProcess` sólo añade `.exe` a un nombre pelado. Corregido en el
  adaptador (design D14): el nombre declarado se resuelve al archivo que la
  plataforma ejecuta, con el mismo conjunto de extensiones que usa el catálogo.
- Tras la corrección, `session` **aprueba**: el CLI se lanza, el handshake
  responde y el adaptador lee `0.77.0`, dentro del rango declarado
  `[0.77.0, 1.0.0)`. Lanzamiento, handshake y lectura de versión quedan
  verificados contra el binario real.
- `streaming` **falla**, y el que dice que no es el proveedor: todo turno vuelve
  con *«The 'gpt-5.6-sol' model requires a newer version of Codex. Please
  upgrade to the latest app or CLI and try again.»* El modelo por defecto de la
  cuenta exige un Codex más nuevo que el 0.77.0 que esta change ancló. El
  adaptador propaga ese rechazo con las palabras del proveedor y su remedio, que
  es exactamente lo que debe hacer; no hay turno que streamear.
- `permissions` y `cancellation` **no se reportan**: sin turno en vuelo no hubo
  a qué pedir permiso ni qué parar.
- **Qué haría falta para otorgarle el nivel**: subir el CLI a una versión que
  sirva el modelo por defecto de la cuenta y **re-anclar el volcado de esquema
  vendorizado** a esa versión — que es uno de los disparadores que esta misma
  página lista. Ni el uno sin el otro: el esquema congelado es lo que dice qué
  versión se soporta.
- El re-anclaje del esquema y el handshake del 2026-07-28 siguen vigentes: se
  verificaron contra este mismo `0.77.0`.

## Corrida anterior — 2026-07-28

Windows 11 (26200), x86_64. Ambos CLIs presentes y con sesión iniciada.

### Dialecto de servidor JSON-RPC — `codex-cli 0.77.0`

- `codex app-server generate-json-schema --out <dir>` produce 129 archivos; los
  21 vendorizados en `core/mock-provider/schemas/codex-app-server/` son
  **idénticos byte a byte** al volcado fresco. El contrato congelado sigue
  siendo el contrato.
- El handshake real responde
  `codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) xterm-256color (meltemi-codex-acp; 0.1.0)`
  al mismo `initialize` que envía el adaptador — el formato de user agent del
  que el adaptador lee la versión, confirmado contra el binario. `0.77.0` cae
  dentro del rango declarado `[0.77.0, 1.0.0)`.
- El turno completo (que sí gasta cuota de la cuenta) no se corrió en esta
  fecha; el handshake y el esquema sí, y son lo que ancla la conformidad por
  versión. (**El 2026-07-31 sí se intentó**, y fue así como se descubrió que el
  lanzamiento del CLI estaba roto en Windows y que el modelo por defecto de la
  cuenta exige un Codex más nuevo: dos cosas que un handshake no puede ver.)

### Dialecto de sesión headless — `claude 2.1.167`

Dos corridas el mismo día: la primera encontró un defecto que ninguna prueba
podía encontrar, y la segunda —tras la tarea 5.3— es la que vale.

**Lo que la primera corrida (17:22Z) encontró, y por qué era invisible**: el CLI
no emite el evento inicial hasta recibir su primera entrada, y el adaptador lo
esperaba *antes* de enviar nada. Resultado: 60 segundos de espera y
`provider_handshake_failed` en toda sesión. **Nivel verificado: 0**, con los dos
criterios ejercidos fallando. Los guiones del cable simulado emitían el evento
inicial *antes* del primer `await-input`, de modo que ninguna prueba de CI podía
verlo: un fixture solo prueba aquello a lo que se le pidió parecerse. La tarea
5.3 corrigió las dos cosas —el adaptador lee el evento en el primer turno y
dicta la identidad de sesión con `--session-id`; los guiones anuncian la sesión
donde el CLI la anuncia— y el orden quedó clavado con una prueba que corre el
cable simulado sin entrada y exige silencio.

**La segunda corrida (18:07Z), después de la corrección**, con el mismo
binario:

- **La sesión abre y el turno completa.** `propose` contra un repositorio
  fixture temporal devolvió `completed`.
- `streaming` **aprueba**: los deltas del turno llegan al log de sesión como
  actualizaciones, no como un bloque tardío.
- `session` **aprueba**: el log registra el binario efectivo y `2.1.167`, la
  versión que el propio CLI respondió.
- `permissions` **aprueba**: el CLI real preguntó por sus herramientas, la
  pregunta llegó a la bandeja como `permission/request` y volvió decidida. Lo
  que quedó ejercido es **el hook**, y conviene decir cuál porque el
  passthrough tiene dos canales y esta corrida no los ejerció los dos. El orden
  documentado del proveedor pone el `PreToolUse` primero y su decisión es
  final; el adaptador lo instala con matcher `*` y su código no tiene camino
  por el que se abstenga —decide `allow` o `deny`, nunca «pregunta a otro»—, de
  modo que el prompt-tool solo se alcanza cuando el hook no puede correr o
  agota su plazo, y ninguna de esas dos cosas ocurrió aquí. El prompt-tool
  sigue configurado en el mismo lanzamiento y CI lo ejerce contra el cable
  simulado (directiva `ask-prompt-tool` en
  `core/meltemid/tests/e2e_adaptadores_claude.rs`); contra un binario que no es
  un fixture **no se ha ejercido todavía**, y es el tirante del cinturón
  precisamente porque el cinturón no falló.
- **`cancellation` no se ejerce**, y por eso el **nivel verificado sigue siendo
  0**: la corrida manual nunca ha ejercido ese criterio, y un nivel se otorga
  solo cuando todos los suyos están presentes y aprobados. Es un 0 distinto del
  de la mañana —tres de cuatro criterios aprobando contra el binario real
  frente a ninguno— y la lista de `criteria` persistida es lo que los
  distingue. (**Superado el 2026-07-31**: la corrida ganó una segunda pierna
  que sí ejerce la cancelación, y esta entrada quedó en nivel verificado 2. La
  frase «gastaría otro turno y sería otra tarea» que aquí figuraba resultó ser
  el techo del procedimiento, no una decisión: mientras estuvo en pie, el nivel
  2 era inalcanzable por construcción.)
- **Coste**: un turno de opus — 8,8k tokens de entrada, 22,2k de escritura de
  caché, 139,7k de lectura de caché y 3,8k de salida.

**Lo que el binario dijo de sí mismo**, en las dos corridas:

- **`apiKeySource` es real y vale `"none"` bajo la sesión iniciada.** El
  nombre era provisional desde la tarea 1.3 y queda anclado: la guarda contra
  el modo de clave de API lee un campo que existe.
- **No hay arreglo `capabilities` en el evento inicial de esta versión.** El
  design D4 lo daba por existente para detección de features; no está. El
  adaptador lo trataba como información y no como requisito, así que nunca
  rehusó por su ausencia — y desde 5.3 ya no lo lee en absoluto: un campo que
  ningún CLI emite no describe nada.
- El evento inicial trae `claude_code_version`, `permissionMode`, `model`,
  `tools`, `slash_commands`, `agents`, `skills`, `plugins`, `output_style`,
  `memory_paths` y `mcp_servers` como objetos `{name, status}`.
- **`--session-id <uuid>` existe y el CLI lo respeta al pie de la letra**: el
  archivo de sesión que dejó el CLI lleva por nombre el UUID que el adaptador
  acuñó. Es lo que permite nombrar la sesión antes de que el CLI hable, y lo
  que sostiene la reanudación de la tarea 3.5.
- `--permission-prompt-tool` **existe y el parser lo acepta** (un flag
  inventado falla con `unknown option`; este no), pero **no aparece en
  `claude --help`**. La infradocumentación que el design D5 fijó como riesgo,
  confirmada en el binario y no de memoria.
- `--bare` sigue siendo opt-in y su propia ayuda dice que la autenticación
  pasa a ser estrictamente `ANTHROPIC_API_KEY`: el flip que D4 teme no ha
  ocurrido en esta versión.
