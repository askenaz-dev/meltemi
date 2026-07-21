// SPDX-License-Identifier: Apache-2.0
// The single message catalog (constitution §11): every visible string of the
// desktop surface lives here, ES and EN from day one. The i18n lint refuses
// hardcoded text in templates.

export type Locale = "es" | "en";

const es = {
  "app.title": "Meltemi",

  "nav.sessions": "Sesiones",
  "nav.project": "Proyecto",
  "nav.permissions": "Permisos",
  "nav.fleet": "Flota",
  "nav.viewLabel": "Vistas",
  "nav.breadcrumb": "ruta de navegación",

  "conn.connecting": "conectando…",
  "conn.connected": "conectado",
  "conn.daemon": "daemon",
  "conn.sessions": "{n} sesión(es)",
  "conn.unreachable": "daemon inalcanzable",
  "conn.endpoint": "endpoint",
  "conn.willDeny":
    "sin cliente conectado, los permisos pendientes se denegarán al vencer",
  "conn.sshHint":
    "si el daemon corre en otro host, reenvía su socket local por SSH (meltemi tunnel); el daemon jamás abre un puerto de red",
  "conn.reconnecting": "reconectando…",

  "state.starting": "iniciando",
  "state.active": "activa",
  "state.waiting_permission": "esperando permiso",
  "state.ended": "finalizada",
  "state.interrupted": "interrumpida",

  "common.back": "Atrás",
  "common.close": "Cerrar",
  "common.cancel": "Cancelar",
  "common.confirm": "Confirmar",
  "common.refresh": "Actualizar",
  "common.loading": "cargando…",
  "common.error": "error",
  "common.remedy": "remedio",
  "common.yes": "sí",
  "common.no": "no",
  "common.language": "Idioma",

  "sessions.col.session": "Sesión",
  "sessions.col.agent": "Agente",
  "sessions.col.state": "Estado",
  "sessions.col.project": "Proyecto",
  "sessions.col.started": "Inicio",
  "sessions.empty.title": "Sin sesiones todavía",
  "sessions.empty.hint":
    "Una sesión es un agente trabajando bajo tus specs. Lanza una desde la paleta ( : ) — por ejemplo `propose` — o revisa la flota disponible.",
  "sessions.empty.fleet": "Ver la flota (4)",
  "sessions.resumable": "reanudable",
  "sessions.detail.transcript": "Registro de la sesión",
  "sessions.detail.live": "en vivo",
  "sessions.detail.cut": "— conexión con el daemon cortada aquí —",
  "sessions.detail.goneAfterReconnect":
    "La sesión terminó con la caída del daemon; este registro no se reanudará.",
  "sessions.cancel": "Cancelar sesión",
  "sessions.cancel.warning":
    "Cancelar termina el subproceso del agente y finaliza la sesión (no solo el turno). Es irreversible.",
  "sessions.direct": "Dirigir",
  "sessions.direct.hint": "instrucción para el siguiente turno",

  "project.title": "Proyecto",
  "project.changes": "Changes",
  "project.specs": "Specs (verdad viva)",
  "project.col.change": "Change",
  "project.col.tasks": "Tareas",
  "project.col.review": "Review",
  "project.col.verify": "Verify",
  "project.col.state": "Estado",
  "project.archived": "archivada",
  "project.active": "activa",
  "project.col.capability": "Capacidad",
  "project.col.requirements": "Requisitos",
  "project.col.scenarios": "Escenarios",
  "project.validate": "Validar la verdad viva",
  "project.validate.clean": "validate — limpio",
  "project.validate.findings": "{n} hallazgo(s)",
  "project.empty.title": "Este directorio no es un proyecto Meltemi",
  "project.empty.hint":
    "No hay `.meltemi/` aquí. Inicializa la constitución y el andamiaje con `constitution` desde la paleta ( : ). Pilotar agentes no exige un proyecto: Sesiones, Permisos y Flota siguen operativas.",

  "permissions.title": "Bandeja de permisos",
  "permissions.empty.title": "Sin permisos pendientes",
  "permissions.empty.hint":
    "Cuando un agente pida autorización, la petición esperará aquí. Sin decisión, vence y se deniega — nunca en silencio.",
  "permissions.waiting": "{n} esperando",
  "permissions.tool": "operación",
  "permissions.session": "sesión",
  "permissions.waitingFor": "esperando hace {s} s",
  "permissions.expiresIn": "vence en {s} s",
  "permissions.expired": "vencida",
  "permissions.timeout.notice":
    "permiso vencido: denegado por plazo (sesión {session}, operación {tool})",
  "permissions.decided": "decisión enviada",
  "permissions.persistRule": "guardar la regla sugerida al decidir",

  "fleet.title": "Flota",
  "fleet.col.agent": "Agente",
  "fleet.col.source": "Origen",
  "fleet.col.level": "Nivel",
  "fleet.col.detected": "Detectado",
  "fleet.col.configured": "Configurado",
  "fleet.source.registry": "registro",
  "fleet.source.custom": "propio",
  "fleet.source.profile": "perfil",
  "fleet.level.verified": "verificado",
  "fleet.level.declared": "declarado",
  "fleet.underlying": "sobre {agent}",
  "fleet.empty.title": "Sin agentes detectados",
  "fleet.empty.hint":
    "Meltemi dirige los binarios oficiales que ya tienes (BYO-agent). Instala tu agente CLI o declara uno propio en la config del proyecto ([[fleet.custom]]).",

  "palette.placeholder": "método o vista… (Esc para salir)",
  "palette.title": "Paleta de comandos",
  "palette.params": "Parámetros (JSON)",
  "palette.run": "Invocar",
  "palette.result": "Resultado",
  "palette.error": "Error",
  "palette.dangerous":
    "Esta operación afecta a todas las sesiones activas y requiere confirmación.",
  "palette.hint": "toda capacidad del daemon es alcanzable desde aquí",
  "palette.nav": "abrir vista",

  "banner.daemonDown": "daemon inalcanzable",
  "banner.retrying": "reintentando con backoff…",

  "notices.title": "Avisos",
  "notices.dismiss": "Descartar",

  "confirm.title": "Confirmación",

  "shutdown.label": "Apagar el daemon",
  "shutdown.warning":
    "Apagar el daemon afecta a todas las sesiones activas. ¿Continuar?",

  "onboarding.title": "Bienvenido a Meltemi",
  "onboarding.intro":
    "Un rumbo, muchas velas: specs claras dirigiendo a tus agentes de codificación.",
  "onboarding.views":
    "Cuatro vistas: Sesiones (1), Proyecto (2), Permisos (3) y Flota (4).",
  "onboarding.palette":
    "La paleta ( : o Ctrl+K) alcanza toda capacidad del daemon, tenga o no vista dedicada.",
  "onboarding.permissions":
    "La tecla `a` salta a la bandeja de permisos desde cualquier vista.",
  "onboarding.help": "Reábreme cuando quieras con `?`.",
  "onboarding.skip": "Empezar (Esc)",

  "help.title": "Ayuda y atajos",
  "help.keys": "Teclas: 1–4 vistas · : o Ctrl+K paleta · a permisos · Esc atrás/cerrar · ? esta ayuda",

  // Palette descriptions: one per client-invocable contract method (paridad).
  "palette.m.status": "estado del daemon: versión, uptime, sesiones",
  "palette.m.shutdown": "apagado ordenado del daemon",
  "palette.m.propose": "andamiar una propuesta de change y delegarla al agente",
  "palette.m.fleet.list": "catálogo de flota: agentes, detección y niveles",
  "palette.m.context.project": "regenerar el contexto proyectado (AGENTS.md, …)",
  "palette.m.session.list": "listar sesiones (activas e históricas)",
  "palette.m.session.log": "leer el registro JSONL de una sesión",
  "palette.m.repo.map": "árbol del repositorio honrando gitignore",
  "palette.m.sdd.constitution": "crear o editar la constitución del proyecto",
  "palette.m.sdd.explore": "deliberar con el agente sin escribir",
  "palette.m.sdd.propose": "iniciar el ciclo de autoría SDD de una change",
  "palette.m.sdd.plan": "refinar design y secuenciar tareas",
  "palette.m.sdd.gate": "decidir una compuerta de autoría pendiente",
  "palette.m.sdd.review": "checklist de revisión de los deltas de una change",
  "palette.m.sdd.review-decide": "decidir un ítem de la revisión",
  "palette.m.session.cancel": "cancelar una sesión activa (notificación)",
  "palette.m.session.direct": "dirigir una instrucción a una sesión existente",
  "palette.m.permission.pending": "cola de permisos pendientes",
  "palette.m.permission.decide": "resolver un permiso pendiente por id",
  "palette.m.worktree.assign": "crear worktrees aislados por agente para tareas",
  "palette.m.worktree.list": "listar los worktrees gestionados",
  "palette.m.worktree.remove": "retirar un worktree gestionado",
  "palette.m.worktree.diff": "diff de cada competidor contra la base común",
  "palette.m.worktree.merge-file": "aplicar un archivo de un worktree a otro (fusión asistida)",
  "palette.m.worktree.dispatch": "correr el turno de un competidor con su propio binario",
  "palette.m.checkpoint.create": "crear el checkpoint pre-tarea de un worktree",
  "palette.m.checkpoint.list": "listar checkpoints por change y tarea",
  "palette.m.checkpoint.revert": "revertir el worktree de una tarea a su checkpoint",
  "palette.m.checkpoint.record-op": "registrar una operación externa aprobada (alcance honesto)",
  "palette.m.commit.task": "el commit atómico por tarea con trazabilidad",
  "palette.m.sdd.verify": "checklist de verificación por requisito",
  "palette.m.sdd.verify-mark": "registrar una verificación manual con nota",
  "palette.m.sdd.archive": "plegar los deltas verificados en la verdad viva",
  "palette.m.sdd.implement": "desplegar al agente sobre tasks.md, tarea a tarea",
  "palette.m.change.list": "listar changes (activas y archivadas) con su estado",
  "palette.m.change.show": "mostrar una change: artefactos y deltas",
  "palette.m.spec.list": "listar capacidades de la verdad viva",
  "palette.m.spec.show": "mostrar una capacidad, sus requisitos y escenarios",
  "palette.m.sdd.validate": "validar una change o la verdad viva",
} as const;

const en: Record<MessageKey, string> = {
  "app.title": "Meltemi",

  "nav.sessions": "Sessions",
  "nav.project": "Project",
  "nav.permissions": "Permissions",
  "nav.fleet": "Fleet",
  "nav.viewLabel": "Views",
  "nav.breadcrumb": "breadcrumb",

  "conn.connecting": "connecting…",
  "conn.connected": "connected",
  "conn.daemon": "daemon",
  "conn.sessions": "{n} session(s)",
  "conn.unreachable": "daemon unreachable",
  "conn.endpoint": "endpoint",
  "conn.willDeny":
    "with no client connected, pending permissions are denied on expiry",
  "conn.sshHint":
    "if the daemon runs on another host, forward its local socket over SSH (meltemi tunnel); the daemon never opens a network port",
  "conn.reconnecting": "reconnecting…",

  "state.starting": "starting",
  "state.active": "active",
  "state.waiting_permission": "waiting permission",
  "state.ended": "ended",
  "state.interrupted": "interrupted",

  "common.back": "Back",
  "common.close": "Close",
  "common.cancel": "Cancel",
  "common.confirm": "Confirm",
  "common.refresh": "Refresh",
  "common.loading": "loading…",
  "common.error": "error",
  "common.remedy": "remedy",
  "common.yes": "yes",
  "common.no": "no",
  "common.language": "Language",

  "sessions.col.session": "Session",
  "sessions.col.agent": "Agent",
  "sessions.col.state": "State",
  "sessions.col.project": "Project",
  "sessions.col.started": "Started",
  "sessions.empty.title": "No sessions yet",
  "sessions.empty.hint":
    "A session is an agent working under your specs. Launch one from the palette ( : ) — for example `propose` — or review the available fleet.",
  "sessions.empty.fleet": "See the fleet (4)",
  "sessions.resumable": "resumable",
  "sessions.detail.transcript": "Session log",
  "sessions.detail.live": "live",
  "sessions.detail.cut": "— daemon connection lost here —",
  "sessions.detail.goneAfterReconnect":
    "The session ended with the daemon crash; this log will not resume.",
  "sessions.cancel": "Cancel session",
  "sessions.cancel.warning":
    "Cancelling terminates the agent subprocess and ends the session (not just the turn). This is irreversible.",
  "sessions.direct": "Direct",
  "sessions.direct.hint": "instruction for the next turn",

  "project.title": "Project",
  "project.changes": "Changes",
  "project.specs": "Specs (living truth)",
  "project.col.change": "Change",
  "project.col.tasks": "Tasks",
  "project.col.review": "Review",
  "project.col.verify": "Verify",
  "project.col.state": "State",
  "project.archived": "archived",
  "project.active": "active",
  "project.col.capability": "Capability",
  "project.col.requirements": "Requirements",
  "project.col.scenarios": "Scenarios",
  "project.validate": "Validate the living truth",
  "project.validate.clean": "validate — clean",
  "project.validate.findings": "{n} finding(s)",
  "project.empty.title": "This directory is not a Meltemi project",
  "project.empty.hint":
    "There is no `.meltemi/` here. Initialize the constitution and scaffolding with `constitution` from the palette ( : ). Piloting agents does not require a project: Sessions, Permissions and Fleet stay fully usable.",

  "permissions.title": "Permission tray",
  "permissions.empty.title": "No pending permissions",
  "permissions.empty.hint":
    "When an agent asks for authorization, the request waits here. Undecided, it expires and is denied — never silently.",
  "permissions.waiting": "{n} waiting",
  "permissions.tool": "operation",
  "permissions.session": "session",
  "permissions.waitingFor": "waiting for {s} s",
  "permissions.expiresIn": "expires in {s} s",
  "permissions.expired": "expired",
  "permissions.timeout.notice":
    "permission expired: denied on deadline (session {session}, operation {tool})",
  "permissions.decided": "decision sent",
  "permissions.persistRule": "save the suggested rule when deciding",

  "fleet.title": "Fleet",
  "fleet.col.agent": "Agent",
  "fleet.col.source": "Source",
  "fleet.col.level": "Level",
  "fleet.col.detected": "Detected",
  "fleet.col.configured": "Configured",
  "fleet.source.registry": "registry",
  "fleet.source.custom": "custom",
  "fleet.source.profile": "profile",
  "fleet.level.verified": "verified",
  "fleet.level.declared": "declared",
  "fleet.underlying": "over {agent}",
  "fleet.empty.title": "No agents detected",
  "fleet.empty.hint":
    "Meltemi drives the official binaries you already have (BYO-agent). Install your CLI agent or declare a custom one in the project config ([[fleet.custom]]).",

  "palette.placeholder": "method or view… (Esc to leave)",
  "palette.title": "Command palette",
  "palette.params": "Parameters (JSON)",
  "palette.run": "Invoke",
  "palette.result": "Result",
  "palette.error": "Error",
  "palette.dangerous":
    "This operation affects every active session and requires confirmation.",
  "palette.hint": "every daemon capability is reachable from here",
  "palette.nav": "open view",

  "banner.daemonDown": "daemon unreachable",
  "banner.retrying": "retrying with backoff…",

  "notices.title": "Notices",
  "notices.dismiss": "Dismiss",

  "confirm.title": "Confirmation",

  "shutdown.label": "Shut the daemon down",
  "shutdown.warning":
    "Shutting the daemon down affects every active session. Continue?",

  "onboarding.title": "Welcome to Meltemi",
  "onboarding.intro":
    "One course, many sails: clear specs steering your coding agents.",
  "onboarding.views":
    "Four views: Sessions (1), Project (2), Permissions (3) and Fleet (4).",
  "onboarding.palette":
    "The palette ( : or Ctrl+K) reaches every daemon capability, with or without a dedicated view.",
  "onboarding.permissions":
    "The `a` key jumps to the permission tray from any view.",
  "onboarding.help": "Reopen me anytime with `?`.",
  "onboarding.skip": "Start (Esc)",

  "help.title": "Help & shortcuts",
  "help.keys": "Keys: 1–4 views · : or Ctrl+K palette · a permissions · Esc back/close · ? this help",

  "palette.m.status": "daemon status: version, uptime, sessions",
  "palette.m.shutdown": "orderly daemon shutdown",
  "palette.m.propose": "scaffold a change proposal and delegate it to the agent",
  "palette.m.fleet.list": "fleet catalog: agents, detection and levels",
  "palette.m.context.project": "regenerate the projected context (AGENTS.md, …)",
  "palette.m.session.list": "list sessions (active and historical)",
  "palette.m.session.log": "read a session's JSONL log",
  "palette.m.repo.map": "repository tree honoring gitignore",
  "palette.m.sdd.constitution": "create or edit the project constitution",
  "palette.m.sdd.explore": "deliberate with the agent without writing",
  "palette.m.sdd.propose": "start a change's SDD authoring cycle",
  "palette.m.sdd.plan": "refine design and sequence tasks",
  "palette.m.sdd.gate": "decide a pending authoring gate",
  "palette.m.sdd.review": "the review checklist of a change's spec deltas",
  "palette.m.sdd.review-decide": "decide one review checklist item",
  "palette.m.session.cancel": "cancel an active session (notification)",
  "palette.m.session.direct": "direct an instruction to an existing session",
  "palette.m.permission.pending": "the pending permission queue",
  "palette.m.permission.decide": "resolve a pending permission by id",
  "palette.m.worktree.assign": "create isolated per-agent worktrees for tasks",
  "palette.m.worktree.list": "list the managed worktrees",
  "palette.m.worktree.remove": "remove a managed worktree",
  "palette.m.worktree.diff": "each competitor's diff against the common base",
  "palette.m.worktree.merge-file": "apply one file across worktrees (assisted merge)",
  "palette.m.worktree.dispatch": "run one competitor's turn with its own binary",
  "palette.m.checkpoint.create": "create a worktree's pre-task checkpoint",
  "palette.m.checkpoint.list": "list checkpoints by change and task",
  "palette.m.checkpoint.revert": "revert a task's worktree to its checkpoint",
  "palette.m.checkpoint.record-op": "record an approved external operation (honest scope)",
  "palette.m.commit.task": "the atomic per-task commit with traceability",
  "palette.m.sdd.verify": "the per-requirement verification checklist",
  "palette.m.sdd.verify-mark": "record a manual scenario verification with a note",
  "palette.m.sdd.archive": "fold verified deltas into the living truth",
  "palette.m.sdd.implement": "deploy the agent over tasks.md, task by task",
  "palette.m.change.list": "list changes (active and archived) with state",
  "palette.m.change.show": "show a change: artifacts and deltas",
  "palette.m.spec.list": "list the living-truth capabilities",
  "palette.m.spec.show": "show a capability, its requirements and scenarios",
  "palette.m.sdd.validate": "validate a change or the living truth",
};

export type MessageKey = keyof typeof es;

export const messages: Record<Locale, Record<MessageKey, string>> = { es, en };
