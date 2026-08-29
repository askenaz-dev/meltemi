# Reglas reales del hub (copia literal)

Estos `RULE.md` son la copia **byte a byte** de las reglas publicadas del Forge
Development Hub, sin una sola edición. Son la autoridad de forma del lector de
front-matter del harness (`harness-global-y-por-agente` design D2/D3): ante
cualquier discrepancia entre estos archivos y lo que el lector espera, **mandan
estos archivos**.

Existen porque un lector probado solo contra ejemplos escritos aquí es un lector
probado contra su propia idea del formato. Los tres hechos que el design tuvo
que corregir del proposal —nueve claves y no cuatro, un comentario al final de
`version`, y listas en línea en todos los casos— salieron de leer estos
archivos, no de recordarlos.

## Procedencia

| Dato | Valor |
| --- | --- |
| Repositorio | `forge-development-hub` |
| Commit | `0fa482c` |
| Fecha de la copia | 2026-08-19 |
| Archivos | las cuatro reglas publicadas en `rules/` |

El nombre de archivo se aplana a `<name>.RULE.md` porque aquí no hay un
directorio por regla que dé el nombre; en el harness real el nombre viene del
directorio y **debe coincidir** con el del front-matter (design D2). Esa
coincidencia se prueba en el test del harness, no aquí.

Actualizar esta copia es traer los archivos otra vez y anotar el commit nuevo.
Editarlos a mano derrotaría el propósito: dejarían de ser lo que el formato es
para pasar a ser lo que creemos que es.
