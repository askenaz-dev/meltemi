# Auditoría

Estado: en curso. No representa verificación de agentes reales ni de instalaciones externas.

| Guía | Fuente que se contrasta | Corrección / límite |
| --- | --- | --- |
| quickstart | `tui/src/cli.rs`, `tui/tests/docs.rs`, UI Home y Settings | Inicio real, comandos, instalación y permisos |
| agentes | `core/meltemid/data/fleet-registry.toml`, adaptadores, tests `agents_guide` | Catálogo vigente y límites efectivos por agente |
| sesion-libre | `session/start`, checkpoints, UI compositor | Raíz real, permiso, checkpoint y dirección de turnos |
| metodo-sdd | `.meltemi/specs`, verbos SDD | Retirar bootstrap OpenSpec obsoleto; no ratificar enmiendas |
| harness | resolución/proyección de reglas y vistas Fleet | Precedencia, consentimiento y fronteras de escritura |
| local-models | perfiles de flota, levers y pruebas del proveedor | Distinguir configuración de una ejecución real no medida |
| acceso-remoto | transporte, bridge/tunnel, máquinas e identidad | SSH, límites de comprobación, móvil y esperas |
| accesibilidad | formato CLI, shell y CSS del escritorio | Vías disponibles y límites sin garantías no verificadas |
| plataformas | `meltemi-client::paths`, empaquetado y transporte | Rutas e instalación actuales; retirar presupuestos/roadmap |
| referencia-cli | `meltemi::cli::reference()` | Canónica sin edición manual; explicación española |
| release | workflow, manifiestos e instaladores | Descarga, comprobación y alcance real de las firmas |
| versionado | asignación automática y manifiesto | Significado de versión/fuente para quien instala |
| faq | doctor, permisos, flota y transporte | Diagnóstico práctico y límites actuales |

## Acuerdos de método preservados

`docs-en-el-sitio` propone una enmienda de §11 con ratificación pendiente. Esta auditoría no
declara inglés como lengua fuente ni cambia el estado de esa propuesta. Tampoco modifica las
enmiendas pendientes sobre sesiones libres, edición o móvil: describe el comportamiento observado.

## Validación

Pendiente.
