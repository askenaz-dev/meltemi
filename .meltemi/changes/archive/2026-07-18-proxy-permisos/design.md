## Context

El passthrough vive: el daemon reenvía `permission/request` al cliente conectado,
deniega por timeout (`permission/timeout`) y por defecto sin cliente. Pero es
todo-o-nada (CLI deniega todo; devclient aprueba todo), el contador de pendientes
es por conexión (deuda confirmada por QA) y no hay reglas ni memoria. QA H1:
`propose` con permiso denegado reporta éxito sin declararlo.

## Goals / Non-Goals

**Goals:** reglas allow/ask/deny evaluadas en el daemon; pendientes de primera
clase (sobreviven reconexión y multi-cliente); bandeja interactiva (interior de la
vista 3); honestidad de resultado (H1/H4/H5); auditoría con procedencia.
**Non-Goals:** controles nativos de agentes niveles 3–4 (#10); políticas de
equipo (fase 3); nuevos subcomandos CLI (las reglas SON la vía scriptable).

## Decisions

### D1 — Reglas en el daemon, evaluadas antes de escalar
Regla = `{efecto: allow|deny, herramienta?, comando? (prefijo), ruta? (prefijo),
ámbito}`. Persistencia TOML: global (`<config>/permissions.toml`) y proyecto
(`.meltemi/permissions.toml`). Evaluación: proyecto sobre global; `deny` gana
empates; sin regla aplicable → `ask` (escalar al humano). Las reglas **nunca
amplían** lo pedido: solo deciden sobre las opciones que el agente ofreció.

### D2 — Pendientes de primera clase, resolución por dos vías
Cola en el daemon: `{id, sesión, herramienta, resumen, opciones, deadline}`.
Nuevos métodos: `permission/pending` (listar) y `permission/decide` (resolver por
id); notificación `permission/changed` a todos los clientes. El push existente
`permission/request` al cliente conectado se conserva (vía rápida, contrato
vivo intacto); la primera resolución gana (la otra vía recibe "ya resuelta").
Reconexión: el cliente repuebla su bandeja con `permission/pending` — muere la
deuda del contador por conexión. Timeout: comportamiento vivo sin cambios, la
entrada queda marcada vencida (visible, no borrada en silencio).

### D3 — Bandeja: el interior de la casa
La vista 3 lista pendientes (edad, deadline con escalado textual), decide con
confirmación ligera, y crea reglas in situ ("permitir siempre esta herramienta en
este proyecto") en un solo gesto con confirmación. `a` (ya global) enfoca la
bandeja. Fatiga: tras N peticiones idénticas aprobadas, el daemon sugiere la
regla (hint en la petición).

### D4 — Honestidad de resultado (H1/H4/H5)
`ProposeResult` gana `denied_permissions: u32`; la CLI lo declara en humano y
`--json`, con palabra de estado estable en minúscula (no `Debug`) y rutas
normalizadas al separador de la plataforma.

### D5 — Auditoría con procedencia
Cada decisión registra en el JSONL quién decidió (`humano | regla | timeout`) y,
si fue regla, cuál (ámbito + contenido). Sin datos nuevos sensibles.

## Risks / Trade-offs

- **Dos vías de resolución** (respuesta al push vs `decide`) → carreras
  resueltas por "primera gana" con respuesta explícita a la perdedora; test e2e
  dedicado.
- **Reglas demasiado amplias** (prefijos golosos) → la creación in situ propone
  la regla más específica posible; el design de #10 revisita para niveles 3–4.
- **Compatibilidad**: el push vivo se conserva; devclient sigue funcionando.

## Migration Plan

Aditivo en contrato (2 métodos + 1 notificación + campos). Sin reglas definidas,
el comportamiento actual se preserva exactamente (ask → push → timeout deniega).

## Open Questions

- Umbral N de sugerencia anti-fatiga (constante inicial, configurable después).
- Formato exacto de la regla de comando (prefijo simple v0.1; glob después).
