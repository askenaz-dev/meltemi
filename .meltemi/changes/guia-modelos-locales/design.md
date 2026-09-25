# Design — guia-modelos-locales

## Context

Verificado hoy (2026-09-25) antes de diseñar, con el OpenCode instalado
(`opencode --version` → `1.18.32`) y sin ollama en la máquina:

- **`OPENCODE_CONFIG` apunta OpenCode a un archivo propio**, fuera del proyecto:
  con él, desde un directorio cualquiera, `opencode models ollama` lista
  `ollama/qwen2.5-coder:7b` y `opencode models litellm` lista
  `litellm/gpt-5-mini`, ambos declarados como proveedores
  `@ai-sdk/openai-compatible` con `baseURL` local
  (`http://localhost:11434/v1`, `http://localhost:4000/v1`).
- **OpenRouter no existe sin declararlo**: `opencode models openrouter` responde
  «Provider not found» hasta que el archivo declara el proveedor
  (`"openrouter": { "options": { "apiKey": "{env:OPENROUTER_API_KEY}" } }`);
  entonces lista su catálogo (385 modelos, `moonshotai/kimi-k2` incluido) sin
  que la clave tenga valor. La sustitución `{env:…}` es de OpenCode.
- **Del lado de Meltemi**, un `[[fleet.profile]]` lleva `env` al proceso del
  agente. `mcp::resolve_ref` resuelve **solo un valor entero** `${VAR}` o
  `$VAR`: un valor como `"${HOME}/x.json"` se resuelve en silencio a cadena
  vacía (defecto anotado aparte). Una ruta literal no es un secreto para el
  lint (`config.rs::looks_like_plaintext_secret` no marca valores con
  separadores).
- **Nivel 3 no se lanza**: los verbos rehúsan `level_not_pilotable`; la frase
  «OpenCode y Aider … ya funcionan hoy» de `docs/plan-de-cambios.md` y de la
  propuesta de `motor-propio-byok` es falsa en su mitad Aider.
- **El README y el sitio no nombran productos de terceros** (sus lints lo
  prohíben: `tui/tests/docs.rs`, `core/meltemid/tests/site.rs`); la guía de
  agentes sí, porque es dato de interoperabilidad.
- **La corrida de conformidad automatizada** (`conformance_real.rs`) cubre solo
  los dos adaptadores propios; automatizar una pierna de nivel 1 es de
  `corrida-integrada`.

## Goals / Non-Goals

**Goals**: que quien quiera un modelo local o autohospedado bajo Meltemi
encuentre tres recetas verificadas y fechadas, con su perfil; que ninguna
receta lleve una clave en claro y que un test lo sostenga; que el backlog deje
de prometer lo que el daemon rehúsa; y que la pierna en vivo quede escrita para
quien tenga el proveedor local.

**Non-Goals**: código del daemon; el motor propio (`motor-propio-byok`); el
runner de nivel 3; automatizar la pierna local; corregir la resolución de
referencias compuestas (otra change).

## Decisions

### D1 — Una guía en inglés junto a la de agentes, con fecha y alcance de la verificación

`docs/local-models.md` cuelga de `docs/agentes.md` y está en inglés como ella.
Declara arriba con qué se verificó —fecha, versión de OpenCode— y **qué**:
«en forma» (el archivo es aceptado y OpenCode lista el modelo) o «en vivo» (un
turno contra el modelo). Hoy las tres recetas son «en forma»; «en vivo» espera
al mantenedor con ollama (D5).

### D2 — Tres recetas, cada una con su archivo de OpenCode y su perfil de Meltemi

1. **ollama**: proveedor `@ai-sdk/openai-compatible`, `baseURL`
   `http://localhost:11434/v1`, el modelo por `ollama pull`.
2. **LiteLLM** como endpoint único para N proveedores: el mismo tipo de
   proveedor, `baseURL` `http://localhost:4000/v1`, clave por
   `{env:LITELLM_API_KEY}`.
3. **OpenRouter**: el proveedor integrado, **declarado** en el archivo con la
   clave por `{env:OPENROUTER_API_KEY}` en el entorno del daemon, jamás en un
   archivo.

Cada una trae su `[[fleet.profile]]` con `agent = "opencode"` y
`env = { OPENCODE_CONFIG = … }`: una ruta absoluta literal (una ruta no es un
secreto) o una referencia **entera** `${VAR}` cuando la ruta cambia por
máquina; la guía advierte que una referencia dentro de un texto más largo no se
expande. Y la alternativa más corta: un `opencode.json` en la raíz del proyecto,
que OpenCode lee solo.

### D3 — El modelo, por el archivo o por la palanca

El `model` del archivo es el que OpenCode usa por omisión. Para elegirlo por
sesión, `--model` o el `model` del perfil viajan por la opción que OpenCode
anuncia (`modelo-por-opcion-anunciada`): Meltemi lo fija y lo confirma antes del
primer prompt, o lo rehúsa listando lo que OpenCode anunció. La guía no promete
qué lista anuncia OpenCode; dice cómo verlo.

### D4 — Qué se mide

Los tokens no llegan por ACP; lo que el agente reporte —contexto y costo— se ve
como cifra suya (`usage-desde-acp`). Para un modelo local el costo es el que el
agente diga, si dice alguno.

### D5 — La pierna de proveedor local, escrita y pendiente

`docs/conformidad-manual.md` gana la pierna «proveedor local» como
procedimiento manual con OpenCode y ollama: sesión, streaming, permisos (el
aprobador del terminal) y cancelación (Ctrl+C), registrando versión de OpenCode,
modelo y tiempos. Sin proveedor local en la máquina de quien la escribió, la
pierna figura **pendiente** con lo verificado en forma, nunca como aprobada.

### D6 — Lo que no se lanza no se promete

`docs/plan-de-cambios.md` y la propuesta de `motor-propio-byok` dejan de decir
que Aider sirve hoy: nivel 3 no es lanzable; lo será si `runner-nivel-3` existe,
y entonces el prefijo de proveedor lo escribe el usuario —el daemon no traduce
proveedores—.

### D7 — Enlaces donde se busca, sin nombrar productos donde no se puede

`docs/agentes.md` (sección OpenCode) enlaza la guía por su nombre. El README, el
LEEME y el sitio (`site/agents.html` y `site/es/agents.html`) la enlazan con
una frase sin nombres de terceros: hoy, un modelo propio pasa por un agente de
la flota que habla con proveedores locales por su propia configuración; el
motor propio de Meltemi no existe todavía.

### D8 — Un lint que no deje envejecer la guía

Un test extrae los bloques de la guía: cada JSON parsea, declara el `$schema` de
OpenCode, sus `baseURL` locales son `localhost` y toda `apiKey` es una
referencia `{env:…}`; cada TOML parsea con `fleet`, sus perfiles apuntan a
`opencode` y ningún valor parece un secreto en claro (el mismo lint del
daemon) ni mezcla una referencia con texto. La página declara la fecha y la
versión con que se verificó.

## Risks / Trade-offs

- **R1 — OpenCode cambia su formato**: la guía declara la versión y la fecha; el
  lint pinea la forma que documenta, no la de OpenCode, así que una versión
  nueva exige volver a verificar a mano (dicho en la página).
- **R2 — «En forma» no es «en vivo»**: se dice en la página y en la pierna.

## Migration

Solo documentación y un test. Ningún cambio de contrato ni de comportamiento.
