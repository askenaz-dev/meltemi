## 1. Máquina de estados y contrato

- [ ] 1.1 Estado del ciclo persistido en la change (artefacto actual, gate, modo, decisiones) con reanudación tras reinicio _(Req: Ciclo con gates — estado sobrevive)_
- [ ] 1.2 `proto/`: métodos `sdd/constitution`, `sdd/explore`, `sdd/propose`, `sdd/plan` y el flujo de gates (pendiente/decisión, patrón de la cola de permisos) — revalidar nombres al frente de la cola _(design D2)_

## 2. Autoría con el motor como árbitro

- [ ] 2.1 Pipeline por artefacto: redacción por el agente configurado → validación `meltemi-spec` (estructura+EARS+deltas en seco) → gate _(Req: Validación como puerta previa)_
- [ ] 2.2 Bucle comentario→reelaboración sin consumir gate ni reiniciar ciclo _(Req: Ciclo — comentario reelabora)_
- [ ] 2.3 Verbo `constitution` con inicialización de `.meltemi/` mínima y gate final _(Req: Verbo constitution)_
- [ ] 2.4 Verbo `explore` en streaming, garantizado sin escrituras (guardia + test) _(Req: Verbo explore)_
- [ ] 2.5 Verbo `plan`: secuenciación por dependencias con solapamiento de archivos anotado _(Req: Verbo plan)_

## 3. Modo dual

- [ ] 3.1 Criterio de proporcionalidad implementado (elegibilidad fast-forward: sin capacidades nuevas, sin MODIFIED/REMOVED) + forzado humano registrado _(Req: Modo dual)_
- [ ] 3.2 Fast-forward: cuatro artefactos, un gate final _(Req: Modo dual — gate único)_

## 4. Superficies

- [ ] 4.1 TUI: acciones del ciclo en la vista Proyecto; gates como modales; progreso por change _(Req: Superficies del ciclo)_
- [ ] 4.2 CLI: `explore`/`plan`/`constitution` operativos; gates scriptables por pasos sin cuelgues; gramática y mapeo del delta acumulativo _(Modified: cli-contract)_

## 5. Tests y calidad

- [ ] 5.1 Unit: máquina de estados (avance, comentario, aborto, reanudación), criterio de elegibilidad
- [ ] 5.2 E2e con mock-agent guionado como autor: ciclo spec-full completo con gates; artefacto inválido vuelve sin consumir gate; fast-forward elegible; explore sin escrituras (árbol intacto)
- [ ] 5.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
