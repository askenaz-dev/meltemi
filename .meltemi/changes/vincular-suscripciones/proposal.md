# vincular-suscripciones

## Why

Es el pedido fundacional, en palabras del mantenedor: «si tengo dos
suscripciones de Claude y tres de Codex, debo poder **vincular** esas
suscripciones en la flota». El motor ya existe y está probado —
`flota-multiproveedor` construyó los perfiles de lanzamiento
(`[[fleet.profile]]`) que redirigen el contexto de autenticación del binario
oficial, la resolución perfil > catálogo > configurado, y la carrera de dos
suscripciones en paralelo con contextos distintos verificada e2e;
`multiproyecto-suscripciones` puso la suscripción en los metadatos de cada
sesión y `tablero-de-carrera` la pintó por calle. La mecánica llega hasta el
CLI real: el adaptador propio lanza el proveedor sin limpiar el entorno, y la
verificación empírica del 2026-08-08 sobre esta máquina lo cerró —
`CODEX_HOME=<dir vacío> codex login status` responde «Not logged in» mientras
el contexto por defecto responde «Logged in using ChatGPT».

Pero **el vínculo como experiencia no existe**. Para «2 Claude + 3 Codex» el
usuario debe hoy: saber por su cuenta que Claude redirige con
`CLAUDE_CONFIG_DIR` y Codex con `CODEX_HOME` (el registro no lo sabe: no es
dato, y los docs solo ejemplifican Claude), escribir cinco bloques TOML a
mano, inventar la disciplina de directorios de contexto, y deducir el gesto
de login que autentica cada contexto nuevo. Ninguna superficie puede crear,
listar como vínculo ni deshacer una suscripción; y si dos perfiles del mismo
proveedor apuntan al mismo directorio — la misma suscripción dos veces — nadie
lo advierte. La feature fundacional está habilitada por dentro y muda por
fuera.

## What Changes

- **La variable de contexto como dato del registro**: cada entrada del
  catálogo que lo tiene declara `auth-context-var` (claude-code →
  `CLAUDE_CONFIG_DIR`, codex-cli → `CODEX_HOME`) y `login-hint` (el gesto de
  autenticación que el proveedor documenta). Datos factuales de
  interoperabilidad, donde ya viven `bin`, `adapter` y las notas legales —
  nunca lógica por proveedor en el código.
- **`subscription/link` y `subscription/unlink`**: vincular crea el perfil con
  nombre propio, su directorio de contexto vacío (por defecto bajo el
  directorio de datos del daemon) y responde con el **gesto de login
  compuesto** — la variable, el valor y el comando documentado del proveedor —
  que Meltemi jamás ejecuta: el binario oficial se autentica solo (§2).
  Desvincular retira el perfil y **nunca borra el directorio de contexto**:
  las credenciales que el proveedor guardó ahí no son nuestras ni para
  leerlas ni para destruirlas.
- **Persistencia propiedad del daemon**: los vínculos viven en
  `subscriptions.toml` junto al config de usuario, archivo gestionado por la
  máquina y cargado **antes** que `config.toml` — lo escrito a mano gana por
  nombre, y desvincular un perfil manual rehúsa con remedio (edita tu
  config). Nada reescribe archivos del usuario.
- **Las tres superficies** (§4): CLI `link <agente> <nombre>` / `unlink
  <nombre>`; en la GUI la ficha del agente en la Flota ofrece «Vincular
  suscripción» (solo donde el registro declara la variable) con el gesto de
  login a un clic de copiar; en el shell, verbo de paleta con captura
  verbatim del nombre (la línea de paleta minusculiza; el nombre viaja tal
  cual se escribió).
- **Aislamiento advertido**: dos perfiles del mismo agente cuyo contexto
  resuelve al mismo valor son la misma suscripción dos veces; la carga de
  configuración lo diagnostica en voz alta (el patrón de higiene que ya
  rehúsa secretos en claro).
- **Docs**: `docs/agentes.md` gana el flujo vinculado (y conserva el manual),
  con la tabla de variables por proveedor citando su verificación.

## Capabilities

### Modified Capabilities

- `fleet-catalog`: + el vínculo de suscripción de primera clase (link/unlink,
  persistencia propia, lo manual gana), + la variable de contexto como dato
  del registro, + el login compuesto jamás ejecutado, + el duplicado de
  contexto advertido.
- `cli-contract`: + los verbos `link`/`unlink` mapeados a sus métodos, con el
  nombre viajando verbatim.
- `gui-shell`: + vincular desde la ficha del agente en la Flota, gesto de
  login copiable, desvincular sin tocar el contexto.
- `tui-shell`: + el verbo de vínculo con captura verbatim y rehúso con
  remedio visible.

## Impact

- `proto/`: métodos `subscription/link`/`unlink` + params/result + esquema +
  conformidad; `FleetAgent` ya expone perfiles — sin cambios de forma ahí.
- `core/meltemid`: campos nuevos del registro (datos), almacén
  `subscriptions.toml`, handlers, diagnóstico de duplicados; el resolver de
  sesión **no cambia** — un vínculo es un perfil como los que ya honra.
- `tui/`, `desktop/ui`: verbos, flujo de Flota, i18n ES/EN, formularios
  tipados regenerados, matriz de paridad con dos filas nuevas.
- Cero dependencias nuevas. El overlay sigue siendo la única mecánica: esta
  change no toca `acp.rs` ni los adaptadores.

## Fuera de alcance

- **Ejecutar el login** (interactivo, del proveedor) o verificar que un
  contexto quedó autenticado: Meltemi compone el gesto; mirar dentro del
  contexto para «comprobar» violaría §2. El estado visible sigue siendo el
  del catálogo (detectado / no detectado).
- **Cuotas, límites o balanceo entre suscripciones**: vincular no es
  repartir; cualquier orquestación por presupuesto es change futura.
- **Perfiles de agentes custom sin variable declarada**: siguen por TOML
  manual, documentado — la superficie solo ofrece lo que el registro sabe
  componer.
- **Migrar perfiles manuales existentes** al archivo del daemon: lo escrito a
  mano es del usuario y ahí se queda.
