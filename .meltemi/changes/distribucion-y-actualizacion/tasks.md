# Tasks — distribucion-y-actualizacion

Vía completa. Un commit atómico por tarea, con referencia
`(distribucion-y-actualizacion N.M)` y sin trailers de co-autoría. Ningún
secreto entra a este repositorio; las tareas *(mantenedor)* no son de un
agente.

## 1. Del mantenedor, antes

- [ ] 1.1 *(mantenedor)* La decisión del tap (design D2); el tap y
  `askenaz-dev/scoop-meltemi` creados, con la GitHub App instalada en los dos
  (`contents: write`)
- [ ] 1.2 *(mantenedor)* La salida de winget (design D3): la cuenta de máquina
  (recomendada) o su propia cuenta, el fork de `microsoft/winget-pkgs` y el
  token `public_repo`, declarados en el Terraform de meltemi-web

## 2. Los manifiestos

- [ ] 2.1 Las plantillas de `packaging/` y su test: renderizadas con un
  `SHA256SUMS` de fixture, cada hash y cada URL coinciden; la fórmula pasa
  `ruby -c` (design D1, D2) — escenario «Los manifiestos salen de lo
  publicado» (la parte de la plantilla)
- [ ] 2.2 El job `distribute` en `release.yml`, tras `publish`, en el entorno
  `release`: `SHA256SUMS` verificado con la atestación; tap y bucket con el
  token de la App; idempotente; cada destino en su paso, y un destino sin
  credencial se salta con aviso (design D1, D2) — escenarios «Sin publicación
  no hay manifiesto» y «Un gestor que falla no deshace nada»; gate: los
  controles estructurales de workflows
- [ ] 2.3 winget con `komac` pineado y verificado: los dos paquetes; verificar
  en una VM de Windows que el zip portable deja los adaptadores junto al
  daemon, o aplazar el paquete del núcleo y decirlo (design D3)
- [ ] 2.4 El cask, solo con el DMG notarizado según `capabilities.json` (design
  D2) — escenario «Sin notarización no hay cask»

## 3. El cliente

- [ ] 3.1 `install_channel` pura, con tests por plataforma sobre rutas de
  fixture (design D4)
- [ ] 3.2 `meltemi version --verbose` y su `--json`; la referencia de la CLI
  regenerada — escenarios «Instalado por un gestor», «Canal desconocido» y «La
  salida corta no cambia»; gate: `tui/tests/docs.rs` y la suite de la CLI
- [ ] 3.3 «Acerca de» en Ajustes: canal, comando copiable y «Ver releases» por
  el abridor del sistema, con las claves en los dos idiomas (design D4) —
  escenarios «El comando de actualización se copia», «Ver releases abre el
  navegador del usuario» y «Canal ambiguo»; gate: `scenarios_shell` y lint de
  i18n

## 4. Los textos

- [ ] 4.1 `docs/release.md`, `README.md` y `LEEME.md`: los comandos de cada
  gestor junto a los instaladores directos, que la atestación se verifica
  igual, y que Meltemi no consulta versiones por red (design D5) — escenarios
  «Los comandos de cada gestor están escritos» y «Ningún cliente HTTP en el
  árbol» (este último ya lo cumple `deny.toml`; se nombra en el texto)

## 5. Verificación

- [ ] 5.1 *(tras la primera release con `distribute`)* En máquinas limpias:
  `brew install`, `scoop install` y `winget install` del núcleo; `meltemi
  doctor` encuentra los adaptadores; `meltemi version --verbose` nombra su
  canal; `brew audit` del tap; anotarlo aquí
- [ ] 5.2 `meltemi validate` y `verify`; archivar
