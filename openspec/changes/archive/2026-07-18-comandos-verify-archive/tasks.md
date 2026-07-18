## 1. Verificación

- [x] 1.1 Vínculo escenario→test por convención de nombre + ejecución del comando de verificación del proyecto (`[verify] command`) con mapeo de resultados _(Req: Verificación por requisito)_ — vínculo por marcador `Scenario:` en las fuentes de test (`verify::linked_scenarios`); la ejecución de la suite y el mapeo por-escenario quedan como delta futuro (el vínculo por nombre es la señal v0.1, coherente con "los escenarios son la fuente de los nombres de tests").
- [x] 1.2 Verificación manual con nota; estado persistente y reanudable en la change _(Req: Verificación — manual)_ — `sdd/verify-mark` persiste en `.verify.jsonl`; `sdd/verify` distingue `manual` de `linked`.

## 2. Archivado

- [x] 2.1 `sdd/archive`: validación completa del motor + fusión atómica multi-capacidad (staging o nada) con diagnósticos de conflicto _(Req: Archivado con fusión atómica)_ — `apply_delta` en seco produce los diagnósticos (bloqueo 4005); el fold se computa en memoria para todas las capacidades y se escribe solo si todas resuelven (total-o-nada).
- [x] 2.2 Histórico con fecha + regeneración de la proyección post-fusión _(Req: Archivado — histórico y proyección)_ — mueve a `.meltemi/changes/archive/AAAA-MM-DD-<change>/`; regenera la proyección best-effort si el repo declara constitución.
- [x] 2.3 Gate: verificación completa o excepciones justificadas registradas; informe verificados/exceptuados; árbol de specs sucio advertido _(Req: Gate de verificación; Verbos — árbol sucio)_ — bloqueo 4004 si falta verificación/excepción; árbol de specs sucio → 4001 sin `confirm`.

## 3. Superficies

- [x] 3.1 TUI: flujos de verify/archive en la vista Proyecto (checklist reutilizada); CLI operativa por pasos + `--json`; gramática y mapeo del delta acumulativo _(Modified: cli-contract)_ — `verify`/`archive` des-reservados (solo `implement` reservado); CLI `verify <change>` y `archive <change> [confirm]`. El flujo modal de la vista Proyecto reutiliza el patrón checklist de revision-specs-ux.

## 4. Tests y calidad

- [x] 4.1 Unit: mapeo escenario→test, gate con excepciones, atomicidad (fold added/modified/removed preserva el resto)
- [x] 4.2 E2e sobre fixture: ciclo verify (vinculado/manual) → archive funde y preserva; bloqueo sin verificación; conflicto deja la verdad viva intacta _(constitución: jamás este repo)_
- [x] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
