# flota-por-suscripcion

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — un delta solo ADDED sobre una capability existente
> (`gui-shell`), ninguna capability nueva, ningún MODIFIED ni REMOVED, y alcance
> de un día: una tabla que agrupa, una etiqueta que se escribe con palabras, sus
> tests y una comprobación sobre el binario.

## Why

El mantenedor lo dijo así: «la flota no permite configurar varios agentes de un
tipo, con el fin de configurar varios claude con distintas suscripciones, varios
codex con distintas suscripciones». La frase describe lo que ve, y lo que ve es
cierto. Lo que la frase supone —que la capacidad falta— no lo es, y conviene
decirlo antes de construir nada.

**La capacidad existe y funciona.** El smoke de `cromo-que-no-estorba` lo
fotografió sin buscarlo: la tabla de Flota listaba `prueba-de-aviso`, `work` y
`thorough` como filas propias, con fuente «perfil», junto a los agentes del
registro. `subscription/link` crea tantas como se le pidan y el daemon las
resuelve. Lo que no existe es la **lectura**.

**Una fila de suscripción no dice de qué agente es.** El contrato lleva
`underlyingAgent` desde `flota-multiproveedor`; el cajón de detalle lo muestra;
**la tabla lo tira**. Así que tres suscripciones de Claude Code y dos de Codex
aparecen como cinco filas sueltas con nombres inventados por el usuario, en el
mismo plano que los diez agentes del catálogo, sin nada que las ate a su
proveedor. Configurar varias es posible; saber qué configuraste, no.

**Y la asimetría es al revés de lo que uno esperaría.** El CLI ya lo dice: cada
perfil se imprime con `(profile → claude-code)` desde `flota-multiproveedor`. La
superficie que sí lo dice es la terminal; la que lo esconde es la gráfica.

**De paso, una violación de spec confirmada en la columna de al lado.** El
mantenedor ya había preguntado por ella: «¿por qué opencode no tiene el check en
el level?». `integration-levels` exige que la vista Flota muestre la distinción
entre nivel declarado y verificado **«con etiqueta textual»**, y la tabla pinta
un `✓` a secas. Un glifo sin palabra es exactamente lo que esa frase prohíbe, y
está en la columna que esta change ya está tocando.

## What Changes

- **La tabla se lee por agente y por suscripción.** Cada agente del catálogo
  aparece seguido de las suscripciones que se le han enlazado, sangradas y
  diciendo con palabras de quién son. El agente lleva el recuento de las suyas,
  de modo que «dos Claude y tres Codex» se lee de un vistazo.
- **Una suscripción huérfana no se esconde**: si su agente subyacente no está en
  el catálogo —una configurada a mano contra un id que ya no existe— se lista
  igual, marcada, con el id que declara. Desaparecer no es un diagnóstico.
- **El nivel se dice con palabras**: «declarado» o «verificado», no un `✓` que
  el usuario tiene que adivinar. Se cumple así un requisito que la superficie
  llevaba incumpliendo, y el test queda enlazado al escenario que ya existe en
  la verdad viva.
- **El cajón del agente enseña sus suscripciones** y desde ahí se añade otra; el
  cajón de una suscripción nombra su agente y ofrece desvincular. Lo que ya
  hacía sigue igual: el gesto de autenticación y la ruta del contexto.

## Capabilities

### Modified Capabilities

- `gui-shell`: + la flota agrupada por agente con sus suscripciones legibles
  como texto, incluida la huérfana. La distinción declarado/verificado **no**
  entra como requisito nuevo: ya existe en `integration-levels` y lo que esta
  change hace es cumplirla y enlazarle un test.

### New Capabilities

- Ninguna.

## Impact

- `desktop/ui/src/lib/fleet-groups.ts` (nuevo, puro: el agrupamiento y su orden),
  `desktop/ui/tests/fleet-groups.test.ts` (nuevo),
  `desktop/ui/src/lib/views/Fleet.svelte`, `messages.ts`,
  `desktop/tests/scenarios_shell.rs`.
- **Cero cambios en el daemon, en el contrato y en la TUI**: todo lo que la
  tabla necesita ya viaja en `fleet/list`, y la terminal ya lo imprime. No nace
  deber de paridad §4 porque no hay capacidad nueva del daemon; lo que hay es
  una superficie que deja de esconder lo que la otra ya enseña.
- Cero dependencias nuevas.

## Fuera de alcance

- **Crear, editar o borrar suscripciones desde la tabla**: se enlazan y se
  desvinculan desde el cajón, como hoy. Mover esas acciones a la fila es otra
  conversación sobre densidad.
- **Agrupar por proveedor** (todos los de Anthropic juntos): el catálogo no
  declara proveedor, y deducirlo del nombre sería adivinar.
- **Ejecutar el gesto de autenticación por el usuario**: sigue siendo suyo, y la
  constitución lo exige.
- **Las pestañas al estilo Chrome**: su propia change, ya pedida.
