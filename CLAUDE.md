@AGENTS.md

<!-- meltemi:context:begin sha256=a8ae111f39d50f557bddce9b836e3bd987a71d7e75cf01d7c1791a46a583ebdc -->
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

## Cambio activo: artefactos-de-cada-push

# artefactos-de-cada-push

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre una capability existente,
> ninguna capability nueva, ningún MODIFIED ni REMOVED (design D7). Alcance de
> un día: un workflow nuevo, sus tests y una sección de documentación.

## Why

El mantenedor lo pidió en una frase: «Me gustaría que esto sea automático con
cada push (para eso tenemos CICD)». Quiere probar el build del día —el DMG de
macOS en particular, que no puede construir en su máquina Windows— sin
ceremonia, cada vez que `main` se mueve.

Lo que **no** puede volverse automático es la release firmada, y no por
comodidad: `procedencia-de-release` lo decidió con lenguaje normativo detrás.
Una cuenta de GitHub comprometida empuja un tag, CI construye, la atestación se
acuña y **verifica perfectamente** —porque registra fielmente el commit del
atacante. La firma manual en una máquina que GitHub no controla es el único
paso que esa cuenta no puede completar. Y la clave en un secreto de Actions es
el caso que SLSA v1.2 prohíbe textualmente («MUST NOT be accessible to the
environment running the user-defined build steps»). A eso se suma que este
repositorio tiene **immutable releases** activado: publicada una release, sus
assets no admiten añadidos, así que firmar precede a publicar y no hay orden
alternativo. Nada de esto se toca aquí.

Pero entre «release firmada» y «nada» hay un escalón que el pipeline ya casi
tiene construido. `release.yml` empaqueta los seis artefactos por plataforma y
**ya los sube con `actions/upload-artifact`** (líneas 187 y 289) antes de que el
job `release` los baje y los fusione. Es decir: la ruta que produce exactamente
lo que el mantenedor quiere descargar existe, funciona, y está apagada fuera de
un tag —cada job lleva `if: startsWith(github.ref, 'refs/tags/')` (39, 119,
129, 194, 303). En un push a `main` hoy solo corren `site-lint` y
`publish-site`. El último metro que falta no es empaquetar: es **dejar que la
ruta de empaquetado corra también en `main` y que su salida quede descargable
desde la página del run**, sin release, sin versión, sin firma y sin insinuar
ninguna de las tres.

La distinción importa porque un artefacto de run **no es distribución**. No
tiene URL estable, caduca, no aparece en `releases/latest`, y GitHub lo sirve
tras autenticación. Es exactamente la forma correcta de «probar un build» y
exactamente la forma incorrecta de instalar Meltemi — y esta change escribe esa
frontera en vez de dejarla a la intuición del que descarga.

## What Changes

- **Workflow nuevo `.github/workflows/build.yml`**, disparado por push a `main`
  y por `workflow_dispatch`. Un job por plataforma que construye en modo
  release, arma el archivo del núcleo (`meltemi`, `meltemid` y los dos
  adaptadores ACP, como manda `adaptadores-propios-acp`), construye el
  instalador de la GUI con `tauri build`, **mide los tres presupuestos** y sube
  todo como artefacto del run.
- **`release.yml` no se toca en su grafo de jobs.** La ruta que crea releases
  sigue siendo solo de tag; la ruta nueva vive en su propio archivo con su
  propio disparador. La razón es medible, no estética: `publish-site` declara
  `needs: [site-lint, gates, deny, package, package-gui]` (línea 382), y `needs`
  no es condicional —encender el empaquetado en `main` dentro de ese archivo
  haría que **cada publicación del sitio esperara un build de tres plataformas
  con Tauri**, y con `!cancelled()` en su condición (384), cancelar el build
  ahora caro se llevaría por delante la publicación del sitio que hoy es lo
  único que `main` hace. Además, `gates` y `deny` en `main` serían un duplicado
  literal de `ci.yml`, que ya corre fmt, clippy, build, la suite y el lint del
  sitio en las tres plataformas, más `cargo-deny`, en cada push a `main`
  (design D1).
- **El artefacto dice lo que es**: `meltemi-unsigned-<SO>-<sha-corto>`. El
  nombre lleva el commit porque un build sin commit es un build que nadie puede
  volver a producir, y lleva `unsigned` porque esa es la propiedad que lo separa
  de una release. Los **archivos dentro conservan los nombres estables** —
  `meltemi-macOS.tar.gz`, `meltemi-desktop-macOS.dmg`— porque el valor del
  ensayo es producir exactamente lo que produce el tag (design D3).
- **Aviso de «sin firmar» donde el humano mira**: un `UNSIGNED-BUILD.txt` dentro
  del artefacto y el mismo texto en el resumen del run, que es la página donde
  está el botón de descarga. Dice qué no tiene (firma, atestación, publicación)
  y adónde ir para instalar de verdad. El texto vive en `scripts/` como archivo
  del árbol, revisable y testeable, no incrustado en el YAML.
- **Los presupuestos gatean esta ruta también**:
  `MELTEMI_TUI_BUDGET_BYTES`, `MELTEMI_GUI_INSTALLER_BUDGET_BYTES` y
  `MELTEMI_ADAPTER_BUDGET_BYTES` con los mismos valores y el mismo `exit 1`. Un
  build que nadie mide es exactamente como se pudre un presupuesto: creciendo un
  poco por commit hasta que el tag lo descubre.
- **`permissions: contents: read` en la cabecera del workflow**, para que la
  ruta nueva no pueda crear una release aunque alguien le añada un paso mañana.
  La imposibilidad es estructural, no una promesa en un comentario.
- **Retención de 7 días**, declarada y justificada: el propósito es «probar el
  último build», y un build de hace dos semanas no es el último.
- **Tests en `core/meltemid/tests/release.rs`** que pinean lo que la change
  afirma: que el empaquetado corre en `main`, que el nombre lleva commit y
  `unsigned`, que los presupuestos son los mismos en los dos archivos, que los
  adaptadores viajan en los tres archivos, y —el conjunto negativo, el que
  importa— que `build.yml` **no** contiene creación de release, ni atestación,
  ni `.minisig`, ni `contents: write`.
- **`docs/release.md`** gana una sección corta que distingue las dos rutas para
  quien lea la documentación en vez del aviso.

## Capabilities

### Modified Capabilities

- `release-distribution`: + cuatro requisitos ADDED sobre la frontera entre
  empaquetar y publicar — builds de integración descargables sin crear release,
  su identidad y caducidad, los presupuestos aplicados a toda ruta que
  empaquete, y la declaración de ausencia de firma donde se descarga. La
  capability que ya posee el pipeline y sus presupuestos es la que debe poseer
  la línea que el pipeline no puede cruzar (design D2).

### New Capabilities

- Ninguna.

## Impact

- Archivos: `.github/workflows/build.yml` (nuevo),
  `scripts/unsigned-build-notice.txt` (nuevo), `core/meltemid/tests/release.rs`,
  `docs/release.md`, `docs/plan-de-cambios.md`. **`release.yml` no cambia.**
- **Costo, dicho y no escondido**: cada push a `main` gana tres jobs que hoy no
  existen, cada uno con un `cargo build --workspace --release` y un `tauri
  build`. Es el gasto más caro que este repositorio haya añadido a `main`, y se
  asume porque el mantenedor lo pidió sabiendo lo que pedía. Se abarata donde se
  puede: un job por plataforma en vez de los dos del camino de tag (el build
  release se comparte con el bundle en vez de compilarse dos veces), y sin
  duplicar los gates que `ci.yml` ya corre sobre el mismo commit. Y se deja con
  dos diales a mano —el bloque `on:` y la matriz de plataformas— para que
  bajarlo a `workflow_dispatch` o a `schedule` sea editar cuatro líneas de un
  archivo, no reestructurar un pipeline (design D5).
- Cero dependencias nuevas; ningún crate, ninguna action de terceros que no use
  ya `release.yml`. El contrato `proto/` no se mueve, el daemon no gana
  capacidad y por tanto **no nace deber de paridad §4**: esto es infraestructura
  del repositorio, no superficie de producto.
- Los enlaces de descarga del sitio (`releases/latest/download/…`) siguen
  resolviendo a la última release **firmada**, porque esta ruta no crea, no
  modifica y no publica release alguna. El test lo pinea por el lado negativo.
- **Lo que solo un push real puede confirmar**: que los tres jobs completan
  dentro del tiempo de runner, que el `tauri build` de macOS produce el DMG en
  esta ruta igual que en la de tag, y cuánto tarda de verdad. Se declara ahora
  para que no se lea como verificado.

## Fuera de alcance

- **Firmar, atestiguar o publicar nada desde esta ruta.** Es el punto entero:
  `procedencia-de-release` decidió que la firma es manual y no se relitiga aquí.
- **Prereleases, tags `nightly` o cualquier release automática**, incluida la
  marcada como draft: contaminaría el espacio de versiones y pondría en riesgo
  `releases/latest`, que es lo que el sitio y los dos README prometen.
- **Builds en pull requests o en ramas de trabajo**: el mantenedor pidió `main`,
  y un fork abriría la ruta a ejecutar empaquetado con código de terceros. Si se
  quiere, es otra change con su análisis de amenaza.
- **Caché compartida entre `ci.yml` y `build.yml`**: `Swatinem/rust-cache` ya
  gestiona lo suyo por job; afinarlo es optimización con evidencia, no de
  entrada.
- **Instaladores para plataformas que el camino de tag no produce** (`.rpm`,
  AppImage): siguen fuera por las razones de `instaladores-linux-sin-webview`.
- **Firma de plataforma** (Authenticode, notarización): deuda declarada en
  `docs/release.md`, ajena a esta change.

### Deltas

- `release-distribution`: 4 requisitos

<!-- meltemi:context:end -->
