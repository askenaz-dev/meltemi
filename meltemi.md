# Meltemi — Documento Fundacional

> **"Un rumbo, muchas velas."**
> Versión 1.5 — enmendada el 31 de julio de 2026 (`lanzador-conversacional`: toda sesión corre gobernada y especificar antes de escribir es el camino más corto, no un peaje previo); ratificación de la v1.5 pendiente del mantenedor fundador, y **gate de archivo** de esa change. Enmiendas previas: v1.4 (`enmienda-agent-boss`: el compañero móvil es el puesto remoto del Agent Boss, ratificación pendiente), v1.3 (`enmienda-edicion-movil`, ratificación pendiente), v1.2 (`formato-artefactos-meltemi`, ratificación pendiente), v1.1 (`enmiendas-fundacionales-v1`). Base v1.0 ratificada el 11 de julio de 2026 por Guillmar Ortiz (`fase-0-fundacion` 1.2).
> Este documento practica lo que predica: define **qué** se construirá y **por qué**, antes de escribir una sola línea de código.

---

## 0. Identidad

### El nombre

El **meltemi** es el viento estival del Egeo. Sopla cada verano con fuerza y regularidad, siempre desde el mismo cuadrante. Los navegantes del Mediterráneo no lo combaten ni lo temen: conocen su dirección, ajustan las velas y llegan más lejos y más rápido de lo que podrían jamás a remo.

Esa es exactamente la relación que queremos entre las personas y los agentes de inteligencia artificial que hoy escriben software: una fuerza enorme y constante que, **con rumbo**, multiplica al que navega — y sin él, solo produce deriva.

### La marca

Una *m* minúscula trazada con confianza que se lee, a la vez, como un pequeño velero — **el viento que escribe**: dos arcos que son **velas asimétricas** sobre una curva mínima que es el **casco**, y un extremo derecho que es **proa y ráfaga** a la vez.

- **Primera lectura** — una *m* en movimiento: una especificación es un solo rumbo compartido.
- **Segunda lectura** — un velero impulsado por el viento: los agentes son velocidad pura que necesita dirección.

El detalle de construcción y uso de la marca vive en `brand/README.md`.

Paleta: *Aegean Night* `#0A1B33` · *Meltemi Blue* `#2563EB` · *Cyan Foam* `#22D3EE` · *Sea Salt* `#F2F7FB`. Tipografía de marca: sans-serif geométrica, minúsculas: **meltemi**.

Nomenclatura técnica: el daemon es **`meltemid`**; la interfaz de terminal es **`meltemi`** (alias corto: `mel`), un único binario que funciona como CLI scriptable y como TUI interactiva; la app de escritorio es **Meltemi Desktop**; y los artefactos del método viven en **`.meltemi/`**.

### El lema

**"Un rumbo, muchas velas."** Una especificación clara —el rumbo— puede impulsar cualquier número de agentes —las velas—, de cualquier fabricante, sin atarse a ninguno.

### La promesa

Meltemi es y será siempre **gratuito, abierto (Apache 2.0) y de la comunidad**. No hay créditos, no hay tarifas ocultas, no hay lock-in: cada usuario trae sus propios agentes, sus propias claves y sus propios modelos. Se construye para cualquiera, en cualquier lugar: para quien programa desde un servidor por SSH en cualquier rincón del mundo y para quien prefiere una interfaz de escritorio pulida — el mismo poder en ambas superficies.

---

## En síntesis

- **Meltemi es el plano de control spec-driven para el desarrollo agéntico**: un entorno 100% open source (Apache 2.0) donde toda sesión de agente corre gobernada —permisos, registro auditable y punto de restauración— y donde especificar antes de escribir es el camino más corto y no un peaje previo; esa disciplina gobierna a **los agentes de codificación que el usuario ya tiene y ya paga** — los de los grandes laboratorios y los open source por igual.
- **No construimos otro agente más; dirigimos a todos.** El MVP orquesta agentes externos a través de **protocolos abiertos** (ACP para pilotar agentes, MCP para herramientas y datos) y de **proyección de contexto** hacia los formatos de instrucciones que los agentes ya leen. Un motor agéntico propio (BYOK) llega en una fase posterior como una vela más — nunca como requisito.
- **Arquitectura**: un núcleo headless en **Rust** (`meltemid`) que concentra specs, orquestación, protocolos y seguridad; sobre él, dos clientes finos con paridad de núcleo: una **TUI** y una **GUI de escritorio en Tauri**. Arranque instantáneo, huella mínima.
- **Juego limpio**: Meltemi ejecuta siempre el **binario oficial** de cada agente, con la autenticación que ese agente gestiona. Nunca suplanta tráfico, nunca toca credenciales ajenas, y muestra con transparencia el nivel de integración y de seguridad de cada agente.

---

## 1. Visión y Misión

### El problema

El desarrollo asistido por IA vive una tensión sin resolver. Los agentes generan código a una velocidad sin precedentes, pero sin una fase explícita de requisitos y diseño producen software frágil, indocumentado y desalineado con la intención real — el patrón que la industria bautizó como *vibe coding*. La respuesta emergente, el **Spec-Driven Development (SDD)**, reconoce que el sector está pasando de "el código es la fuente de verdad" a "**la intención es la fuente de verdad**".

Pero el panorama actual del SDD y del desarrollo agéntico está fragmentado en tres familias, cada una incompleta:

1. **Plataformas integradas pero cerradas**: imponen disciplina de specs, pero atada a un único proveedor de nube y de modelo, con precios por consumo difíciles de predecir.
2. **Metodologías portátiles pero sin motor**: excelentes marcos de trabajo de specs que dependen por completo de un agente externo, sin entorno propio, sin experiencia de revisión y con fricción alta.
3. **Orquestadores potentes pero sin disciplina**: entornos que corren múltiples agentes en paralelo con gran experiencia de uso, pero que no imponen ni generan una fase de requisitos: multiplican la velocidad sin aportar rumbo.

Nadie une las tres cosas. Ese es el hueco de Meltemi.

### La misión

**Democratizar el desarrollo agéntico con disciplina de ingeniería, para toda la humanidad**, mediante cinco compromisos:

1. **Apertura radical**: código 100% abierto, sin lock-in de proveedor de IA, de nube ni de agente.
2. **Disciplina spec-first como valor por defecto**, con una vía rápida y proporcional para lo trivial.
3. **Paridad de núcleo terminal ↔ escritorio**: todo el poder del núcleo disponible por igual desde un servidor headless por SSH o desde una GUI pulida.
4. **Neutralidad de agente y de modelo**: los agentes y modelos del mercado — comerciales, open source o locales — como velas intercambiables bajo un mismo rumbo.
5. **Interoperabilidad por estándares abiertos**: protocolos públicos y formatos de artefactos legibles, nunca integraciones privadas frágiles.

### No-objetivos

Tan importante como lo que Meltemi es, es lo que deliberadamente **no** es:

1. **No es un editor de código de propósito general ni un IDE clásico**: la autoría sostenida de código ocurre en el editor que cada usuario ya usa, siempre a un salto de distancia ("Abrir con…" con archivo:línea exacto). La superficie de código de Meltemi es de *revisión y edición utilitaria* al servicio del bucle agéntico (revisar → retocar → dirigir): Meltemi optimiza para que salir sea **infrecuente, no imposible**. La cerca normativa de lo que la edición incluye y excluye vive en la spec `edit-surface`.
2. **No es otro agente de codificación**: hasta la fase 2 no existe motor propio, y cuando exista será opcional — jamás un requisito ni un canal privilegiado.
3. **No es un servicio en la nube ni un backend gestionado**: todo corre en las máquinas del usuario.
4. **No es una plataforma de CI/CD ni de despliegue.**
5. **No es un marketplace de extensiones.**

Todo lo que no esté en el roadmap (§10) está fuera de alcance salvo propuesta de cambio aprobada.

### Para quién

Usuarios objetivo del MVP, en orden:

1. **El desarrollador individual** que ya usa y paga uno o más agentes CLI y trabaja en terminal, local o por SSH.
2. **El tech lead** que quiere imponer disciplina de specs en un equipo pequeño sin obligar a nadie a cambiar de agente.
3. **Mantenedores open source** sobre bases de código maduras, donde el contexto que sobrevive a la sesión vale oro.

Las funciones de equipo y organización llegan en fase 3.

---

## 2. Fundamentos adoptados

Este documento no compara productos: destila los **patrones que la industria ya validó** entre 2023 y 2026 y los declara fundamentos de Meltemi. Cada uno existe porque alguien demostró su valor en producción; Meltemi los adopta como fundamentos de un solo sistema abierto.

### 2.1 La especificación como artefacto persistente

El chat se evapora; los artefactos permanecen. Cada cambio se define en un conjunto de **artefactos versionados junto al código** — propuesta, requisitos, diseño, deltas y tareas — cuyo corazón son tres documentos: **requisitos, diseño y tareas**. Los requisitos se escriben en **notación EARS** (*Easy Approach to Requirements Syntax*, presentada en la conferencia IEEE RE'09), que elimina la ambigüedad con un conjunto pequeño de patrones. Las **palabras clave estructurales y de EARS van en inglés** (el canon internacional y lo que el motor de specs reconoce); la **prosa descriptiva, en español neutro**:

- **Ubicuo**: "The system SHALL …"
- **Dirigido por estado**: "**WHILE** ⟨estado⟩, the system SHALL …"
- **Dirigido por evento**: "**WHEN** ⟨disparador⟩, the system SHALL …"
- **Comportamiento no deseado**: "**IF** ⟨condición⟩, **THEN** the system SHALL …"
- **Opcional**: "**WHERE** ⟨capacidad⟩, the system SHALL …"

Los artefactos estructurados sobreviven a la sesión y dan a cualquier agente futuro un contexto muy superior al historial de commits.

### 2.2 Constitución y rumbo persistente

Un archivo de **constitución** establece los principios no negociables del proyecto (políticas de testing, restricciones de stack, estilo) y se inyecta en cada fase del ciclo. Lo acompañan los **archivos de rumbo**: el porqué del producto, el stack técnico y las convenciones de estructura, con modos de inclusión configurables (siempre / por patrón de archivos / manual).

### 2.3 Verdad viva y cambios como deltas

Separación estricta entre **la verdad viva** (cómo funciona el sistema hoy) y **las propuestas de cambio**. Cada cambio describe solo sus **deltas** — requisitos `ADDED`, `MODIFIED`, `REMOVED` (`RENAMED` para renombres) — en lugar de reescribir specs completas. Al archivar un cambio aprobado e implementado, sus deltas se funden en la verdad viva y el andamiaje (propuesta, diseño, tareas) pasa al histórico. Este enfoque hace al método tan útil en bases de código maduras (*brownfield*) como en proyectos nuevos.

### 2.4 Orquestación paralela con worktrees

Cada tarea o experimento corre en su propio **git worktree aislado**, con su propia sesión de agente. Esto permite lanzar varios agentes — o el mismo agente con estrategias distintas — sobre el mismo problema en paralelo, comparar resultados línea a línea y quedarse con el mejor, sin que ninguno pise el trabajo de otro. El aislamiento por worktree es, además, la primera frontera de contención de daños.

### 2.5 Modos de ejecución graduados y reversibilidad

Dos ejes de control validados por el mercado: **planificar/actuar** (el agente propone un plan revisable antes de tocar nada) y **supervisado/autónomo** (aprobación cambio por cambio, o autonomía dentro de guardarraíles explícitos: comandos de confianza permitidos, todo lo demás denegado, acción acotada al workspace). Complementados por **checkpoints automáticos antes de cada tarea** y reversión granular.

### 2.6 Disciplina git de primer orden

**Un commit atómico por tarea**, trazable hasta el requisito que lo originó. La revisión de diffs — con comentarios que vuelven al agente como instrucciones — es ciudadana de primer orden en ambas superficies.

### 2.7 Protocolos abiertos, no integraciones privadas

- **ACP (Agent Client Protocol)**: el estándar abierto (Apache 2.0, JSON-RPC 2.0 sobre stdio) que desacopla el entorno del agente, igual que LSP desacopló la inteligencia de lenguaje del editor. Hoy lo hablan — de forma nativa o mediante adaptadores abiertos — un número creciente de los agentes CLI relevantes del mercado; está gobernado de manera conjunta y pública por más de un proveedor, y publica un **registro público de agentes** legible por máquina. Crucialmente para un orquestador, ACP canaliza las **peticiones de permiso del agente hacia el cliente**: un punto único de gobernanza sobre flotas heterogéneas.
- **MCP (Model Context Protocol)**: el estándar abierto — gobernado por una fundación neutral — que conecta agentes con herramientas y datos (tools, resources, prompts) sobre JSON-RPC 2.0 (stdio o streamable HTTP). ACP y MCP son complementarios: MCP conecta un agente con sus herramientas; ACP conecta un entorno con sus agentes.
- **LSP** para inteligencia de código en la superficie de revisión (definiciones, referencias, diagnósticos sobre los diffs de los agentes), integrado directamente, sin depender de catálogos de terceros.

### 2.8 Proyección de contexto: los formatos que todos ya leen

Los agentes del mercado convergieron en archivos de instrucciones en Markdown en la raíz del repositorio — con `AGENTS.md` como estándar de facto y variantes propias de cada agente. Meltemi **compila** su constitución, su rumbo y la spec activa hacia todos esos formatos automáticamente. Resultado: cualquier agente que lea archivos de instrucciones del repositorio — hoy, la práctica totalidad — queda orientado por las specs de Meltemi **sin necesidad de integración alguna**.

### 2.9 Distribución sin fricción y BYO-todo

**Binario único autocontenido** para el núcleo y la TUI, instalable con un comando, sin runtimes pesados ni entornos virtuales; el cliente de escritorio se apoya en el webview del sistema con un instalador mínimo. **BYO-agent** (tus agentes, tus suscripciones), **BYOK** (tus claves de API) y **BYO-modelo** (incluidos modelos locales para privacidad total). Meltemi no cobra, no mide créditos y no penaliza la disciplina.

### 2.10 Anti-patrones que evitamos por diseño

El mercado ya demostró qué no funciona; lo declaramos aquí como principios de diseño:

- **La sobre-ingeniería mata la adopción**: aplicar el ceremonial completo de specs a un bugfix trivial es usar un mazo para cascar una nuez. La disciplina debe ser **proporcional al tamaño del cambio**.
- **La disciplina no puede tener peaje**: cualquier fricción económica al flujo spec-driven empuja a los usuarios de vuelta al vibe coding. Por eso Meltemi no cobra ni mide créditos.
- **La autonomía sin controles es un incidente esperando fecha**: sin revisión humana ni reversibilidad, el daño es cuestión de tiempo. La aprobación explícita y la vuelta atrás no son opcionales.
- **Los marketplaces cerrados son una trampa**: depender de un catálogo de extensiones de terceros somete al producto a términos, tarifas y riesgos de seguridad que no controlamos. La extensibilidad de Meltemi se apoya en estándares abiertos (MCP, ACP) y registros públicos, no en un marketplace propietario.
- **La fiabilidad también es producto**: tareas que se atascan y pierden contexto al reintentar queman la confianza del usuario. El estado de cada sesión de agente debe ser persistente, inspeccionable y recuperable.

---

## 3. Propuesta de valor

Las cinco promesas de la misión, hechas producto:

1. **El mejor SDD del mercado, unificado**: corazón de tres documentos legibles + notación EARS + constitución y rumbo persistentes + deltas sobre verdad viva. Nadie más une estas piezas.
2. **Orquestación multi-agente gobernada por specs**: worktrees paralelos y revisión de diffs de primera clase, donde cada agente — sea cual sea su proveedor — ejecuta tareas que nacen de una spec aprobada.
3. **Neutralidad real**: de agente (vía ACP y proyección de contexto), de modelo (BYOK y modelos locales) y de nube (ninguna dependencia).
4. **Confianza como arquitectura**: binarios oficiales, credenciales intactas, permisos visibles, reversibilidad acotada y honesta.

---

## 4. Principios de Diseño

1. **Spec-first, proporcional**: el flujo por defecto exige una spec aprobada antes de implementar; una vía rápida (`fast-forward`) genera todos los artefactos de una vez para cambios pequeños. Disciplina sí; ceremonial vacío, no.
2. **Paridad de núcleo**: toda capacidad del daemon es accesible por igual desde la TUI y la GUI. Las superficies pueden diferir en *experiencia* (el editor visual de specs brillará más en escritorio), nunca en *poder*.
3. **Abierto de verdad**: sin lock-in, sin telemetría oculta, sin marketplace propietario. Estándares y registros públicos.
4. **Agnóstico de agente y de modelo**: la lógica del núcleo no asume ningún proveedor. El agente es una vela: se iza, se cambia, se combina.
5. **Extensible**: el núcleo es una librería/SDK con API pública; MCP y ACP son los mecanismos primarios de extensión; plugins y skills declarativas por encima.
6. **Seguro por defecto**: aislamiento por worktree, aprobación explícita de acciones peligrosas, menor privilegio, y transparencia del nivel de protección real en cada plataforma y agente.
7. **Juego limpio**: Meltemi ejecuta el binario oficial de cada agente con la autenticación que el propio agente gestiona. Nunca lee, almacena ni reutiliza credenciales ajenas; nunca suplanta el tráfico de red de otro producto; y muestra el estatus de cada integración — plena (niveles 1-2), parcial (nivel 3), solo artefactos (nivel 4) — sin condiciones ocultas. Los términos de los proveedores difieren y cambian: la arquitectura de Meltemi hace imposible violarlos por accidente.
8. **Trazabilidad extremo a extremo**: cada línea de código es rastreable hasta la tarea, el diseño y el requisito EARS que la originó.
9. **Legibilidad de artefactos**: la experiencia de *revisar* specs — diffs de deltas, detección de contradicciones, checklists — es tan importante como la de generarlas. Es el punto débil que todo el mercado comparte — y nuestra obsesión.
10. **Fiabilidad como contrato**: sesiones persistentes, estado inspeccionable, recuperación sin pérdida de contexto.

---

## 5. El Flujo Spec-Driven de Meltemi

### 5.1 Estructura de artefactos: el directorio `.meltemi/`

```
.meltemi/
├── constitution.md            # Principios no negociables del proyecto
├── rumbo/                     # Contexto persistente del proyecto
│   ├── product.md             #   el "porqué" del producto
│   ├── tech.md                #   stack y restricciones técnicas
│   ├── structure.md           #   organización y convenciones
│   └── *.md                   #   front-matter: siempre | por-patrón | manual
├── specs/                     # VERDAD VIVA: cómo funciona el sistema hoy
│   └── <capability>/
│       └── spec.md            #   requisitos vigentes en EARS
├── changes/                   # PROPUESTAS de cambio
│   ├── <change-name>/
│   │   ├── proposal.md        #   por qué y qué cambia
│   │   ├── requirements.md    #   historias + criterios de aceptación (EARS)
│   │   ├── design.md          #   arquitectura, modelos de datos, interfaces
│   │   ├── specs/             #   DELTAS: ## ADDED / ## MODIFIED / ## REMOVED Requirements
│   │   └── tasks.md           #   tareas secuenciadas por dependencias
│   └── archive/               #   cambios completados (histórico + ADRs opcionales)
└── hooks/                     # automatizaciones por evento (fase 2)
```

### 5.2 El ciclo de vida (comandos unificados)

```
/constitution   → establece o edita los principios del proyecto
/explore        → socio de pensamiento sin compromiso: lee el código,
                  sopesa opciones, propone rumbo antes de escribir nada
/propose <idea> → crea changes/<name>/ con proposal, requirements (EARS),
                  design, deltas y tasks. Con revisión humana en cada paso.
/review         → revisión de specs de primer nivel: diff de deltas,
                  detección de contradicciones y huecos, checklist de calidad
/plan           → refina design.md y secuencia tasks.md
/implement      → despliega los agentes sobre las tareas, en modo
                  planificar/actuar, con checkpoints
/verify         → valida la implementación contra la spec fuente de verdad
/archive        → funde los deltas aprobados en specs/ y preserva el histórico
```

**Modo dual**: `spec-full` (disciplina completa, cambios grandes) y `fast-forward` (todos los artefactos de una vez, cambios pequeños). La constitución y el rumbo se inyectan como contexto en cada fase — y se **proyectan** automáticamente a los formatos de instrucciones de cada agente conectado.

`/verify` en el MVP combina una checklist guiada por requisito EARS con la ejecución de los tests vinculados a cada criterio de aceptación; la verificación automática y continua spec↔código llega en fase 3.

---

## 6. Funcionalidades Clave

1. **Editor de specs** con vista Markdown enriquecida, diff de deltas ADDED/MODIFIED/REMOVED, validación EARS en vivo y detección de contradicciones y huecos.
2. **Flota de agentes**: catálogo de agentes detectados en la máquina (vía el registro público de ACP y detección local), con estado, nivel de integración y controles por agente.
3. **Orquestación paralela**: N agentes sobre M tareas en worktrees aislados; carreras de agentes sobre la misma tarea. La mezcla de resultados es un **merge asistido por humano**: los diffs en competencia se presentan lado a lado, el usuario elige una base y aplica parches selectivos; los conflictos se minimizan secuenciando en `tasks.md` las tareas que comparten archivos.
4. **Modos planificar/actuar y supervisado/autónomo**, con guardarraíles configurables por proyecto y por agente.
5. **Proxy de permisos unificado**: las peticiones de permiso de todos los agentes ACP fluyen a una sola bandeja de permisos, con reglas persistentes (permitir/preguntar/denegar) por herramienta, comando y ruta.
6. **Hooks** por evento (guardar/crear/borrar/manual) para tests, docs, escaneo de secretos y commits convencionales *(fase 2)*.
7. **Checkpoints y rollback** granulares antes de cada tarea.
8. **Integración Git**: commit atómico por tarea, revisión línea a línea con comentarios que vuelven al agente como instrucciones.
9. **Gestión de contexto**: mapa del repositorio, referencias `@archivo`/`@carpeta`, compilación de contexto por agente (proyección).
10. **MCP**: passthrough de servidores MCP hacia los agentes que lo soporten *(fase 1)*; cliente MCP nativo en el motor propio *(fase 2)*.
11. **Sesiones persistentes e inspeccionables**: cada conversación de agente queda registrada, reanudable y auditable.
12. **Métricas SDD locales** *(fase 2)*: panel de métricas del proyecto calculadas íntegramente en local; compartición agregada solo mediante telemetría opt-in, desactivada por defecto.
13. **Edición utilitaria in situ** *(GUI en fase 2; TUI vía `$EDITOR` o mini-edición de hunks)*: retoques y ajustes en contexto con inteligencia LSP (autocompletado, diagnósticos, navegación), edición de hunks en el diff y "Abrir con…" hacia el editor del usuario con archivo:línea. Toda edición in situ pasa por el daemon y queda registrada como evento `human_edit` en el log de sesión.

---

## 7. Arquitectura Técnica

### 7.1 Decisión 1 — Aplicación independiente, no fork de editor

Meltemi se construye desde cero. Partir de un fork de un editor existente impone un impuesto permanente de mantenimiento para seguir el ritmo del upstream, hereda el peso de un motor de navegador completo y hace depender la extensibilidad de catálogos de terceros fuera de nuestro control. Nuestro objetivo (SDD + orquestación + paridad terminal/escritorio) no necesita nada de eso: la inteligencia de código llega por **LSP integrado directamente** sobre la superficie de revisión, y la extensibilidad por **MCP/ACP**.

### 7.2 Decisión 2 — Núcleo headless + clientes finos

**Patrón: daemon con toda la lógica + clientes finos vía JSON-RPC**, el modelo probado por décadas de herramientas robustas (daemon + CLI + GUI):

```
┌────────────────────────────────────────────────────────────┐
│  CLIENTES FINOS                                            │
│  ┌───────────────────┐        ┌─────────────────────────┐  │
│  │  TUI  (meltemi)   │        │  GUI escritorio (Tauri) │  │
│  └─────────┬─────────┘        └───────────┬─────────────┘  │
│            │      JSON-RPC 2.0            │                │
│            │      (stdio / socket local)  │                │
├────────────┴───────────────────────────────┴───────────────┤
│  meltemid — NÚCLEO HEADLESS (Rust)                         │
│  • Motor de specs (constitución, deltas, EARS, ciclo)      │
│  • Orquestador de flota (ACP, headless, proyección)        │
│  • Proxy de permisos unificado  • Git/worktrees            │
│  • MCP: passthrough (f1), cliente (f2)  • Checkpoints      │
│  • Sesiones persistentes                                   │
└────────────────────────────────────────────────────────────┘
```

El daemon arranca bajo demanda y puede ejecutarse en remoto: la TUI o la GUI se conectan por túnel SSH a un `meltemid` en un servidor, con paridad total. Beneficios: **paridad garantizada** (ambos clientes consumen la misma API), soporte headless trivial, y el mismo núcleo expuesto como SDK para integraciones.

### 7.3 Decisión 3 — Lenguaje del núcleo: **Rust**

**Recomendación firme: Rust para `meltemid` y para la TUI.** Razones:

- **Los estándares que adoptamos tienen SDK oficial en Rust**: tanto ACP como MCP publican crates oficiales mantenidos por sus gobernanzas. Un núcleo Rust consume los protocolos de primera mano, sin reimplementaciones propias que mantener.
- **Coherencia con la GUI**: Tauri es Rust — el backend del cliente de escritorio y el núcleo comparten lenguaje, tipos y tooling. Un solo lenguaje de sistemas en todo el producto.
- **El carácter del producto**: un daemon que orquesta procesos concurrentes, multiplexa sesiones JSON-RPC y media permisos de seguridad es exactamente el terreno donde Rust brilla: memoria segura sin GC, concurrencia sin data races, binario único pequeño y arranque instantáneo.
- **TUI**: el ecosistema de interfaces de terminal en Rust es maduro y de altísimo rendimiento, y comparte el 100% de los tipos con el núcleo, sin FFI.

*Alternativas evaluadas*: Go (excelente para daemons, pero sin SDK oficial de ACP: obligaría a mantener una implementación propia del protocolo central de la arquitectura) y TypeScript (el ecosistema de SDKs de agentes es rico, pero un runtime empaquetado multiplica el peso y debilita la historia de daemon de sistema). La decisión se revisará solo si la primera propuesta de cambio de implementación revela un bloqueo material.

**Windows es plataforma primaria de desarrollo**, no un puerto posterior (constitución §7): toda la abstracción de plataforma se diseña y prueba primero allí, donde el aislamiento de procesos y sockets es más restrictivo.

### 7.4 Decisión 4 — Escritorio: **Tauri**

El cliente de escritorio usa **Tauri** (webview nativo del sistema + backend Rust): instaladores de unos pocos megabytes, decenas —no cientos— de MB de RAM en reposo, arranque inferior al segundo y capacidades denegadas por defecto como modelo de seguridad. La alternativa clásica —empaquetar un motor de navegador completo con la aplicación— implica un coste en tamaño de instalador un orden de magnitud mayor y una memoria en reposo varias veces superior, y solo se justificaría si necesitáramos render idéntico al píxel en todas las plataformas, que no es el caso: la lógica pesada vive en `meltemid`, y la GUI es una superficie fina.

**Contrapartida asumida**: el webview nativo difiere entre sistemas — cada sistema operativo aporta su propio motor web —, lo que exige pruebas en las tres plataformas y disciplina de compatibilidad CSS. Mitigación: framework web con buen soporte cross-webview, polyfills y CI visual en los tres SO. Ventaja adicional: la misma base habilita un compañero móvil en fase futura.

### 7.5 Decisión 5 — La capa de agentes: dirigir, no reemplazar

La decisión que define a Meltemi. El núcleo **no compite con los agentes: los dirige**, en cuatro niveles de integración, siempre ejecutando el binario oficial de cada uno:

| Nivel | Mecanismo | Qué habilita |
|---|---|---|
| **1. ACP nativo** | El agente corre como subproceso hablando ACP (JSON-RPC/stdio) | Integración plena: streaming, diffs, sesiones, cancelación, y **permisos canalizados al proxy de Meltemi** |
| **2. Adaptador ACP** | Adaptadores abiertos del ecosistema ACP para agentes sin soporte nativo | La misma integración plena, con una pieza intermedia abierta mantenida por el ecosistema del protocolo |
| **3. Headless estructurado** | Modo no-interactivo del agente con salida JSON/JSONL | Ejecución programática de tareas, sin canal de permisos rico: se ejecuta dentro de guardarraíles (worktree + reglas del propio agente) |
| **4. Solo artefactos** | Proyección de contexto (§2.8): el agente lee las instrucciones y specs de Meltemi desde el repositorio | Gobernanza por specs sin integración de proceso: funciona con cualquier herramienta que lea instrucciones del repositorio |

Componentes de la capa:

- **Catálogo de flota**: consumo del registro público de agentes ACP + detección local de binarios instalados. Cada agente se muestra con su nivel de integración, su modelo de permisos y su estatus de compatibilidad — sin sorpresas.
- **Proyección de contexto**: compilación automática de constitución + rumbo + spec activa hacia `AGENTS.md` y las variantes de instrucciones que cada agente lee, mantenidas en sincronía por el daemon. Si el repositorio ya contiene archivos de instrucciones propios, Meltemi **nunca sobrescribe contenido del usuario**: la proyección escribe dentro de bloques delimitados y marcados como generados, dejando intacto el resto del archivo.
- **Proxy de permisos**: para agentes ACP, todas las peticiones de permiso confluyen en una sola bandeja de permisos con reglas persistentes. Para niveles 3-4, Meltemi configura los controles nativos del agente (allowlists, modos de aprobación) desde un solo lugar.
- **Sesiones**: persistentes, reanudables y auditables por agente y por tarea.

### 7.6 Decisión 6 — Motor propio: fase 2, como una vela más

En fase 2, Meltemi suma un **motor agéntico propio** con BYOK (claves del usuario) y abstracción de proveedores de modelos — incluidos modelos locales para privacidad total. Entra a la flota como un agente más, bajo las mismas reglas: mismas specs, mismo proxy de permisos, mismos checkpoints. Con él llega el **sandbox de ejecución propio** a nivel de SO (perfiles de aislamiento por plataforma), que también podrá envolver a agentes de nivel 3-4 donde el sistema operativo lo permita.

---

## 8. Seguridad y Confianza

La seguridad de Meltemi es **por capas**, y es honesta sobre lo que cada capa cubre:

1. **Superficie del propio daemon**: `meltemid` escucha por defecto únicamente en un socket local con permisos exclusivos del usuario; el acceso remoto se realiza exclusivamente a través de túnel SSH — nunca HTTP expuesto a red. Cualquier transporte de red futuro requerirá autenticación explícita y estará desactivado por defecto.
2. **Aislamiento por worktree** (fase 1): la frontera primaria de contención. Ningún agente toca el árbol principal; todo cambio llega por revisión y merge.
3. **Permisos heredados y visibles** (fase 1): cada agente del mercado trae su propio modelo de aprobaciones y — en algunos casos y plataformas — su propio sandbox de SO. Meltemi no los reemplaza en fase 1: los **configura de forma centralizada y los muestra sin maquillaje**, incluyendo las diferencias reales entre plataformas (los niveles de aislamiento disponibles en Windows, macOS y Linux no son equivalentes, y el usuario debe saberlo).
4. **Proxy de permisos ACP** (fase 1): una sola bandeja de permisos, reglas permitir/preguntar/denegar por comando, ruta y herramienta, y registro auditable de cada decisión.
5. **Checkpoints y reversión** (fase 1), con alcance honesto: los checkpoints revierten el estado del workspace (archivos dentro del worktree). **No revierten efectos externos** — tráfico de red, instalación de paquetes, migraciones, publicaciones. Por eso las acciones con efectos externos potencialmente irreversibles requieren aprobación explícita incluso en modo autónomo, y el proxy de permisos las clasifica como tales por defecto.
6. **Sandbox propio** (fase 2): perfiles de aislamiento a nivel de SO por plataforma, aplicables al motor propio y, donde sea posible, a agentes externos.
7. **Higiene MCP**: los riesgos documentados del ecosistema de herramientas (inyección de prompts, tools envenenadas) se mitigan con consentimiento explícito por servidor, revisión de descripciones de tools, control de acceso y auditoría.

Y una capa que no es técnica sino contractual: el principio de **juego limpio** (§4.7). La confianza de los usuarios y de los proveedores de agentes es un activo de seguridad: la arquitectura la protege haciendo imposible, por construcción, el uso indebido de credenciales o la suplantación de clientes.

---

## 9. Estrategia Open Source

### 9.1 Licencia: Apache 2.0

**Apache 2.0** para el núcleo, la TUI, la GUI y el SDK: permisiva (máxima adopción, incluida la institucional), con concesión y protección explícita de patentes — relevante para un proyecto que recibirá contribuciones externas y será usado por organizaciones grandes. Se reserva la opción de licencias copyleft **solo** para eventuales componentes de servidor/nube futuros, si algún día existen; el corazón del producto jamás cambiará de licencia.

### 9.2 Estructura de repositorio (monorepo)

```
meltemi/                       (Apache-2.0)
├── core/          # meltemid: motor Rust (specs, flota, protocolos, seguridad)
├── tui/           # cliente de terminal `meltemi` (CLI + TUI, Rust)
├── desktop/       # cliente GUI (Tauri)
├── sdk/           # SDK público para integradores y plugins
├── proto/         # esquemas JSON-RPC compartidos daemon↔clientes
├── brand/         # identidad visual
├── docs/          # documentación
├── .meltemi/      # el propio proyecto se desarrolla con Meltemi (dogfooding)
└── CONTRIBUTING.md, GOVERNANCE.md, CODE_OF_CONDUCT.md, SECURITY.md
```

### 9.3 Comunidad y gobernanza

- **Sin ánimo de lucro, con custodia clara**: Meltemi es un proyecto sin fines de lucro. La marca, el nombre y las claves de firmado de releases serán custodiados por una entidad sin ánimo de lucro (fundación o *fiscal sponsor*); mientras se constituye, los mantenedores los administran en fideicomiso con compromiso público.
- **CLA acotado y vinculante con la promesa de licencia**: el CLA concede derechos únicamente para relicenciar a licencias aprobadas por la OSI iguales o más permisivas que Apache 2.0 — nunca a licencias propietarias ni más restrictivas. Así, la promesa de §9.1 no depende de la buena voluntad futura: es contractual.
- **Dogfooding radical**: Meltemi se construye con Meltemi. El `.meltemi/` del propio repositorio — su constitución, sus specs, sus cambios archivados — es la mejor demo y el mejor tutorial.
- **Distribución**: binario único (instalador de una línea, gestores de paquetes de cada plataforma, releases firmadas).
- **Contribuciones vía el propio método**: toda funcionalidad entra como una propuesta de cambio spec-driven. La barrera de entrada es leer una spec, no navegar doscientos archivos.
- **Este documento se gobierna con el mismo método que predica**: vive en el repositorio y se versiona con él; su ratificación inicial corresponde a los mantenedores fundadores, y toda modificación posterior entra como propuesta de cambio con la aprobación definida en `GOVERNANCE.md`. **Excepción interina (bootstrap en dos etapas)**: hasta que el motor de specs de Fase 1 permita a Meltemi hospedar sus propios cambios, las enmiendas a este documento se tramitan con OpenSpec en `openspec/changes/`; la migración a `.meltemi/changes/` es la change `migracion-openspec-a-meltemi`. Esta excepción quedó ratificada en `enmiendas-fundacionales-v1`.
- **Canales**: repositorio público con discusiones abiertas + chat comunitario + desarrollo en vivo periódico.

---

## 10. Roadmap por Fases

### Fase 0 — Fundación (mes 0-1)
- Ratificar este documento y la `constitution.md`. Definir esquemas JSON-RPC (`proto/`).
- Esqueleto de `meltemid` (Rust): ciclo de vida del daemon, sesión, un agente ACP conectado de extremo a extremo.
- **Hito**: `meltemid` pilota un agente real vía ACP y ejecuta un `/propose` mínimo por JSON-RPC.

### Fase 1 — MVP: el orquestador spec-driven (mes 2-5)
- Motor de specs completo (`.meltemi/`, EARS, deltas, ciclo constitution→explore→propose→review→plan→implement→verify→archive).
- Capa de agentes: ACP nativo + adaptadores + headless estructurado; catálogo de flota; proyección de contexto a los formatos del mercado; proxy de permisos; passthrough MCP; sesiones persistentes; suite de conformidad por nivel de integración.
- Orquestación paralela con worktrees; checkpoints/rollback; integración Git (commit atómico por tarea).
- TUI completa con paridad de núcleo.
- **Hito v0.1**: un desarrollador lleva una funcionalidad de idea a código íntegramente en terminal, con specs revisables, usando dos agentes de proveedores distintos en paralelo. Métricas objetivo: primeras 1.000 estrellas, 20 contribuidores externos.

### Fase 2 — v1.0: paridad de escritorio y motor propio (mes 6-10)
- GUI Tauri con paridad de núcleo: editor de specs enriquecido, revisión de diffs línea a línea y edición utilitaria in situ con inteligencia LSP, bandeja de permisos, panel de flota. El design de esta fase resuelve la política de concurrencia humano↔agente sobre un mismo worktree.
- Motor agéntico propio (BYOK, multi-proveedor, modelos locales) como un agente más de la flota.
- Sandbox de ejecución propio por plataforma. Hooks. Sistema de plugins/skills sobre el SDK.
- **Hito v1.0**: paridad de núcleo verificada por CI; instalable en macOS/Windows/Linux (núcleo y TUI como binario único autocontenido; GUI con instalador mínimo); funciona headless por SSH.

### Fase 3 — Horizonte (mes 11+)
- Compañero móvil: el **puesto remoto del Agent Boss** — superficie compañera para **monitorear, aprobar, revisar y dirigir** la flota desde fuera de la oficina. Revisar es decidir (gates, checklist, adopción de archivos con confirmación), nunca autoría; acceso únicamente vía túnel SSH; aviso de espera opt-in y autohospedado con contenido mínimo (spec `remote-access`); regla de subconjunto respecto de TUI/GUI (spec `mobile-companion`). Prerrequisitos de daemon, paridad ×3: espera humana configurable sin denegación por caída de conexión, estado `waiting_permission` real y gates pendientes descubribles, y stream de eventos para clientes que no iniciaron la sesión.
- Funciones de equipo: specs compartidas multi-repo, archivos de rumbo gobernados por políticas de la organización.
- Esquemas SDD personalizados (research-first, ADR-first). Verificación automática y continua spec↔código — más allá del `/verify` bajo demanda — y property-based testing derivado de requisitos EARS.
- Registro comunitario de skills, hooks y perfiles de agente.

---

## 11. Riesgos y Mitigaciones

| Riesgo | Impacto | Mitigación |
|---|---|---|
| **El nicho se llena rápido** (varios productos nuevos por trimestre en orquestación y SDD) | Perder la ventana de "primeros" | Velocidad de ejecución + el diferenciador real (SDD unificado + neutralidad + apertura); dogfooding como demo viva |
| **Sobre-ingeniería del SDD** | Fricción, abandono | Modo dual `spec-full`/`fast-forward`; specs proporcionales al tamaño del cambio |
| **UX de revisión de specs pobre** | Los devs prefieren revisar código a Markdown | El editor y el diff de specs son inversión prioritaria desde el MVP |
| **Los términos de los proveedores de agentes cambian** | Integraciones que se vuelven inviables de un día para otro | Principio de juego limpio (nada que revocar); capa de agentes desacoplada por niveles; el nivel 4 (artefactos) funciona siempre |
| **Evolución del protocolo ACP** | Refactors de la capa central | SDK oficial + gobernanza pública del protocolo + suite de conformidad propia por nivel de integración |
| **Seguridad de agentes autónomos** | Daño en workspace o efectos externos | Capas del §8: worktrees, proxy de permisos, checkpoints con alcance honesto, transparencia por plataforma |
| **Riesgos de MCP** (inyección, tools envenenadas) | Exfiltración de datos | Consentimiento por servidor, revisión de tools, control de acceso, auditoría |
| **Inconsistencias de webview en Tauri** | Bugs de UI por SO | Pruebas en las tres plataformas, polyfills, framework cross-webview; lógica pesada en el daemon |
| **Fatiga del mantenedor / bus factor** | Estancamiento | Gobernanza abierta, contribución vía specs (barrera de entrada baja), CLA acotado, roadmap público |
| **Sostenibilidad sin ingresos** | Falta de fondos para infraestructura | Costos cercanos a cero por diseño (BYO-todo, sin backend); donaciones/sponsors abiertos; cualquier servicio gestionado futuro sería un proyecto separado |

---

## 12. Métricas de Éxito

Las métricas de producto y flota se calculan **íntegramente en local** (§6.12) y solo se agregan mediante telemetría opt-in, desactivada por defecto; las de adopción y ecosistema provienen de señales públicas. La telemetría agregada es **post-v1**: la operaría la entidad custodio sin ánimo de lucro (§9.3), con los datos y la política de privacidad especificados y publicados antes de existir (constitución §9); hasta entonces, todo se calcula y queda en local.

**Adopción**: estrellas y contribuidores externos del repositorio; descargas de releases; instalaciones por gestores de paquetes; proyectos con `.meltemi/` públicos en GitHub.

**Producto (calidad SDD)**: % de funcionalidades entregadas con spec aprobada previa; trazabilidad código→requisito; tiempo idea→código; tasa de retrabajo post-implementación.

**Flota**: número de agentes distintos usados por usuario; % de tareas ejecutadas por ≥2 agentes en paralelo; % de usuarios con modelos locales (indicador de la promesa de privacidad).

**Paridad y rendimiento**: cobertura de paridad de núcleo TUI↔GUI (objetivo: 100% de la API del daemon); instalador GUI < 15 MB; memoria de la GUI en reposo < 80 MB; arranque < 1 s; binario TUI < 25 MB.

**Ecosistema**: servidores MCP y skills comunitarias; agentes soportados por nivel de integración; ratio de issues abiertas/cerradas (la fricción alta es el síntoma que más rápido mata la confianza).

---

*Fin del documento fundacional. Como corresponde a un proyecto spec-first, el siguiente paso NO es escribir código: es ratificar la `constitution.md` y abrir la primera propuesta de cambio en `.meltemi/changes/` — construir Meltemi con Meltemi, desde el primer día.*

*El viento ya sopla. Lo único que falta es izar las velas.*
