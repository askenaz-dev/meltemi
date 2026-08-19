<!-- SPDX-License-Identifier: Apache-2.0 -->
# Modelo y esfuerzo — validación manual contra los CLIs reales (pendiente)

Esta change hace que el modelo y el esfuerzo viajen a los adaptadores propios.
La mitad que CI **no puede** verificar —y que por tanto no se afirma— es que los
CLIs reales acepten lo que se les manda, en la versión que el usuario tenga.

## Por qué no la cubre CI

Constitución §5: los e2e usan `mock-agent`, jamás agentes reales ni red. Lo que
sí está probado sin intervención:

- que el núcleo transporta las cadenas **sin interpretarlas**, y que lo que
  efectivamente rigió queda en el evento de resolución y es recuperable del log;
- que la precedencia va en un solo sentido (la sesión pisa el default del
  perfil) y que una cadena vacía no es una elección;
- que pedir una palanca que el agente no admite **rehúsa antes de crear nada**,
  nombrando al agente y la palanca;
- que cada palanca se serializa **donde el esquema del proveedor la define** —
  `model` en el hilo de Codex, `effort` solo en su turno— incluido el lado
  negativo (ningún `effort` en el hilo, ningún `model` en el turno);
- que el nombre de la variable de entorno con que el daemon se lo dice al
  adaptador **está escrito igual en los dos lados**, comprobado leyendo la
  fuente del otro.

## Qué hay que mirar, y con qué

### Claude

1. `meltemi session "..." --model <nombre>` sobre un proyecto de prueba.
2. **Anotar la versión del CLI** (`claude --version`).
3. Comprobar que el turno corre con ese modelo y que el transcript lo refleja.
4. Comprobar el rehúso: `--effort high` **debe rehusarse** con el motivo, sin
   crear sesión.

### Codex

1. `meltemi session "..." --model <nombre> --effort high`.
2. Anotar la versión del CLI.
3. Comprobar que el hilo arranca con el modelo y que el turno lleva el esfuerzo.

## Lo declarado por adelantado

- **El esfuerzo de Claude no está cableado**, y no por olvido: no está
  verificado contra el CLI pineado que exista una bandera equivalente. Se rehúsa
  con ese motivo. El día que se verifique es un booleano.
- Un nombre de modelo inválido lo rechaza **el CLI del proveedor**, no Meltemi:
  el núcleo no tiene ni puede tener una tabla de modelos (§5). El síntoma
  esperado es el error del proveedor mostrado con su motivo.

| | Claude | Codex |
|---|---|---|
| Versión probada | _pendiente_ | _pendiente_ |
| Fecha | _pendiente_ | _pendiente_ |
| Resultado | _pendiente_ | _pendiente_ |
