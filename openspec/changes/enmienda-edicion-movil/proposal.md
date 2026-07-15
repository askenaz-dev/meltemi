# Propuesta: Enmienda — edición utilitaria in situ y compañero móvil

## Why

La exploración del 2026-07-14 detectó dos huecos en la visión antes de que las fases 2 y 3 los hereden: (1) el no-objetivo #1 de `meltemi.md` ("la superficie de código es de *revisión*, no de edición") es más absoluto de lo que el bucle agéntico necesita — obliga a salir del plano de control hasta para un retoque trivial, rompiendo la experiencia que el producto promete; y (2) el compañero móvil de fase 3 está infra-especificado ("supervisar la flota"), sin límites que impidan que derive en un editor de bolsillo o en una violación del principio de acceso remoto. Conviene enmendar ahora, con la ingeniería de fase 1 recién arrancando, cuando el costo es solo documental. Al ser cambios acotados de documentos, se usa la **vía rápida** (`fast-forward`): todos los artefactos de una vez.

## What Changes

- **No-objetivo #1 (`meltemi.md` §3)**: se reemplaza la cláusula absoluta "revisión, no edición" por **edición utilitaria en contexto** al servicio del bucle agéntico (revisar → retocar → dirigir), gobernada por una **cerca explícita** de lo que la superficie de edición incluye y excluye para siempre. "No es un editor de propósito general ni un IDE clásico" **se mantiene intacto**.
- **Principio rector nuevo**: "Meltemi optimiza para que salir sea **infrecuente, no imposible**" — el criterio objetivo contra el que se juzga toda futura petición de features de edición.
- **"Abrir con…" de primera clase**: deep-link desde diff y árbol de proyecto al editor que el usuario ya usa, con archivo:línea exacto (en la TUI, suspensión a `$EDITOR` y retorno).
- **Trazabilidad de ediciones humanas**: toda edición in situ se materializa como capacidad del daemon (escritura en worktree + evento `human_edit` en el log de sesión JSONL), preservando la constitución §8 (cada línea rastreable).
- **Compañero móvil (fase 3) precisado**: superficie **compañera reducida** limitada a **monitorear + aprobar + dirigir**; sin edición; acceso remoto únicamente vía túnel SSH (constitución §3); exenta de paridad plena bajo la regla de subconjunto: todo lo que el móvil hace, la TUI y la GUI también lo hacen (el espíritu de la constitución §4 se preserva — ninguna capacidad del daemon es exclusiva de una superficie).
- **Roadmap**: fase 2 (GUI) incorpora la edición utilitaria in situ con inteligencia LSP; fase 3 adopta el alcance preciso del compañero móvil.
- **Rumbo de producto** (`.meltemi/rumbo/product.md`): se matiza "Qué NO es" para reflejar la edición utilitaria sin renunciar a "ni un editor de propósito general".
- **Sin cambios en la constitución**: §3 (acceso remoto solo vía túnel SSH) y §4 (paridad de núcleo) ya cubren el compañero móvil tal como se precisa; esta enmienda lo hace explícito en vez de modificarlos.
- **Versión**: `meltemi.md` pasa de v1.2 a **v1.3**, con nota de enmienda; la ratificación corresponde al mantenedor fundador (no se auto-ratifica, per `method-bootstrap`).

## Capabilities

### New Capabilities

- `edit-surface`: alcance de la superficie de edición de código — la cerca dentro/fuera de la edición utilitaria, el principio "infrecuente, no imposible", el deep-link "Abrir con…" y el requisito de trazabilidad `human_edit`. Como `method-bootstrap`, es una spec de gobernanza de alcance: fija los requisitos que las changes de implementación de fase 2 deberán satisfacer.
- `mobile-companion`: alcance de la superficie móvil — limitada a monitorear/aprobar/dirigir, sin edición, acceso solo vía túnel SSH, y la regla de subconjunto respecto de TUI/GUI. Gobierna la change de fase 3 que la implemente.

### Modified Capabilities

<!-- Ninguna. No cambian requisitos de las specs existentes (`method-bootstrap`, `daemon-lifecycle`, `acp-session`, `cli-contract`, `spec-engine`, `spec-merge`, `artifact-format`, `propose-flow`). -->

## Impact

- **Documentos**: `meltemi.md` (§3 no-objetivos, §6 funcionalidades, §10 fases 2 y 3, cabecera de versión), `.meltemi/rumbo/product.md` (sección "Qué NO es"), `AGENTS.md` (proyección manual del rumbo, per su propia regla de actualización), `docs/plan-de-cambios.md` (anotar el alcance añadido a las changes de fase 2/3).
- **Código**: ninguno. No toca `core/`, `proto/` ni los tests.
- **Gobernanza**: enmienda a documentos ratificados — requiere **aprobación del mantenedor fundador** antes de aplicarse y deja la v1.3 pendiente de ratificación (`method-bootstrap`). Nota: la ratificación de v1.2 sigue pendiente; esta enmienda se encadena sin sustituirla.
- **Fuera de alcance de esta change**: la implementación de la edición in situ y del deep-link (changes de GUI de fase 2 y TUI correspondientes); la app móvil (fase 3); la **política de concurrencia humano↔agente** sobre el mismo worktree (decisión de design delegada explícitamente a la change de GUI de fase 2); la elección del componente de edición embebido; el mecanismo de notificaciones del compañero móvil (pregunta abierta para la change de fase 3).
