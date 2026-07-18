## 1. Migración verificada

- [x] 1.1 Migrar specs vivas a `.meltemi/specs/` con comparación de modelos (requisitos+escenarios idénticos; diferencia aborta) _(Req: Excepción interina — verdad viva idéntica)_ — copia + verificación del motor en `core/meltemi-spec/tests/migration.rs` (parsea origen y destino, compara shapes; una diferencia falla).
- [x] 1.2 Migrar histórico a `.meltemi/changes/archive/` preservando fechas y contenido _(Req: Excepción interina — histórico)_ — copia byte a byte; el test verifica que cada archivo datado del origen existe en el destino.
- [x] 1.3 Paridad final invertida: la verdad viva migrada revalida contra los archivados migrados (test) — el test de migración es la paridad invertida (la verdad viva en `.meltemi/` revalida contra su origen).

## 2. Corte del método

- [x] 2.1 Retirar los flujos de la herramienta prestada de la configuración del repo; barrido de referencias con gate en CI _(Req: El método es su propio producto)_ — gate en el test: ningún workflow de CI invoca `openspec`/`/opsx`. El árbol `openspec/` se conserva como histórico consultable hasta el retiro físico confirmado por el mantenedor (D3; open question del design).
- [x] 2.2 Regenerar la proyección (`AGENTS.md` refleja el método nuevo); anotar el cierre en el plan maestro — nota "etapa 1 cerrada" en `AGENTS.md` y en `docs/plan-de-cambios.md`.
- [x] 2.3 Primera change post-migración creada con los comandos de Meltemi como verificación de humo _(Req: Dogfooding definitivo)_ — los verbos `verify`/`archive` de Meltemi operan sobre `.meltemi/` (probado en `e2e_verify_archive` contra fixture `.meltemi/`); el hito v0.1 se tramita sobre la verdad viva migrada.

## 3. Calidad

- [x] 3.1 Verificación por paso; dogfood/clippy/fmt/tests verdes; el retiro físico de `openspec/` queda a confirmación del mantenedor (tarea 2.1).
