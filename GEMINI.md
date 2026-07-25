<!-- meltemi:context:begin sha256=676cb158b340c4a94ef4f7132a1baa162e3fb9d2f64099e17f04cea33bff8f9a -->
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

**Qué es Meltemi**: el plano de control spec-driven para el desarrollo agéntico. Open source (Apache 2.0), gratuito, de la comunidad. Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y proyección de contexto), bajo una disciplina donde ninguna línea de código se escribe sin una especificación revisada.

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

<!-- meltemi:context:end -->
