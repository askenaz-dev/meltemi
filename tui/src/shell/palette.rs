// SPDX-License-Identifier: Apache-2.0

//! The command palette registry (design D7): every capability reachable by
//! typing, even before it has a dedicated key or view. This is the core-parity
//! catch-all: every client-invocable method of the daemon contract is declared
//! by exactly one entry's `methods` list, and the parity gate
//! (`tui/tests/parity.rs`, gui-tauri-paridad design D3) fails when a contract
//! method has no home here, in the GUI registry or in `docs/paridad-nucleo.md`.

use meltemi_proto::methods as m;

use crate::shell::messages::Lang;

/// One palette entry: a typeable command name, whether its interactive wiring
/// is still reserved (announced, never an error), and the contract methods it
/// gives a home to.
pub struct Entry {
    pub name: &'static str,
    pub reserved: bool,
    /// The client-invocable contract methods this entry exercises.
    pub methods: &'static [&'static str],
    desc_es: &'static str,
    desc_en: &'static str,
}

impl Entry {
    /// The localized description.
    #[must_use]
    pub fn desc(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Es => self.desc_es,
            Lang::En => self.desc_en,
        }
    }
}

/// The registry, mirroring the CLI grammar and the daemon capabilities.
/// Reserved verbs are listed so the palette announces them rather than erroring.
pub const ENTRIES: &[Entry] = &[
    Entry {
        name: "status",
        reserved: false,
        methods: &[m::STATUS],
        desc_es: "refrescar el estado del daemon",
        desc_en: "refresh daemon status",
    },
    Entry {
        name: "fleet",
        reserved: false,
        methods: &[m::FLEET_LIST],
        desc_es: "abrir la Flota y refrescar el catálogo",
        desc_en: "open the Fleet and refresh the catalog",
    },
    Entry {
        name: "project",
        reserved: false,
        methods: &[m::CONTEXT_PROJECT],
        desc_es: "regenerar el contexto proyectado (AGENTS.md, ...)",
        desc_en: "regenerate the projected context (AGENTS.md, ...)",
    },
    Entry {
        name: "sessions",
        reserved: false,
        methods: &[m::SESSION_LIST, m::SESSION_LOG],
        desc_es: "ir a Sesiones (activas e históricas)",
        desc_en: "go to Sessions (active and historical)",
    },
    Entry {
        name: "permissions",
        reserved: false,
        methods: &[m::PERMISSION_PENDING, m::PERMISSION_DECIDE],
        desc_es: "abrir la bandeja de permisos",
        desc_en: "open the permission tray",
    },
    Entry {
        name: "cancel",
        reserved: false,
        methods: &[m::SESSION_CANCEL],
        desc_es: "cancelar la sesión activa (confirma)",
        desc_en: "cancel the active session (confirms)",
    },
    Entry {
        name: "shutdown",
        reserved: false,
        methods: &[m::SHUTDOWN],
        desc_es: "apagar el daemon (confirma)",
        desc_en: "shut down the daemon (confirms)",
    },
    Entry {
        name: "quit",
        reserved: false,
        methods: &[],
        desc_es: "salir de meltemi (confirma)",
        desc_en: "quit meltemi (confirms)",
    },
    Entry {
        name: "propose",
        reserved: true,
        methods: &[m::PROPOSE, m::SDD_PROPOSE],
        desc_es: "proponer un cambio (ciclo SDD)",
        desc_en: "propose a change (SDD cycle)",
    },
    Entry {
        name: "gate",
        reserved: true,
        methods: &[m::SDD_GATE],
        desc_es: "decidir una compuerta de autoría pendiente",
        desc_en: "decide a pending authoring gate",
    },
    Entry {
        name: "explore",
        reserved: false,
        methods: &[m::SDD_EXPLORE],
        desc_es: "deliberar con el agente sin escribir",
        desc_en: "deliberate with the agent without writing",
    },
    Entry {
        name: "constitution",
        reserved: false,
        methods: &[m::SDD_CONSTITUTION],
        desc_es: "crear/editar la constitución (gate)",
        desc_en: "create/edit the constitution (gate)",
    },
    Entry {
        name: "plan",
        reserved: false,
        methods: &[m::SDD_PLAN],
        desc_es: "secuenciar tareas de una change (gate)",
        desc_en: "sequence a change's tasks (gate)",
    },
    Entry {
        name: "review",
        reserved: true,
        methods: &[m::SDD_REVIEW, m::SDD_REVIEW_DECIDE],
        desc_es: "revisar los deltas de una change como checklist",
        desc_en: "review a change's deltas as a checklist",
    },
    Entry {
        name: "implement",
        reserved: true,
        methods: &[m::SDD_IMPLEMENT],
        desc_es: "desplegar al agente sobre tasks.md",
        desc_en: "deploy the agent over tasks.md",
    },
    Entry {
        name: "verify",
        reserved: true,
        methods: &[m::SDD_VERIFY, m::SDD_VERIFY_MARK],
        desc_es: "checklist de verificación por requisito",
        desc_en: "the per-requirement verification checklist",
    },
    Entry {
        name: "archive",
        reserved: true,
        methods: &[m::SDD_ARCHIVE],
        desc_es: "plegar deltas verificados en la verdad viva",
        desc_en: "fold verified deltas into the living truth",
    },
    Entry {
        name: "validate",
        reserved: true,
        methods: &[m::SDD_VALIDATE],
        desc_es: "validar una change o la verdad viva",
        desc_en: "validate a change or the living truth",
    },
    Entry {
        name: "projects",
        reserved: false,
        methods: &[m::PROJECT_LIST],
        desc_es: "proyectos conocidos; `projects <texto>` acota el ambito",
        desc_en: "known projects; `projects <text>` narrows the scope",
    },
    Entry {
        name: "changes",
        reserved: true,
        methods: &[m::CHANGE_LIST],
        desc_es: "listar changes con su estado agregado",
        desc_en: "list changes with aggregated state",
    },
    Entry {
        name: "show",
        reserved: true,
        methods: &[m::CHANGE_SHOW],
        desc_es: "mostrar una change: artefactos y deltas",
        desc_en: "show a change: artifacts and deltas",
    },
    Entry {
        name: "specs",
        reserved: true,
        methods: &[m::SPEC_LIST, m::SPEC_SHOW],
        desc_es: "capacidades de la verdad viva",
        desc_en: "the living-truth capabilities",
    },
    Entry {
        name: "worktrees",
        reserved: true,
        methods: &[m::WORKTREE_LIST],
        desc_es: "listar los worktrees gestionados",
        desc_en: "list the managed worktrees",
    },
    Entry {
        name: "assign",
        reserved: true,
        methods: &[m::WORKTREE_ASSIGN],
        desc_es: "crear worktrees aislados por agente",
        desc_en: "create isolated per-agent worktrees",
    },
    Entry {
        name: "race",
        reserved: true,
        methods: &[m::WORKTREE_DIFF],
        desc_es: "diff de cada competidor contra la base común",
        desc_en: "each competitor's diff against the common base",
    },
    Entry {
        name: "dispatch",
        reserved: true,
        methods: &[m::WORKTREE_DISPATCH],
        desc_es: "correr el turno de un competidor con su binario",
        desc_en: "run one competitor's turn with its own binary",
    },
    Entry {
        name: "apply-edit",
        reserved: true,
        methods: &[m::WORKTREE_APPLY_EDIT],
        desc_es: "aplicar una edición humana trazable vía el daemon",
        desc_en: "apply a traceable human edit through the daemon",
    },
    Entry {
        name: "merge",
        reserved: true,
        methods: &[m::WORKTREE_MERGE_FILE],
        desc_es: "aplicar un archivo entre worktrees (fusión asistida)",
        desc_en: "apply one file across worktrees (assisted merge)",
    },
    Entry {
        name: "worktree-remove",
        reserved: true,
        methods: &[m::WORKTREE_REMOVE],
        desc_es: "retirar un worktree gestionado",
        desc_en: "remove a managed worktree",
    },
    Entry {
        name: "checkpoints",
        reserved: true,
        methods: &[
            m::CHECKPOINT_LIST,
            m::CHECKPOINT_CREATE,
            m::CHECKPOINT_RECORD_OP,
        ],
        desc_es: "checkpoints pre-tarea (listar, crear, registrar op externa)",
        desc_en: "pre-task checkpoints (list, create, record external op)",
    },
    Entry {
        name: "revert",
        reserved: true,
        methods: &[m::CHECKPOINT_REVERT],
        desc_es: "revertir el worktree de una tarea a su checkpoint",
        desc_en: "revert a task's worktree to its checkpoint",
    },
    Entry {
        name: "commit",
        reserved: true,
        methods: &[m::COMMIT_TASK],
        desc_es: "el commit atómico por tarea con trazabilidad",
        desc_en: "the atomic per-task commit with traceability",
    },
    Entry {
        name: "direct",
        reserved: true,
        methods: &[m::SESSION_DIRECT],
        desc_es: "dirigir una instrucción a una sesión existente",
        desc_en: "direct an instruction to an existing session",
    },
    Entry {
        name: "map",
        reserved: true,
        methods: &[m::REPO_MAP],
        desc_es: "árbol del repositorio honrando gitignore",
        desc_en: "repository tree honoring gitignore",
    },
];

/// The entries whose name contains the (case-insensitive) query.
#[must_use]
pub fn matches(query: &str) -> Vec<&'static Entry> {
    let q = query.trim().to_ascii_lowercase();
    ENTRIES
        .iter()
        .filter(|e| q.is_empty() || e.name.contains(&q))
        .collect()
}

/// The union of contract methods registered across all entries — the TUI side
/// of the core-parity matrix (gui-tauri-paridad design D3).
#[must_use]
pub fn registered_methods() -> std::collections::BTreeSet<&'static str> {
    ENTRIES
        .iter()
        .flat_map(|e| e.methods.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_lists_everything() {
        assert_eq!(matches("").len(), ENTRIES.len());
    }

    #[test]
    fn query_filters_by_substring() {
        let names: Vec<&str> = matches("re").iter().map(|e| e.name).collect();
        assert!(names.contains(&"review"));
        assert!(!names.contains(&"status"));
    }

    #[test]
    fn reserved_verbs_are_present_for_discovery() {
        // Core parity: reserved capabilities are reachable/announced, not errors.
        assert!(ENTRIES.iter().any(|e| e.name == "archive" && e.reserved));
    }

    #[test]
    fn fleet_is_registered_as_an_operational_command() {
        // Obligación viva de tui-shell: todo método nuevo se registra.
        assert!(ENTRIES.iter().any(|e| e.name == "fleet" && !e.reserved));
    }

    #[test]
    fn every_entry_method_is_unique_to_one_entry() {
        // One home per method keeps the matrix unambiguous.
        let mut seen = std::collections::BTreeSet::new();
        for entry in ENTRIES {
            for method in entry.methods {
                assert!(seen.insert(*method), "method {method} declared twice");
            }
        }
    }
}
