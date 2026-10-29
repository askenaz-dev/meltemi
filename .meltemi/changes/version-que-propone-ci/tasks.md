# Tasks — version-que-propone-ci

Vía completa. Un commit atómico por tarea, con referencia
`(version-que-propone-ci N.M)` y sin trailers de co-autoría. Los tests de la
herramienta corren contra repositorios fixture temporales con tags propios,
nunca contra este repositorio ni contra la red.

## 1. La herramienta

- [ ] 1.1 `pending_changes` extraída de `write_changelog`, y `next` con su
  salida legible y `--json` (versión, regla, señales, lista para 1.0); sin
  nada pendiente sale con 3 (design D1, D6) — escenario «Sin nada pendiente no
  queda PR de release»; gate: tests de `meltemi-release`
- [ ] 1.2 La ruptura del contrato: diff de `proto/schemas/v1/` y de
  `PROTOCOL_VERSION` contra el último tag, con un fixture por cada forma de
  ruptura y por cada forma aditiva que no lo es (design D2) — escenario
  «Ruptura medida en el contrato»
- [ ] 1.3 La ruptura de la gramática: subcomandos, banderas y códigos de
  salida de `docs/referencia-cli.md` contra el último tag, y un test que corre
  el lector sobre la referencia real del árbol (design D2) — escenario
  «Ruptura medida en la gramática de la CLI»
- [ ] 1.4 Las líneas `> **Ruptura**:` y `> **Corrección**:` de la propuesta
  (design D2, D4) — escenarios «Ruptura declarada por la propuesta» y «Sin
  declaración cuenta como aditiva»
- [ ] 1.5 El hito: informe añadido desde el tag, `Release:` con la última
  versión y veredicto que publica (design D3) — escenarios «El informe que
  publica sube el minor» y «Un informe que no publica no es hito»
- [ ] 1.6 Las reglas después de 1.0 (design D4) — escenarios «Ruptura después
  de 1.0 sube el major» y «Solo correcciones suben el patch»
- [ ] 1.7 La lista para 1.0 con sus seis gates y sus informativos, `--facts`
  para `assets.json` y `capabilities.json`, rojo con «sin dato», y 1.0.0 con
  todo en verde (design D5) — escenarios «Cada PR de release lleva la lista»,
  «Todos los gates en verde proponen 1.0.0» y «Un dato que falta no es un
  gate cumplido»
- [ ] 1.8 El test que lee el Hito v1.0 de `meltemi.md` §10 y exige un gate por
  criterio (design D5) — escenario «El hito del documento fundacional y la
  lista no divergen»
- [ ] 1.9 `prepare`: la sección con la línea `**Breaking**` por ruptura y el
  `--report` del hito, la versión con `set_version`, y el cuerpo de la PR en
  `--body` (design D6) — escenario «Lo archivado sin release abre la PR de la
  versión siguiente» (la parte de la herramienta)

## 2. El workflow

- [ ] 2.1 `.github/workflows/release-pr.yml` con el token de la App desde el
  entorno `release-pr`, la rama `release/next` regenerada desde `main` y la PR
  creada o editada; cerrar la PR y borrar la rama con salida 3 (design D7)
- [ ] 2.2 Sin la clave de la App: aviso, resumen y job en verde (design D7) —
  escenario «Sin la credencial de la App no hay PR y la rama principal sigue
  verde»
- [ ] 2.3 El test estático del workflow: acción pineada por commit, permisos
  declarados, entorno `release-pr`, y ningún push a `main`, tag, merge ni
  aprobación (design D7) — escenario «La PR de release no publica sola»; gate:
  la suite de controles estructurales

## 3. Los documentos

- [ ] 3.1 `docs/release.md`: el camino por PR como el normal, el manual como
  respaldo, la App y su entorno, en inglés y en el resumen en español (design
  D7, D8)
- [ ] 3.2 `docs/versionado.md`: las reglas después de 1.0 y las líneas
  `> **Ruptura**:` y `> **Corrección**:` (design D4) — escenario de la
  política del requisito «Reglas de versión después de 1.0»
- [ ] 3.3 `AGENTS.md`: la versión la propone CI en la PR de release y la
  publica el merge del mantenedor; un agente sigue sin escribirla en `main`
  (design D8)

## 4. Verificación

- [ ] 4.1 *(mantenedor, tras `entrega-sin-huecos` 3.1)* La primera PR real: un
  push a `main` abre «Release 0.2.1» (o «0.3.0» si el informe ya llegó), su CI
  corre en las tres plataformas, y el cuerpo trae el porqué y la lista para 1.0
  con la firma y la prueba en rojo; el resultado queda anotado aquí
- [ ] 4.2 `meltemi validate` y `verify`; archivar
