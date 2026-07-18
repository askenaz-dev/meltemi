## Context

Monitorear y aprobar ya funcionan en remoto sobre un túnel vivo (la cola de
permisos sobrevive reconexiones; `session/list`/`session/log` consultan el
histórico). Falta el tercer verbo — dirigir — y la ergonomía del único camino
sancionado (túnel SSH manual). El bucle de sesión actual es de **un turno**:
`run_session(prompt)` → outcome; no hay canal para un segundo prompt.

**Nota de provisionalidad**: la mecánica exacta del multi-turno se revalida
contra `acp.rs` real al frente de la cola (como toda change de integración).

## Goals / Non-Goals

**Goals:** `session/direct` con semántica clara por estado de sesión (activa /
reanudable / no reanudable); helper `tunnel` auditable por plataforma; frontera
honesta documentada; paridad de superficies.
**Non-Goals:** app móvil (fase 3); push sin conexión (jamás sin enmienda);
transporte propio o puerto (§3); gestión de claves SSH.

## Decisions

### D1 — Dirigir según el estado: encolar, reanudar o rehusar
`session/direct(sessionId, instruction)` resuelve por estado:
- **Activa**: la instrucción entra a una cola por sesión; al concluir el turno
  en curso, el bucle de sesión la despacha como siguiente prompt **sobre la
  misma sesión ACP** (el subproceso sigue vivo). El bucle pasa de un turno a
  "turnos mientras haya cola" — la pieza nueva real de esta change.
- **Terminada y reanudable**: reanudación con la instrucción como prompt
  (máquina de `sesiones-reanudables`: `load_session` + prompt); nace una sesión
  nueva enlazada (`resumed_from`).
- **No reanudable / inexistente**: rehúsa con 2004 y remedio (listar sesiones).
Cada instrucción queda en el JSONL (evento `instruction_queued` + el
`prompt_sent` existente al despacharse): la dirección remota es auditable.

### D2 — Interrupción no destructiva
Dirigir jamás corta el turno en curso (cortar es `session/cancel`, ya existe).
La cola es FIFO y sobrevive dentro de la vida del proceso del daemon; no se
persiste a disco en v1 — una caída con instrucciones sin despachar las pierde y
el log lo evidencia (encoladas sin `prompt_sent`). Trade-off aceptado y anotado:
persistir la cola es delta futuro si el uso lo pide.

### D3 — El helper de túnel compone, no transporta
`meltemi tunnel [user@host]` conoce el endpoint local por plataforma (UDS en
macOS/Linux; named pipe en Windows) e imprime: (a) el comando `ssh` exacto de
reenvío del socket, (b) el snippet de `~/.ssh/config` equivalente, y (c) el
valor de `MELTEMI_ENDPOINT` que el otro extremo debe usar (con la advertencia
git-bash/MSYS ya documentada). Con `--exec` lanza el `ssh` del usuario tal cual
(proceso hijo visible, nunca en background silencioso). El helper es un
**formateador**: cero sockets propios, cero dependencias nuevas.

### D4 — Honestidad de plataforma: Windows como servidor
OpenSSH reenvía sockets Unix (`-L local:remote` con streamlocal), pero **no**
named pipes de Windows: un daemon en Windows no es alcanzable por túnel SSH
estándar hoy. El helper lo declara con diagnóstico y remedio (usar la máquina
Unix como servidor del daemon, o esperar el delta de bridging), en lugar de
imprimir un comando que no puede funcionar. Windows como **cliente** (dirigir
un daemon remoto Unix) sí funciona y se prueba.

### D5 — Paridad y frontera
`session/direct` se consume desde CLI (`direct <session> "<instr>"`), TUI y GUI
— el móvil (fase 3) será un consumidor más, nunca el único (§4). La
documentación de acceso remoto declara la frontera: con túnel vivo, todo; sin
túnel, nada — y por qué eso es la postura de privacidad y no una carencia.

## Risks / Trade-offs

- **Bucle multi-turno**: es la cirugía real (hoy un turno por sesión); riesgo de
  interacción con cancelación y permisos → e2e dedicados (dirigir durante
  turno activo, dirigir tras terminar, cancelar con cola no vacía).
- **Cola no persistida** (D2) → pérdida visible en el log; delta futuro.
- **Deriva de endpoints SSH** entre versiones de OpenSSH → el helper imprime,
  el usuario ve y ejecuta; nada oculto que romper.

## Migration Plan

Aditivo: método, evento y subcomandos nuevos; el bucle de un turno se conserva
para sesiones sin instrucciones encoladas. Reversión: retirar método y helper.

## Open Questions

- ¿Persistir la cola de instrucciones (sobrevivir reinicios)? v1 no; evidencia
  de uso decide.
- Bridging del named pipe en Windows-servidor (¿relay local propio dentro de la
  misma máquina?): delta futuro con design propio; el helper mientras tanto
  rehúsa honesto (D4).
