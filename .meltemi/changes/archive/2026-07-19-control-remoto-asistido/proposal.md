## Why

El control remoto de Meltemi ya es constitucionalmente posible — el daemon
atiende a cualquier cliente que llegue al socket local, y un túnel SSH del
usuario es indistinguible de una conexión local (§3; `mobile-companion`) — pero
en la práctica está incompleto en dos puntos. Primero, de los tres verbos del
compañero remoto (**monitorear**, **aprobar**, **dirigir**), los dos primeros ya
existen (`session/list`/`status` y la cola de permisos que sobrevive
reconexiones), pero **no hay método para dirigir una sesión existente**: nada
permite enviar una instrucción a una sesión en curso o reanudable — solo
crearlas (`propose`/`implement`) o cancelarlas. Segundo, el único camino
sancionado de acceso remoto (túnel SSH al socket local) es **enteramente
manual**: el usuario debe conocer el endpoint por plataforma y construir su
comando `ssh` a mano. La fricción convierte la postura de privacidad en un
castigo en lugar de una ventaja.

## What Changes

- **`session/direct`**: enviar una instrucción a una sesión existente. Sobre una
  sesión activa, la instrucción se encola y se despacha como el siguiente turno
  al concluir el actual; sobre una sesión terminada pero reanudable, reanuda con
  la instrucción (máquina de `sesiones-reanudables`); sobre una no reanudable,
  rehúsa con diagnóstico. Todo queda en el JSONL. Por paridad (§4) el verbo es
  de todas las superficies: CLI `direct`, TUI, GUI — no solo del móvil.
- **Helper de túnel auditable** (`meltemi tunnel`): imprime (u opcionalmente
  ejecuta con el `ssh` del usuario) el comando exacto para llevar el endpoint
  local de esta plataforma al otro extremo, más el snippet de configuración SSH
  equivalente. Jamás un transporte propio, jamás un puerto (§3): el helper solo
  compone la invocación del `ssh` que el usuario ya tiene.
- **Frontera honesta documentada**: el control remoto funciona **con el túnel
  vivo**; no existe (ni existirá sin enmienda) notificación push sin conexión —
  eso exigiría un relé en la nube, rechazado por rumbo. La documentación lo
  declara como postura, no como carencia.

## Capabilities

### New Capabilities
- `remote-access`: el helper de túnel auditable y la frontera honesta del
  acceso remoto.

### Modified Capabilities
- `acp-session`: + dirección de una sesión existente (`session/direct`).

## Impact

- `core/meltemid` (cola de instrucciones por sesión + bucle multi-turno o
  reanudación con instrucción; `session/direct`), `proto/` (método + evento
  aditivos), `tui/` (subcomandos `direct` y `tunnel`), `docs/` (acceso remoto).
- Sin cambios de transporte: el daemon no distingue túnel de local (ya spec en
  `mobile-companion`); esta change no toca esa regla, la aprovecha.

## Fuera de alcance

- **La app móvil** (cliente de fase 3, acotado por `mobile-companion`); esta
  change construye el sustrato daemon+CLI que esa superficie consumirá.
- **Push sin túnel / relé en la nube / puerto de red**: rechazado por §3 y
  rumbo; la frontera se documenta, no se negocia.
- **Gestión de claves o configuración SSH del usuario**: el helper compone
  comandos; nunca genera, lee ni almacena material de claves.
- Edición de código/specs remota (el alcance móvil la excluye por spec).
