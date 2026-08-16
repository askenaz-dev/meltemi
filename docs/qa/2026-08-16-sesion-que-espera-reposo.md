<!-- SPDX-License-Identifier: Apache-2.0 -->
# Sesión que espera — coste del reposo, medido (2026-08-16)

El proposal declaró un riesgo mayor y pidió no suponerlo: **una sesión que
espera es un subproceso de agente vivo**, es decir memoria y un proceso real del
proveedor. Esto es la medición.

## Cómo

Binarios **release** en un árbol de destino aparte, daemon propio en un pipe
propio (`\\.\pipe\meltemid-idle-probe`), proyecto de prueba temporal apuntando a
`mock-agent`, y cinco sesiones arrancadas con `detach: true`. Confirmado con
`meltemi --json sessions` que las cinco quedaron efectivamente en
`waiting_instruction` —no `ended`, no `active`— antes de medir.

Configuración del fixture:

```toml
[sessions]
idle-timeout = 600
max-idle = 10
```

## Lo medido (Windows 11, 2026-08-16)

| | MB residentes |
|---|---|
| `meltemid` en reposo, sin sesiones | 15.7 |
| `meltemid` con **5 sesiones esperando** | 21.7 |
| **Coste en el daemon** | **+6.0 total ≈ 1.2 por sesión** |
| `mock-agent` (cada uno de los cinco) | 6.7 – 7.7 |
| **Total del árbol con 5 esperando** | **≈ 56.5** |

Cinco procesos `mock-agent` vivos y contados, uno por sesión: la relación es
exactamente uno a uno, que es lo que había que comprobar.

## Lo que este número NO dice, y es lo importante

`mock-agent` es un binario Rust diminuto. **Un agente real es el número que
manda**, y es de otro orden: Claude Code, Codex y Copilot corren sobre Node y
pesan entre uno y dos órdenes de magnitud más por proceso. Cinco sesiones
esperando con agentes reales no son 35 MB de agentes; son cientos.

Por eso la cota no es higiene:

- `idle-timeout` por defecto **900 s** (quince minutos): largo para pensar,
  corto para que una ventana olvidada no se convierta en una fuga.
- `max-idle` por defecto **3**: al llegar al tope se cierra la espera **más
  antigua**, nunca se rehúsa la sesión nueva.
- Y la tercera salida, que no hubo que escribir: sin ningún cliente conectado de
  forma sostenida, la espera termina. Una sesión que espera instrucciones sin
  nadie que pueda enviarlas está esperando a nadie.

Con los tres por defecto, el techo del reposo es **tres agentes**, no los que
quepan.

## Pendiente, y dicho

- **La medición con un agente real** (Claude Code o Codex sobre este mismo
  guion) la puede hacer solo el mantenedor, en su máquina y con su
  autenticación: CI no ejecuta agentes reales ni red (constitución §5). El guion
  para repetirlo está en `scripts/measure-idle-sessions.ps1`.
- macOS y Linux no se midieron aquí. No hay motivo para esperar otra forma —el
  mecanismo es el mismo proceso hijo— pero el número sería otro y no se afirma.
