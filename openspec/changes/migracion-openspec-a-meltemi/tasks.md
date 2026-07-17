## 1. Migración verificada

- [ ] 1.1 Migrar specs vivas a `.meltemi/specs/` con comparación de modelos (requisitos+escenarios idénticos; diferencia aborta) _(Req: Excepción interina — verdad viva idéntica)_
- [ ] 1.2 Migrar histórico a `.meltemi/changes/archive/` preservando fechas y contenido _(Req: Excepción interina — histórico)_
- [ ] 1.3 Paridad final invertida: la verdad viva migrada revalida contra los archivados migrados (test)

## 2. Corte del método

- [ ] 2.1 Retirar los flujos de la herramienta prestada de la configuración del repo; barrido de referencias con gate en CI _(Req: El método es su propio producto)_
- [ ] 2.2 Regenerar la proyección (`AGENTS.md` refleja el método nuevo); anotar el cierre en el plan maestro
- [ ] 2.3 Primera change post-migración creada con los comandos de Meltemi como verificación de humo _(Req: Dogfooding definitivo)_

## 3. Calidad

- [ ] 3.1 Todo en rama con verificación por paso; dogfood/clippy/fmt/tests verdes antes del merge; confirmación del mantenedor para el retiro de `openspec/`
