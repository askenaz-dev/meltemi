# Design — archivo-sin-cabeceras-arrastradas

## Context

`fold_text` parte la spec viva y el delta en bloques con `split_requirements`:
todo lo anterior al primer `### Requirement:` es preámbulo y cada bloque dura
hasta el siguiente `### Requirement:`. En un delta, las secciones `## ADDED
Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` y `##
RENAMED Requirements` separan requisitos, así que la cabecera de la segunda
sección cae dentro del último bloque de la primera y viaja con él.

## Decisions

### D1 — Una cabecera de sección del delta cierra el bloque y no se copia

`split_requirements` reconoce las cuatro cabeceras de sección del delta
(`## ADDED|MODIFIED|REMOVED|RENAMED Requirements`): cierran el bloque en curso,
y ellas y lo que las sigue hasta el siguiente requisito no pertenecen a ningún
bloque. Como la misma función parte la spec viva, una cabecera arrastrada que
ya estaba ahí desaparece en la siguiente fusión de esa capacidad.

Solo esas cuatro. Cualquier otro `##` sigue siendo texto del bloque, como hoy:
el barrido del 2026-10-28 no encontró ningún otro `##` después del primer
requisito en las 36 specs vivas, y tratar todo `##` como frontera tiraría el
texto de una sección legítima que alguien añada mañana.

### D2 — Antes de archivar, dos comprobaciones; después, el orden de aterrizaje

Archivar en lote sustituye texto. Dos casos lo harían perder:

1. Dos changes sin archivar que modifican el mismo requisito: la que archiva
   última pisa a la otra. Barrido del 2026-10-28 sobre las 28 terminadas y
   `adaptadores-con-rawinput`: **ninguno**. La casa lo evitaba a propósito
   (solo ADDED compone sobre lo que otra change tiene sin archivar).
2. Un MODIFIED escrito antes de que una change **ya archivada** cambiara ese
   requisito: archivarlo reescribiría la verdad viva con texto viejo. Barrido:
   **uno**, `maquinas-remotas` sobre «El punto de encuentro en dos vías está
   documentado» de `remote-access`, escrito el 2026-10-01 y cambiado por
   `identidad-propia`, archivada el 2026-10-02. Ver D3.

Sin conflictos, el orden es el de aterrizaje (el último commit de cada
`tasks.md`), que es el orden en que cada change dio por buena la verdad viva
que tenía delante.

### D3 — `maquinas-remotas` se reconcilia antes de archivarse

Su cuerpo ya contiene la frase que `identidad-propia` añadió (la identidad y
la lista de equipos de la malla como capacidad presente que Meltemi lee): se
escribió encima de ella. Su tercer escenario, «Lo de fase 3 está anotado y no
prometido», mismo título que el vivo, perdió la aserción que `identidad-propia`
le había puesto. El delta recupera esa aserción como un `AND` más, y entonces
fundirlo no quita nada de lo vivo.

### D4 — Lo que no se archiva

`lanzador-conversacional`: gate de la firma del mantenedor (pendiente 1c).
`primer-arranque-del-home`: tarea 3.2 abierta (7º de la tanda). Las de la tanda
que aún no terminan se archivan al terminar. Una change con `verify`
incompleto no se archiva hasta completarlo o declarar su excepción.

## Verification

- Tests de `archive.rs`: un ADDED seguido de otra sección no arrastra su
  cabecera; una cabecera arrastrada en la viva desaparece al fundir.
- `dogfood.rs` y `migration.rs` verdes después de cada tanda de archivos.
