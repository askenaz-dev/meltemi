# codex-al-dia

> Vía completa (proposal → design → specs → tasks): el design es obligatorio
> porque el esquema vendorizado del app-server decide qué versión se soporta,
> y re-anclarlo exige leer el diff del contrato antes de tocar el adaptador.
> Nace del comparativo con Orca y de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha AI-1). Actualizada
> tras el design y su revisión adversarial: el alcance creció a lo que el cable
> nuevo exige gobernar, y dos afirmaciones de la versión anterior eran falsas
> (el techo y la confirmación pendiente del modelo); quedan corregidas aquí.

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
  unknown (meltemi-codex-acp; 0.1.1)`, medido a través de Meltemi, donde el
  token de terminal es `unknown` (lanzado desde un cliente propio que hereda la
  terminal, es `xterm-256color`). `version::of`
  (`core/meltemi-adapters/src/codex/version.rs:26-41`) busca el token con
  prefijo `USER_AGENT_PRODUCT` = `codex_cli_rs` (`wire.rs:52`) y no lo
  encuentra, así que devuelve `provider_version_unreadable` con un remedio que
  culpa al CLI («Check that the CLI is the official one…»), que es
  exactamente lo que no pasa. (La versión anterior de esta propuesta decía que,
  aun leída, el techo `(1, 0, 0)` la rehusaría. Es falso: `0.156.1 < 1.0.0`. El
  rango aceptaba 79 minors sin verificar, y lo único que lo impidió fue que la
  versión no se podía leer.)

La postura del adaptador —leer o rehusar, jamás adivinar— es correcta y se
conserva. Lo que falla es que la regla de lectura se ancló a un detalle del
user agent que el proveedor cambió, y el rango soportado no se ha vuelto a
medir desde julio. Entre tanto, el registro sigue diciendo `codex-cli` nivel 2
`ready` y la flota lo pinta verde: papel que un push real desmiente.

Y el cable nuevo trae gobierno que el adaptador no mira. `approvalsReviewer`
puede enrutar las aprobaciones a un subagente del proveedor en lugar del
cliente; el control remoto de Codex puede dejar que otro cliente conteste; y el
adaptador nunca configuró la compuerta dura nativa del CLI que `own-adapters`
exige, así que con una configuración permisiva del usuario hay herramientas que
corren sin pasar por el proxy. Re-anclar sin mirar eso sería otorgar un nivel
verificado a una sesión que no gobierna.

## What Changes

- **Lectura del user agent tolerante a la forma actual**: `version::of` lee
  solo el primer token `<producto>/<semver>` y acepta como producto
  `codex_cli_rs` **o el nombre que el adaptador anunció en `clientInfo.name`**
  (el eco del cliente es la forma nueva del CLI). Cualquier otra forma sigue
  rehusando con `provider_version_unreadable`, pero el remedio pasa a decir qué
  se vio y qué se esperaba. El user agent leído ya viaja verbatim en la
  procedencia `_meta` del log de sesión; la versión leída se contrasta además
  con `thread.cliVersion` al abrir el hilo.
- **Re-anclaje del esquema vendorizado** con
  `codex app-server generate-json-schema` desde el CLI actual, siguiendo el
  procedimiento de `core/mock-provider/schemas/codex-app-server/PROVENANCE.md`.
  El design parte del diff entre 0.77.0 y la versión nueva: qué peticiones de
  servidor aparecieron (`apagado-entero-y-modos` dejó anotadas
  `item/permissions/requestApproval`, `item/tool/requestUserInput`,
  `turn/steer`, `thread/fork|rollback|compact` como inexistentes en 0.77.0) y
  cuáles de las dos que el adaptador cubre cambiaron de forma. Lo que cambió se
  mapea; lo que apareció y no toca este alcance se anota con nombre.
- **Rango soportado dicho con honestidad**: el piso pasa a la versión
  verificada (0.156.1) y el techo se queda en el mayor siguiente; una versión
  posterior a la verificada abre **declarando la distancia** en la sesión, en el
  log y en el catálogo, nunca como verificada. Un CLI fuera del rango sigue
  rehusando con las dos versiones en el mensaje.
- **La compuerta dura nativa, pedida y verificada**: el adaptador pide al
  abrir el hilo la política de aprobación y el sandbox más estrictos de Codex y
  al cliente como revisor, lee lo que el CLI hizo efectivo y rehúsa con remedio
  si no coincide; lo que Codex sigue corriendo sin preguntar bajo esa compuerta
  (comandos de confianza de solo lectura, búsqueda web, servidores MCP de su
  configuración) se declara a la vista. Un control remoto de Codex activo
  rehúsa la sesión.
- **Un turno solo lo mueve lo que es suyo**, y toda petición del servidor tiene
  una respuesta declarada y visible; el eco del prompt no se muestra como
  herramienta; las aprobaciones muestran lo que de verdad conceden (red,
  terminal, raíz de escritura); los avisos del proveedor se anotan.
- **Corrida de conformidad de dos piernas** contra el binario real
  (`session`, `streaming`, `permissions`, `cancellation`), registrada en
  `docs/conformidad-manual.md` con fecha y versión. El nivel 2 se **otorga
  solo si se mide**; si algo falla, la página lo dice con las palabras del
  proveedor, como hoy. Manual y opt-in: CI sigue sin ejecutar agentes reales
  ni red (constitución §5).
- **`docs/agentes.md`** declara la versión mínima de Codex que el adaptador
  soporta, el remedio cuando el CLI es más viejo o más nuevo, la compuerta que
  configura y lo que no cubre, y el nombre con que el adaptador se presenta.
- **El catálogo dice contra qué versión se midió** el nivel verificado, en la
  GUI y en la TUI.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: MODIFIED «Conformidad por versión y desfase rehusado»
  (verificado frente a aceptado, valores de gobierno que fallan cerrados, guion
  frente a transcripción) y MODIFIED «Passthrough de permisos con compuerta dura
  y pérdidas visibles» (la compuerta más estricta del CLI y la declaración de lo
  que no cubre); ADDED «La versión se lee donde el proveedor la escribe», «La
  compuerta del servidor se pide, se verifica y falla cerrada», «Un turno solo
  lo mueve lo que es suyo» y «Toda petición del servidor tiene una respuesta
  declarada».
- `integration-levels`: ADDED «El nivel verificado de un adaptador propio se
  re-otorga al re-anclar» y «El nivel verificado dice contra qué versión se
  midió».
- `initial-docs`: ADDED «La guía de agentes declara la versión verificada de
  cada adaptador propio con rango».

## Impact

- Archivos: `core/meltemi-adapters/src/codex/{version.rs,wire.rs,mod.rs}`,
  `core/mock-provider/schemas/codex-app-server/` (volcado y `PROVENANCE.md`),
  `core/mock-provider` (fixtures del cable con la forma nueva),
  `docs/conformidad-manual.md`, `docs/agentes.md`,
  `core/meltemid/data/fleet-registry.toml` si la entrada gana versión mínima.
- Cero dependencias nuevas. Ningún RPC nuevo; `fleet/list` gana un campo
  aditivo (`verifiedVersion`) que la GUI y la TUI muestran (§4).
- Quien configuró Codex con `never`, `auto_review`, un sandbox abierto o el
  control remoto no abre sesión de Codex bajo Meltemi, con el remedio dicho.
- Riesgo real: el diff del esquema puede mostrar que el cable v2 cambió más
  que el saludo (tipos de ítem, forma de las aprobaciones). Por eso el design
  va después del diff y no antes; la propuesta no promete nivel 2, promete
  medirlo.
- Lo que ya confirmó la grabación del 2026-09-24: la cuenta del mantenedor
  sirve el modelo por defecto con 0.156.1 a un cliente que se presenta como
  `meltemi-codex-acp`. Lo que solo la corrida confirma: que una escritura
  aprobada bajo el sandbox de solo lectura se aplica, y cuánto tarda un turno.

## Fuera de alcance

- Reanudar hilos (`thread/resume`) y MCP para Codex: `codex-reanudar-y-mcp`,
  que depende de esta.
- Mapear `turn/steer` a `redirigir-turno`: se decide con el diff en la mano,
  como change propia si el esquema lo trae.
- Bajar el CLI del usuario a una versión vieja: Meltemi no gestiona
  instalaciones ajenas (constitución §2).
