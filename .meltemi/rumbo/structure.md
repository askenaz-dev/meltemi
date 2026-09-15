---
inclusion: siempre
ratificado: 2026-07-11
ratificador: Guillmar Ortiz
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
│   └── harness/       # harness de ámbito proyecto (reglas; ver abajo)
└── openspec/          # método SDD actual del proyecto (ver nota de migración)
```

**El harness** (añadido por `harness-global-y-por-agente`, 2026-09-15): lo que
se le dice a todo agente antes de que empiece, en cuatro ámbitos con dos raíces.
Dentro del repositorio viven los dos de proyecto; los dos de usuario viven en el
directorio de configuración, que no es parte del monorepo:

```
<config>/meltemi/harness/rules/<name>/RULE.md                 (1) usuario
<config>/meltemi/harness/per-agent/<id>/rules/<name>/RULE.md   (2) usuario × agente
<repo>/.meltemi/harness/rules/<name>/RULE.md                   (3) proyecto
<repo>/.meltemi/harness/per-agent/<id>/rules/<name>/RULE.md     (4) proyecto × agente
```

Precedencia 1 < 2 < 3 < 4 — lo específico pisa lo general, proyecto pisa
usuario: la misma dirección que ya rige config, permisos y perfiles. El eje por
agente es un **directorio**, no un campo: un directorio es forma, que el daemon
valida; un campo sería semántica, que el daemon no interpreta. `<id>` sale del
catálogo de flota. El formato de `RULE.md` es el de FDH, adoptado entero.

La frontera de escritura es la parte que no se puede relajar: **el harness de
usuario nunca entra a un archivo del repositorio**, y las reglas de proyecto
entran al bloque gestionado como el resto del contexto proyectado. Detalle
completo en `docs/harness.md`.

**Método de trabajo (dogfooding en dos etapas)**: hasta que Meltemi pueda hospedar sus propias specs, el proyecto se desarrolla con OpenSpec (`openspec/changes/`, comandos `/opsx:*`). La constitución y el rumbo ya viven en `.meltemi/` (formato destino). Cuando el motor de specs de fase 1 esté operativo, se migrarán las specs vivas de `openspec/specs/` a `.meltemi/specs/` mediante una change dedicada.

**Convenciones**:
- Changes en kebab-case; un commit atómico por tarea con referencia `(<change> <tarea>)`.
- Código, identificadores y commits en inglés; artefactos del método en español neutro.
- Los escenarios de spec (`#### Scenario:`) son la fuente de los nombres de tests.
- Nada se implementa si no está en la change activa; lo que surja se anota como propuesta futura, no se cuela.
