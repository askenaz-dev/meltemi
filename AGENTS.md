# Meltemi — contexto para agentes

Proyección manual de `.meltemi/constitution.md` y `.meltemi/rumbo/` (dogfooding de meltemi.md §2.8 hasta que exista la proyección automática). Si editas constitución o rumbo, actualiza este archivo.

## Qué es este proyecto

Meltemi: plano de control spec-driven open source (Apache 2.0) que orquesta agentes de codificación externos vía ACP. Daemon headless `meltemid` (Rust) + TUI `meltemi` + GUI Tauri (fase 2). Documento fundacional: `meltemi.md` (v1.3 enmendada; ratificación de v1.2/v1.3 pendiente del mantenedor fundador; base v1.0 y constitución/rumbo ratificados 2026-07-11). La edición de código es *utilitaria al servicio del bucle agéntico*, acotada por la spec de gobernanza `edit-surface`; el compañero móvil (fase 3) está acotado por `mobile-companion`. Backlog maestro: `docs/plan-de-cambios.md`.

Workspace Cargo en la raíz: `core/meltemid` (daemon), `core/meltemi-spec` (motor de specs), `core/mock-agent` (agente ACP simulado para e2e), `proto/meltemi-proto` (tipos del contrato), `tui/` (binario `meltemi`: CLI scriptable + TUI). Toolchain pineado en `rust-toolchain.toml` (1.97.0).

## Reglas no negociables (constitución — resumen operativo)

1. **Spec-first**: nada se implementa sin propuesta de cambio aprobada en `openspec/changes/` (método actual; ver bootstrap abajo). Los escenarios de las specs son la definición de "terminado".
2. **Juego limpio**: solo binarios oficiales de agentes con su propia auth. Prohibido leer/almacenar credenciales ajenas o suplantar clientes.
3. **Seguridad**: daemon solo en socket local; deny-by-default sin cliente; sin puertos de red, jamás.
4. **Paridad de núcleo**: ninguna feature del daemon accesible desde una sola superficie.
5. **Calidad**: `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las 3 plataformas antes de merge. Windows es primera clase.
6. **Sin telemetría**: métricas solo locales; cualquier telemetría futura es opt-in y especificada antes de existir.

## Convenciones

- **Idiomas**: artefactos del método en español neutro; código, identificadores, strings del contrato `proto/` y mensajes de commit en inglés.
- **Commits**: atómicos, uno por tarea, con referencia `(<change> <tarea>)`. **Sin trailers de co-autoría.**
- **Dependencias**: mínimas, pineadas, justificadas en el design de su change (auditoría con cargo-deny en CI).
- **Licencia**: Apache-2.0; todo archivo fuente lleva cabecera SPDX (`docs/politica-spdx.md`).
- **Tests e2e**: siempre contra repos fixture temporales, nunca contra la raíz de este repo. En CI se usa `mock-agent`, nunca agentes reales ni red.

## Bootstrap del método (dos etapas)

El desarrollo de Meltemi usa OpenSpec (`openspec/changes/`, comandos `/opsx:*`) hasta que el motor de specs de fase 1 esté operativo; entonces se migrará a `.meltemi/`. La constitución y el rumbo ya viven en `.meltemi/` (formato destino). Detalle: design D9 de `fase-0-fundacion`.

## Referencias

- `meltemi.md` — visión y decisiones D1–D6
- `.meltemi/constitution.md` + `.meltemi/rumbo/{product,tech,structure}.md`
- `docs/plan-de-cambios.md` — backlog ordenado de changes
- `docs/research/integracion-agentes.md` — matriz de integración por agente (interno)
