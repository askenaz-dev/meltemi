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

**Qué cuesta**: una sesión real por entrada — un `propose` corto contra un
repositorio fixture temporal. Si la sesión no llega a abrirse, no se gasta
nada: el turno nunca se envía.

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
  "verifiedLevel": 0,
  "agentVersion": "2.1.167",
  "runAt": "2026-07-28T17:22:14Z",
  "criteria": [
    { "level": 2, "name": "streaming", "passed": false },
    { "level": 2, "name": "session", "passed": false }
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

## Última corrida — 2026-07-28

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
  versión.

### Dialecto de sesión headless — `claude 2.1.167`

- **`apiKeySource` es real y vale `"none"` bajo la sesión iniciada.** El
  nombre era provisional desde la tarea 1.3 y queda anclado: la guarda contra
  el modo de clave de API lee un campo que existe.
- **No hay arreglo `capabilities` en el evento inicial de esta versión.** El
  design D4 lo daba por existente para detección de features; no está. El
  adaptador ya lo trataba como información y no como requisito, así que no
  rehúsa por su ausencia — pero la premisa del design era falsa y quedó
  enmendada.
- El evento inicial trae además `claude_code_version`, `permissionMode`,
  `model`, `tools`, `slash_commands`, `agents`, `skills` y `mcp_servers` como
  objetos `{name, status}`.
- `--permission-prompt-tool` **existe y el parser lo acepta** (un flag
  inventado falla con `unknown option`; este no), pero **no aparece en
  `claude --help`**. La infradocumentación que el design D5 fijó como riesgo,
  confirmada en el binario y no de memoria.
- `--bare` sigue siendo opt-in y su propia ayuda dice que la autenticación
  pasa a ser estrictamente `ANTHROPIC_API_KEY`: el flip que D4 teme no ha
  ocurrido en esta versión.
- **La corrida falló, y con eso hizo su trabajo**: el CLI no emite el evento
  inicial hasta recibir su primera entrada, y el adaptador lo espera *antes*
  de enviar nada. Resultado: 60 segundos de espera y
  `provider_handshake_failed`. **Nivel verificado: 0.** El defecto y su
  corrección están en la tarea 5.3 de la change `adaptadores-propios-acp`.
