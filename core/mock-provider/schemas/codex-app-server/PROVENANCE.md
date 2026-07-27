# Esquema del cable del servidor JSON-RPC del proveedor (volcado oficial)

Estos `.json` son el **volcado literal** del generador de esquemas del CLI
oficial, sin una sola edición a mano. Son la autoridad del contrato para el
adaptador `meltemi-codex-acp` (adaptadores-propios-acp design D6, tarea 2.1):
ante cualquier discrepancia entre estos archivos y el guion del cable simulado
o los tipos del adaptador, **manda el volcado**.

## Procedencia

| Dato | Valor |
| --- | --- |
| CLI | `codex-cli 0.77.0` |
| Comando | `codex app-server generate-json-schema --out <dir>` |
| Fecha del volcado | 2026-07-27 |
| Plataforma del volcado | Windows 11 (26200), x86_64 |
| Handshake verificado | `initialize` → `{"userAgent":"codex_cli_rs/0.77.0 (…)"}` |

El volcado completo produce ~90 archivos entre la raíz, `v1/` y `v2/`. Aquí se
vendorizan **solo los que el adaptador toca**, con su nombre de archivo
original y su contenido byte a byte (incluida la ausencia de salto de línea
final, que es como los escribe el generador). `InitializeParams.json` e
`InitializeResponse.json` vienen de `v1/`; el resto de los `*Notification`,
`Thread*` y `Turn*` vienen de `v2/`; los `*RequestApproval*` y
`JSONRPCMessage.json` viven en la raíz del volcado. No hay colisión de nombres
entre esas tres procedencias para este subconjunto.

## Superficie elegida: la v2 (hilo / turno / ítem)

El CLI expone dos generaciones de métodos en el mismo servidor. El adaptador
habla la **v2** — `thread/start`, `turn/start`, `turn/interrupt`, y las
notificaciones `turn/started`, `item/started`, `item/agentMessage/delta`,
`item/completed`, `turn/completed` — porque sus primitivas son exactamente las
que el design D6 mapea a la sesión ACP. La generación anterior
(`newConversation`, `sendUserTurn`, `interruptConversation`, `codex/event`)
sigue existiendo en el binario y queda fuera de alcance: una superficie por
dialecto, con su prueba escrita.

## Cómo re-anclar esto a una versión nueva

1. `codex app-server generate-json-schema --out <dir>` con el CLI instalado.
2. Copiar sobre este directorio los archivos aquí listados, sin editarlos.
3. Actualizar la tabla de procedencia de arriba.
4. Actualizar el rango soportado del adaptador
   (`core/meltemi-adapters/src/codex/version.rs`) si el piso se mueve.
5. Correr `cargo test -p meltemi-adapters --test codex_conformance`: los
   campos que cambiaron aparecen señalados uno por uno, y el guion del cable
   simulado se valida contra este mismo volcado, así que tampoco puede
   quedarse atrás en silencio.

CI no ejecuta el CLI oficial jamás (constitución §5 y design D10): estos
archivos son lo que permite que la conformidad corra sin binario de proveedor
en ninguna parte. Lo que ningún fixture puede afirmar es que el contrato
observado siga vigente; eso lo dice la corrida manual contra el CLI real
(tarea 5.2).
