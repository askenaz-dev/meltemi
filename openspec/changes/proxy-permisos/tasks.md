## 1. Contrato y motor

- [ ] 1.1 Tipos en `proto/`: regla, `PermissionPendingParams/Result`, `PermissionDecideParams/Result`, notificación `permission/changed`, campo `deniedPermissions` en `ProposeResult` _(Req: Cola de pendientes; Decisión por id; Honestidad del resultado)_
- [ ] 1.2 Motor de reglas en `meltemid`: modelo, carga TOML global+proyecto con diagnósticos de malformadas, evaluación con precedencia proyecto>global y deny>allow _(Req: Motor de reglas; Persistencia de reglas)_
- [ ] 1.3 Integrar la evaluación antes del escalado en el bridge ACP (allow/deny directo; ask → cola) _(Req: Motor de reglas)_

## 2. Cola de pendientes

- [ ] 2.1 Cola de primera clase en el daemon con plazos; handler `permission/pending` y broadcast `permission/changed` _(Req: Cola de pendientes consultable)_
- [ ] 2.2 `permission/decide` con reconciliación primera-gana frente al push vivo; respuesta "ya resuelta" a la vía perdedora _(Req: Decisión por id y reconciliación)_
- [ ] 2.3 Vencidas visibles (no borradas) y sugerencia anti-fatiga tras N aprobaciones idénticas _(Req: Cola; Bandeja — sugerencia)_

## 3. Superficies

- [ ] 3.1 TUI: bandeja operativa en la vista 3 (lista con edad/plazo, decidir, crear regla in situ con confirmación); contador del chrome desde `permission/pending` al (re)conectar _(Req: Bandeja interactiva)_
- [ ] 3.2 CLI/JSON: `deniedPermissions` en salida humana y `--json`; palabra de estado en minúsculas estables; rutas normalizadas _(Req: Honestidad del resultado — H1/H4/H5)_

## 4. Auditoría

- [ ] 4.1 Procedencia en el JSONL: humano | regla (ámbito+contenido) | vencimiento _(Req: Auditoría con procedencia)_

## 5. Tests y calidad

- [ ] 5.1 Unit: precedencia de reglas (proyecto>global, deny>allow, sin-regla→ask), malformadas no derriban, regla nunca amplía opciones
- [ ] 5.2 E2e contra daemon efímero + mock-agent: regla allow sin escalado; pending sobrevive reconexión; decide tras reconexión; doble resolución reconciliada; propose declara denegaciones _(escenarios homónimos)_
- [ ] 5.3 TUI: render de bandeja con `TestBackend` (accesibilidad baseline) y flujo de crear-regla con confirmación
- [ ] 5.4 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes en el workspace
