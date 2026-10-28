# aviso-de-espera-opt-in

> Vía completa (proposal → design → specs → tasks): el contenido exacto del
> aviso tiene que estar **especificado públicamente antes de existir**
> (constitución §9, spec `remote-access`), y el design es donde se escribe.
> Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas MOB-3 y R6).
> Depende de que `avisos-de-escritorio` esté archivada y aprovecha
> `estado-en-vivo`.

## Why

«Started Orca and Claude building it, then went out shopping. Exciting to
get the notification on my phone.» Es el testimonio que Orca pone en su
portada, y lo consigue con un relé en la nube y push sin túnel — las dos
cosas que la constitución §3 y la spec `remote-access` («Sin push sin túnel
— y por qué») rechazan por escrito.

Pero la misma spec deja **una** puerta abierta, y está sin usar: «el usuario
MAY configurar un aviso de espera opt-in: cuando algo queda esperando su
decisión, un cliente conectado o un proceso del propio usuario —nunca el
daemon— MAY emitir un aviso mínimo a un endpoint que el usuario opera».
Desactivado por defecto, contenido especificado antes de existir, sin
contenido del proyecto. `avisos-de-escritorio` (11/11) cubre el escritorio
con la app abierta; nada cruza hasta un teléfono. Y hoy el único push de
facto es `MELTEMI_BELL=1` sobre `ssh -t`, que las apps de terminal convierten
en notificación local — útil, indocumentado, y solo con la TUI en pantalla.

## What Changes

- **`[notices] on-waiting = "<comando>"`, de ámbito usuario únicamente**
  (`<config>/meltemi/config.toml`; jamás en el `.meltemi/` de un proyecto: un
  repositorio no puede hacer que tu cliente ejecute nada). Lo ejecuta el
  cliente conectado —GUI o TUI— cuando el daemon señala que algo espera al
  humano: permiso, instrucción o gate. **El daemon nunca lo ejecuta ni lo
  lee** (§3).
- **Contrato del contenido, publicado antes**: el comando recibe por
  variables de entorno solo el hecho (`MELTEMI_WAITING=permission|instruction|gate`),
  un identificador opaco de sesión, el nombre de la máquina y la hora.
  Ninguna ruta, ningún texto de petición, ningún nombre de proyecto. El
  design lo escribe como contrato y `docs/acceso-remoto.md` lo publica con
  la change, no después.
- **Coalescencia y prueba**: un aviso por evento de espera, no uno por
  petición encolada; «Enviar aviso de prueba» en Ajustes y en la paleta;
  registro local en el cliente de cuándo disparó (no en el log de sesión,
  que es del daemon).
- **Recetas con infraestructura del usuario**: ntfy autohospedado, Gotify,
  Apprise, y la campana de la TUI como el caso sin servidor; con la
  advertencia de que cada una es del usuario, como su `ssh`.
- **Paridad ×3**: la GUI y la TUI lo ejecutan; el CLI lo expone con
  `meltemi watch` (de `verbos-del-agent-boss`) como pieza de script, sin
  RPC nuevo.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `remote-access`: + requisito «Emisor del aviso de espera en el cliente,
  con contrato de contenido mínimo y ámbito usuario» (cumple lo que el
  requisito «Frontera honesta del acceso remoto» ya permite).
- `gui-shell` y `tui-shell`: + configuración y prueba del aviso.

## Impact

- Archivos: `core/meltemi-client/src/` (lectura de config de usuario),
  `desktop/ui/src/lib/{attention.ts, notices.ts, views/Settings.svelte}`,
  `desktop/src/` (lanzar el comando desde el backend Rust, no desde el
  webview), `tui/src/shell/bell.rs`, `docs/acceso-remoto.md`.
- Cero dependencias nuevas; cero RPC; el daemon no cambia.
- Riesgo real: un comando que cuelga bloquea el cliente. Se lanza sin
  esperar, con tope de tiempo y sin heredar el stdio del cliente.

## Fuera de alcance

- Cualquier relé, puerto o push desde el daemon: constitución §3.
- Contenido del proyecto en el aviso, aunque el usuario lo pida: §9 y la
  spec lo fijan; ampliarlo exige enmienda.
- El compañero móvil que recibe el aviso con su propia app:
  `companero-movil`.
