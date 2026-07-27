# Design — pulido-pre-anuncio

## Context

Dos fuentes alimentan esta change. La auditoría conducida de la GUI (dist
reconstruido servido en local, geometría medida por CDP — el método que
`gui-acabado-y-cierre-sdd` estableció) encontró la causa raíz del apilado de
botones, su inventario completo (7 rotos, 8 re-declaraciones correctas, los de
solo texto y los de solo icono catalogados) y, de propina, el falso contador
del «(4)». La investigación de adaptadores, hecha para decidir sobre
adaptadores propios, encontró de pasada que los comandos de instalación del
registro apuntan a un proyecto archivado y a un scope deprecado. Este design
fija las decisiones para que las correcciones no se «arreglen» en otra
dirección más adelante.

## Goals / Non-Goals

**Goals**: sanar los siete botones en la raíz y que ningún botón futuro pueda
repetir el defecto; que ningún remedio de la flota recomiende una ruta muerta;
que ninguna etiqueta se lea como contador que no es. Todo implementable en un
día y verificable antes del anuncio.

**Non-Goals**: refactor de componentes de la GUI, cambios del contrato
`proto/` o de la semántica de detección, re-evaluación de estatus legales,
adaptadores propios de Meltemi.

## Decisions

### D1 — Regla global en el skin, no componente Button compartido

La causa raíz son dos reglas que interactúan: el skin global de `button` no
declara `display` (los botones quedan en flujo inline por defecto del UA) y el
svg de `Icon` es `display: block` — cualquier botón icono+texto sin regla flex
local rompe la línea. El inventario demuestra que la re-declaración por
componente ES el bug: ocho componentes declaran la regla correctamente y siete
la olvidaron. Dos formas de arreglarlo: (a) un componente Button compartido, o
(b) que el skin global gane la regla de display.

Se elige (b). El componente (a) centralizaría, pero exige tocar todos los call
sites de la superficie, y aún así no protege al `<button>` plano que alguien
escriba mañana. La regla global sana los siete en la raíz, retira la necesidad
misma de re-declarar — la nota de auditoría queda registrada: la repetición
por componente fue la causa raíz, y la corrección elimina la clase del
defecto, no sus instancias — y los overrides locales que quieran otro ritmo
siguen ganando por especificidad (el scoping de Svelte añade una clase al
selector; la regla global es un selector de elemento pelado).

Detalles decididos: el `gap` global es `var(--sp-2)`, el ritmo que ya usa la
mayoría de los botones correctos (barra superior); los overrides más
estrechos (`--sp-1`, herramientas del detalle de sesión) se conservan como
decisión local deliberada. `inline-flex` sobre botones de solo texto no
produce delta visual (un único ítem flex anónimo); los botones con varios
hijos inline ganan `gap` entre ítems — están inventariados y el smoke los
verifica (D5). Solo se retiran las re-declaraciones que dupliquen exactamente
la regla global: retirar un override deliberado sería una regresión estética
silenciosa. En `EmptyState`, `.actions` pasa a `align-items: center`: el
estirado por defecto era lo que igualaba un botón sano a uno roto y lo que
desalineaba los renglones al envolver; con las alturas naturales ya
consistentes, ningún control debe estirarse para fingir consistencia.

### D2 — El «(4)» se elimina; no se re-renderiza como afordancia

Tres opciones sobre la mesa. Dejarlo: se lee como recuento, y en un estado
vacío que dice «sin sesiones» un número junto a «flota» es exactamente el tipo
de dato que el lector asume vivo y obsoleto. Renderizarlo como `kbd` dentro
del botón: duplicaría la afordancia que el sidebar ya renderiza por ítem
(`<kbd>{item.key}</kbd>`), y los atajos numéricos pertenecen a las vistas, no
a botones concretos — un `kbd` dentro de un botón de acción insinuaría que el
atajo activa ese botón. Quitarlo: la etiqueta dice lo que hace, el atajo sigue
visible en su casa, y la cadena del catálogo deja de mezclar prosa localizable
con estado del keymap. Se elige quitarlo, y el requisito nuevo lo generaliza:
los atajos no viajan incrustados en cadenas del catálogo. El keymap vigente no
se toca (requisito «Arquitectura visual de aplicación de escritorio» intacto).

### D3 — Los valores del registro se verifican en la fuente, no de memoria

Hechos verificados contra el registro npm y GitHub el 2026-07-27:

- `@zed-industries/claude-agent-acp` está **deprecado** con el aviso «This
  package has been renamed to @agentclientprotocol/claude-agent-acp»; su
  última publicación es de 2026-03-26. La distribución vigente es
  `@agentclientprotocol/claude-agent-acp` (0.62.0 al consultarla, `bin`
  `claude-agent-acp`, repo `agentclientprotocol/claude-agent-acp`).
- `zed-industries/codex-acp` (Rust) fue **archivado** el 2026-07-22 con
  aviso de migración; `cargo install codex-acp` instala hoy un proyecto de
  solo lectura. La distribución vigente es `@agentclientprotocol/codex-acp`
  (1.1.7, publicada el mismo 2026-07-22, `bin` `codex-acp`), que arranca el
  app-server oficial del CLI del proveedor.

Decisiones derivadas. Los `adapter-install` pasan a
`npm i -g @agentclientprotocol/claude-agent-acp` y
`npm i -g @agentclientprotocol/codex-acp`. Se elige `npm i -g` y no el
`npx -y` que el README del adaptador también ofrece porque la detección de
Meltemi resuelve binarios en `PATH`: `npx` no deja nada que detectar, y el
remedio de la flota debe producir exactamente el estado que la detección
reporta como sano. Los campos `bin`/`adapter` no cambian (los `bin` de npm
conservan ambos nombres), `candidate-paths` tampoco (los shims globales de
npm caen en `PATH` y el sondeo `.cmd`/`.ps1` de Windows ya existe), y el
`version` de la instantánea sube a `2026-07-27` — ningún test pinea el valor
embebido, verificado por grep. Las notas legales no se tocan: siguen siendo
ciertas para las distribuciones nuevas (el adaptador de Codex sigue
envolviendo el app-server oficial y el CLI se autentica solo; el de Claude
sigue montado sobre el Agent SDK y la zona gris sigue siendo la
autenticación). La vigencia upstream no es verificable en CI (sin red
externa): sus escenarios quedan como verificación documentada con fuente y
fecha, el mismo trato honesto que `motor-propio-byok` prometió para su guía.

### D4 — Elegibilidad fast-forward: por criterio, con tripwire declarado

El criterio del motor (design D3 del método, `fast_forward_eligible`): sin
capability nueva y deltas sin MODIFIED ni REMOVED. Esta change cumple ambos
genuinamente, no por maquillaje: los tres requisitos nuevos son superficie
normativa que no existía — el skin nunca prometió alineación, el catálogo de
mensajes nunca prohibió atajos incrustados, la instantánea nunca prometió
vigencia de sus rutas — y ningún requisito vigente necesita reescribirse.
En particular, «Remedio por capa accionable» exige *el comando exacto
declarado en el registro*, no un comando concreto: el refresco de datos lo
satisface sin enmendarlo. Tripwire explícito: si durante la implementación
cualquier requisito vigente exigiera enmienda (por ejemplo, si la semántica
de dos capas resultara insostenible frente a un adaptador que empaqueta su
propio CLI), esa enmienda sale a su propia change o esta pasa a spec-full —
se declara ahora, no se descubre después.

### D5 — La verificación mira la ventana, otra vez

Los wiring tests de `desktop/tests` leen código: sirven para apuntalar la
decisión (el test puede exigir que la regla `button` de `app.css` declare
`inline-flex`, como el precedente prohibió `grid-template-rows` en el shell)
pero no ven geometría, y el apilado fue invisible a todos los tests
existentes mientras era visible al primer vistazo humano. La verificación de
esta change incluye el smoke conducido por CDP sobre el binario reconstruido,
midiendo el inventario completo de botones (los siete rotos en una línea, las
alturas del par del estado vacío, la etiqueta sin «(4)»), publicado en
`docs/qa/` según el método de `gui-acabado-y-cierre-sdd`. Automatizarlo como
gate de CI sigue fuera de alcance.
