# codex-al-dia

> Vía completa (proposal → design → specs → tasks): el design es obligatorio
> porque el esquema vendorizado del app-server decide qué versión se soporta,
> y re-anclarlo exige leer el diff del contrato antes de tocar el adaptador.
> Nace del comparativo con Orca y de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha AI-1).

## Why

Codex no arranca bajo Meltemi con ninguna versión disponible hoy, y las dos
causas son distintas:

- Con `codex-cli 0.77.0` —la versión a la que `adaptadores-propios-acp` ancló
  el esquema— el handshake pasa y el proveedor rechaza cada turno: «The
  'gpt-5.6-sol' model requires a newer version of Codex». Está medido y
  escrito en `docs/conformidad-manual.md` («nivel verificado 0»), con el
  remedio dicho allí mismo: subir el CLI **y** re-anclar el esquema, ni el uno
  sin el otro.
- Con `codex-cli 0.156.1` (la versión que instala `npm i -g @openai/codex`
  hoy) el adaptador **rehúsa el handshake**. El CLI ya no anuncia
  `codex_cli_rs/<versión>`: anuncia el nombre que el propio adaptador le dio en
  `clientInfo` — `meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64)
  unknown (meltemi-codex-acp; 0.1.1)`. `version::of`
  (`core/meltemi-adapters/src/codex/version.rs:26-41`) busca el token con
  prefijo `USER_AGENT_PRODUCT` = `codex_cli_rs` (`wire.rs:52`) y no lo
  encuentra, así que devuelve `provider_version_unreadable` con un remedio que
  culpa al CLI («Check that the CLI is the official one…»), que es
  exactamente lo que no pasa. Y aunque leyera la versión, el techo declarado
  `SUPPORTED_CEILING = (1, 0, 0)` (`wire.rs:48`) la rehusaría por desfase.

La postura del adaptador —leer o rehusar, jamás adivinar— es correcta y se
conserva. Lo que falla es que la regla de lectura se ancló a un detalle del
user agent que el proveedor cambió, y el rango soportado no se ha vuelto a
medir desde julio. Entre tanto, el registro sigue diciendo `codex-cli` nivel 2
`ready` y la flota lo pinta verde: papel que un push real desmiente.

## What Changes

- **Lectura del user agent tolerante a la forma actual**: `version::of` acepta
  el primer token `<producto>/<semver>` cuyo producto sea `codex_cli_rs` **o el
  nombre que el adaptador anunció en `clientInfo.name`** (el eco del cliente es
  la forma nueva del CLI). Cualquier otra forma sigue rehusando con
  `provider_version_unreadable`, pero el remedio pasa a decir qué se vio y qué
  se esperaba. La forma leída viaja en la procedencia `_meta` del log de sesión
  (design D7 de `adaptadores-propios-acp`), para que la próxima vez el dato esté
  en el registro y no en la memoria de nadie.
- **Re-anclaje del esquema vendorizado** con
  `codex app-server generate-json-schema` desde el CLI actual, siguiendo el
  procedimiento de `core/mock-provider/schemas/codex-app-server/PROVENANCE.md`.
  El design parte del diff entre 0.77.0 y la versión nueva: qué peticiones de
  servidor aparecieron (`apagado-entero-y-modos` dejó anotadas
  `item/permissions/requestApproval`, `item/tool/requestUserInput`,
  `turn/steer`, `thread/fork|rollback|compact` como inexistentes en 0.77.0) y
  cuáles de las dos que el adaptador cubre cambiaron de forma. Lo que cambió se
  mapea; lo que apareció y no toca este alcance se anota con nombre.
- **Rango soportado re-medido, no ensanchado a ciegas**: `SUPPORTED_FLOOR` y
  `SUPPORTED_CEILING` pasan a la ventana que la corrida verifique
  (`[versión-anclada, siguiente-mayor)`), y `VENDORED_SCHEMA_VERSION` dice la
  versión real. Un CLI fuera de la ventana sigue rehusando con las dos
  versiones en el mensaje.
- **Corrida de conformidad de dos piernas** contra el binario real
  (`session`, `streaming`, `permissions`, `cancellation`), registrada en
  `docs/conformidad-manual.md` con fecha y versión. El nivel 2 se **otorga
  solo si se mide**; si algo falla, la página lo dice con las palabras del
  proveedor, como hoy. Manual y opt-in: CI sigue sin ejecutar agentes reales
  ni red (constitución §5).
- **`docs/agentes.md`** declara la versión mínima de Codex que el adaptador
  soporta y el remedio cuando el CLI es más viejo o más nuevo.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: + requisito «Lectura de versión tolerante a la forma del
  user agent» (producto vendorizado o nombre anunciado por el cliente; rehúso
  con ambas formas en el mensaje) y enmienda del rango soportado a la ventana
  medida.
- `integration-levels`: + escenario de re-anclaje: el nivel verificado de un
  adaptador propio caduca con la versión anclada y se re-otorga midiendo.
- `initial-docs`: + versión mínima y remedio en la página de agentes.

## Impact

- Archivos: `core/meltemi-adapters/src/codex/{version.rs,wire.rs,mod.rs}`,
  `core/mock-provider/schemas/codex-app-server/` (volcado y `PROVENANCE.md`),
  `core/mock-provider` (fixtures del cable con la forma nueva),
  `docs/conformidad-manual.md`, `docs/agentes.md`,
  `core/meltemid/data/fleet-registry.toml` si la entrada gana versión mínima.
- Cero dependencias nuevas. Ningún RPC nuevo: no nace deber de paridad §4.
- Riesgo real: el diff del esquema puede mostrar que el cable v2 cambió más
  que el saludo (tipos de ítem, forma de las aprobaciones). Por eso el design
  va después del diff y no antes; la propuesta no promete nivel 2, promete
  medirlo.
- Lo que solo la corrida confirma: que la cuenta ChatGPT del mantenedor sirve
  el modelo por defecto con el CLI nuevo, y cuánto tarda un turno.

## Fuera de alcance

- Reanudar hilos (`thread/resume`) y MCP para Codex: `codex-reanudar-y-mcp`,
  que depende de esta.
- Mapear `turn/steer` a `redirigir-turno`: se decide con el diff en la mano,
  como change propia si el esquema lo trae.
- Bajar el CLI del usuario a una versión vieja: Meltemi no gestiona
  instalaciones ajenas (constitución §2).
