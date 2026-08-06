@AGENTS.md

<!-- meltemi:context:begin sha256=b3a309c135232e6adf6e8579403444dbd99df32eec721d727e0ee51ae4e699c8 -->
# Meltemi — contexto proyectado

_Compilado desde `.meltemi/` por `meltemi project`. El contenido del bloque gestionado se regenera; no editarlo a mano._

## Constitución

# Constitución de Meltemi

> **Estado: RATIFICADA v1.0** — 11 de julio de 2026, por Guillmar Ortiz (`fase-0-fundacion` 1.2).
> Estos son los principios no negociables del proyecto. Se inyectan como contexto en toda propuesta de cambio y en toda sesión de agente que trabaje sobre este repositorio. Toda modificación de este documento requiere una propuesta de cambio aprobada.

## Principios

### 1. Spec-first, proporcional
Ninguna funcionalidad se implementa sin una propuesta de cambio aprobada (proposal → design → specs → tasks). Los cambios triviales usan la vía rápida (`fast-forward`: todos los artefactos de una vez), nunca la vía nula. Los escenarios de las specs son la definición de "terminado": cada escenario debe quedar cubierto por un test o una verificación documentada.

### 2. Juego limpio — innegociable
Meltemi ejecuta únicamente los binarios oficiales de los agentes, con la autenticación que cada agente gestiona. Prohibido: leer, almacenar o reutilizar credenciales de agentes; suplantar el tráfico o la identidad de otro cliente; empaquetar agentes de terceros sin permiso expreso de su licencia. Ante la duda, la respuesta es no.

### 3. Seguridad por defecto
El daemon escucha solo en socket local con permisos exclusivos del usuario; el acceso remoto es únicamente vía túnel SSH. Sin cliente conectado, toda petición de permiso se deniega. Los agentes operan en worktrees aislados. Las acciones con efectos externos irreversibles requieren aprobación explícita incluso en modo autónomo.

### 4. Paridad de núcleo
Toda capacidad nueva del daemon debe ser consumible desde la TUI y la GUI por igual. Está prohibido añadir al daemon funcionalidad accesible desde una sola superficie.

### 5. Agnosticismo de agente y de modelo
El núcleo no asume ningún proveedor. Ninguna dependencia del workspace puede requerir una cuenta o clave de un proveedor concreto para compilar o pasar los tests (los tests e2e usan el agente simulado).

### 6. Estándares abiertos primero
ACP para pilotar agentes, MCP para herramientas, LSP para inteligencia de código, JSON-RPC 2.0 para transporte. Antes de inventar un protocolo o formato propio, hay que demostrar por escrito que ningún estándar abierto lo cubre.

### 7. Calidad verificable
`cargo clippy -- -D warnings`, `cargo fmt --check` y la suite de tests deben pasar en las tres plataformas (Windows, macOS, Linux) antes de cualquier merge. Windows es plataforma de primera clase, no un puerto posterior.

### 8. Trazabilidad
Un commit atómico por tarea; el mensaje referencia la change y la tarea (`fase-0-fundacion 3.2`). Cada línea de código debe poder rastrearse hasta el requisito que la originó.

### 9. Sin telemetría oculta
Toda métrica se calcula en local. Cualquier telemetría futura será opt-in, desactivada por defecto, y su contenido exacto estará especificado públicamente antes de existir.

### 10. Dependencias mínimas y pineadas
Cada dependencia nueva se justifica en el design de la change que la introduce. Versiones pineadas; auditoría de licencias y vulnerabilidades en CI.

### 11. Idioma
Documentación de producto y artefactos del método: español neutro internacional. Código, identificadores, mensajes de commit y comentarios: inglés (el estándar de la comunidad global). Los textos de cara al usuario final se diseñan para internacionalización desde el inicio (español e inglés como primeros idiomas).

### 12. Apache 2.0, para siempre
El núcleo, los clientes y el SDK son Apache 2.0 y no cambiarán de licencia. Ninguna contribución se acepta bajo términos que comprometan esta promesa.

## Rumbo

### product

# Rumbo: Producto

> **Enmienda pendiente de ratificación** — 31 de julio de 2026
> (`lanzador-conversacional`, design D5). El párrafo «Qué es Meltemi» ya no
> presenta la especificación revisada como condición previa, sino el gobierno
> de toda sesión como piso y la disciplina spec-first como el camino más
> corto. La firma del mantenedor fundador es **gate de archivo** de esa change;
> mientras no llegue, ese párrafo está aplicado pero no ratificado. El resto
> del documento sigue ratificado el 11 de julio de 2026.

**Qué es Meltemi**: el plano de control spec-driven para el desarrollo agéntico. Open source (Apache 2.0), gratuito, de la comunidad. Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y proyección de contexto): toda sesión corre gobernada —proxy de permisos con deny-by-default, registro apend-only y punto de restauración declarado— y sobre ese piso la disciplina spec-first está siempre a un gesto: proponer, planificar y verificar viven en el mismo compositor donde se empieza a trabajar. La especificación revisada es el estándar que Meltemi hace fácil de sostener, no el peaje que impide empezar.

**Qué NO es**: ni un editor de propósito general (la superficie de código admite edición utilitaria al servicio del bucle agéntico; la autoría sostenida vive en el editor del usuario), ni otro agente de codificación (el motor propio de fase 2 es opcional), ni un servicio en la nube, ni CI/CD, ni un marketplace.

**Para quién (MVP, en orden)**: (1) el desarrollador individual que ya usa y paga agentes CLI y trabaja en terminal; (2) el tech lead que quiere disciplina de specs sin imponer herramientas; (3) mantenedores open source en bases de código maduras.

**El lema**: "Un rumbo, muchas velas." Una spec clara impulsa cualquier número de agentes, de cualquier fabricante, sin atarse a ninguno.

**Principio comercial**: no hay créditos, ni tarifas, ni lock-in. BYO-agent, BYOK, BYO-modelo.

**Referencia completa**: `meltemi.md` (documento fundacional, versión 0.2).

### structure

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

### tech

# Rumbo: Stack técnico y restricciones

**Lenguaje**: Rust estable (toolchain pineado en `rust-toolchain.toml`). Un solo lenguaje de sistemas en todo el producto.

**Arquitectura**: daemon headless `meltemid` (toda la lógica) + clientes finos (TUI `meltemi`, GUI Tauri) vía JSON-RPC 2.0 con delimitación por líneas sobre socket local (UDS 0700 en macOS/Linux; named pipe con ACL de usuario en Windows). Sin puertos de red, jamás.

**Dependencias clave (pineadas)**: `tokio` (runtime async), crate oficial del Agent Client Protocol (integración de agentes), `serde` (tipos del contrato `proto/`), `directories` (rutas por plataforma). Toda dependencia nueva se justifica en el design de su change.

**Contrato**: los JSON Schemas de `proto/` son la fuente de verdad del protocolo daemon↔clientes; los tipos Rust de `meltemi-proto` deben pasar el test de conformidad contra ellos.

**Persistencia**: logs de sesión JSONL apend-only en el directorio de datos del usuario; artefactos del método en `.meltemi/` dentro de cada repositorio.

**Plataformas soportadas**: Windows 10 1809+ / Windows 11, macOS 13+, Linux (glibc mainstream). CI obligatorio en las tres; Windows es primera clase.

**Calidad**: `cargo clippy -- -D warnings`, `cargo fmt --check`, tests por escenario de spec. Los e2e de CI usan `mock-agent` (nunca agentes reales ni red).

**Prohibiciones**: credenciales de agentes (ni leerlas ni tocarlas); transporte de red en el daemon; dependencias que exijan cuentas de proveedores para compilar o testear; features del daemon accesibles desde una sola superficie.

## Cambio activo: avisos-de-escritorio

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

<!-- meltemi:context:end -->
