# QA — Smoke visual conducido del tablero de carrera (2026-08-07, tablero-de-carrera)

Verificación de `tablero-de-carrera` **mirando la ventana**, por el método que
estableció `gui-acabado-y-cierre-sdd` (docs/qa/2026-07-26-gui-acabado-smoke.md) y
repitieron `pulido-pre-anuncio` y `lanzador-conversacional`. La change es en su
mitad superficie —calles lado a lado, procedencia por calle, acciones de carrera
con confirmación, vida del tablero— y los tests de cableado leen fuentes: no ven
layout, ni lo que un texto se vuelve al envolverse, ni si un diálogo se alza de
verdad entre el clic y el daemon. Este smoke sí, y encontró una cosa que ninguno
de ellos podía ver (§Hallazgos).

## Método

- Binario de release construido con la CLI de Tauri (`ui/node_modules/.bin/tauri
  build --no-bundle`; **nunca `cargo build --release`**, que no activa
  `tauri/custom-protocol`), con `additionalBrowserArgs:
  --remote-debugging-port=9455` **temporal** en `tauri.conf.json`, revertido y el
  binario **reconstruido limpio** al terminar: el binario publicado no expone
  puerto alguno.
- Entorno aislado: `MELTEMI_DATA_DIR` y `MELTEMI_CONFIG_DIR` propios en el
  scratchpad, y un repo fixture (`harbour`) con regla `allow`, dos proveedores en
  registro local (`provider-a`, `provider-b`) y dos perfiles de lanzamiento
  (`work`, `thorough`) con marcador de contexto distinto. El único agente es
  `mock-agent`. Nunca este repositorio, nunca agentes reales, nunca red.
- Las dos calles se crearon **de verdad**: `meltemi dispatch dark-mode 1.1 work`
  y `… thorough` contra ese fixture, cada uno con su worktree, su checkpoint y su
  commit.
- Driver Node sobre el WebSocket de CDP: `Runtime.evaluate` leyendo el DOM real y
  midiendo cajas, clicks reales sobre los nodos, `Page.captureScreenshot` para la
  mirada humana. Ventana 1200×800 lógicos.
- Contraste de dos fuentes: lo que la ventana muestra frente a lo que el daemon
  responde (`meltemi --json race dark-mode 1.1`), para que la procedencia pintada
  sea la procedencia registrada y no una coincidencia.

## Resultado: lo que la change promete, medido sobre el binario

| # | Lo prometido | Medido |
|---|---|---|
| 1 | El tablero abre desde la superficie del método | `Revisar diffs` → tarea `dark-mode · 1.1 — work, thorough`; las calles cargan en **3 689 ms** (el diff de cada worktree lo calcula git en el momento) |
| 2 | Calles lado a lado | 2 calles, cajas **462×580** cada una, en rejilla de dos columnas |
| 3 | Procedencia visible por calle | `resolución: perfil` · `suscripción: work` · `nivel: L1` — y la otra calle con `suscripción: thorough`. Idéntico a lo que el daemon reporta por contrato (`source: profile`, `profile: work/thorough`, `level: 1`) |
| 4 | Estado en señal **y** palabra | `■ turno concluido` · `◆ comiteado e13dfa7afe` · `⌖ con checkpoint`; la otra calle con `2fb2f1435b` |
| 5 | Cada calle con su base | `base común d361d3e4ad` declarada en cada calle, no solo en la cabecera |
| 6 | Las acciones de la carrera, en la calle | `Despachar un turno`, `Revertir al checkpoint`, `Commit de la tarea` por calle |
| 7 | Merge asistido por archivo | `Aplicar en thorough` en la calle `work`, `Aplicar en work` en la otra: por archivo, hacia la otra calle, nunca "elige ganador" |
| 8 | La acción se compone en el formulario tipado del contrato | panel `checkpoint/revert` con `projectRoot*`, `change*`, `task*`, `agent*`, `confirm`, y el pie `generado desde checkpoint.schema.json#revertParams` |
| 9 | El `confirm` del contrato no viene pre-marcado | casilla `confirm` **sin marcar** en el formulario: la guarda del daemon la decide el humano |
| 10 | Acción destructiva solo con confirmación explícita | pulsar `Enviar` alza el diálogo en **1 ms** en vez de enviar: «"Revertir al checkpoint" sobre la calle de work. Es una operación destructiva y se envía al daemon en cuanto la confirmes.» |
| 11 | Cancelar no envía nada | tras `Cancelar`: **0** diálogos abiertos y los shas de las dos calles **idénticos** (`e13dfa7afe`, `2fb2f1435b`) antes y después — no hubo revert |
| 12 | El límite de lo que el tablero puede seguir, declarado | «El tablero sigue en vivo los turnos que lanza esta ventana. Un despacho iniciado en otra superficie no llega hasta aquí: actualiza para verlo», junto al botón `Actualizar` |
| 13 | Sin desbordes horizontales | `scrollWidth` 1200 = `clientWidth` 1200 |

## Hallazgos

**H1 (corregido en esta misma change) — la nota de vida se leía como parte de la
base.** La cabecera del tablero era una fila flex con tres hijos, así que
`base común: d361d3e4ad`, la nota y `Actualizar` caían en la misma línea y la
nota partía a mitad de frase («…no llega hasta aquí: / actualiza para verlo»)
pegada al sha. Se lee como una sola oración sobre la base, que no es lo que dice.
Corregido: la cabecera pasa a rejilla, con `base común` y `Actualizar` en su fila
y la nota en su propio párrafo acotado a 68ch. Verificado sobre el binario
reconstruido (captura `03-board`). Ningún test de cableado podía verlo: los tres
nodos estaban presentes y en orden, que es todo lo que una aserción de fuente
comprueba.

**No es hallazgo — binario obsoleto del operador.** En la primera pasada las dos
calles decían `sin turno despachado` pese a haber sido despachadas. La causa no
era el tablero: el `meltemi.exe` de `target/release` era del **26-jul**, anterior
a las tareas 2.1/2.2, así que aquellos despachos nunca escribieron registro de
índice. Reconstruidos los binarios y rehechos los despachos, la procedencia
aparece completa. Queda anotado porque el modo de fallo —medir la superficie
nueva con una CLI vieja— se repetirá si no se nombra.

## Frontera de esta verificación

- Un smoke conducido prueba **esta** plataforma (Windows 11, WebView2
  `Edg/151.0.4129.59`) y **este** recorrido. Ni es una prueba de las tres
  plataformas ni sustituye a los tests: es la mirada que los tests no tienen.
- El agente es el simulado. Que la calle muestre `turno concluido / comiteado` se
  midió contra el mock, no contra un proveedor real; lo que se verifica aquí es
  que la superficie pinta lo que el daemon registró, no lo que un agente concreto
  hace.
- El tablero del shell (TUI) no entra en este smoke: su verificación es por tests
  de buffer, incluida la presentación ASCII, que es la forma en que una terminal
  se mira.

Capturas y mediciones crudas del recorrido (`measured.json`, `confirm.json`) se
conservaron fuera del repositorio, en el scratchpad de la sesión: contienen rutas
absolutas de la máquina y no aportan a la verdad viva.
