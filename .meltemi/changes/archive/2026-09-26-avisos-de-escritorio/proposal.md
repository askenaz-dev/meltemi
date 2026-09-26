# avisos-de-escritorio

## Why

Tres changes enseñaron al daemon a esperar al humano, y ninguna superficie
avisa al humano de que se le espera. `espera-humana` hizo de la cola la única
fuente de resolución: mientras haya cliente conectado, el flujo interactivo
espera — la premisa entera de la política es «hay un humano que llega».
`sesion-esperando` fijó `waiting_permission` y `gatePending` para que las
superficies lo pinten; `eventos-para-tardios` puso el hub de eventos y
`session/watch` para que cualquier conexión lo escuche. Todo eso termina hoy
en cromo que solo se ve mirándolo: la bandeja, un contador, un badge. Con la
app detrás de otra ventana — el caso normal de quien lanzó un turno largo y
volvió a su editor — un permiso puede esperar minutos en silencio. El turno no
avanza, nadie falló, y nada suena.

La carencia está verificada, no supuesta: los únicos «notification» del
cliente de escritorio son vocabulario JSON-RPC del bridge
(`desktop/src/bridge.rs`, `desktop/src/lib.rs`); ninguna superficie toca una
API de avisos del SO, y la TUI no emite campana ni secuencia de aviso alguna.

El comparativo de mercado del 2026-07-31 (capturas del mantenedor sobre Orca,
de Stably AI) mostró lo mismo desde afuera: un orquestador de agentes
paralelos trata el aviso de escritorio como pieza de primera clase — con
detección del permiso del SO, remedio y prueba — porque el producto entero es
«deja trabajando y vuelve cuando te necesite». Meltemi tiene mejor gobierno de
la espera que ese producto y peor timbre: la plomería existe completa, falta
el último metro.

Y hay una frontera que esta change hereda ya trazada: la enmienda del Agent
Boss dejó escrito que el aviso de espera remoto es opt-in, autohospedado y que
**nunca lo emite el daemon**. La misma regla vale aquí, por la razón §3 de
siempre: esta change es cromo local de los clientes sobre eventos que ya
reciben; el daemon no gana transporte, ni red, ni conocimiento nuevo.

## What Changes

- **Capability nueva `attention-notices`**: cuándo avisa una superficie (una
  petición de permiso entra en espera de decisión humana; un gate SDD queda
  esperando; una sesión termina o falla), bajo qué condición (la GUI avisa
  solo sin foco — al frente, la bandeja y los estados de hoy ya son el aviso,
  jamás los dos a la vez; la TUI delega en la semántica de campana del
  emulador, que es su mecanismo nativo de atención en segundo plano), y qué
  contiene: sesión, proyecto y motivo, **nunca el texto del prompt ni del
  turno** — el centro de notificaciones del SO persiste fuera del log
  gobernado, y un prompt puede llevar exactamente lo que no debe salir del
  repositorio.
- **GUI**: el plugin oficial de notificaciones de Tauri (dependencia nueva del
  cliente, justificada §10 en el design — el mismo patrón que el plugin de
  diálogo que `lanzador-conversacional` introdujo). El permiso del SO se pide
  en el primer aviso real, no en el arranque; si el SO no entrega avisos, el
  estado se muestra con remedio (abrir los ajustes del sistema), nunca se
  finge que se avisó. Ajustes gana conmutador y «probar aviso». Activar el
  aviso trae la app al frente y navega a lo que espera: la bandeja si es un
  permiso, la sesión si es un cierre.
- **TUI**: campana de terminal y/o secuencia OSC de aviso, opt-in por
  configuración, con los mismos disparadores; la guía documenta qué emuladores
  la honran y no se promete lo que el emulador no dé.
- **Sin spam por construcción**: se avisa en la transición, no en el estado —
  un permiso que espera es un aviso, no uno por repintado; una ráfaga de
  peticiones de la misma sesión colapsa en un aviso con recuento.
- **El daemon, intacto**: cero RPC y cero transporte nuevos. Si al cliente le
  falta un evento para enterarse de una transición sin polling, se añade como
  evento aditivo al hub existente — jamás un canal nuevo.
- Textos en es/en por la i18n existente de cada superficie.

## Capabilities

### New Capabilities

- `attention-notices`: los avisos locales de atención de las superficies —
  disparadores, regla de foco por superficie, contenido mínimo sin texto de
  turno, honestidad del permiso del SO, opt-out en GUI y opt-in en TUI. (Una
  sola capability, cross-superficie y sin nombres de terceros: qué SO o
  emulador honra qué es dato de la guía, no verdad viva.)

### Modified Capabilities

- Ninguna. La bandeja, los estados de espera y el hub quedan como están; la
  capability nueva los consume sin enmendarlos.

## Impact

- Superficies: `desktop/` (plugin, regla de foco, clic-navega, sección de
  Ajustes) y `tui/` (campana opt-in). El contrato `proto/` no se mueve en la
  meta; si un evento aditivo del hub resulta necesario, entra con su schema
  como todo lo aditivo.
- Dependencias: una, el plugin oficial de notificaciones de Tauri, confinada
  al cliente GUI; pasa por cargo-deny y el gate de tamaño de instaladores se
  re-mide — el presupuesto no se supone, se mide.
- Paridad §4: el daemon no gana capacidad, así que no nace deber de paridad —
  aun así las dos superficies reciben el cromo; `docs/paridad-nucleo.md` gana
  la nota, no una fila (la matriz es por RPC).
- Verificación honesta: CI corre headless en los 3 SO y un toast real no se
  puede asertar allí. Los escenarios se parten como `conformidad-manual` ya
  estableció: la lógica (evento→intención de aviso, regla de foco, colapso,
  estados del permiso del SO) se prueba automatizada; el render real por
  plataforma queda como verificación manual documentada con verify-mark.
- Riesgos pineados para el design: en macOS una app sin bundle firmado puede
  no mostrar avisos en desarrollo (se documenta cómo probar sobre el bundle);
  en Windows el toast exige identidad de aplicación — la da el MSI, y el
  ejecutable suelto se declara; en Linux depende de un daemon de
  notificaciones DBus presente — su ausencia degrada a silencio declarado en
  Ajustes, jamás a error fatal; y el clic-navega varía por plataforma — donde
  el plugin no lo dé, el aviso sigue siendo veraz sin navegación y se
  declara.
- §9 intacto: todo local, nada sale de la máquina.

## Fuera de alcance

- **El aviso remoto del Agent Boss** (`mobile-companion`/`remote-access`,
  fase 3): opt-in, autohospedado, nunca del daemon — otra change y otra
  superficie; esta no lo adelanta ni lo condiciona.
- **Sonidos personalizados, historial o centro de avisos propio**: el SO ya
  tiene centro de notificaciones; duplicarlo es cromo sin evidencia.
- **Avisos de progreso** (por token, por tool call, por tarea que avanza):
  solo transiciones que piden humano o cierran sesión; lo demás es la sesión
  abierta, que para eso está.
- **Badges de dock/taskbar y contadores por plataforma**: futuro con
  evidencia.
- **La CLI scriptable**: sus salidas son stdout/stderr y códigos de salida;
  un script que quiera avisar ya tiene el SO entero a mano.
