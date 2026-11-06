<!-- SPDX-License-Identifier: Apache-2.0 -->

# El método SDD

[English](../metodo-sdd.md)

Usa un cambio cuando quieras conservar junto a tu repositorio los requisitos, el diseño, las
tareas de implementación y la verificación. Las especificaciones vivas están en `.meltemi/specs/`;
los artefactos de un cambio permanecen en `.meltemi/changes/<name>/` hasta que lo archivas.

También puedes empezar una [sesión libre](sesion-libre.md) y elegir Proponer o Explorar en el
mismo compositor cuando necesites el método. Esta página describe esos controles existentes; no
cambia la constitución de tu proyecto ni aprueba una propuesta por ti.

## Un cambio y sus artefactos

Un cambio tiene un nombre en kebab-case y cuatro artefactos:

1. **proposal**: por qué se hace y qué cambia, con las capacidades añadidas o modificadas.
2. **specs**: requisitos EARS agrupados por capacidad, cada uno con sus escenarios.
3. **design**: decisiones y sus contrapartidas, con la justificación de cada dependencia nueva.
4. **tasks**: tareas atómicas, con un commit por tarea.

Revisa la propuesta y sus escenarios antes de empezar a implementar. Para un cambio pequeño
puedes preparar los artefactos juntos y mantener la misma revisión y verificación.

## El ciclo y sus verbos

- `propose <idea>`: crea la estructura de un cambio y delega la propuesta en un agente.
- `plan <change>`: precisa el diseño y ordena `tasks.md` según las dependencias.
- `review <change>`: revisa los cambios de especificación como una lista de comprobación.
- `verify <change>`: presenta la lista de verificación por requisito; cada escenario se vincula
  con un test mediante la convención de nombres escenario→test o se marca como manual.
- `implement <change> <agent>`: ejecuta el agente sobre `tasks.md` y compone cada tarea:
  punto de control → turno en el árbol de trabajo → commit de la tarea → marcarla.
- `archive <change> [confirm]`: muestra una vista previa; añade `confirm` después de verificar
  para aplicar los cambios de especificación y conservar el cambio en el histórico fechado.

## Requisitos EARS

Los requisitos usan la formulación EARS con marcadores explícitos: `WHEN`, `WHILE`, `IF`, `THEN`,
`WHERE` y `AND`. Un requisito conforme tiene al menos un escenario y el nombre de cada escenario
determina el nombre de su test.

## Trazabilidad

Las tareas de implementación producen commits con las referencias `Meltemi-Task: <change>/<task>`
y `Meltemi-Req: <capability>/<requirement>`. Inspecciona el resultado y el diff de la tarea; después
usa la lista de verificación para registrar qué escenarios pasaron. Una lista de comprobación es
evidencia que debe revisarse, no un sustituto de ejecutar los tests pertinentes.

## Archivar un cambio verificado

`meltemi archive <change>` muestra una vista previa. Añade el argumento final `confirm` para
aplicar los cambios a las especificaciones vivas y conservar el cambio en el histórico fechado.
Los controles de verificación pueden rechazar un cambio incompleto: lee el requisito o escenario
que se informa y completa su evidencia antes de volver a intentarlo.

Consulta la [referencia CLI](referencia-cli.md) para conocer la sintaxis de los comandos y los
códigos de salida.
