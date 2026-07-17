## 1. Verificación

- [ ] 1.1 Vínculo escenario→test por convención de nombre + ejecución del comando de verificación del proyecto (`[verify] command`) con mapeo de resultados _(Req: Verificación por requisito)_
- [ ] 1.2 Verificación manual con nota; estado persistente y reanudable en la change _(Req: Verificación — manual)_

## 2. Archivado

- [ ] 2.1 `sdd/archive`: validación completa del motor + fusión atómica multi-capacidad (staging o nada) con diagnósticos de conflicto _(Req: Archivado con fusión atómica)_
- [ ] 2.2 Histórico con fecha + regeneración de la proyección post-fusión _(Req: Archivado — histórico y proyección)_
- [ ] 2.3 Gate: verificación completa o excepciones justificadas registradas; informe verificados/exceptuados; árbol de specs sucio advertido _(Req: Gate de verificación; Verbos — árbol sucio)_

## 3. Superficies

- [ ] 3.1 TUI: flujos de verify/archive en la vista Proyecto (checklist reutilizada); CLI operativa por pasos + `--json`; gramática y mapeo del delta acumulativo _(Modified: cli-contract)_

## 4. Tests y calidad

- [ ] 4.1 Unit: mapeo escenario→test, gate con excepciones, atomicidad (fallo simulado a mitad → verdad viva intacta)
- [ ] 4.2 E2e sobre fixture: ciclo verify (test verde/rojo/manual) → archive funde y el motor revalida la verdad viva resultante; bloqueo sin verificación
- [ ] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
