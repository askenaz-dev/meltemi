# Diseño: Motor de specs — parser y validador de `.meltemi/`

## Context

`artifact-format` fija las reglas del formato; falta el código que las aplica. Este motor es el primer componente que lee `.meltemi/` y lo valida. Debe ser una librería (paridad de núcleo: el daemon y el tooling la consumen), determinista y sin red. El formato es deliberadamente line-oriented, lo que permite un parser simple y robusto sin un motor Markdown completo.

## Goals / Non-Goals

**Goals:**
- Descubrir la estructura `.meltemi/` y parsear specs y rumbo a un modelo en memoria.
- Validar el subconjunto **estructural** de `artifact-format` y reportar diagnósticos con ubicación (archivo, línea, regla).
- Dogfooding: validar los propios artefactos del proyecto en un test.
- Dependencias nuevas: cero si es razonable.

**Non-Goals:**
- Validación de palabras clave EARS y detección de contradicciones/huecos (`motor-ears-deltas`).
- Parseo semántico y **aplicación/fusión** de deltas (`motor-ears-deltas`).
- Comandos del ciclo (`/review`, `/plan`, `/verify`, `/archive`) y su integración en el daemon.
- Edición o escritura de artefactos: este motor solo lee y valida.

## Decisions

### M1 — Crate `core/meltemi-spec` (librería)
Nuevo miembro del workspace. Expone el modelo, el parser, el descubrimiento de árbol y el validador. Sin binario. El daemon lo añadirá como dependencia cuando el ciclo de comandos lo requiera (change posterior).

### M2 — Modelo en memoria
- `Spec { capability: String, requirements: Vec<Requirement>, source: PathBuf }`
- `Requirement { name: String, description: String, scenarios: Vec<Scenario>, line: usize }`
- `Scenario { name: String, steps: Vec<Step>, line: usize }`
- `Step { marker: StepMarker, text: String }` con `StepMarker { When, Then, And, Other }`
- `RumboFile { path: PathBuf, inclusion: Inclusion, ratified: Option<Ratification>, body: String }`
- `Inclusion { Always, OnMatch(Vec<String>), Manual }`
- `MeltemiTree { root, constitution: Option<PathBuf>, rumbo: Vec<RumboFile>, specs: Vec<Spec>, changes: Vec<ChangeDir>, archive: Vec<ChangeDir> }`
Los deltas (`## ADDED/...`) se reconocen y clasifican, pero su semántica (aplicación) queda para `motor-ears-deltas`; aquí solo se valida que las cabeceras sean canónicas.

### M3 — Parser line-oriented
Escaneo por líneas, sin motor Markdown:
- `## <OP> Requirements` → cabecera de delta (OP ∈ {ADDED, MODIFIED, REMOVED, RENAMED}); cualquier otra `## … Requirements` es cabecera no reconocida.
- `### Requirement: <nombre>` abre un requisito; la prosa hasta el primer `####` es su descripción.
- `#### Scenario: <nombre>` (exactamente 4 `#`) abre un escenario; un `###`/`#####` no cuenta como escenario.
- `- **WHEN|WHILE|IF|THEN|WHERE|AND** …` → paso; el marcador se clasifica, el texto se conserva.
Cada elemento guarda su número de línea para los diagnósticos.

### M4 — Front-matter mínimo, sin dependencia nueva
El front-matter de rumbo es un bloque YAML acotado (`---` … `---`) con formas simples: `clave: valor` y `clave: [a, b, c]`. Un parser propio de ~40 líneas cubre `inclusion`, `fileMatch`, `ratificado`, `ratificador` sin añadir un crate YAML (constitución §10, dependencias mínimas). *Alternativa evaluada*: `serde_yml` — descartada por ahora; se reconsiderará si el front-matter crece más allá de estas formas (se anota en Open Questions).

### M5 — Validación estructural (subconjunto de `artifact-format`)
Reglas implementadas aquí (las semánticas EARS y de deltas se difieren):
- Todo `Requirement` tiene ≥1 `Scenario` (si no, diagnóstico).
- Un `#### Scenario:` mal nivelado (3 o 5 `#`) no se reconoce → el requisito puede quedar sin escenarios.
- Nombre de capacidad en kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`).
- Front-matter de rumbo presente; si `inclusion: por-patrón`, exige `fileMatch` no vacío.
- Cabecera de delta canónica; una no reconocida (p. ej. `## AÑADIDO Requirements`) → diagnóstico.

### M6 — Diagnósticos
`Diagnostic { file: PathBuf, line: usize, rule: Rule, message: String }`, con `Rule` como enum estable (p. ej. `RequirementWithoutScenario`, `UnknownDeltaHeader`, `NonKebabCapability`, `MissingFrontMatter`, `OnMatchWithoutFileMatch`). El validador devuelve `Vec<Diagnostic>` (vacío = conforme). Los mensajes en inglés (son strings del motor; §11 de la constitución).

### M7 — Dogfooding
Un test integra el motor contra los artefactos reales del repo: descubre `.meltemi/` (constitución + rumbo), parsea las specs vivas del formato (las de `openspec/specs/`, que ya son conformes al canon), y asevera **cero diagnósticos**. Esto ancla el motor a la realidad y detecta derivas de formato en el propio proyecto.

## Risks / Trade-offs

- **[Parser propio vs Markdown real]** → El formato es line-oriented por diseño; un escáner de líneas es más simple y predecible que un motor Markdown, y evita dependencias. Riesgo: casos borde de Markdown (código en bloques, tablas) — se ignoran salvo las líneas ancla reconocidas.
- **[Front-matter propio]** → Cubre solo las formas actuales; si crece, se migra a un parser YAML (Open Questions).
- **[Solapamiento con `artifact-format`]** → Este motor implementa el subconjunto estructural; los escenarios de `artifact-format` que dependen de EARS/deltas se satisfarán en `motor-ears-deltas`. Se documenta qué reglas cubre cada change.

## Migration Plan

Aditivo: nuevo crate, sin cambios en código existente. Rollback = quitar el crate del workspace.

## Open Questions

- ¿El motor valida también los deltas dentro de `changes/<name>/specs/` (cabeceras y estructura), o solo las specs vivas? Propuesta: reconocer y validar la **estructura** de los deltas (cabeceras canónicas, requisito/escenario), dejando la **aplicación** para `motor-ears-deltas`.
- ¿Front-matter permitido en `constitution.md` y `specs/*/spec.md`? Propuesta: opcional y solo metadatos de ratificación; ignorado por el parser de requisitos.
