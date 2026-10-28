# harness-global-y-por-agente

> Spec-full deliberado: capability nueva (`agent-harness`), descartada la vía
> rápida por criterio D7. Nace de la directiva del mantenedor (2026-08-09):
> definir comportamiento base y tecnologías una sola vez —globalmente— y
> ajustarlo por agente y por proyecto, reutilizando su fábrica Forge
> Development Hub (FDH) como base de formatos. Fase 1 del harness: el pilar
> Rules más la vista efectiva; Skills, Hooks y Subagentes quedan nombrados
> como changes futuras, no coladas aquí.

## Why

El pedido del mantenedor, en su propio ejemplo: «global decir el backend
siempre es en FastAPI y usa las tools como xxx, pero en local para un
proyecto poder customizar un agente y sobreescribir eso». Hoy Meltemi no
puede decir nada una sola vez. La proyección de contexto
(`context-projection`) es sólida y es **solo de repositorio**: compila
constitución, rumbo y change activa de `.meltemi/` a los dialectos de cada
agente (CLAUDE.md, AGENTS.md, GEMINI.md — mapa data-driven en
`core/meltemid/data/context-targets.toml`) con bloques gestionados por SHA
que preservan byte a byte lo ajeno. El directorio global del usuario
(`config_dir`) guarda config, permisos y suscripciones — y **cero contexto
proyectable**; por agente solo existe el overlay de `env` de los perfiles.
Quien trabaja en diez repos con las mismas convenciones las repite diez
veces, o deja de decirlas.

La base no se inventa: el mantenedor ya construyó la fábrica. FDH (CLI Go
`fdh` v0.4.1 con 66 archivos de test, hub de contenido, portal en
producción) modela los cuatro pilares como Markdown con front-matter —
`RULE.md` con `scope:` por glob y severidad, `SKILL.md` (el formato de
skills de facto del ecosistema), `AGENT.md`, `hook.json` — y los proyecta
con un mapa data-driven por agente y por ámbito user/project
(`pkg/adapters/builtin.yaml`), con bloques gestionados marcados. Es el mismo
patrón que Meltemi ya usa en `context-targets.toml` y en su bloque
`meltemi:context`: los dos proyectos convergieron por su cuenta. La decisión
aquí: **Meltemi adopta los formatos de FDH tal cual** como formato local del
harness — se reutiliza el trabajo hecho y `fdh` sigue siendo la fábrica de
autoría y distribución. La prioridad es Meltemi (directiva del mantenedor);
alinear FDH para que entregue fuentes al harness en vez de instalar directo
cuando Meltemi está presente es trabajo del lado de FDH, posterior y fuera
de este repositorio.

Dos deudas con lo ya decidido, saldadas y no escondidas. Una: «harness» era
la palabra del manifiesto TOML del motor propio (`motor-propio-byok`, sin
implementar); por decisión del mantenedor esa propuesta dice ahora
**manifiesto del motor** y la palabra harness nombra esto — el conjunto
rules/skills/agents/hooks que equipa a la flota (nota terminológica en esa
propuesta). Dos: la decisión registrada sobre Forge («separado, conectado
por contrato; el daemon jamás descarga») **se conserva y se extiende**: el
contrato ahora incluye los formatos de los pilares, e instalar sigue siendo
copiar archivos que el daemon valida y lista.

Y la mitad del valor no es proyectar: es **ver**. El pedido explícito:
«desde la UI quiero ver el harness que está configurado globalmente y para
cada agente». La vista efectiva — por agente y por proyecto, qué le aplica y
de qué capa viene cada pieza, al estilo `git config --show-origin` — es lo
que convierte cuatro directorios de archivos en una configuración que una
compañía o un developer puede razonar y ajustar a su realidad (frontend,
backend, UI). Ver primero; editar desde la superficie, después y con
evidencia.

## What Changes

- **Cuatro ámbitos de harness**, con la precedencia que el producto ya usa
  (lo específico pisa lo general; proyecto pisa usuario):
  `<config>/meltemi/harness/` (global del usuario) →
  `<config>/meltemi/harness/per-agent/<id>/` (ajuste global por agente) →
  `.meltemi/harness/` (del proyecto, commiteable) →
  `.meltemi/harness/per-agent/<id>/` (override del proyecto por agente).
  Los `<id>` son los del catálogo de flota. Los directorios de contenido
  usan los nombres de FDH (`rules/`, y a futuro `skills/`, `agents/`,
  `hooks/`); el eje por agente se llama `per-agent/` precisamente para no
  chocar con el pilar `agents/` (subagentes) cuando llegue.
- **Pilar Rules en fase 1, formato FDH tal cual**: `rules/<name>/RULE.md`
  con front-matter (`name`, `scope` glob, `severity`, `description`) y
  cuerpo Markdown. El daemon **valida forma y lista con fuente; jamás
  interpreta la semántica** — la misma regla que rige los manifiestos del
  motor. Resolución por nombre: la capa más específica gana entera, sin
  merge parcial de una misma regla; la vista dice qué quedó pisado y por
  quién.
- **La proyección aprende dos capas**: las reglas de ámbito proyecto entran
  al bloque gestionado existente de los archivos del repo (los mismos
  destinos de `context-targets.toml`, con el `scope` declarado en prosa
  junto a cada regla — los destinos nativos de reglas con ámbito quedan
  para después); las reglas globales se proyectan a los **archivos de
  usuario de cada agente** (`~/.claude/CLAUDE.md` y equivalentes — el mapa
  de destinos gana rutas de ámbito usuario, data-driven como ya es).
  **Guardián escrito: el contenido global del usuario jamás entra a
  archivos del repositorio** — lo personal no viaja en un commit del
  equipo, y un `git status` limpio lo prueba.
- **Un solo escritor y un solo gesto**: `meltemi project` proyecta el
  harness junto al contexto, con bloques gestionados por SHA y preservación
  byte a byte como hoy. La primera escritura sobre un archivo de usuario de
  un agente pide consentimiento explícito y queda registrada — es config
  ajena, el cuidado de mcp-passthrough aplica (§2).
- **Vista efectiva ×3 (§4)**: RPC nuevo `harness/effective` (proyecto
  opcional, agente opcional) que devuelve las piezas resueltas con su
  origen por capa; verbo CLI `meltemi harness [--agent <id>]` con `--json`;
  drill-in desde la Flota en la GUI y en la TUI (colocación exacta y
  densidad con el design system, en el design). La vista muestra también lo
  que **no** aplica y por qué — la visibilidad es el producto.
- **Front-matter**: el design decide si el subconjunto YAML que FDH usa
  (escalares y listas) se cubre extendiendo el parser de front-matter
  propio existente o justifica una dependencia nueva (§10) — decisión
  escrita, no accidente de implementación.

## Capabilities

### New Capabilities

- `agent-harness`: los cuatro ámbitos y su precedencia, el pilar rules en
  formato FDH, la resolución con origen, el guardián global/repo y la vista
  efectiva en las tres superficies.

### Modified Capabilities

- `context-projection`: + capas global y por-agente en la proyección,
  destinos de ámbito usuario en el mapa data-driven, reglas dentro del
  bloque gestionado con su scope en prosa.
- `cli-contract`: + verbo `harness` (efectivo por agente/proyecto,
  `--json`, códigos de salida y stdout/stderr según la disciplina vigente).
- `gui-shell`: + drill-in de harness efectivo desde la Flota.
- `tui-shell`: + ídem, con la paridad de navegación del shell.

## Impact

- Archivos: `core/meltemid` (descubrimiento, validación, resolución,
  proyección extendida, RPC `harness/effective`),
  `core/meltemid/data/context-targets.toml` (rutas de ámbito usuario por
  agente), `proto/` (tipos del RPC nuevo), `tui/`, `desktop/ui`,
  `docs/paridad-nucleo.md`, `docs/harness.md` (nueva),
  `rumbo/structure.md` (+ directorios). `motor-propio-byok` ya renombrada
  en el mismo gesto (manifiesto del motor).
- Dependencias: ninguna decidida aquí; si el front-matter exige YAML real,
  su crate se justifica en el design (§10) con la alternativa del parser
  propio evaluada primero.
- Riesgos nombrados: (1) **dos escritores** — FDH standalone instala hoy en
  rutas de agente con su propio bloque `_fdh_managed`; en v1 los bloques de
  cada herramienta usan marcadores distintos y no se tocan entre sí, y la
  alineación real (FDH entrega fuentes cuando Meltemi está presente) es el
  trabajo posterior del lado FDH. (2) **Config ajena** — escribir archivos
  de usuario de agentes es tocar lo que el agente gestiona; consentimiento
  explícito, bloque gestionado, y jamás sobrescribir contenido del usuario:
  la garantía existente se extiende, no se relaja.
- Lo que solo el uso confirmará: si «la capa específica gana entera» basta
  como resolución o hace falta composición más fina; se decide después con
  evidencia, no por adelantado.

## Fuera de alcance

- **Skills, Hooks y Subagentes** — pilares 2 a 4, changes futuras
  nombradas: `harness-skills` (prueba §6 favorable: SKILL.md es el formato
  de facto y FDH ya lo usa), `harness-hooks` (exige la prueba §6 del
  no-estándar y el cuidado §2 de escribir config **ejecutable** ajena) y
  `harness-subagentes` (matriz honesta: hoy solo Claude los soporta). Su
  relación con `plugins-skills-sdk` y `hooks-eventos` —que hablan de
  plugins y hooks **de Meltemi**, no proyectados a agentes— se resuelve al
  proponerlas, no aquí.
- **Bundles con nombre** (`harnesses.yaml` de FDH: `backend-team`,
  `frontend-team`): en v1 el harness ES lo que los ámbitos componen; el
  bundle nombrado llega con la integración FDH↔Meltemi.
- **Destinos nativos de reglas con ámbito** (`.cursor/rules`, `applyTo` de
  Copilot): enriquecen el mapa después; v1 proyecta a los destinos
  existentes con el scope en prosa.
- **Editar el harness desde la superficie**: la vista es de lectura; la
  autoría vive en archivos y en FDH. Editar llega con evidencia de uso.
- **Registro, descarga o marketplace**: el daemon jamás descarga; la
  decisión registrada sobre Forge se conserva.
- **El trabajo del lado FDH** (entregar fuentes al harness, ganar el eje
  por-agente en su modelo): repositorio aparte, después, alineado a lo que
  esta change fije.
