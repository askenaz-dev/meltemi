---
inclusion: siempre
---

# Rumbo: Estructura y convenciones

**Monorepo** (destino; se materializa en `fase-0-fundacion`):

```
meltemi/
├── core/meltemid/     # binario del daemon (Rust)
├── core/mock-agent/   # agente ACP simulado para tests e2e
├── proto/             # JSON Schemas del contrato + crate meltemi-proto
├── tui/               # cliente de terminal `meltemi` (fase 1)
├── desktop/           # cliente GUI Tauri (fase 2)
├── sdk/               # SDK público (fase 2)
├── brand/             # identidad visual (V2 vigente; ver brand/README.md)
├── docs/              # documentación y research interno
├── .meltemi/          # constitución, rumbo y (a futuro) specs del propio proyecto
└── openspec/          # método SDD actual del proyecto (ver nota de migración)
```

**Método de trabajo (dogfooding en dos etapas)**: hasta que Meltemi pueda hospedar sus propias specs, el proyecto se desarrolla con OpenSpec (`openspec/changes/`, comandos `/opsx:*`). La constitución y el rumbo ya viven en `.meltemi/` (formato destino). Cuando el motor de specs de fase 1 esté operativo, se migrarán las specs vivas de `openspec/specs/` a `.meltemi/specs/` mediante una change dedicada.

**Convenciones**:
- Changes en kebab-case; un commit atómico por tarea con referencia `(<change> <tarea>)`.
- Código, identificadores y commits en inglés; artefactos del método en español neutro.
- Los escenarios de spec (`#### Scenario:`) son la fuente de los nombres de tests.
- Nada se implementa si no está en la change activa; lo que surja se anota como propuesta futura, no se cuela.
