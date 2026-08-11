<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — pensamiento-a-la-vista (2026-08-10)

Medición sobre el **binario de release** con la GUI conducida por CDP. Receta
de `docs/qa/2026-08-09-piel-de-pestanas-smoke.md` —patch de puerto revertido al
terminar y `WEBVIEW2_USER_DATA_FOLDER` propio y nuevo— y con la regla que el
smoke anterior dejó escrita: **recargar antes de medir y no inyectar DOM
propio**.

## El agente simulado aprendió a pensar

El escenario necesitaba un agente que emitiera pensamiento y el mock no lo
hacía. Ahora lo hace tras `--think`, siguiendo el patrón con el que este
binario ya declara lo que sabe (`--load-session`, `--mcp`), y **apagado por
defecto a propósito**: un mock que piensa siempre cambiaría lo que leen todos
los tests de transcript existentes. Sigue sin red y sin agentes reales, como
exige la regla de CI.

## Resultado: **incompleto**, y lo que encontró de camino

El smoke **no llegó a medir el bloque de pensamiento**: el driver no consiguió
abrir la sesión que piensa, y se cortó ahí en vez de dar por buena una medición
que no se hizo. Queda pendiente y la tarea 3.2 sigue abierta.

Lo que sí encontró, buscando esa sesión, vale más que la medición que no logró:
**la tabla de sesiones no mostraba el título**. Se estaba buscando la sesión
por su nombre en la lista y allí solo había un hash de ocho caracteres —
exactamente lo que `titulo-de-sesion` existe para arreglar, en una superficie
que su propia tarea 4.1 nombraba y que se había marcado como hecha con solo las
pestañas implementadas. Corregido: la celda lleva id **y** título.

La lección queda escrita en esa tarea: un escenario cubierto por un test de la
pestaña no prueba que las demás superficies del mismo requisito estén hechas.

## Reversión

Patch de puerto retirado y su ausencia verificada. Los procesos del mantenedor
no se detuvieron.
