# pulido-pre-anuncio

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED, ninguna capability nueva
> (design D4). Alcance de un día, pensado para aterrizar antes del anuncio.

## Why

Al preparar el anuncio quedaron a la vista tres defectos de acabado: dos los
encontró la auditoría conducida de la GUI (geometría medida en vivo sobre el
dist reconstruido, el método que `gui-acabado-y-cierre-sdd` publicó) y uno la
investigación de adaptadores hecha para otra decisión. Ninguno rompe función;
los tres rompen la percepción de producto terminado en el primer minuto de
uso — y uno hace que Meltemi recomiende instalar desde rutas muertas.

1. **Siete botones apilan el icono sobre la etiqueta.** El skin global de
   botones (`desktop/ui/src/app.css`) no declara `display`, y el componente
   `Icon` fuerza su svg a `display: block`: todo botón icono+texto sin regla
   flex local rompe la línea y el icono aterriza en su propia fila. No existe
   componente Button compartido; cada componente re-declara (u olvida) la
   regla en su hoja local, y esa repetición por componente es exactamente la
   causa raíz de que el defecto esté disperso: siete botones rotos hoy
   (estado vacío de Sesiones, Flota ×2, lanzador, Editor ×2, Ajustes) junto a
   ocho re-declaraciones correctas que no deberían necesitar existir. El par
   del estado vacío agrava el cuadro: `.actions` estira por defecto
   (`align-items: stretch`), así que el botón sano se estira para igualar al
   roto, y cuando la fila envuelve cada renglón estira por su cuenta y las
   alturas divergen — exactamente la captura que vio el mantenedor.

2. **El registro recomienda instalar desde rutas muertas.** Los
   `adapter-install` de las dos entradas de nivel 2 quedaron obsoletos: el
   adaptador Rust de la entrada de Codex fue archivado upstream el 2026-07-22
   (su `cargo install` compila hoy un proyecto de solo lectura) y el scope
   npm del adaptador de la entrada de Claude Code está deprecado con aviso
   explícito de renombre. Ambos adaptadores viven hoy bajo la organización
   neutral `agentclientprotocol` — verificado contra el registro npm y GitHub
   el 2026-07-27, no citado de memoria (design D3). El remedio «con el
   comando exacto» que la flota promete en las tres superficies es hoy un
   comando que instala un proyecto muerto o una versión congelada en marzo.

3. **«Ver la flota (4)» se lee como contador obsoleto.** El «(4)» está
   incrustado en la cadena del catálogo (ES y EN, `messages.ts`): es la pista
   del atajo de teclado de la vista 4, pero en un estado vacío que dice «sin
   sesiones» un número entre paréntesis junto a «flota» se lee como recuento
   vivo — y falso. El atajo ya tiene afordancia propia: el sidebar renderiza
   un `kbd` por ítem de navegación.

## What Changes

- `desktop/ui/src/app.css`: la regla global `button` gana
  `display: inline-flex; align-items: center; gap: var(--sp-2)` — los siete
  botones rotos sanan en la raíz y todo botón futuro con icono y etiqueta
  nace correcto. `EmptyState.svelte` fija `align-items: center` en
  `.actions` para que ninguna acción se estire a la altura de otra. Las
  re-declaraciones locales que dupliquen exactamente la regla global se
  retiran; los overrides deliberados (gap más estrecho) se conservan.
- `desktop/ui/src/lib/messages.ts`: `sessions.empty.fleet` pasa de
  «Ver la flota (4)» / «See the fleet (4)» a «Ver la flota» /
  «See the fleet». El atajo sigue visible donde vive: el `kbd` del ítem
  Flota del sidebar (design D2).
- `core/meltemid/data/fleet-registry.toml`: los `adapter-install` de las dos
  entradas de nivel 2 apuntan a las distribuciones canónicas vigentes bajo
  `@agentclientprotocol`, verificadas contra npm el 2026-07-27
  (`npm i -g @agentclientprotocol/claude-agent-acp` y
  `npm i -g @agentclientprotocol/codex-acp`); el campo `version` de la
  instantánea sube a `2026-07-27`. Los nombres de binario no cambian
  (`claude-agent-acp`, `codex-acp`: los `bin` de npm los conservan) y
  `candidate-paths` tampoco — es un cambio de datos puro, ni una línea de
  código del daemon.
- `docs/agentes.md`: las secciones de las dos entradas actualizan sus
  comandos en el mismo commit — el test de coherencia registro↔guía
  (`core/meltemid/tests/agents_guide.rs`) falla si divergen, y eso es por
  diseño.

## Capabilities

### Modified Capabilities

- `gui-shell`: + requisito «Alineación global de los controles con icono»
  (el skin alinea; los componentes no re-declaran) y + requisito «Etiquetas
  de acción sin atajo incrustado».
- `fleet-catalog`: + requisito «Vigencia de las rutas de instalación de la
  instantánea» (comandos verificados contra la fuente de distribución,
  sucesión obligada ante archivado o renombre).

## Impact

- Superficies: GUI (skin de botones y una cadena del catálogo). El refresco
  del registro alcanza a las tres superficies sin cambio propio: los
  remedios fluyen por `fleet/list` a la CLI (`--json`), la vista Flota de la
  TUI y el drawer de la GUI por igual — paridad heredada. El contrato
  `proto/` no se mueve; cero dependencias nuevas.
- Riesgo asumido de la regla global: `inline-flex` toca todos los botones,
  incluidos los de solo texto (inventariados en la auditoría; un único ítem
  flex anónimo, sin delta visual esperable) y los de varios hijos inline,
  que ganan `gap` entre ítems. El smoke visual verifica el inventario
  completo, no solo los siete rotos (design D5).
- Tests: dos wiring tests nuevos en `desktop/tests/scenarios_shell.rs`; la
  coherencia registro↔guía ya la vigilan los tests existentes de
  `agents_guide.rs`, que se re-anclan a los datos nuevos sin tocarse. La
  vigencia de las distribuciones upstream no es testeable en CI (sin red
  externa, jamás): queda como verificación documentada con fuente y fecha.
- El draft de la v0.1.0 precede a estas correcciones: el build que acompañe
  el anuncio debe reconstruirse tras el merge, no reutilizar los artefactos
  del draft.

## Fuera de alcance

- **Componente Button compartido**: la regla global resuelve la clase entera
  del defecto sin tocar ningún call site; el componente sería un refactor de
  toda la superficie para comprar lo mismo (design D1).
- **Semántica de dos capas ante adaptadores que empaquetan su CLI**: el
  adaptador vigente de la entrada de Codex incluye un CLI compatible y honra
  `CODEX_PATH`; si eso amerita repensar el estado `cli_missing` es una
  pregunta de semántica de detección con su propia change y su evidencia.
- **Adaptadores propios de Meltemi** para las dos entradas de nivel 2:
  evaluados en la investigación; propuesta separada si el mantenedor la pide.
- **Estatus y notas legales del registro**: gris sigue gris, tolerado sigue
  tolerado — esta change refresca rutas de instalación, no relitiga posturas.
- **Smoke visual como gate de CI**: sigue fuera, como lo declaró
  `gui-acabado-y-cierre-sdd`.
- Las demás mejoras apuntadas por la auditoría (acciones de gate en la GUI,
  `project/forget`, orden y filtro de tablas): cada una con su change.
