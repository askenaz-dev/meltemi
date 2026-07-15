// SPDX-License-Identifier: Apache-2.0

//! The message table (design D4, constitution §11): every visible string is
//! routed through here in Spanish and English, so no user-facing text is
//! hardcoded at a call site.

/// A user-interface language. Spanish and English are the first two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
}

impl Lang {
    /// Resolves the UI language from a `LANG`-style locale (defaulting to
    /// Spanish, the project's primary artifact language).
    #[must_use]
    pub fn from_locale(locale: Option<&str>) -> Lang {
        match locale {
            Some(l) if l.to_ascii_lowercase().starts_with("en") => Lang::En,
            _ => Lang::Es,
        }
    }
}

/// A visible message key. Every variant has a Spanish and an English form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Tagline,
    Connecting,
    Unreachable,
    NoSessions,
    NoProject,
    NoPermissions,
    NoAgents,
    HintKeys,
    HintExitField,
    QuitConfirm,
    HelpTitle,
    PaletteTitle,
}

/// The text of a message in a language. Never empty.
#[must_use]
pub fn text(msg: Msg, lang: Lang) -> &'static str {
    match (msg, lang) {
        (Msg::Tagline, Lang::Es) => "plano de control spec-driven para agentes",
        (Msg::Tagline, Lang::En) => "spec-driven control plane for agents",
        (Msg::Connecting, Lang::Es) => "conectando...",
        (Msg::Connecting, Lang::En) => "connecting...",
        (Msg::Unreachable, Lang::Es) => "daemon inalcanzable",
        (Msg::Unreachable, Lang::En) => "daemon unreachable",
        (Msg::NoSessions, Lang::Es) => "sin sesiones — inicia trabajo de agente con /propose",
        (Msg::NoSessions, Lang::En) => "no sessions — start agent work with /propose",
        (Msg::NoProject, Lang::Es) => "este directorio no es un proyecto .meltemi/",
        (Msg::NoProject, Lang::En) => "this directory is not a .meltemi/ project",
        (Msg::NoPermissions, Lang::Es) => "sin permisos pendientes",
        (Msg::NoPermissions, Lang::En) => "no pending permissions",
        (Msg::NoAgents, Lang::Es) => "sin agentes detectados",
        (Msg::NoAgents, Lang::En) => "no agents detected",
        (Msg::HintKeys, Lang::Es) => "1-4 vistas | : paleta | ? ayuda | a permisos | q salir",
        (Msg::HintKeys, Lang::En) => "1-4 views | : palette | ? help | a permissions | q quit",
        (Msg::HintExitField, Lang::Es) => "Esc para salir",
        (Msg::HintExitField, Lang::En) => "Esc to exit",
        (Msg::QuitConfirm, Lang::Es) => "¿Salir de meltemi? Enter confirma - Esc cancela",
        (Msg::QuitConfirm, Lang::En) => "Quit meltemi? Enter confirms - Esc cancels",
        (Msg::HelpTitle, Lang::Es) => "Ayuda — mapa de teclas",
        (Msg::HelpTitle, Lang::En) => "Help — key map",
        (Msg::PaletteTitle, Lang::Es) => "Paleta de comandos",
        (Msg::PaletteTitle, Lang::En) => "Command palette",
    }
}

/// Every message key, for the completeness test.
pub const ALL: &[Msg] = &[
    Msg::Tagline,
    Msg::Connecting,
    Msg::Unreachable,
    Msg::NoSessions,
    Msg::NoProject,
    Msg::NoPermissions,
    Msg::NoAgents,
    Msg::HintKeys,
    Msg::HintExitField,
    Msg::QuitConfirm,
    Msg::HelpTitle,
    Msg::PaletteTitle,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_has_both_languages_non_empty() {
        for &m in ALL {
            assert!(!text(m, Lang::Es).is_empty(), "missing ES for {m:?}");
            assert!(!text(m, Lang::En).is_empty(), "missing EN for {m:?}");
        }
    }

    #[test]
    fn language_resolves_from_locale() {
        assert_eq!(Lang::from_locale(Some("en_US.UTF-8")), Lang::En);
        assert_eq!(Lang::from_locale(Some("es_CL.UTF-8")), Lang::Es);
        assert_eq!(Lang::from_locale(None), Lang::Es);
    }
}
