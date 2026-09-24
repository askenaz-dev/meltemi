# guia-modelos-locales

> Vía rápida (fast-forward): documentación verificada, una pierna nueva de la
> corrida de conformidad manual y la corrección de una afirmación falsa del
> backlog. Ningún código del daemon. Nace del comparativo con Orca y de la
> prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas LM-1, LM-2, LM-5,
> LM-6, LM-8 y P5).

## Why

Orca no tiene historia de modelos locales: es «trae tu agente y tu
suscripción», no «trae tu modelo». El rumbo de Meltemi dice BYO-agente,
BYOK y BYO-modelo desde el primer día, y la vía existe: OpenCode (nivel 1)
habla con ollama, con cualquier endpoint OpenAI-compatible y con OpenRouter
por su propio `opencode.json`. El 2026-09-24 se verificó en forma: un
`opencode.json` con proveedores `ollama` (`http://localhost:11434/v1`) y
`litellm` (`http://localhost:4000/v1`) es aceptado y `opencode models ollama`
lista el modelo declarado; y `meltemi session … --agent opencode` corrió con
el modelo elegido en ese archivo. Lo único que faltó fue un modelo vivo,
porque esta máquina no tiene ollama.

Nada de eso está escrito. `grep -ri 'ollama|litellm|openrouter'` en `docs/`,
`README.md`, `site/` y `core/meltemid/data` devuelve solo el backlog y la
propuesta de `motor-propio-byok`. Y lo que el backlog dice es falso en una
mitad: `docs/plan-de-cambios.md` («correr un modelo local bajo Meltemi ya es
posible hoy y sin código nuevo: OpenCode (nivel 1) y Aider (nivel 3)…») y la
misma frase en la propuesta del motor. Aider es nivel 3 y **nivel 3 no se
lanza**: los cinco verbos rehúsan con `level_not_pilotable`
(`core/meltemid/src/free_session.rs:101-112`, `propose.rs:102-113`,
`sdd_flow.rs:477-486`, `server.rs`). Prometer en el backlog lo que el daemon
rehúsa es la clase de deuda que un usuario descubre a la primera.

La pieza de configuración ya existe: `[[fleet.profile]]` lleva `env` con
referencias `${VAR}` y lint de secretos (`core/meltemid/src/config.rs`,
`FleetProfile`), así que «OpenCode contra ollama» puede ser un perfil con
nombre, elegible desde las tres superficies, sin tocar Meltemi.

## What Changes

- **`docs/local-models.md`** (en inglés, como `docs/agentes.md`, que es la
  página de la que cuelga): tres recetas **verificadas y fechadas** contra
  las versiones vigentes, nunca citadas de memoria —
  (1) OpenCode + ollama, con el `opencode.json` exacto y el `ollama pull`;
  (2) OpenCode + LiteLLM como endpoint único para N proveedores (`localhost:4000/v1`,
  clave por `{env:…}`);
  (3) OpenCode + OpenRouter, con `OPENROUTER_API_KEY` en el entorno del
  daemon y **jamás** en `config.toml` (constitución §2; el lint de secretos ya
  rechaza literales).
  Cada receta trae su `[[fleet.profile]]` (nombre, agente, `env` con la
  variable de configuración que OpenCode documenta para apuntar a un archivo
  propio) y una frase sobre qué se mide y qué no (tokens: solo si el agente
  los reporta, `usage-desde-acp`).
- **Pierna «proveedor local» en `docs/conformidad-manual.md`**: OpenCode +
  ollama, las cuatro comprobaciones de nivel 1 (sesión, streaming, permisos,
  cancelación) con la versión de OpenCode, el modelo y los tiempos medidos.
  Manual y opt-in, como toda la página (constitución §5).
- **Corrección de la afirmación sobre Aider** en `docs/plan-de-cambios.md` y
  en `.meltemi/changes/motor-propio-byok/proposal.md`: nivel 3 no es lanzable
  hoy; lo será si `runner-nivel-3` existe, y entonces el prefijo de proveedor
  (`ollama_chat/…`) lo escribe el usuario y el registro aporta la bandera —
  el daemon no traduce proveedores.
- **Enlaces donde se busca**: `docs/agentes.md` (sección OpenCode), los dos
  README y la sección BYO del sitio (`site/agents.html` y `site/es/agents.html`)
  apuntan a la guía con una frase que dice lo que hay: «hoy, vía OpenCode;
  el motor propio es otra change».
- **Un lint que no deje envejecer la guía**: los bloques JSON de las recetas
  se extraen y se parsean en un test (formato válido, `baseURL` locales,
  ninguna clave literal), y la página declara la fecha y las versiones con
  las que se verificó.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `initial-docs`: + requisito «Guía de modelos locales verificada y fechada»
  (tres recetas, perfiles, sin secretos literales, lint de formato).
- `integration-levels`: + pierna «proveedor local» en la corrida de
  conformidad manual.
- `product-site`: + enlace BYO-modelo con la frase honesta.

## Impact

- Archivos: `docs/local-models.md` (nuevo), `docs/conformidad-manual.md`,
  `docs/agentes.md`, `docs/plan-de-cambios.md`,
  `.meltemi/changes/motor-propio-byok/proposal.md`, `README.md`, `LEEME.md`,
  `site/agents.html`, `site/es/agents.html`, un test de docs.
- Cero dependencias nuevas; cero RPC.
- Lo que solo el mantenedor puede hacer: instalar ollama (`winget install
  Ollama.Ollama`, `ollama pull qwen2.5-coder:7b`) y correr la pierna. Sin eso,
  la receta (1) se publica como «verificada en forma» y lo dice.

## Fuera de alcance

- El motor propio con manifiesto (`motor-propio-byok`) y el runner de nivel 3.
- Un catálogo de modelos o cualquier lectura del nombre del modelo por parte
  del daemon (constitución §5).
- Empaquetar ollama, LiteLLM o cualquier proveedor.
