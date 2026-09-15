# harness-global-y-por-agente — design

> El proposal pidió design por delante y dejó tres preguntas abiertas: el
> front-matter (parser propio o dependencia), la resolución entre capas, y la
> colocación de la vista. Las tres se responden abajo **contra lo que hay en
> disco**, no de memoria: los `RULE.md` reales de FDH, su instalador de reglas,
> el parser de front-matter de este repo y el mapa de destinos vigente.
>
> Y leer eso movió dos cosas que el proposal daba por sentadas. Están en D4 y
> D6, marcadas como hallazgos.

## Lo verificado, con su cita

**El formato real de una regla de FDH** (`forge-development-hub/rules/no-any-cast/RULE.md`):

```yaml
---
name: no-any-cast
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx}"]
severity: error
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits the TypeScript escape hatches ..."
tags: [typescript, type-safety, quality, strictness]
owner_team: dx-platform
---
```

Tres hechos que el proposal no tenía a la vista:

1. **Son nueve claves, no cuatro.** El proposal nombró `name`, `scope`,
   `severity`, `description`. Los archivos reales traen además `kind`,
   `version`, `agents_supported`, `tags`, `owner_team`.
2. **Hay comentarios al final de un escalar**: `version: 0.2.0 # x-release-please-version`.
3. **Todas las listas son en línea** (`[a, b, c]`). Ninguna usa la forma de
   bloque con guiones.

**El parser de front-matter de este repo** (`core/meltemi-spec/src/frontmatter.rs`)
cubre exactamente `key: escalar` y `key: [a, "b", 'c']`, con `unquote` y
`parse_list` ya escritos y probados. Su propio encabezado dice: «If the
front-matter ever grows richer shapes, this is the place to swap in a real YAML
parser». No cubre comentarios al final de línea ni listas de bloque.

**El instalador de reglas de FDH** (`fdh/pkg/adapters/rule_adapter.go:20,100-103`)
escribe `<agent-root>/rules/<name>.md` —`.claude/rules/`, `.codex/rules/`,
`.github/rules/`, `.opencode/rules/`— **cada regla en su propio archivo**, con
un marcador en un archivo aparte.

**La proyección de este repo** (`core/meltemid/src/context.rs`) compila **un
solo string** y lo escribe dentro de un bloque `<!-- meltemi:context:begin
sha256=… -->` en cada destino de `context-targets.toml` (`AGENTS.md`,
`CLAUDE.md`, `GEMINI.md`), preservando byte a byte lo de fuera, con escritura
atómica e idempotente por huella.

**El catálogo de flota de este repo** (`core/meltemid/data/fleet-registry.toml`)
usa los ids `claude-code`, `codex-cli`, `copilot-cli`, `opencode`, `cursor-cli`,
`gemini-cli`, `kiro-cli`, `kilo-code`, `aider`, `antigravity`.

## D1 — Cuatro ámbitos, y el eje por agente es un directorio

```
<config>/meltemi/harness/rules/<name>/RULE.md              (1) usuario
<config>/meltemi/harness/per-agent/<id>/rules/<name>/RULE.md (2) usuario × agente
<repo>/.meltemi/harness/rules/<name>/RULE.md               (3) proyecto
<repo>/.meltemi/harness/per-agent/<id>/rules/<name>/RULE.md (4) proyecto × agente
```

Precedencia 1 < 2 < 3 < 4: lo específico pisa lo general, proyecto pisa usuario
— la misma dirección que ya rige config, permisos y perfiles. `<id>` sale del
catálogo de flota; un `<id>` que no esté en el catálogo se **lista como
desconocido** en la vista y no se proyecta, porque proyectar a un agente que
Meltemi no sabe nombrar es escribir a ciegas.

El eje por agente es un **directorio** y no un campo dentro del archivo. Dos
razones, y la segunda es la que decide: (a) `per-agent/` no choca con el pilar
`agents/` (subagentes) cuando llegue; (b) un campo obligaría al daemon a
**interpretar** a quién aplica cada regla, y la regla de esta change —la misma
que rige los manifiestos del motor— es que el daemon valida forma y lista con
fuente, jamás interpreta semántica. Un directorio es forma. Un campo es
semántica.

## D2 — El formato es el de FDH, entero y sin recortar

`rules/<name>/RULE.md` tal cual, incluidas las cinco claves que el proposal no
enumeró. El daemon **conoce** cuatro (`name`, `scope`, `severity`,
`description`) porque son las que la vista muestra y la proyección usa, y
**conserva las demás sin tocarlas**: se leen, se listan y se muestran verbatim.

Rechazar una clave desconocida convertiría cada campo que FDH agregue en un
error aquí, y la fábrica es de otro repositorio con su propio ritmo. Ignorarlas
sin mostrarlas escondería información que el autor de la regla puso a
propósito. Se conservan y se muestran: es la única opción que no le miente a
ninguno de los dos lados.

`name` debe coincidir con el nombre del directorio. Si no coinciden, la regla se
lista **inválida con diagnóstico** y no se proyecta: el nombre del directorio es
la identidad con la que resuelve la precedencia, y una regla cuyo front-matter
dice otra cosa haría que la vista y la resolución hablaran de cosas distintas.

## D3 — Se extiende el parser propio; no entra una dependencia

**Decisión: no hay dependencia nueva** (§10). Los `RULE.md` reales usan solo
escalares y listas en línea, que es literalmente lo que `frontmatter.rs` ya sabe
leer y tiene probado. Traer un YAML completo para no usar el 95 % de él sería
pagar superficie de ataque y tiempo de compilación por nada.

Lo que sí hay que hacer, y es poco:

1. **Generalizar el escáner a un mapa** `clave → valor`. Hoy tiene las cuatro
   claves de `rumbo/` incrustadas en un `match`; el harness necesita las suyas y
   además conservar las que no conoce (D2). Sale un `parse_front_matter(&str) ->
   Vec<(String, Value)>` reutilizado por los dos, con `Value::Scalar` y
   `Value::List` — las dos formas que existen.
2. **Recortar el comentario al final de un escalar.** `version: 0.2.0 #
   x-release-please-version` hoy devolvería `0.2.0 # x-release-please-version`.
   Es el defecto que un archivo real de FDH tiene ahora mismo y que ningún test
   de este repo podía encontrar, porque `rumbo/` no usa comentarios. Se recorta
   desde el primer `#` que no esté entre comillas.

Y **lo que no se soporta se rehúsa, no se adivina**: una lista de bloque
(`scope:` seguido de líneas con guion) hace que la regla se liste **inválida
con diagnóstico** que nombra la clave y la forma esperada. Leer solo la primera
línea y seguir daría un `scope` silenciosamente equivocado — una regla que
aplica donde no debía. El día que un `RULE.md` real la use, el diagnóstico dirá
exactamente qué agregar, y sigue siendo el sitio donde entra un YAML de verdad
si alguna vez se justifica.

## D4 — `agents_supported` se conserva y NO se interpreta (hallazgo)

FDH escribe `agents_supported: [claude-code, codex, copilot, opencode]`. El
catálogo de este repo dice `codex-cli` y `copilot-cli`. **Los vocabularios no
coinciden**, y el proposal no lo sabía.

Mapear `codex` → `codex-cli` sería inventar una equivalencia que FDH nunca
declaró: hoy parece obvia, y basta con que la fábrica agregue un `codex-web` o
que este catálogo gane una segunda entrada de Codex para que la equivalencia
adivinada empiece a aplicar reglas al agente equivocado en silencio.

Entonces, en v1: **el campo se conserva, se muestra verbatim en la vista, y no
gobierna nada**. Quién recibe una regla lo decide el eje `per-agent/<id>/` (D1),
que usa los ids de este catálogo y no admite ambigüedad. Un humano que vea
`agents_supported: [codex]` en la ficha tiene la información para decidir; el
daemon no la usa para decidir por él.

Alinear los dos vocabularios es trabajo del contrato FDH↔Meltemi, del lado de
FDH y posterior, como todo lo demás de esa integración.

## D5 — La capa más específica gana entera, y la vista lo dice

Resolución **por nombre de regla**: si `no-any-cast` existe en la capa 1 y en la
4, gana la 4 **completa** — no se fusionan campos. Un merge parcial produciría
una regla que nadie escribió, con el `scope` de una capa y el cuerpo de otra, y
nadie podría explicar de dónde salió.

La vista muestra las dos: la vigente con su capa, y la pisada con su capa y
quién la pisó — al estilo `git config --show-origin`, que es el precedente que
el proposal nombra. **Lo que no aplica se muestra igual, con su motivo.** La
visibilidad es el producto: una vista que solo lista lo vigente responde «qué
hay» y deja sin responder «por qué no está la mía», que es la pregunta con la
que la gente llega.

El proposal ya declaró que si «gana entera» alcanza solo lo dirá el uso. Se
mantiene: la composición más fina se decide con evidencia.

## D6 — Dos contenidos, dos destinos, y el guardián es estructural (hallazgo)

El proposal temía **dos escritores** pisándose con FDH. Leído el instalador de
FDH, el temor es menor de lo que parecía y conviene decirlo con la evidencia:
FDH escribe **archivos propios** (`.claude/rules/<name>.md` y su marcador),
Meltemi escribe **dentro de su bloque gestionado** de `CLAUDE.md`/`AGENTS.md`.
Son conjuntos de archivos disjuntos. En v1 no hay colisión que resolver, y esto
no es una promesa: es dónde escribe cada uno.

La proyección se extiende así:

| contenido | fuentes | destinos |
|---|---|---|
| **de repositorio** (hoy) | constitución + rumbo + change activa + **reglas de ámbito proyecto (3, 4)** | `context-targets.toml` dentro del repo |
| **de usuario** (nuevo) | **solo reglas de ámbito usuario (1, 2)** | archivos de instrucciones de ámbito usuario de cada agente |

**El guardián es estructural, no una comprobación.** La función que compila lo
de repositorio no recibe las fuentes globales: no hay parámetro por el que
puedan entrar. Un guardián implementado como un `if` se puede olvidar en la
siguiente refactorización; uno implementado como una firma que no admite el dato
no. El test que lo acompaña proyecta con harness global presente y exige `git
status` limpio.

Las reglas de proyecto entran al bloque con su `scope` **en prosa** junto a cada
una («aplica a `src/**/*.ts`»), porque el bloque es Markdown para un agente que
lee instrucciones, no un formato con campos. Los destinos nativos con ámbito
(`.cursor/rules`, `applyTo` de Copilot) siguen fuera, como declaró el proposal.

## D7 — Destinos de ámbito usuario: solo los verificados

`context-targets.toml` gana rutas de ámbito usuario. Pero **solo las que estén
verificadas contra documentación del proveedor**, con su `verified_on` como ya
hace FDH en su propio mapa.

Lo que hay hoy en este repositorio: `docs/research/integracion-agentes.md`
documenta el archivo global de **OpenCode** (`AGENTS.md` global + proyecto) y de
ningún otro. `~/.claude/CLAUDE.md` es de uso corriente y el proposal lo nombra,
pero esta change **no cita de memoria** — igual que `modelo-y-esfuerzo-por-sesion`
no cableó el esfuerzo de Claude sin verificarlo.

Entonces: el mapa gana la **columna** de ámbito usuario y las entradas que se
puedan verificar al implementar, con su cita. Un agente sin ruta verificada
aparece en la vista como **«sin destino global verificado»**, que es una
ausencia declarada y no un silencio. Agregar una ruta después es una línea de
datos, que es exactamente para lo que el mapa es data-driven.

**Consentimiento en la primera escritura.** Un archivo de usuario de un agente es
config ajena: la primera vez que Meltemi va a escribir en uno, pide
consentimiento explícito, lo registra, y a partir de ahí solo toca su propio
bloque. El cuidado de `mcp-passthrough` (§2) aplica entero, y nunca se
sobrescribe contenido del usuario.

## D8 — La vista, y qué RPC la sirve

`harness/effective` con `projectRoot` opcional y `agent` opcional:

- **sin proyecto**: solo las capas 1 y 2 — el harness global del usuario;
- **con proyecto**: las cuatro, resueltas;
- **con agente**: además de resolver, marca qué vino del eje por agente.

Devuelve, por pieza: nombre, capa de origen con su ruta, estado (vigente /
pisada por / inválida con diagnóstico), el front-matter conocido y el
conservado. Es decir: todo lo que hace falta para responder «qué le aplica a
este agente en este proyecto, y de dónde viene cada cosa» sin abrir un archivo.

Tres superficies (§4): verbo `harness [--agent <id>]` con `--json`, drill-in
desde la Flota en GUI y en TUI. **Solo lectura** en v1, como declaró el
proposal: la autoría vive en archivos y en FDH.

## D9 — Lo que esta change no puede verificar sola

- **Que cada agente lea de verdad su archivo de ámbito usuario** en la forma que
  el mapa declare: se verifica agente por agente al implementar D7, con su cita,
  y lo no verificado no se escribe.
- **Si «la capa más específica gana entera» alcanza** (D5): lo dirá el uso.
- **Que el vocabulario de ids converja con FDH** (D4): trabajo del otro
  repositorio, posterior a esta change y alineado a lo que aquí queda fijado.
