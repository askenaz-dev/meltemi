<!-- SPDX-License-Identifier: Apache-2.0 -->
# Preguntas del agente — validación manual contra el CLI real (pendiente)

Esta change levanta el rehúso de `AskUserQuestion` en el adaptador de Claude. La
mitad que CI **no puede** verificar —y que por tanto no se afirma— es la forma
exacta que el CLI real espera en `updatedInput`.

## Por qué no la cubre CI

Constitución §5: los e2e usan `mock-agent`, jamás agentes reales ni red. Lo que
sí está probado sin intervención:

- que la pregunta se releva con **las opciones del agente** y sus rótulos
  verbatim (unitario del adaptador + e2e contra el mock);
- que un input con una forma que el adaptador **no reconoce** se rehúsa en vez de
  adivinarse, con cinco formas distintas de no reconocerlo;
- que **solo** una pregunta completa su propio input, y que cualquier otra
  herramienta sigue viajando byte a byte;
- que una **regla de permisos no contesta** una pregunta por el usuario;
- y el bucle entero —preguntar, elegir, y el turno continuando con la elección—
  contra el mock.

## Qué hay que mirar, y con qué

1. Sesión real con el adaptador (`meltemi-claude-acp`) sobre un proyecto de
   prueba, con una instrucción que provoque una pregunta del agente.
2. **Anotar la versión del CLI** (`claude --version`) junto al resultado.
3. Comprobar en la GUI: la pregunta aparece **en el compositor** con las opciones
   del agente, se contesta con teclado, y el turno continúa.
4. Comprobar en el transcript del CLI que la elección llegó como respuesta a la
   pregunta y no como un error de herramienta.

## Lo declarado por adelantado

El campo bajo el que la respuesta se escribe (`answer`, dentro de la pregunta que
el agente envió) **no lo especificamos nosotros**. Si el proveedor lo cambia, el
requisito de **conformidad por versión** de `own-adapters` es lo que rehúsa antes
que adivinar, y el síntoma será una denegación visible con su motivo — nunca una
respuesta silenciosa a otra pregunta.

| | |
|---|---|
| Versión del CLI probada | _pendiente_ |
| Fecha | _pendiente_ |
| Resultado | _pendiente_ |
