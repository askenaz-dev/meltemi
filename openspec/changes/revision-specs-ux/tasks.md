## 1. Motor: diagnósticos semánticos

- [ ] 1.1 Detector de duplicados (nombre normalizado post-aplicación; fijar normalización) _(Req: Diagnóstico de requisito duplicado)_
- [ ] 1.2 Detector de MODIFIED sin efecto _(Req: Diagnóstico de modificación sin efecto)_
- [ ] 1.3 Detector de referencias colgantes «Requirement: X» _(Req: Diagnóstico de referencia colgante)_

## 2. Render y checklist

- [ ] 2.1 Diff semántico de deltas por requisito/escenario (MODIFIED alineado por nombre; palabra+glifo; ASCII/NO_COLOR) _(Req: Diff de deltas por requisito)_
- [ ] 2.2 Checklist persistente en la change (estados, reanudación, cierre exige decisión total; diagnósticos anclados) _(Req: Checklist de revisión persistente)_

## 3. Bucle con el ciclo

- [ ] 3.1 Comentario→instrucción de reelaboración citando el requisito; reapertura del gate de specs; vínculo registrado _(Req: Comentario convertido en instrucción)_

## 4. Superficies

- [ ] 4.1 TUI: flujo de review en la vista Proyecto (lista→detalle→decisión)
- [ ] 4.2 CLI: `review` operativo por pasos + `--json`; gramática y mapeo del delta acumulativo _(Modified: cli-contract)_

## 5. Tests y calidad

- [ ] 5.1 Unit: los 3 detectores (positivos y negativos exactos; cero falsos positivos por diseño), normalización
- [ ] 5.2 E2e: review completa sobre una change fixture (aprobar/comentar/rechazar; reanudación; cierre bloqueado con pendientes); comentario reabre gate con mock-agent
- [ ] 5.3 Render con TestBackend: diff MODIFIED alineado legible en ASCII/monocromo
- [ ] 5.4 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
