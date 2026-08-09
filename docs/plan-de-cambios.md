# Plan maestro de cambios — de cero al MVP (hito v0.1)

> Backlog ordenado de propuestas de cambio que llevan a Meltemi desde el repositorio vacío hasta el hito v0.1 de Fase 1 (meltemi.md §10). Este documento es el mapa; **cada change escribe sus cuatro artefactos (proposal, design, specs, tasks) inmediatamente antes de implementarse** — nunca durante, y nunca se implementa nada que no esté aquí o en una change aprobada. Producto de la auditoría de planificación del 2026-07-11 (5 lentes, 64 hallazgos).
>
> **Cierre del bootstrap (2026-07-18)**: la etapa OpenSpec queda **cerrada**. El motor de specs de fase 1 hospeda la verdad viva del proyecto en `.meltemi/specs/` y el histórico en `.meltemi/changes/archive/` (migrados con verificación del motor, `migracion-openspec-a-meltemi`); el método del proyecto son los comandos de Meltemi sobre `.meltemi/`. El árbol `openspec/` se conserva como histórico consultable hasta el retiro físico confirmado por el mantenedor (D3 de la migración).
>
> **Desviación aprobada por el mantenedor (2026-07-16, ampliada el mismo día)**: el backlog está **completamente materializado** — las 19 changes restantes existen en `openspec/changes/` con sus **cuatro artefactos** (proposal, design, specs EARS, tasks), listas para `/opsx:apply` en el orden declarado. Disciplina que lo hace sostenible: (1) los **deltas de `cli-contract` son acumulativos** — cada uno asume archivadas las changes anteriores del orden; alterar el orden exige revisar esos deltas; (2) los designs de las changes profundas marcan sus decisiones como **provisionales** y se **revalidan obligatoriamente al llegar al frente de la cola** (en especial `ciclo-sdd-autoria`, `orquestacion-worktrees` y `comando-implement`, la change de integración); (3) los hallazgos de QA y la deuda registrada están absorbidos en las specs correspondientes (H1/H4/H5 → `proxy-permisos`; H6 → `documentacion-inicial`).

## Estado de "listo para programar"

| Prerequisito | Estado |
|---|---|
| Documento fundacional (meltemi.md v1.0) | ✅ Ratificado 2026-07-11 |
| Constitución + rumbo (`.meltemi/`) | ✅ Ratificados 2026-07-11 |
| Change de Fase 0 completa (4 artefactos + enmiendas de auditoría) | ✅ |
| Proyección de contexto raíz (`AGENTS.md`/`CLAUDE.md`) | ✅ |
| Research de integración de agentes persistido | ✅ `docs/research/integracion-agentes.md` |
| Plan maestro de cambios | ✅ Este documento |
| **Ratificación humana (Guillermo)** de meltemi.md, constitución y rumbo | ✅ 2026-07-11 |
| **Registros operativos** (org GitHub, org npm, dominio, crates) | 🟡 org GitHub y npm hechas (`askenaz-dev`); DNS de `meltemi.dev` apuntado a Pages (2026-07-26; el sitio sirve por HTTP, certificado HTTPS aún en provisión); **v0.1.0 publicada** (2026-07-26, 10 assets firmados); crates por reservar (verificados libres el 2026-07-27) |

## Fase 0 (en curso)

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 0 | ✅ `fase-0-fundacion` | Constitución ratificada, contrato `proto/`, esqueleto `meltemid`, sesión ACP e2e, `/propose` mínimo | — |

## Fase 1 — backlog ordenado (MVP)

**Ola 1 — método y fundamentos** (arranca al cerrar Fase 0):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 1 | ✅ `enmiendas-fundacionales-v1` | Vía rápida: ratifica el bootstrap en dos etapas (excepción a §9.3), actualiza §0 a la marca V2, asigna fase a §6.12 (métricas), aclara telemetría post-v1, añade plataforma primaria de desarrollo | 0 |
| 2 | ✅ `formato-artefactos-meltemi` | Esquema canónico de `.meltemi/`: nombres de artefactos, cabeceras de delta, canon EARS (política bilingüe: estructura en inglés vs prosa en español — resolver la contradicción §5.1/práctica), front-matter de rumbo | 0 |
| 3 | ✅ `motor-specs-artefactos` | Parseo y validación de la estructura `.meltemi/` completa (constitución, rumbo, specs, changes, archive) | 2 |
| 4 | ✅ `motor-ears-deltas` | Validación EARS en vivo, parseo/aplicación de deltas; la detección semántica de contradicciones/huecos quedó explícitamente diferida a una change futura | 3 |
| 4b | ✅ `enmienda-edicion-movil` | Vía rápida (2026-07-15): cerca de la edición utilitaria in situ (spec `edit-surface`) y alcance del compañero móvil monitorear/aprobar/dirigir vía túnel SSH (spec `mobile-companion`); meltemi.md v1.2 → **v1.3** (ratificación pendiente) | 1 |

**Ola 2 — superficies** (en paralelo con Ola 1 tras el #0):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 5 | ✅ `cli-contrato` | Gramática de subcomandos, regla de despacho CLI↔TUI, códigos de salida, `--json`, disciplina stdout/stderr; mapeo comando↔RPC junto a `proto/` | 0 |
| 6 | ✅ `tui-nucleo-ux` | Arquitectura de información de la TUI (paneles, navegación), estados vacíos y onboarding de primer uso, accesibilidad terminal (no-solo-color, NO_COLOR, fallback ASCII) | 5 |
| 7 | `catalogo-flota` | Registro público ACP + detección local de binarios; nivel de integración, permisos y estatus por agente | 0 |
| 8 | `sesiones-reanudables` | Reanudar sesión ACP, recuperación tras caída, inspección/navegación de sesiones | 0 |

**Ola 3 — capa de agentes completa**:

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 9 | `proxy-permisos` | Motor de reglas permitir/preguntar/denegar + UX de la bandeja (cola de peticiones concurrentes, creación de reglas in situ, mitigación de fatiga de aprobación, auditoría) | 7 |
| 10 | `niveles-integracion-conformidad` | Niveles 2 (adaptadores), 3 (headless JSON) y 4 (artefactos); suite de conformidad por nivel con criterios pasa/no-pasa | 7 |
| 11 | `proyeccion-contexto` | Compilación constitución+rumbo+spec activa → AGENTS.md y variantes; bloques gestionados que nunca sobrescriben contenido del usuario | 3 |
| 12 | `gestion-contexto-repo` | Mapa del repositorio, referencias `@archivo`/`@carpeta` en prompts | 11 |
| 13 | `mcp-passthrough` | Declaración de servidores MCP e inyección a agentes compatibles; higiene §8.7 | 7 |

**Ola 4 — el ciclo SDD completo**:

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 14 | `ciclo-sdd-autoria` | `/constitution`, `/explore`, `/propose` completo (requirements EARS + design + deltas + tasks con gates humanos), `/plan`; modo dual `spec-full`/`fast-forward` con criterio escrito de proporcionalidad | 4, 6 |
| 15 | `revision-specs-ux` | La obsesión (§4.9): render de diff de deltas en terminal, presentación de contradicciones, checklist interactiva de `/review`, comentario→instrucción al agente | 14 |
| 16 | `orquestacion-worktrees` | Worktrees aislados, N agentes × M tareas, carreras, merge asistido lado a lado | 8 |
| 17 | `checkpoints-rollback` | Checkpoint automático pre-tarea, reversión granular, alcance honesto (qué NO se revierte) enganchado a la clasificación del proxy | 16 |
| 18 | `git-commit-por-tarea` | Commit atómico con trailer de trazabilidad hasta el requisito EARS; convención de mensajes | 16 |
| 19 | `comandos-verify-archive` | `/verify` (checklist por requisito + tests vinculados a criterios) y `/archive` (fusión de deltas en la verdad viva) | 4, 18 |
| 20 | `comando-implement` | `/implement`: despliegue de agentes sobre `tasks.md` en modo planificar/actuar con checkpoints | 14, 9, 16, 17, 18 |

**Ola 5 — salida al mundo** (antes de que el repo sea público / del hito v0.1):

| # | Change | Alcance | Depende de |
|---|---|---|---|
| 21 | `gobernanza-comunidad` | GOVERNANCE.md, CONTRIBUTING.md (contribución vía specs), CODE_OF_CONDUCT.md, SECURITY.md, texto del CLA acotado (§9.3) + tooling de firma, plantillas `.github/` | — (antes de público) |
| 22 | `documentacion-inicial` | README raíz, quickstart, esqueleto de docs/ y tooling de documentación | 6 |
| 23 | `distribucion-releases` | Versionado, firmado y custodia de claves, empaquetado por plataforma, instalador de una línea | 6 |
| 24 | `migracion-openspec-a-meltemi` | El dogfooding definitivo: las specs vivas del propio proyecto migran a `.meltemi/` | 19 |
| 25 | `hito-v01-aceptacion` | El escenario del hito como spec ejecutable: una feature de idea a código en terminal con dos agentes de proveedores distintos en paralelo | todo lo anterior |

## Orden de abordaje declarado (2026-07-16)

Secuencia canónica de las 19 changes restantes (todas con proposal pre-redactada). Satisface el grafo de dependencias y prioriza: completar la capa de agentes con gobierno de permisos, luego el contexto, luego el ciclo SDD, luego el paralelismo, y al final la salida al mundo. Nada queda por fuera.

| Orden | Change | Por qué aquí |
|---|---|---|
| 1º | `catalogo-flota` | Desbloquea #9/#10/#13; sin flota no hay orquesta |
| 2º | `proxy-permisos` | Agentes reales exigen gobierno real; corrige H1 (QA) y la deuda del contador de pendientes |
| 3º | `proyeccion-contexto` | Cierra la proyección manual de AGENTS.md; vehículo del nivel 4; independiente |
| 4º | `niveles-integracion-conformidad` | Cubre el mercado completo (niveles 2/3/4), con la proyección ya disponible |
| 5º | `mcp-passthrough` | Los agentes llegan con sus herramientas; capa de agentes completa |
| 6º | `sesiones-reanudables` | Robustez e histórico; puerta del paralelismo (#16) |
| 7º | `gestion-contexto-repo` | Mapa y `@refs` para dirigir agentes antes de la autoría pesada |
| 8º | `ciclo-sdd-autoria` | El corazón del método — llega con flota, permisos y contexto ya operativos |
| 9º | `revision-specs-ux` | La obsesión (§4.9): sin revisión placentera, el método muere |
| 10º | `orquestacion-worktrees` | N agentes × M tareas sin pisarse; carreras |
| 11º | `checkpoints-rollback` | Autonomía solo con deshacer barato y honesto |
| 12º | `git-commit-por-tarea` | Trazabilidad línea→requisito (constitución §8) |
| 13º | `comandos-verify-archive` | Cierra los verbos del ciclo; el motor ya demostró la fusión |
| 14º | `comando-implement` | La composición de todo (requiere 2º, 8º, 10º, 11º, 12º ya hechas) |
| 15º | `gobernanza-comunidad` | Antes de abrir el repositorio al mundo |
| 16º | `documentacion-inicial` | El quickstart ya describe un producto real, no una promesa |
| 17º | `distribucion-releases` | Empaquetado, firmado, instalador; materializa reserva de crates y alias `mel` |
| 18º | `migracion-openspec-a-meltemi` | El dogfooding definitivo: se retira la herramienta prestada |
| 19º | `hito-v01-aceptacion` | El hito como spec ejecutable; la meta de v0.1 |

> Paralelizable sin romper el orden: 15º–16º (documentos, sin código) pueden avanzar en cualquier momento por una sesión aparte; 3º puede adelantarse en paralelo tras 1º. El orden canónico es el de la tabla; toda desviación se anota aquí.

## Documentos transversales (se crean durante Fase 1, referenciados por las changes)

- `docs/plataformas.md` — matriz de soporte: SO mínimos, arquitecturas, webviews (antes de #23).
- `docs/presupuestos-rendimiento.md` — los números de §12 traducidos a criterios EARS y gates de CI (antes de #6).
- `docs/paridad-nucleo.md` — matriz viva capacidad → RPC → TUI → GUI; toda change que añada un RPC la actualiza (desde #5).
- `docs/operaciones/checklist-lanzamiento.md` — registros y activos (ver pendientes del arquitecto).

## Fase 2 (v1.0) — abierta el 2026-07-20

**Archivadas el 2026-07-25** (183 escenarios verificados, 64 requisitos
revisados uno por uno, deltas plegados a la verdad viva):

| Change | Qué dejó en la verdad viva |
|---|---|
| `gui-tauri-paridad` | `desktop/` Tauri 2 + Svelte 5, crate compartido `core/meltemi-client`, matriz de paridad como gate de CI, política de concurrencia humano↔agente (`worktree/apply-edit` + `human_edit` + nota al siguiente turno), LSP BYO, instaladores con gate < 15 MB; creó `docs/ux/design-system.md` y `docs/paridad-nucleo.md` |
| `gui-clase-mundial` | Shell de tres zonas con la densidad del design system, identidad de entidades, drawer, superficie de Ajustes, paleta con difusa/grupos/formularios tipados, la sesión como acción primaria, guarda de trabajo sin guardar |
| `flota-deteccion-guia` | Detección en dos capas (CLI oficial + adaptador) con estado compuesto, remedio por capa con su comando exacto, estatus legal sin maquillaje y `docs/agentes.md` verificada contra el registro |
| `multiproyecto-suscripciones` | Registro de proyectos (`project/list`), agente y suscripción en los metadatos de sesión, árbol Proyecto→Sesiones en la GUI, agrupación y ámbito conmutable en la TUI |
| `analitica-consumo-local` | `analytics/usage`: actividad plegada de los registros locales, tokens solo donde la salida oficial los reporta, frontera medido/no-reportado con motivo estable y declaración de honestidad junto a las cifras |
| `sitio-web-producto` | `site/` estático (sin JS, sin orígenes externos), descargas a la última release firmada con nombres estables, tokens derivados del cliente y lint del sitio como gate |
| `tablero-de-carrera` | **Archivada el 2026-08-08.** La carrera visible: procedencia por calle en `worktree/diff` (fuente, suscripción, nivel, sesión, commit y base propia, todo aditivo), el despacho asentando registro de primera clase, el tablero de la GUI (calles lado a lado, acciones con confirmación, merge por archivo) y el del shell (glifo+palabra con gemelos ASCII, despacho sin congelar el bucle); `docs/qa/2026-08-07-tablero-de-carrera-smoke.md` |

Pendientes de Fase 2: `motor-propio-byok` (propuesta redactada, activa) ·
`lanzador-conversacional` (abierta 2026-07-27, **implementada y verificada
106/106; no archiva hasta la ratificación de la enmienda**) ·
`adaptadores-propios-acp` (abierta 2026-07-27, implementada 24/24 con
conformidad real anclada; espera la decisión del mantenedor sobre enviar con
Codex en nivel 0) ·
`texto-intacto-al-agente` (abierta 2026-07-31, vía rápida: el prompt se
doble-codifica y el agente recibe acentos rotos; verify limpio, lista para
archivar) ·
`avisos-de-escritorio` (abierta 2026-07-31, del comparativo de mercado: nada
avisa al escritorio cuando un permiso espera) ·
`primer-arranque-del-home` (abierta 2026-07-31, vía rápida, del mismo
comparativo) ·
`artefactos-de-cada-push` (abierta 2026-08-05, vía rápida: el build de cada
push a `main` descargable como artefacto de la ejecución, jamás como release) ·
`vincular-suscripciones` (abierta 2026-08-08, planificada: el pedido
fundacional — vincular N suscripciones por proveedor desde cualquier
superficie, con la variable de contexto como dato del registro) ·
`registro-agentes-en-superficie` · `menu-nativo-aplicacion` · `sandbox-propio` ·
`hooks-eventos` · `plugins-skills-sdk` · `i18n-superficies` ·
`metricas-sdd-locales` · `lsp-superficie-revision`.
(`pulido-pre-anuncio`, la vía rápida del acabado pre-anuncio, quedó archivada
el 2026-07-27.)

### `motor-propio-byok` — propuesta activa desde el 2026-07-25

Nace de tres peticiones del mantenedor: una vía para **modelos autohospedados**
(ollama y cualquier endpoint OpenAI-compatible), el **harness como concepto de
primera clase con un default**, y una decisión sobre su proyecto **Forge
Harnesses**. La forma la decidió meltemi.md D6: el motor propio entra a la flota
como un agente ACP de nivel 1 (`core/meltemi-engine`), pilotado por stdio igual
que cualquier agente externo — misma detección, mismo proxy de permisos, mismos
worktrees y checkpoints, **jamás un canal privilegiado**. Todo el tráfico de red
vive en el subproceso: `meltemid` no enlaza pila HTTP/TLS alguna, propiedad
auditable que sostiene §3.

Un harness es un manifiesto TOML v1 (dialecto, `base-url`, modelo, prompt,
política de herramientas, límites) que el daemon **valida y lista, nunca
interpreta**; el default va embebido apuntando a `http://localhost:11434/v1`,
el único que no privilegia a proveedor comercial alguno. Sin modelo alcanzable
rehusa con remedio; nunca degrada en silencio. Las claves BYOK entran solo por
referencia `${VAR}`, con el lint de higiene existente.

**Decisión sobre Forge Harnesses: se mantiene como proyecto separado**,
conectado por contrato — este repositorio publica el esquema versionado del
manifiesto (JSON Schema + fixtures de conformidad) y Forge produce manifiestos
que se prueban contra ellos. Absorberlo sometería un laboratorio de iteración de
prompts a la constitución entera (spec-first por cada ajuste, clippy en 3 SO,
Apache-2.0 + CLA desde el día uno), no compraría ni un test de integración en CI
(que por regla no ejecuta agentes reales ni red) y arrastraría a Meltemi hacia el
marketplace que su rumbo explícitamente no es. Si alguna vez se absorbe, entra
Apache-2.0 bajo CLA, sin excepciones.

Mientras la change no se implemente, correr un modelo local bajo Meltemi ya es
posible **hoy y sin código nuevo**: OpenCode (nivel 1) y Aider (nivel 3) de la
flota actual hablan con endpoints ollama/OpenAI-compatible por su propia
configuración. La guía de docs que lo explique entra con la change, verificada
contra las versiones vigentes y no citada de memoria.

El design system del mantenedor vive en `design-system/` y es la fuente visual
normativa (gui-clase-mundial D11).

### `instaladores-linux-sin-webview` — activa desde el 2026-07-25

El gate de tamaño disparó en Linux la primera vez que llegó a apuntar ahí: el
AppImage pesa 78 678 520 B contra 15 MB. El número no se ensancha porque no es un
número: el design D7 de `gui-tauri-paridad` dejó escrito que codifica «no
empaquetamos motor de navegador», y meltemi.md §7 rechaza esa vía por su nombre.
La change retira el AppImage, declara `libwebkit2gtk-4.1-0` y `libgtk-3-0` en el
`.deb` —que hoy instala limpio y no arranca— y corrige la prosa pública en los
dos idiomas. El hueco queda nombrado: fuera de la familia Debian no hay
instalador gráfico hasta que exista un `.rpm`, que es change propia porque exige
verificar los nombres de paquete en Fedora/RHEL en vez de adivinarlos.

### `procedencia-de-release` — activa desde el 2026-07-26

Nace de una pregunta del mantenedor tras firmar la v0.1.0 a mano: las empresas
grandes no firman desde un portátil. Es cierto — usan HSM/KMS con clave no
exportable, o firma keyless sin clave de largo plazo; la ceremonia humana firma
claves, no releases. Pero la conclusión no es mover la clave a CI: un secreto de
Actions es exactamente el caso que SLSA v1.2 prohíbe en lenguaje normativo, y
**la firma manual es el único paso que una cuenta de GitHub comprometida no puede
completar** — un atacante empuja un tag, CI construye y atestigua, y la
atestación verifica perfectamente porque registra fielmente su commit.

La change deja la firma donde está y añade lo que falta: una **atestación de
build** sobre el `SHA256SUMS` publicado, que liga los artefactos al repositorio,
al commit y al workflow. Enmienda además el requisito de custodia para que sus
promesas sean cumplibles — el ancla de confianza vive en el repositorio y no en
la página de release que autentica; el almacenamiento es offline, no
hardware-backed (minisign no tiene HSM ni PKCS#11); y «revocar» queda definido
como publicar clave nueva y repudiar la vieja, porque minisign carece de
mecanismo de revocación.

**Deuda declarada al archivar** (no se archivó nada fingiendo que estaba
hecho): ✅ la captura de escritorio del sitio está publicada y su procedimiento
es un script (`scripts/capture-desktop.ps1`, `docs/ux/capturas.md`); la firma de MSI/DMG sigue pendiente
de infraestructura de certificados; el arranque y la RAM en macOS y Linux se
publican en el QA de la primera release que incluya la GUI.

### `pulido-pre-anuncio` — vía rápida, archivada el 2026-07-27

Tres defectos de acabado a la vista del anuncio. Uno: **siete botones de la
GUI apilan el icono sobre la etiqueta** — el skin global de botones no declara
`display` y el svg de `Icon` es block, así que todo botón icono+texto sin
regla flex local rompe la línea; la regla global gana `inline-flex` y las
re-declaraciones locales duplicadas se retiran (la repetición por componente
era la causa raíz). Dos: «Ver la flota (4)» incrusta el atajo de teclado como
falso contador; el atajo ya tiene su afordancia `kbd` en el sidebar. Tres:
los `adapter-install` del registro **recomiendan instalar desde rutas
muertas** — el `codex-acp` Rust de Zed quedó archivado el 2026-07-22 y los
adaptadores canónicos viven bajo `@agentclientprotocol` (verificado contra
npm el 2026-07-27, no citado de memoria). Deltas ADDED-only sobre `gui-shell`
y `fleet-catalog`; el refresco del registro es puente honesto mientras
`adaptadores-propios-acp` no aterrice, y el build que acompañe el anuncio se
reconstruye tras el merge.

### `lanzador-conversacional` — abierta el 2026-07-27

Nace de la prueba directa del mantenedor: el modal de lanzamiento actual le
pareció «ordinario», y el mockup que siguió fijó la dirección final en tres
directivas. **Supersede y absorbe** los borradores `sesion-conversacional` y
`gestion-proyectos-en-superficie`, que no entran al backlog como changes
propias. Uno: **home conversacional** — compositor al centro con el contexto
como chips (proyecto, agente, modo); enviar navega hacia adentro de la
sesión, a una vista de conversación con compositor persistente sobre
`session/direct`, burbujas de turno **sobre** el log de eventos (el log de
operador sigue a un conmutador: el transcript es la verdad) y tarjetas de
permiso en línea, con estados honestos — «encolada» jamás finge respuesta.
Dos: **el modo libre es el default** — una sesión nueva es una sesión libre
gobernada sobre el proyecto elegido; Proponer y Explorar son modos opt-in
del mismo compositor. Tres: **los proyectos viven en la navegación** —
sección persistente con sesiones anidadas, «Abrir carpeta…» en el nav y en
el chip (diálogo nativo vía el plugin oficial de Tauri, dependencia nueva
del cliente justificada §10), acción rápida por proyecto.

El default libre obliga a una honestidad de producto que la propuesta nombra
con sus frases exactas: hoy **no existe RPC para iniciar una sesión libre**
(`session/direct` exige sesión existente; `propose`/`sdd/explore` son verbos
del método; `worktree/dispatch` exige change y tarea), y la promesa pública
dice lo contrario — «ninguna línea de código se escribe sin una
especificación revisada» (rumbo de producto y tesis de meltemi.md). La
resolución: la sesión libre queda **gobernada siempre** (proxy
deny-by-default, aislamiento y checkpoints, log apend-only — §3 no se
negocia) pero **no spec-gated**; el método pasa de cerrojo a propuesta de
valor por sesión. La redacción de rumbo/product.md y meltemi.md se enmienda
con **ratificación del mantenedor como gate**; la constitución del propio
repositorio no se toca — el desarrollo de Meltemi sigue spec-first.

Al daemon entran solo los verbos que la dirección genuinamente no tiene: el
arranque de sesión libre (forma final en el design), `project/register`
(alta validada y canonicalizada) y `project/forget` (baja **solo del
registro**, una línea de olvido en el JSONL que el plegado last-wins
resuelve; jamás toca el disco), más el parámetro `agent` aditivo en el verbo
nuevo y en `propose`/`sdd/explore` — resuelto por `resolve_fleet_agent`, con
error estructurado que lista candidatos detectados en vez del string crudo.
La conversación en sí es composición cliente sobre `eventos-para-tardios`.
Paridad ×3 (§4): verbos CLI, `direct` interactivo en la TUI (hoy reservado
en la paleta), render de proyectos, `docs/paridad-nucleo.md` al día; el
cromo exclusivo de la GUI no tiene deber de paridad, toda capacidad nueva
del daemon sí. Fuera de alcance: `menu-nativo-aplicacion` y
`registro-agentes-en-superficie` siguen siendo changes propias (registrar
agentes ≠ seleccionarlos), y el render conversacional del histórico
archivado más allá del conmutador de log queda para futuro con evidencia.

**Desenlace (2026-07-31) — implementada y verificada; el archivo espera una
firma.** Los nueve bloques de tareas están cerrados, `meltemi validate` sale
limpio y `meltemi verify` da **106/106 escenarios, sin una sola marca manual**.
Lo que quedó decidido y conviene leer aquí, porque son las cosas que la
propuesta dejó abiertas:

- **Nombres finales de los verbos CLI** (design D9): `meltemi session
  <instruction> [project-root] [--agent <id|perfil>]` —no `start`, que se leería
  como arrancar el daemon junto a `stop`— y `meltemi projects register|forget
  <path>`, colgados del listado en plural porque `project` ya toma una raíz
  posicional y habría leído `register` como una ruta. `--agent` se parsea una
  sola vez en `plan` y se entrega al subcomando, de modo que vale en cualquier
  posición; un subcomando que no arranca sesión la rehúsa con diagnóstico en vez
  de tragársela.
- **El guardián de reversión**, que entró en esta change y no en una futura
  (design D2): `checkpoint/revert` rehúsa todo punto de restauración cuyo árbol
  no sea un worktree gestionado, con el remedio de `git restore --source`.
  Escribir el checkpoint de la sesión libre sin ese guardián habría armado un
  verbo existente contra el árbol del usuario —`reset --hard` más `clean -fd`
  sobre trabajo humano sin commitear—; ninguna superficie ofrece ya ese control.
- **La enmienda de la promesa está aplicada y sin ratificar**: `rumbo/product.md`
  y la tesis de meltemi.md (v1.5) llevan el texto de D5 con su nota de pendiente.
  **Sin la firma del mantenedor esta change no archiva** — pendiente 1c del
  arquitecto, donde además está lo que hay que hacer al firmar.
- **Una enmienda de spec en marcha** (design D12): la vista de aterrizaje de
  `gui-shell` era «Sesiones» en la verdad viva y estaba fijada por un test; se
  enmendó el requisito, no la implementación, porque el compositor como llegada
  es la directiva entera de la change.
- **Documentación**: `docs/sesion-libre.md` (qué gobierna una sesión sin spec,
  dónde opera, y por qué el punto de restauración no es una reversión ofrecida)
  y el smoke visual con 21 medidas en
  `docs/qa/2026-07-31-lanzador-conversacional-smoke.md`.

**Lo que el smoke encontró y esta change no arregla** (rumbo de estructura: lo
que surge se anota, no se cuela). Dos merecen decisión antes de archivar:

1. **El texto no ASCII llega corrupto al agente** — `out.push(bytes[i] as char)`
   en `core/meltemid/src/repo_map.rs:120` convierte cada byte UTF-8 en un code
   point Latin-1, así que «acción» viaja como «acciÃ³n» en **todo** prompt que
   pasa por la expansión de `@`. Es anterior a esta change y de gravedad alta:
   el agente recibe la frase rota, no solo la pantalla. Anotado arriba como
   `texto-intacto-al-agente`.
2. **Olvidar un proyecto con sesiones no quita su nodo del árbol**: el daemon lo
   saca del listado, pero el cliente lo reconstruye como nodo inferido
   (`desktop/ui/src/lib/tree.ts:81`) y no distingue los inferidos. Esconderlo
   dejaría sesiones sin casa en el árbol, que es justo lo que el nodo inferido
   evita: es decisión de producto, no un arreglo obvio.

Los otros tres (posición en la cola tapada por el aviso de permiso, cierre de
turno como línea neutra tras una tarjeta, y el primer `fleet/list` saliendo antes
de que el ámbito de proyecto se resuelva) están descritos con su evidencia en el
informe de QA.

### `adaptadores-propios-acp` — abierta el 2026-07-27 (directiva del mantenedor)

Dos adaptadores ACP propios, en Rust y en este monorepo, para los dos agentes
cuyo nivel 2 depende hoy de adaptadores de terceros: Claude Code y Codex. Esta
change **revierte una decisión registrada** — `niveles-integracion-conformidad`
dejó fuera de alcance «Mantener adaptadores propios: se consumen adaptadores
abiertos existentes» — y la reversión se argumenta, no se esconde. El suelo se
movió: Zed archivó su `codex-acp` en Rust el 2026-07-22 y los adaptadores
canónicos viven ahora en TypeScript bajo la org `agentclientprotocol` — la
opción Rust upstream murió, y las dos filas `adapter-install` del registro ya
apuntan a rutas rancias. La zona gris del adaptador de Claude basado en Agent
SDK es permanente: los términos de Anthropic (feb-2026) nombran al SDK como no
autorizado para OAuth de suscripción, y ningún fork lo arregla porque la zona
gris vive en la superficie de auth, no en quién mantiene el adaptador. Y los
adaptadores propios pueden viajar `bundled = true` en los instaladores —
reutilizando la detección de directorio hermano de `motor-propio-byok` —
matando el muro de onboarding «adaptador no detectado» que
`flota-deteccion-guia` diagnosticó.

El adaptador de Claude pilota el **binario oficial `claude` con la sesión que
el usuario ya inició**, vía `-p --input-format stream-json --output-format
stream-json` — jamás el Agent SDK, jamás `--bare`; el flip anunciado de
`--bare` como default de `-p` queda pineado como riesgo en el design, no se
descubre después. Los permisos pasan por `--permission-prompt-tool` (un shim
MCP por stdio que releva al socket existente de meltemid; el daemon no gana
transporte alguno) con hooks `PreToolUse` como compuerta dura, y las pérdidas
se declaran: `AskUserQuestion` se auto-deniega en modo no interactivo y el
contrato del prompt-tool está infradocumentado upstream (issue #1175). El
adaptador de Codex lanza el CLI oficial `codex` en modo `app-server` —
JSON-RPC NDJSON por stdio, documentado, la misma interfaz que usa la extensión
VS Code del propio proveedor, con esquema por versión vía
`generate-json-schema` que es historia de conformidad lista — y **no** el
patrón de embeber `codex-core` como librería de los adaptadores Rust
archivados, que choca con §2 aunque sea limpio de licencia.

**Honestidad legal sin maquillaje**: esto no vuelve «sancionado» a Claude —
gris pasa a tolerado-con-nota en el mejor caso, y la nota del registro se
mantiene veraz; Codex sigue tolerado y mejora, porque desaparece la
dependencia de supply chain sobre un proyecto archivado. Anclas: §2 (binarios
oficiales, auth gestionada por el agente — el adaptador jamás toca tokens),
§5 (los adaptadores entran a la flota como cualquier otra entrada), §6
(prueba por escrito: ACP es el estándar del lado Meltemi; del lado proveedor
no hay estándar — stream-json y app-server son la superficie programática
oficial de cada uno), §10 (dependencias confinadas a los crates adaptadores)
y un solo lenguaje de sistemas. Fuera de alcance: forkear los adaptadores TS
y toda vía basada en SDK. Por directiva del mantenedor (2026-07-27: «no
quiero ACP de terceros… construimos los propios»), los adaptadores propios
pasan a ser la capa por defecto del registro, empaquetados `bundled` en los
instaladores; los de terceros siguen alcanzables por configuración del
usuario, pero dejan de ser la vía recomendada.

### `texto-intacto-al-agente` — abierta el 2026-07-31, vía rápida

El primero de los cinco hallazgos del smoke conducido del 2026-07-31, y el
único de gravedad alta: **el prompt llega al agente con los acentos rotos**.
`out.push(bytes[i] as char)` en `expand_refs`
(`core/meltemid/src/repo_map.rs`) recorre el prompt como bytes y convierte
cada byte UTF-8 en su code point Latin-1 homónimo, así que «acción íntegra
ñandú» entra con 20 caracteres y el registro de la sesión guarda 24. No es un
defecto de pintado: lo corrompido es el prompt, y alcanza a **todo** camino de
prompt del daemon —`free_session.rs` y `propose.rs` pasan ambos por la misma
función—, lo que en un proyecto que se escribe en español significa casi toda
frase enviada. Es anterior a `lanzador-conversacional`; esa change solo lo hizo
visible, porque el compositor volvió al usuario lector de su propia frase.

La corrección copia el tramo literal entre referencias como una sola rebanada
`&str` en vez de empujar byte a byte, conservando los índices de byte que la
lógica del token necesita para rebanar (design D1). De la misma raíz cuelga el
defecto hermano que la change arregla en el mismo aliento: `is_ref_char` solo
admitía `is_ascii_alphanumeric`, así que un archivo acentuado —ordinario en
esta máquina— **no se podía referenciar** y `@informé.md` diagnosticaba «no
encontrado» sobre un `informe` que el usuario nunca escribió; el token pasa a
`char` con `is_alphanumeric`, que admite cualquier alfabeto y sigue dejando
que la puntuación española («¿», «—», «…») lo cierre en vez de tragársela
(design D2). Deltas ADDED-only sobre `repo-context`: dos requisitos que
escriben lo que hasta hoy era conducta accidental — el texto no referenciado
viaja intacto (con `@@` literal, que tampoco estaba en la verdad viva) y las
rutas fuera de ASCII se referencian como cualquier otra. Cero dependencias
nuevas y ningún movimiento del contrato `proto/`. El barrido de `as char` se
hizo y queda escrito (design D5): un único sitio en código propio, el que esta
change arregla.

### `tablero-de-carrera` — abierta el 2026-07-31

Nace de la demostración de aceptación: el daemon corre la carrera
multiproveedor entera — worktrees aislados, despacho con el binario y la
suscripción de cada proveedor, procedencia persistida, commit trazable —
pero **ninguna superficie la muestra como carrera**, y para verla hubo que
construir un tablero externo desechable. El inventario honesto: la GUI
tiene medio tablero por accidente (el drill-in de revisión compara diffs
de competidores, sin procedencia ni estado ni acciones), el shell anuncia
los diez verbos de carrera como «(reservado)» y no renderiza ninguno, y
debajo hay tres huecos de contrato que ninguna superficie puede rodear:
el despacho no nombra la sesión que abre, el diff por calle calla la
procedencia que el daemon conoce, y las sesiones de despacho ni siquiera
asientan registro en el índice (aparecen reconstruidas, con nivel
mentiroso). La change ensancha el contrato con campos aditivos por calle
(cero ruptura, sin verbo nuevo, la matriz de paridad no cambia de filas),
hace que el despacho asiente registro de primera clase, y abre las dos
superficies: en la GUI el drill-in de revisión evoluciona a tablero
(calles lado a lado con procedencia, acciones con confirmación explícita,
actualización al concluir turnos); en el TUI el verbo `race` deja de estar
reservado y abre el tablero sin tocar el contrato de dígitos 1–4. Cuatro
deltas ADDED-only (`worktree-orchestration`, `session-history`,
`gui-shell`, `tui-shell`), doce escenarios, cero dependencias nuevas;
spec-full deliberado — es superficie nueva sobre la feature fundacional,
no un ajuste. Fuera de alcance, por escrito: canal push de worktrees
(sigue diferido en `eventos-para-tardios`), merge automático o ranking, y
toda vista numerada nueva.

**Desenlace (archivada el 2026-08-08).** Quince tareas, veintiséis commits,
`verify` 12/12 enlazado **sin una sola marca manual**, suite en 858. El design
salió enmendado por la implementación en un punto que importa: daba por hecho
que `level`, `agent_id` y `profile` bastaban para que una calle declarara su
resolución, y no bastan — un id de catálogo y un agente configurado que nombra
un id se ven idénticos en esos campos, así que deducir la fuente habría sido
inventarla; `SessionRecord` ganó un `source` opcional que escriben todos los
caminos que ya resolvían por la flota. Dos decisiones de implementación que el
design no anticipó y quedaron escritas: `committed` es un hecho del árbol (la
cabeza difiere de su base) y no una marca que deje el despacho, de modo que una
calle comiteada a mano cuenta; y cada calle conserva **su** base, porque dos
calles de una tarea solo la comparten cuando nadie replicó la asignación contra
un HEAD movido. El smoke conducido sobre el binario de release
(`docs/qa/2026-08-07-tablero-de-carrera-smoke.md`) encontró y corrigió un
defecto que ningún test de cableado podía ver —la nota de vida partía a mitad
de frase pegada al sha y se leía como una oración sobre la base— y dejó anotado
un falso positivo del operador que se repetirá si no se nombra: medir una
superficie nueva con un binario de `target/release` viejo se parece exactamente
a un bug de la superficie nueva.

### `vincular-suscripciones` — abierta el 2026-08-08

El pedido fundacional en palabras del mantenedor: «si tengo dos suscripciones
de Claude y tres de Codex, debo poder vincular esas suscripciones en la
flota». El motor existe y está probado (`flota-multiproveedor`: perfiles con
overlay de contexto, resolución sin degradar, carrera de dos suscripciones
verificada e2e; el overlay llega al CLI real a través de los adaptadores
propios, que lanzan sin limpiar el entorno), y la verificación empírica del
2026-08-08 sobre la máquina de desarrollo lo cerró: `CODEX_HOME=<dir vacío>
codex login status` responde «Not logged in» mientras el contexto por defecto
responde «Logged in using ChatGPT». Lo que no existe es el **vínculo como
experiencia**: ninguna superficie crea, deshace ni compone una suscripción —
solo TOML a mano, sabiendo por cuenta propia qué variable redirige cada
proveedor. La change: la variable de contexto y el gesto de login como
**datos del registro** (jamás un match por proveedor en el código),
`subscription/link`/`unlink` con persistencia en un archivo propiedad del
daemon cargado antes que lo manual (lo escrito a mano gana), el login
**compuesto y jamás ejecutado** (§2: el contexto se crea vacío, no se lee,
no se borra al desvincular), el duplicado de contexto advertido, y las tres
superficies (§4). Cuatro deltas ADDED-only (`fleet-catalog`, `cli-contract`,
`gui-shell`, `tui-shell`), diecinueve escenarios, cero dependencias nuevas.
Fuera de alcance por escrito: ejecutar o verificar logins, balanceo entre
suscripciones, y migrar los perfiles manuales existentes.

### `avisos-de-escritorio` — abierta el 2026-07-31 (comparativo de mercado)

Nace del comparativo conducido por el mantenedor contra Orca (Stably AI, 13
capturas) y de una verificación en código que lo confirmó: `espera-humana`
enseñó al daemon a esperar al humano, `sesion-esperando` y
`eventos-para-tardios` hicieron la espera visible y suscribible — y ninguna
superficie avisa al escritorio; los únicos «notification» del cliente son
mensajes JSON-RPC del bridge, y la TUI no tiene campana. Un permiso puede
esperar minutos con la app detrás de otra ventana, con el turno detenido en
silencio. La change: capability nueva `attention-notices` — aviso local del SO
desde los clientes (permiso esperando, gate esperando, sesión terminada o
fallida), con regla de foco (al frente, la bandeja de hoy; sin foco, el
aviso), contenido sin texto del turno, permiso del SO pedido en el primer
aviso real y estado denegado con remedio; GUI vía el plugin oficial de
notificaciones de Tauri (el precedente §10 del plugin de diálogo) con «probar
aviso» en Ajustes; TUI con campana/OSC opt-in. El daemon no gana transporte
alguno: los eventos ya existen, esto es el último metro — y el aviso remoto
autohospedado sigue siendo fase 3, jamás del daemon.

### `primer-arranque-del-home` — abierta el 2026-07-31, vía rápida

El residuo chico del mismo comparativo, verificado en `Home.svelte`: el rehúso
de agente ya guía ejemplarmente (candidatos con estado y remedio dentro del
chip), pero solo después de fallar; la cara del chip dice «agente del
proyecto» en tono neutro aunque no haya un solo lanzable — el chip de
proyecto, al lado, sí advierte cuando falta la carpeta — y la pista del menú
vacío nombra la Flota sin abrirla. Deltas ADDED-only sobre `gui-shell`: el
chip advierte proactivamente, el menú vacío gana el gesto a la vista de
flota, y la primera llegada con flota poblada muestra «N detectados» una sola
vez. El wizard modal y la checklist persistente del comparativo se rechazan
por escrito: el compositor con chips es el asistente, en su sitio.

**Hallazgos del comparativo anotados, no colados** (rumbo de estructura):
setup scripts al crear worktrees → su casa es `hooks-eventos` cuando llegue;
issues de GitHub/Jira/Linear como tareas vía CLIs oficiales (`gh`/`glab` con
su propia auth, patrón compatible con §2) → funciones de equipo, fase 3;
automations recurrentes e importar una carpeta con N repos como grupo →
futuro con evidencia; y para el anuncio, un contraste que es mensaje y no
change: ese producto embarca «Yolo / Dangerously skip permissions» marcado
por defecto — exactamente lo que el deny-by-default constitucional de Meltemi
existe para no ser.

### `artefactos-de-cada-push` — abierta el 2026-08-05, vía rápida

Nace de una petición del mantenedor: «me gustaría que esto sea automático con
cada push (para eso tenemos CICD)» — quiere probar el build del día, y en
particular el DMG de macOS que su máquina Windows no construye. Lo que **no**
puede automatizarse es la release firmada: `procedencia-de-release` fijó que la
firma minisign ocurre fuera de CI porque es el único paso que una cuenta de
GitHub comprometida no puede completar, y con immutable releases activado firmar
precede a publicar sin orden alternativo. La change da el escalón intermedio:
`.github/workflows/build.yml`, un job por plataforma en cada push a `main`, que
produce los mismos artefactos que el camino de tag y los sube como artefactos de
la ejecución — sin release, sin versión, sin firma y sin insinuar ninguna de las
tres. `release.yml` no cambia ni una línea, y esa es la decisión central: hacerlo
allí habría hecho que cada publicación del sitio esperara un build de Tauri de
tres plataformas (`publish-site` declara `needs` sobre los jobs de empaquetado) y
habría duplicado los gates que `ci.yml` ya corre sobre el mismo commit. Los tres
presupuestos de tamaño gatean también esta ruta, con test que exige el mismo
valor en los dos archivos; el artefacto se llama `meltemi-unsigned-<SO>-<sha>` y
caduca a los 7 días; el aviso de «sin firmar» viaja al resumen del run y dentro
del artefacto. Costo dicho sin maquillar —tres jobs caros por push— con dos
diales declarados para bajarlo: el bloque `on:` y la matriz de plataformas.
Cuatro deltas ADDED sobre `release-distribution`, cero dependencias nuevas.

### `cromo-que-no-estorba` — abierta el 2026-08-08, vía rápida

Tres frases del mantenedor sobre una captura: el scroll del panel derecho, los
mensajes que «se quedan pegados» y la paleta que no cierra al hacer clic fuera.
El cajón declaraba `overflow: auto` en los dos ejes con 268 px fijos, así que
una ruta larga sacaba barra horizontal; ahora es un solo eje y el contenido se
parte. Los avisos no caducaban **ninguno**, lo cual es obligatorio solo para lo
que la spec nombra —un vencimiento «no se descarta en silencio»— y no para un
«enlace creado»: ahora los informativos se retiran a los 6 s y los de
advertencia y error **no tienen reloj**, probado por ausencia avanzando cien
veces el plazo; apuntar o enfocar detiene la cuenta y salir la reinicia. Y el
velo de la paleta no tenía manejador de clic mientras el del conmutador sí: en
vez de arreglar ese caso, un barrido exige cierre en **todos** los velos y
encontró otros cuatro. La política de avisos salió a su propio módulo puro,
porque el store no es importable bajo `node` y una obligación de spec merece un
test que corra. Nota: `docs/qa/2026-08-08-cromo-que-no-estorba-smoke.md`.

### `flota-por-suscripcion` — abierta el 2026-08-09, vía rápida

El mantenedor dijo que la flota «no permite configurar varios agentes de un
tipo». Lo que veía era cierto; lo que suponía, no: enlazar varias ya funcionaba
y el smoke de la change anterior lo había fotografiado sin buscarlo. Lo que
faltaba era la **lectura**: una fila de perfil nunca decía de qué agente era,
aunque el contrato lleva `underlyingAgent` desde `flota-multiproveedor` y el CLI
ya imprime `(profile → claude-code)`. La superficie que lo decía era la
terminal; la que lo escondía, la gráfica. Ahora cada agente va seguido de sus
suscripciones, cada una diciendo «suscripción de X» **como texto** —la sangría
solo acompaña—, con el recuento en el agente; una suscripción cuyo agente no
está en el catálogo se lista marcada, con el id que declara, en vez de
desaparecer. De paso se cumple un requisito vivo de `integration-levels` que la
superficie incumplía: el nivel se dice «declarado» o «verificado» con palabras y
no con un `✓` a secas, que es justo lo que esa spec prohíbe. Nota:
`docs/qa/2026-08-09-flota-por-suscripcion-smoke.md`.

### `pestanas-como-chrome` — abierta el 2026-08-09

«El sistema de tabs quiero que se parezca al de google chrome», con grupos y
botones `<` `>`. La tira envolvía —`flex-wrap: wrap`— y cinco sesiones ya
producían una segunda fila que empujaba el contenido. Ahora es una sola fila:
las pestañas encogen hasta un mínimo legible y solo entonces la tira se
desplaza, con controles que **existen únicamente mientras hay desbordamiento**,
medido con `ResizeObserver`, y que se deshabilitan en su extremo en vez de
esconderse. La pestaña activa se trae a la vista con `nearest`, sin lo cual las
flechas del patrón ARIA mueven el foco fuera de pantalla. Lo que **no** se copia
de Chrome es su respuesta al desbordamiento —Chrome encoge hasta el favicon y
nunca desplaza—: el mantenedor pidió las flechas y la petición manda sobre la
referencia. Los grupos llevan nombre y color; el color identifica y **el nombre
viaja en el nombre accesible de cada pestaña**, plegar declara cuántas guarda y
no cierra ninguna, y si la activa estaba dentro, la actividad sale a una pestaña
visible. Se agrupa por menú y no arrastrando, porque el equivalente accesible
del arrastre es otra conversación. Nota:
`docs/qa/2026-08-09-pestanas-como-chrome-smoke.md`.

> **Deuda declarada de esta tanda**: las pestañas y sus grupos **no se
> persisten** entre arranques (`sidebar-ajustable-y-pestanas` D7, heredado); si
> se persisten, la primera tarea de esa change es medir el arranque con ocho
> pestañas. Sigue pendiente la **auditoría de intuitividad** que el mantenedor
> pidió como barrido completo de la aplicación.

### `sidebar-ajustable-y-pestanas` — abierta el 2026-08-08

Nace de una captura del mantenedor y tres frases: la línea divisoria entre las
opciones y los proyectos «debe poder moverse», el scroll de la sección de
proyectos «se ve fea», y las sesiones «deben ser tabs, de esa forma puedo tener
varios tabs abiertos». Tres cosas de naturaleza distinta y **una sola change**,
por una razón medible: el divisor **fabrica** la segunda barra de scroll —darle
alto a `nav` obliga a `overflow-y: auto` y la columna de 216 px estrena otra
barra clásica—, así que entregar la primera sin la segunda duplica el defecto
del que trata la segunda.

Lo que la investigación encontró antes de escribir nada: la «línea» no era un
elemento sino un `border-top` del encabezado PROYECTOS; no existía un solo
`pointerdown` en toda la superficie de escritorio; no había **ningún** estilo de
barra de desplazamiento en el repositorio, así que lo que se veía era el
defecto de plataforma; y las sesiones se pisaban porque el shell guardaba **un**
identificador que cuatro sitios sobrescribían —de ahí que un borrador escrito en
una sesión sobreviviera dentro de otra.

Dos decisiones cargan el peso. Las pestañas van **contenidas en la vista
Sesiones** y no sobre la región principal, porque las dos formas rivales
modifican el mismo encabezado de requisito que `lanzador-conversacional` ya
modifica en un delta sin archivar bloqueado en la firma del mantenedor: solo un
delta ADDED puede fusionarse sin pisar texto ajeno. Y los paneles se **montan y
se ocultan** en vez de desmontarse, que es lo que mantiene vivos transcript,
búsqueda y borrador; el lado del daemon es seguro porque su conjunto de
vigilancias es un `HashSet` por conexión sin tope, leído y citado, no supuesto.

El smoke conducido sobre el binario encontró cuatro cosas que el código fuente
no podía enseñar, y todas se corrigieron: que `scrollbar-width` **no** hereda;
que la barra angosta conserva sus botones de flecha en WebView2 y ninguna
propiedad estándar los quita; que las dos familias de estilo de barra son
**excluyentes** en Chromium —de ahí las dos ramas `@supports`—; y que el anillo
de foco del separador parecía un campo de texto vacío. Nota completa en
`docs/qa/2026-08-08-sidebar-ajustable-y-pestanas-smoke.md`.

Veinte escenarios, tres requisitos ADDED sobre `gui-shell`, cero dependencias,
cero métodos del contrato, un solo archivo Rust de producto tocado.

**Seguimientos nombrados**: (1) adoptar `TabStrip` en el Editor, cuya tira
propia no sigue el patrón ARIA; (2) un tope de líneas del transcript en la GUI
—la TUI ya tiene el suyo— si ocho transcripts montados lo piden; (3) persistir
el conjunto de pestañas al arrancar, cuya primera tarea sería medir el arranque
con ocho.

> **Gobernanza de alcance** (changes `enmienda-edicion-movil` y `enmienda-agent-boss`): la edición in situ de Fase 2 está acotada por la cerca de la spec `edit-surface`; el compañero móvil de Fase 3 (`companero-movil`, meltemi.md §10) es el puesto remoto del **Agent Boss** — monitorear/aprobar/revisar/dirigir, sin autoría, túnel SSH exclusivamente, aviso de espera opt-in autohospedado — por las specs `mobile-companion` y `remote-access`.

### Prerrequisitos de daemon del Agent Boss (antes de `companero-movil`, sirven a TUI/GUI hoy)

| Change | Alcance |
|---|---|
| ✅ `espera-humana` (2026-07-26) | Política de espera configurable (`[permissions]`: `wait`/`implement-wait`/`no-client-grace`): los flujos interactivos esperan al humano mientras haya cliente conectado; el fallo del push ya no resuelve la petición (la cola es la única fuente); sin clientes, denegación constitucional auditada tras la gracia |
| ✅ `sesion-esperando` (2026-07-26) | El daemon fija `waiting_permission` mientras una petición espera decisión (contado, para peticiones simultáneas) y `change/list` declara `gatePending` con el artefacto que espera, leído del estado del ciclo |
| ✅ `eventos-para-tardios` (2026-07-26) | Hub de eventos en el daemon y `session/watch`: la conexión que inició la sesión sigue recibiendo su stream, y cualquier otra lo pide por sesión. **Diferido con razón escrita**: las formas asíncronas de `sdd/gate`, `sdd/review-decide` y `worktree/dispatch` tocan la promesa ratificada de «pasos scriptables» y el stream las mitiga; entran como change propia si la evidencia de uso lo pide |

## Fase 3 — se planifica al cerrar Fase 2

`companero-movil` (gobernada por las specs `mobile-companion` y `remote-access`: el puesto remoto del Agent Boss, cuatro verbos, sin autoría, túnel SSH exclusivamente, aviso de espera opt-in; su design debe resolver la frontera Windows del túnel — named pipe no reenviable por OpenSSH — sin abrir jamás un puerto de red del daemon) · funciones de equipo y organización (meltemi.md §3).

## Namespaces del proyecto

- **Organización** (identidad paraguas): `askenaz-dev` — [GitHub](https://github.com/askenaz-dev), [npm](https://www.npmjs.com/org/askenaz-dev). Paquetes npm futuros (SDK, fase 2): `@askenaz-dev/<pkg>`.
- **Producto**: `meltemi` — dominio `meltemi.dev`, binarios `meltemid`/`meltemi`, crates `meltemi`/`meltemid`/`meltemi-proto` (crates.io es namespace plano, sin org).
- Repositorio: `github.com/askenaz-dev/meltemi`.

## Pendientes exclusivos del arquitecto (ningún agente debe hacerlos)

1. ✅ **Ratificar** meltemi.md, `constitution.md` y `rumbo/` — hecho 2026-07-11.
1b. ⬜ **Ratificar las enmiendas encadenadas de meltemi.md**: v1.1→v1.2 (`formato-artefactos-meltemi`), v1.2→v1.3 (`enmienda-edicion-movil`) y v1.3→v1.4 (`enmienda-agent-boss`). Todas las versiones están aplicadas al documento; falta la firma del mantenedor fundador (`method-bootstrap`: la herramienta no se auto-ratifica).
1c. ⬜ **Ratificar la enmienda de la promesa de producto** — v1.4→v1.5 de meltemi.md y el párrafo «Qué es Meltemi» de `.meltemi/rumbo/product.md` (`lanzador-conversacional`, design D5, aplicada el 2026-07-31): toda sesión corre gobernada y la disciplina spec-first deja de presentarse como condición previa. A diferencia de las anteriores, **esta ratificación es gate de archivo**: sin la firma, `lanzador-conversacional` no archiva. Ambos textos están aplicados y marcados como pendientes en sus propios documentos. Al ratificar quedan dos cosas por hacer, en este orden: correr `meltemi project` (el bloque proyectado compila el rumbo) y refrescar la frase pública, que sigue diciendo lo viejo en `README.md`, `LEEME.md`, `site/index.html` y `site/es/index.html` — deliberadamente sin tocar, porque anunciar al mundo una promesa sin firmar sería exactamente lo que la enmienda evita. Con ese refresco va también la fila de `docs/sesion-libre.md` en el índice de documentación de los dos READMEs: enlazar «trabajo sin spec» bajo la promesa vieja publicaría la contradicción en vez de resolverla.
2. **Registrar**: ✅ org GitHub `askenaz-dev` · ✅ org npm `askenaz-dev` · 🟡 dominio `meltemi.dev` (comprado el 2026-07-25; falta apuntar el DNS a Pages) · ⬜ crates `meltemi`, `meltemid`, `meltemi-proto` en crates.io (verificados LIBRES el 2026-07-11 — reservar con un `cargo publish` placeholder o en el primer release para evitar squatting).
3. **Decidir** el mecanismo de firma del CLA cuando llegue #21.
