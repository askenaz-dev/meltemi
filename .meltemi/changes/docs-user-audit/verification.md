# Auditoría

Estado: auditoría editorial y validación local completas el 6 de noviembre de 2026.
La CI de revisión en las tres plataformas y la publicación del portal se comprueban por separado.

| Guía | Fuente que se contrasta | Corrección / límite |
| --- | --- | --- |
| quickstart | `tui/src/cli.rs`, `tui/tests/docs.rs`, UI Home y Settings | Instalación desde release; slug real `add-a-dark-mode-toggle-to-the-settings-page`; ciclo planificar → implementar → verificar → previsualizar archivo → confirmar archivo. No se pide compilar ni gestionar credenciales. |
| agentes | `core/meltemid/data/fleet-registry.toml`, `fleet.rs`, configuración y adaptadores, tests `agents_guide` | Niveles y controles reales por agente; nivel 3 no pilotable; `ready` comprueba binarios, no autenticación. Referencias de entorno completas `${CLAUDE_PERSONAL_DIR}`/`${CLAUDE_WORK_DIR}` en vez de interpolaciones parciales no admitidas. Se retira la matriz histórica de conformidad. |
| sesion-libre | `core/meltemid/src/{free_session,checkpoints,session}.rs`, CLI y compositor | Trabajo en la raíz, checkpoint `refs/meltemi/checkpoints/free`, restauración por ruta mediante Git y sin reversión completa del árbol en Meltemi. El cliente CLI espera el turno y termina; el escritorio puede continuar. Autonomía, modo de trabajo y permisos se explican por separado. |
| metodo-sdd | `.meltemi/specs`, `core/meltemid/src/sdd_flow.rs`, verbos CLI | Se retira el bootstrap OpenSpec obsoleto. Artefactos actuales en `.meltemi/`; `archive` sólo previsualiza, el argumento final `confirm` aplica. No ratifica enmiendas. |
| harness | `core/meltemid/src/harness.rs`, proyección, vistas Fleet | Ruta real `<config-dir>/harness`, precedencia de los cuatro ámbitos, sustitución completa de una regla y consentimiento para destinos de usuario. Las instrucciones no conceden permisos. No se publican narrativa FDH ni detalles internos del desarrollo. |
| local-models | perfiles, `session_config.rs`, adaptadores, tests `local_models_guide` | Se conservan tres recetas y sus ejemplos; comprobación histórica del formato fechada y acotada a OpenCode 1.18.32, sin afirmar ejecución de inferencia. Se retiran conteos de pruebas y planes del motor propio. |
| acceso-remoto | `core/meltemi-client/src/{transport,machines,notices}.rs`, `core/meltemid/src/identity.rs`, bridge/tunnel, UI Settings | SSH y puente local reales; Windows conserva límites de comprobación con `sshd` real. Identidad de malla de sólo lectura y prueba de campo pendiente contra v1.102.5. Máquinas sin agregación ni material de claves. Android existe en fuente, sin APK de release ni distribución iOS. Avisos de espera existentes en Herramientas; variables y tiempos reales. Se retiran fases, licencias investigadas y protocolos de trabajo del equipo. |
| accesibilidad | formato CLI, shell y CSS del escritorio | Salida sencilla/JSON limitada a comandos que la admiten, `MELTEMI_ASCII`, `MELTEMI_BELL` y movimiento reducido. No se promete compatibilidad de lector de pantalla que no se haya medido. |
| plataformas | `core/meltemi-client/src/paths.rs`, empaquetado y transporte | Rutas e instaladores actuales; se retiran presupuestos de tamaño y planes de RPM. Encabezado Windows utilizable como ancla, sin sintaxis de ID no soportada. |
| referencia-cli | `meltemi::cli::reference()` y prueba de frescura | La canónica permanece idéntica a `928e23c`; wrapper español completo con gramática real inglesa sin traducir comandos ni salida. |
| release | `.github/workflows/release.yml`, manifiestos e instaladores, tests `release` | Descarga, matriz de archivos, checksums, atestación y verificación offline; alcance agregado de la atestación sin prometer firmas de plataforma. Procedimiento y controles del publicador pasan a `docs/operations/release-pipeline.md`, fuera de las guías públicas. |
| versionado | asignación automática, manifiesto y tests `release` | Significado de versión de producto, fuente y compatibilidad; reserva de tag no equivale a publicación; recuperación inmutable. Se retiran algoritmos del mantenedor y fases futuras. |
| faq | doctor, permisos, flota, transporte y `desktop/ui/src/lib/messages.ts` | Diagnóstico práctico; Settings → About → Diagnosis / Ajustes → Información → Diagnóstico. El registro y los permisos se acotan a lo que el agente entrega; los proveedores y MCP propios del agente no se presentan como tráfico gestionado por Meltemi. |

## Inventario y traducciones

Trece parejas completas: quickstart, agentes, sesion-libre, metodo-sdd, harness, local-models,
acceso-remoto, accesibilidad, plataformas, referencia-cli, release, versionado y faq.
Con los índices de producto son 28 páginas genuinas. Se conservan la raíz mixta y todos los
nombres: acceso remoto canónico en español y su traducción en `docs/en/`; los otros doce
originales ingleses tienen traducción en `docs/es/`.

Las parejas conservan la estructura completa de encabezados y 47 ejemplos idénticos, incluidos
etiquetas de lenguaje, comandos, flags, JSON/YAML y salida. Las etiquetas visibles del escritorio
se contrastan con `messages.ts`; los fragmentos de enlaces se ajustan al idioma del destino.
No se exige traducir investigación, QA, arquitectura, operaciones ni otros materiales internos.

Las pruebas nuevas comprueban el inventario de guías públicas de la raíz y los locales explícitos:
una guía pública futura requiere su pareja e inclusión en los dos índices. La referencia CLI
continúa verificada contra el generador, sin edición manual.

## Acuerdos de método preservados

`docs-en-el-sitio` propone una enmienda de §11 con ratificación pendiente. Esta auditoría no
declara inglés como lengua fuente ni cambia el estado de esa propuesta. Tampoco modifica las
enmiendas pendientes sobre sesiones libres, edición o móvil: describe el comportamiento observado.

## Validación

En Windows, sobre el contenido del commit `8787409`, se ejecutaron:

- `cargo test --offline -p meltemi --test docs --no-fail-fast`: 25/25.
- `cargo test --offline -p meltemid --test release --test agents_guide --test local_models_guide --no-fail-fast`:
  32 + 7 + 5 = 44/44. Total focalizado: 69/69.
- `cargo clippy --offline -p meltemi --test docs -- -D warnings`: verde.
- `cargo clippy --offline -p meltemid --test release -- -D warnings`: verde.
- `cargo fmt --all -- --check` y `git diff --check`: verdes.
- Inventario independiente: 13 parejas, 28 páginas con índices, 47 ejemplos exactos,
  referencia CLI generada sin cambios y cero fallos.
- Revisión independiente de las cuatro páginas de remoto/sesión libre, además de traducciones:
  sin hallazgos accionables. No se editaron fuentes durante esa revisión.

Las pruebas ejecutan `meltemi version` y `meltemi help` del binario real; comprueban frescura de la
referencia, enlaces, gramática, perfiles y recetas parseables, catálogo, controles y workflows.
No ejecutan las recetas con agentes reales. Las comprobaciones de controles internos de entrega se
conservan, apuntando al documento de operaciones en lugar de exigir narrativa interna en la guía
del instalador.

El ensamblado coordinado de Meltemi Web encontró 28 páginas IDE genuinas, ningún aviso de idioma
y todos los enlaces y fragmentos IDE válidos. El conversor Web es un cambio acompañante: reconoce
`docs/en/`, `docs/es/`, la raíz de idioma mixto y el destino del mismo idioma. Su CI y despliegue se
registran en ese repositorio.

Evidencia local conservada fuera del repositorio:

- `C:/meltemi/artifacts/ide-docs-user-audit/guide-inventory.json`.
- `C:/meltemi/artifacts/ide-docs-user-audit/tui-docs-tests.log` y `core-docs-tests.log`.
- `C:/meltemi/artifacts/ide-docs-user-audit/clippy-docs.log`, `clippy-release.log` y `fmt.log`.
- `C:/meltemi/artifacts/docs-user-audit/render-check.json` para el ensamblado del portal.

## Límites de esta auditoría

No se ejecutó una suite local de producto completa ni pruebas nuevas en macOS/Linux; la CI de
revisión multiplataforma sigue siendo gate de merge. No se accedió a credenciales, no se
autenticaron agentes, no se invocaron modelos ni se iniciaron servidores de inferencia. Tampoco
se midieron SSH de campo, clientes de malla, instalaciones móviles ni lectores de pantalla.
Los límites y verificaciones pendientes de esas guías se conservan como pendientes. Estos commits
no publican, no despliegan y no alteran runtime, dependencias ni enmiendas fundacionales.
