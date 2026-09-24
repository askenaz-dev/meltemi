# Esquema del cable del servidor JSON-RPC del proveedor (volcado oficial)

Estos `.json` son el **volcado literal** del generador de esquemas del CLI
oficial, sin una sola edición a mano. Son la autoridad del contrato para el
adaptador `meltemi-codex-acp` (adaptadores-propios-acp design D6, tarea 2.1;
re-anclado por codex-al-dia, design D3): ante cualquier discrepancia entre
estos archivos y el guion del cable simulado o los tipos del adaptador,
**manda el volcado**.

## Procedencia

| Dato | Valor |
| --- | --- |
| CLI | `codex-cli 0.156.1` |
| Comando | `codex app-server generate-json-schema --out <dir>` (**sin** `--experimental`) |
| Fecha del volcado | 2026-09-24 |
| Plataforma del volcado | Windows 11 (26200), x86_64 |
| Handshake verificado | `initialize` → `{"userAgent":"meltemi-codex-acp/0.156.1 (…) (meltemi-codex-acp; 0.1.1)","codexHome":…,"platformFamily":…,"platformOs":…}` |
| Transcripción del cable | grabada el 2026-09-24 contra el mismo binario y vendorizada saneada; el guion `../../scripts/codex-app-server.ndjson` sigue su orden |

El anclaje anterior era `codex-cli 0.77.0`, un binario publicado el
2025-12-21 y volcado el 2026-07-27. Desde 0.81 el servidor escribe como primer
token del user agent el `clientInfo.name` del cliente en lugar de
`codex_cli_rs`; el adaptador lee las dos formas (`version.rs`).

El volcado completo de 0.156.1 produce 310 archivos entre la raíz, `v1/` (solo
el handshake) y `v2/`. Aquí se vendorizan **solo los que el adaptador toca**,
con su nombre de archivo original y su contenido byte a byte (incluida la
ausencia de salto de línea final y el orden alfabético de las claves, que es
como los escribe el generador de esta versión). `InitializeParams.json` e
`InitializeResponse.json` vienen de `v1/`; las notificaciones, `Thread*` y
`Turn*`, de `v2/`; los `*RequestApproval*` y `JSONRPCMessage.json`, de la
raíz. No hay colisión de nombres entre esas tres procedencias para este
subconjunto.

Además de lo que el adaptador envía y mapea, se vendorizan
`ThreadStatusChangedNotification.json`,
`ThreadTokenUsageUpdatedNotification.json` y
`ServerRequestResolvedNotification.json`: tres notificaciones que el servidor
real intercala en cada turno y que el adaptador no mapea. El guion las lleva
para probar que ignorarlas no cuesta nada, y por eso se validan.

La transcripción del servidor real vive junto al guion, saneada:
`../../scripts/codex-app-server-0.156.1.transcript.ndjson`. Es evidencia, no un
guion: la conformidad comprueba que el guion sigue su orden. El servidor envía
además campos que este volcado no declara (en la respuesta de `thread/start`:
`runtimeWorkspaceRoots`, `activePermissionProfile`, `multiAgentMode`, y en el
hilo `environments`, `extra`, `canAcceptDirectInput`, `daybreakEnabled`); el
adaptador no los lee y el guion no los lleva.

## Superficie elegida: la estable de la v2 (hilo / turno / ítem)

El adaptador habla la **v2** — `thread/start`, `turn/start`, `turn/interrupt`,
y las notificaciones `turn/started`, `item/started`,
`item/agentMessage/delta`, `item/completed`, `turn/completed` — porque sus
primitivas son exactamente las que el design D6 mapea a la sesión ACP. En
0.156.1 la generación anterior ya no expone métodos de cliente:
`ClientRequest.json` no contiene `newConversation`, `sendUserTurn` ni
`interruptConversation`.

La superficie es la **estable**: el volcado se genera sin `--experimental` y
el adaptador no declara `capabilities` en `initialize`, así que
`experimentalApi` queda en su valor por defecto, falso. Es la
superficie en la que el adaptador puede apoyarse; declarar lo contrario haría
llegar métodos y campos que este volcado no describe.

## Cómo re-anclar esto a una versión nueva

1. `codex app-server generate-json-schema --out <dir>` con el CLI instalado,
   sin `--experimental`.
2. Copiar sobre este directorio los archivos aquí listados, sin editarlos.
3. Grabar una transcripción del servidor real con un cliente que se presente
   como el adaptador (`clientInfo.name = "meltemi-codex-acp"`): un turno de
   solo texto y uno que pida escribir un archivo. Vendorizarla saneada junto al
   guion y reescribir el guion `../../scripts/codex-app-server.ndjson` con su
   orden, con ids, rutas y modelo simulados y sus diferencias en la cabecera.
4. Actualizar la tabla de procedencia de arriba.
5. Actualizar `VENDORED_SCHEMA_VERSION` y el piso soportado del adaptador
   (`core/meltemi-adapters/src/codex/wire.rs`): el piso es la versión volcada.
6. Correr `cargo test -p meltemi-adapters --test codex_conformance`: los campos
   que cambiaron aparecen señalados uno por uno, el guion se valida contra este
   mismo volcado y se contrasta con el orden de la transcripción.
7. Correr la conformidad manual contra el CLI real
   (`docs/conformidad-manual.md`): el nivel verificado se otorga solo si se
   mide.

CI no ejecuta el CLI oficial jamás (constitución §5 y design D10): estos
archivos son lo que permite que la conformidad corra sin binario de proveedor
en ninguna parte. Lo que ningún fixture puede afirmar es que el contrato
observado siga vigente; eso lo dice la corrida manual contra el CLI real.
