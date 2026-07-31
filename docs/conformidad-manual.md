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

## Última corrida — 2026-07-31

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
