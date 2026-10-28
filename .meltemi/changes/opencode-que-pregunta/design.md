# Design — opencode-que-pregunta

## Context

Un agente de nivel 1 se lanza con `bin` y `acp-args` de su entrada del registro
(`resolve_id_launch`, `core/meltemid/src/levels.rs`). La sesión recibe además
un entorno superpuesto (`ResolvedAgent.env`) que hoy solo llenan los perfiles
(`[[fleet.profile]].env`), y que el lanzamiento pasa como tokens `NOMBRE=valor`
al frente de la orden, sin registrar jamás los valores. `ResolvedAgent` se
construye en cuatro sitios: perfil, id de catálogo, agente configurado del
proyecto (como respaldo de una etiqueta libre) y agente configurado cuando la
sesión no nombra ninguno.

## Decisions

### D1 — `launch-env` es dato del registro, no código por agente

Una tabla `launch-env` en la entrada del registro, con pares nombre-valor. Ningún
`if id == "opencode"` en el daemon: la casa ya decidió que nada se decide por id
(`adaptadores-propios-acp` D8), y el registro ya declara cómo se lanza cada
agente. Un agente que mañana necesite lo mismo lo declara en su entrada.

### D2 — Se aplica en toda resolución del id, nunca a un comando literal

Los cuatro sitios que construyen `ResolvedAgent` toman el `launch-env` de la
entrada que resuelven: el del agente subyacente para un perfil, el del id para
un id, el del `agent.id` configurado para el agente del proyecto. Un
`agent.command` literal no tiene entrada: lo que el usuario escribió se lanza
como lo escribió.

### D3 — Lo que el usuario fijó manda

Precedencia, de menor a mayor: `launch-env` del registro, el entorno del
usuario (una variable del mismo nombre ya presente en el entorno del daemon) y
el `env` de su perfil. Si el usuario ya usa `OPENCODE_CONFIG_CONTENT` para otra
cosa, Meltemi no lo pisa ni intenta mezclar JSON ajeno: la guía de agentes dice
qué añadirle para que pregunte.

### D4 — Qué pregunta OpenCode

```json
{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","websearch":"ask","task":"ask","external_directory":"ask"}}
```

Editar, ejecutar comandos, ir a la red, lanzar subagentes y tocar rutas fuera
del proyecto pasan por el proxy. Leer, buscar y listar dentro del proyecto no:
no cambian nada ni salen de la máquina, y preguntarlos convertiría cada sesión
en una ráfaga de aprobaciones. `doom_loop` ya es `ask` por omisión.

### D5 — Lo que CI no puede probar, escrito

CI no corre agentes reales. Los tests prueban que la resolución lleva la
variable, con su precedencia y sin tocar el comando literal. Que OpenCode,
lanzado así, mande sus preguntas por ACP lo confirma la pierna manual de
`docs/conformidad-manual.md`, con lo que se observa si sí y si no.

## Risks / Trade-offs

- Si OpenCode no relevara `ask` por ACP, sus ediciones esperarían una respuesta
  que no llega y la sesión no editaría: falla cerrada, que es lo que la
  constitución pide, y la pierna manual lo detecta.
- Un `agent` de OpenCode con reglas propias más permisivas podría tener
  precedencia sobre la configuración global; la pierna manual lo observa.
