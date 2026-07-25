## Why

El modelo mental del mantenedor es el correcto y el daemon ya lo soporta; las
superficies no lo muestran. Hoy: los perfiles de lanzamiento
(`[[fleet.profile]]`, flota-multiproveedor) ya permiten múltiples
suscripciones —incluso dos de Claude— redirigiendo el contexto de
autenticación del binario oficial, y las sesiones ya se resuelven por
agente/perfil. Pero la GUI y la TUI están ancladas a UN proyecto (el cwd), los
perfiles solo se ven como filas más de la flota, y no existe el árbol
Proyecto → Sesiones (agente · suscripción) que el usuario describe:

    Proyecto A: sesión con Claude (suscripción A), sesión con Codex,
                sesión con Claude (suscripción B)
    Proyecto B: sesión con Codex, sesión con Claude (suscripción B)

El daemon es uno por usuario y ya distingue `project_root` por sesión: falta
que las superficies dejen de esconderlo.

## What Changes

- **Multiproyecto de primera clase**: el daemon gana la noción de proyectos
  recientes/registrados (persistida en su directorio de datos, alimentada por
  el uso real); `session/list` sin filtro ya es global — se añade la
  agregación por proyecto que las superficies consumen.
- **Sidebar con árbol Proyecto → Sesiones** en la GUI (sobre el shell de
  `gui-clase-mundial`): conmutador y árbol de proyectos con sus sesiones
  vivas, cada una con agente + perfil (suscripción) visible con su avatar;
  la TUI gana el equivalente (vista Sesiones agrupada por proyecto y filtro).
- **Suscripciones como concepto visible**: los perfiles se presentan como
  "suscripciones/perfiles" con nombre en el lanzador de sesión, el drawer y
  las listas — el binario y el contexto de auth que ya registra el log de
  sesión, ahora legibles de un vistazo.
- **Lanzador multiproyecto**: "Nueva sesión" (gui-clase-mundial) gana el
  selector de proyecto además de agente/perfil y modo.
- **Guía**: `docs/agentes.md` (flota-deteccion-guia) gana la sección de
  perfiles multi-suscripción con el ejemplo canónico de dos cuentas Claude.

## Capabilities

### New Capabilities
- _Por decidir en design_: la agregación multiproyecto puede caber como
  requisitos nuevos en `session-history`/`fleet-catalog` o merecer capacidad
  propia (`project-registry`); se resuelve con el design.

### Modified Capabilities
- `session-history`: + listado global agregado por proyecto; + proyectos
  recientes persistidos.
- `gui-shell` y `tui-shell`: + árbol proyecto→sesiones, + perfil visible por
  sesión, + selector de proyecto en el lanzador.

## Impact

- `core/meltemid` (registro de proyectos recientes, agregación), `proto/`
  (aditivo), `tui/`, `desktop/ui`. Depende de `gui-clase-mundial` (sidebar) y
  de que `gui-tauri-paridad` esté archivada.
- E2e: dos fixtures-proyecto simultáneos con sesiones de perfiles distintos
  del mock; el árbol agregado refleja ambos.

## Fuera de alcance

- Cuota/costo por suscripción (constitución §2/§9): eso es
  `analitica-consumo-local`, y solo con contabilidad local.
- Credenciales: jamás — los perfiles solo redirigen contexto; el binario se
  autentica solo (§2).
- Workspaces multi-repo de equipo (fase 3, funciones de equipo).
