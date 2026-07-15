// SPDX-License-Identifier: Apache-2.0

//! The command palette registry (design D7): every capability reachable by
//! typing, even before it has a dedicated key or view. This is the core-parity
//! catch-all; new daemon methods are registered here so nothing lacks a home.

use crate::shell::messages::Lang;

/// One palette entry: a typeable command name and whether it is reserved (a
/// future capability announced, never an error).
pub struct Entry {
    pub name: &'static str,
    pub reserved: bool,
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

/// The registry, mirroring the CLI grammar and the daemon capabilities. Reserved
/// SDD verbs are listed so the palette announces them rather than erroring.
pub const ENTRIES: &[Entry] = &[
    Entry {
        name: "status",
        reserved: false,
        desc_es: "refrescar el estado del daemon",
        desc_en: "refresh daemon status",
    },
    Entry {
        name: "shutdown",
        reserved: false,
        desc_es: "apagar el daemon (confirma)",
        desc_en: "shut down the daemon (confirms)",
    },
    Entry {
        name: "quit",
        reserved: false,
        desc_es: "salir de meltemi (confirma)",
        desc_en: "quit meltemi (confirms)",
    },
    Entry {
        name: "propose",
        reserved: true,
        desc_es: "proponer un cambio (ciclo SDD)",
        desc_en: "propose a change (SDD cycle)",
    },
    Entry {
        name: "explore",
        reserved: true,
        desc_es: "explorar (reservado)",
        desc_en: "explore (reserved)",
    },
    Entry {
        name: "review",
        reserved: true,
        desc_es: "revisar specs (reservado)",
        desc_en: "review specs (reserved)",
    },
    Entry {
        name: "plan",
        reserved: true,
        desc_es: "planificar (reservado)",
        desc_en: "plan (reserved)",
    },
    Entry {
        name: "implement",
        reserved: true,
        desc_es: "implementar (reservado)",
        desc_en: "implement (reserved)",
    },
    Entry {
        name: "verify",
        reserved: true,
        desc_es: "verificar (reservado)",
        desc_en: "verify (reserved)",
    },
    Entry {
        name: "archive",
        reserved: true,
        desc_es: "archivar (reservado)",
        desc_en: "archive (reserved)",
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
}
