# Design — vincular-suscripciones

## Context

Estado verificado en el código y contra los CLI reales (2026-08-08):

- Los perfiles (`FleetProfile { name, agent, env }`) se cargan de
  `config.toml` de usuario y de proyecto, se fusionan **por nombre** (el
  último gana) y la higiene rehúsa valores que parecen secretos en claro. El
  resolver (`levels::resolve_fleet_agent`) los honra: perfil > id de catálogo
  > configurado, 2001 sin degradar.
- El overlay de env viaja como tokens `NAME=value` al lanzamiento ACP y los
  adaptadores propios lanzan el CLI del proveedor **sin** `env_clear`
  (`core/meltemi-adapters/src/supervisor.rs`): el hijo hereda el contexto.
- Verificación empírica en esta máquina: `CODEX_HOME=<dir vacío> codex login
  status` → «Not logged in»; el contexto por defecto → «Logged in using
  ChatGPT». `CLAUDE_CONFIG_DIR` es el equivalente documentado de Claude Code.
- No existe RPC alguno que gestione perfiles. Sí existe escritura de
  configuración desde superficies: `permission/decide` con `persist_rule`
  agrega reglas TOML a `permissions.toml` — en el mismo directorio de config
  donde D2 pone su archivo — y es el precedente más cercano (la diferencia de
  D2: archivo **gestionado por la máquina** con reescritura completa, no
  agregado a un archivo que el usuario también edita). El registro de
  proyectos (`projects.rs`) es el precedente del directorio de datos.
- El registro de flota (`fleet-registry.toml`) ya carga datos factuales por
  entrada (`bin`, `adapter`, `legal-note`, capas): el patrón para campos
  nuevos con `#[serde(default)]` está asentado.

## Goals / Non-Goals

**Goals**: que «vincular una suscripción» sea un gesto de cualquier
superficie; que el conocimiento por proveedor (variable de contexto, gesto de
login) sea dato del registro; que lo manual siga mandando; que el duplicado
de contexto se advierta; §2 intacto en cada decisión.

**Non-Goals**: ejecutar o verificar logins; tocar el resolver, `acp.rs` o los
adaptadores; balanceo entre suscripciones; migrar TOML manual; gestionar
perfiles de entradas sin variable declarada.

## Decisions

### D1 — Un vínculo ES un perfil; el daemon solo gana un almacén

Alternativas: (a) un concepto nuevo «suscripción» con su propio registro y
resolución; (b) **vincular = escribir un `FleetProfile` con nombre, cuyo env
fija la variable de contexto del proveedor al directorio del vínculo** — la
elegida. (a) duplicaría la resolución que ya funciona y obligaría a cada
superficie a distinguir dos cosas que el usuario percibe como una. Con (b),
nada cambia en tiempo de sesión: el resolver, el despacho, el tablero y los
metadatos ya honran perfiles; esta change solo añade cómo nacen y mueren. El
resultado de `fleet/list` no cambia de forma: los vínculos aparecen como las
filas de perfil que ya existen.

### D2 — Persistencia: archivo propiedad del daemon, cargado antes que lo manual

Alternativas: (a) reescribir `config.toml` del usuario — descartada:
preservar comentarios y forma de un TOML ajeno es frágil, y un archivo que el
usuario edita no debe ser también el que una máquina reescribe; (b) el
registro JSONL del directorio de datos (precedente de proyectos) —
descartada: los perfiles son configuración legible que el usuario puede
querer mirar y llevarse, no bitácora; (c) **`<config_dir>/subscriptions.toml`,
gestionado por la máquina** (cabecera que lo declara, reescritura completa en
cada cambio, solo bloques `[[fleet.profile]]`) — la elegida. Se carga
**antes** que `config.toml` de usuario y de proyecto: con la fusión por
nombre vigente, cualquier perfil escrito a mano gana sobre un vínculo
homónimo, que es la jerarquía honesta (lo explícito del usuario manda).
`subscription/unlink` solo opera sobre este archivo; desvincular un perfil
que vive en config manual rehúsa con remedio («edítalo en tu config.toml»).

### D3 — El conocimiento por proveedor es dato del registro

La entrada del catálogo gana dos campos opcionales: `auth-context-var` (la
variable que redirige el contexto de autenticación del binario oficial) y
`login-hint` (el gesto de autenticación tal como el proveedor lo documenta).
Snapshot inicial: `claude-code` → `CLAUDE_CONFIG_DIR` / «claude` y dentro
`/login`»; `codex-cli` → `CODEX_HOME` / «codex login» (verificación del
2026-08-08 anotada en la instantánea del registro con su `version`). Una
entrada **sin** `auth-context-var` no ofrece vínculo por superficie: el
remedio nombra la vía manual documentada. Regla de siempre: datos en el
registro, jamás un `match provider` en el código.

### D4 — El contexto es del proveedor: se crea vacío, no se lee, no se borra

`subscription/link` crea `<data_dir>/subscriptions/<nombre>/` vacío y escribe
el perfil con `env = { <var> = "<ruta absoluta>" }` (ruta absoluta: sin
`${VAR}` que dependa del entorno del daemon). Ahí termina la relación de
Meltemi con ese directorio: **jamás lista, lee ni verifica su contenido** —
comprobar «si quedó logueado» exigiría mirar donde viven credenciales (§2).
`subscription/unlink` retira el perfil y **no borra el directorio**: las
credenciales que el proveedor guardó no son nuestras ni para destruirlas; la
respuesta nombra la ruta que queda atrás para que el humano decida. El nombre
del vínculo se valida como componente seguro de ruta (kebab-case, sin
separadores), porque nombra un directorio.

**Colisión encontrada por la crítica, resuelta aquí**: la lente de higiene
vigente (`looks_like_plaintext_secret`) marca como secreto opaco cualquier
valor sin `$` de ≥20 caracteres del alfabeto `[A-Za-z0-9-_./+=]` — y una ruta
absoluta de Linux (`/home/<u>/.local/share/...`) cae entera en ese alfabeto:
el perfil que este design escribe sería **rehusado en silencio en Linux**
(Windows escapa por `:` y `\`; macOS por el espacio de «Application
Support»). La resolución: un valor que contiene separador de ruta no entra a
la rama `opaque` de la heurística — una ruta no es una credencial opaca — con
escenario propio que fija que un perfil con ruta POSIX absoluta sobrevive la
carga en las tres plataformas. La lente sigue rehusando secretos reales: un
token sin separadores sigue cayendo donde caía.

### D5 — El login se compone, jamás se ejecuta

El resultado del vínculo entrega el gesto completo: variable, valor, y el
`login-hint` del proveedor — por plataforma donde aplique (PowerShell
`$env:VAR="..."` / POSIX `VAR=... `). El patrón es el del túnel
(control-remoto-asistido D3): componer la invocación exacta del binario del
usuario y dejar que el humano la corra. Ejecutar un login interactivo desde
el daemon sería piloteo de credenciales ajeno a §2; y «verificarlo» después,
peor. La GUI ofrece el gesto con el botón de copiar que la Flota ya tiene
para los remedios; la CLI lo imprime; el shell lo muestra en el aviso.

### D6 — El duplicado de contexto se advierte en la carga

Dos perfiles del mismo agente cuyo valor de contexto resuelve idéntico son la
misma suscripción con dos nombres — legal (el usuario puede quererlo) pero
casi siempre un error silencioso. `Config::apply` ya recorre los perfiles con
la lente de higiene (secretos en claro); gana una segunda lente: mismo
`agent` + mismo valor resuelto de la variable de contexto → diagnóstico de
advertencia (no rehúso), visible donde ya se muestran los diagnósticos de
flota. En `subscription/link`, un nombre ya vinculado rehúsa (el remedio:
desvincular primero o elegir otro nombre).

### D7 — Paridad: dos métodos, tres superficies, una onda conocida

`subscription/link`/`unlink` entran al contrato con su esquema y conformidad;
la onda es la ya pagada por cada método nuevo: entrada de paleta en el shell
(verbo `link`, con overlay de captura verbatim para `agente nombre` — la
línea de paleta minusculiza y un nombre con mayúsculas sería otro
directorio), registry + formularios generados en la GUI, filas en la matriz
de paridad, verbos `link`/`unlink` en la gramática CLI con la referencia
regenerada. El flujo rico de la GUI vive en la ficha del agente (drawer de la
Flota), que ya muestra remedios por capa: «Vincular suscripción» aparece solo
en entradas con `auth-context-var`.

## Risks / Trade-offs

- **Reescritura completa del TOML propio** (D2): simple y segura porque el
  archivo es nuestro; el precio es que una edición manual DENTRO de
  `subscriptions.toml` puede perderse en el siguiente link/unlink — la
  cabecera del archivo lo declara y el remedio estándar apunta a
  `config.toml` para lo manual.
- **`login-hint` puede envejecer** con las versiones de los CLI: es dato de
  la instantánea del registro, versionada y con verificación anotada — el
  mismo trato que `adapter` y las notas legales.
- **Nombres de vínculo como directorios**: la validación kebab-case evita
  traversal y sorpresas de mayúsculas en Windows; el costo (no se permiten
  espacios) se documenta en el rehúso.
- **Un vínculo no garantiza login**: el estado «vinculada pero sin
  autenticar» existe y es invisible para Meltemi por diseño (§2). La
  respuesta del link lo dice en voz alta: «autentica con este gesto antes del
  primer turno».

## Migration Plan

Aditivo puro: campos opcionales del registro, archivo nuevo, métodos nuevos.
Los perfiles manuales existentes siguen funcionando idénticos (y ganan sobre
vínculos homónimos). Reversión: retirar métodos y superficies; un
`subscriptions.toml` huérfano sigue siendo TOML válido de perfiles que el
usuario puede copiar a su config.

## Open Questions

- ¿Estado «autenticado» visible algún día? Solo si algún proveedor expone una
  señal que no exija leer su contexto (p. ej. un exit code de `login status`
  ejecutado por el usuario); hoy queda fuera por §2.
- ¿Vincular desde el compositor (Home) además de la Flota? El selector de
  perfiles del compositor ya lista vínculos; añadir el alta ahí es UX de una
  change futura si la práctica lo pide.
