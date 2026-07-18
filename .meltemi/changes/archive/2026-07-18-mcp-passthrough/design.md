## Context

ACP permite pasar servidores MCP al agente al crear la sesión; hoy Meltemi no
pasa ninguno y el usuario configura sus servidores N veces (una por agente, cada
una con su formato). La higiene §8.7 manda: las credenciales de los servidores
son del usuario; Meltemi las referencia, jamás las copia ni almacena.

## Goals / Non-Goals

**Goals:** declarar servidores una vez por proyecto; inyectarlos en la creación
de sesión a agentes con soporte anunciado; degradación honesta; higiene de
secretos verificada; visibilidad y registro.
**Non-Goals:** cliente MCP nativo (fase 2); gestión/instalación de servidores;
marketplace (jamás).

## Decisions

### D1 — Declaración en config, secretos por referencia
`[[mcp.servers]]` en `.meltemi/config.toml` (y global): nombre, transporte
(`stdio` con comando/args/env, o `http` con url), donde todo valor sensible se
declara **por referencia a variable de entorno** (`env = { KEY = "$VAR" }`), no
literal. Un lint de higiene marca valores con pinta de secreto en claro
(diagnóstico con remedio), sin bloquear.

### D2 — Inyección negociada en la creación de sesión
Si el agente anuncia soporte MCP en el handshake ACP, los servidores declarados
se pasan en la creación de sesión (forma que el protocolo define). Sin soporte
anunciado: la sesión arranca igual y la superficie lo declara ("servidores no
entregados: el agente no anuncia MCP") — degradación honesta, jamás silenciosa.

### D3 — Visibilidad sin secretos
El log JSONL registra el evento de inyección con **nombres** de servidores (nunca
env resueltos ni urls con credenciales); el detalle de Sesión muestra qué recibió
el agente; `fleet/list` expone el soporte MCP como atributo (dato del handshake
previo o del registro).

## Risks / Trade-offs

- **Formas MCP divergentes por agente** → se pasa la forma ACP estándar; rarezas
  por agente viven en datos del registro, no en código.
- **Secretos en claro pese al lint** → el lint es aviso con remedio; la política
  dura (bloquear) se decidirá con telemetría de uso... no: sin telemetría —
  se decidirá con feedback de comunidad. Aviso por ahora.

## Migration Plan

Aditivo: config nueva opcional; sin servidores declarados, nada cambia.

## Open Questions

- ¿Filtro por agente (`only = ["id"]`) en la declaración? Propuesto simple:
  todos los servidores a todos los agentes compatibles; filtro como delta futuro.
