<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — pensamiento-a-la-vista (2026-08-10)

Sobre el **binario de release**, con la receta de
`docs/qa/2026-08-09-piel-de-pestanas-smoke.md` (patch de puerto revertido al
terminar, user data folder propio y nuevo, binarios y endpoint aparte de los
del mantenedor). Sin inyectar DOM y recargando antes de medir, que es la trampa
que el smoke del compositor dejó escrita.

## El fixture que piensa

El mock aprendió a pensar detrás de `--think`, apagado por defecto para no
cambiar lo que leen los tests de transcript que ya existen. Un proyecto propio
(`thinker`) cuyo registro pasa esa bandera, y una sesión real corrida contra
él. El log lo confirma antes de mirar ninguna superficie:

```
1 "sessionUpdate":"agent_thought_chunk"   "Reading the proposal before writing it."
1 "sessionUpdate":"agent_message_chunk"   "Filling in the proposal."
```

Pensamiento y prosa son textos **distintos**, que es lo que permite comprobar
que la superficie los separa en vez de suponerlo.

## Lo confirmado

Con el proyecto en ámbito y su sesión abierta:

| | medido |
|---|---|
| bloques de pensamiento | 1 |
| desplegado | **sí** (`open: true`) |
| rótulo | `thinking` |
| contenido | `Reading the proposal before writing it.` |

Es decir: el pensamiento que el agente emitió **se ve**, con su propio rótulo y
separado de la prosa. Era el objetivo de la change y está en el binario.

## Hallazgo: el turno histórico no se marca cerrado

Y aquí lo que solo el binario podía enseñar. Tras **veinte segundos de espera
activa** —no un sleep fijo—, el transcript de una sesión **ya terminada** sigue
reportando su turno como abierto: no se renderiza el `.stop` que acompaña a un
turno cerrado, y en consecuencia el pensamiento **permanece desplegado en
reposo**, que es justo lo contrario de la política que esta change declara.

El log de esa sesión **sí** contiene `turn_completed`, y el pliegue de la
conversación tiene su rama para marcarlo (`conversation.ts:301-308`). Así que
el fallo no está en el dato ni en la regla nueva: está en el camino que
reconstruye una conversación **desde el log**, y por eso afecta igual al
indicador de fin de turno, que es anterior a esta change y no se estrenó aquí.

**No se arregla en esta change**, porque no es suya: `open={!item.closed}`
obedece correctamente a un `closed` que llega en falso. Queda anotado en el
backlog como hallazgo con su evidencia, que es lo que el rumbo manda hacer con
lo que aparece de paso.

Lo que sí queda medido y verdadero: **mientras el turno corre —el caso que la
change existe para servir— el pensamiento se ve**. Lo que no se ha podido
confirmar en el binario es el plegado en reposo, y se dice aquí en vez de
darse por bueno.

## El terminal

El pliegue del transcript de la TUI está probado por su test unitario, que
ejercita `summarize_event` con prosa, pensamiento, herramienta, fragmento vacío
y evento sin contenido, en los dos idiomas. **No se condujo la TUI real**:
requiere un pty y no hay arnés para eso hoy. Se declara para que la fuerza de
esa prueba se lea sin averiguarla.

## Reversión

Patch de `additionalBrowserArgs` retirado y su ausencia verificada antes de
commitear. Los procesos del mantenedor no se detuvieron.
