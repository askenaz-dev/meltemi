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
    FleetLoading,
    FleetByoHint,
    TrayHint,
    TrayFatigueHint,
    HintKeys,
    HintExitField,
    QuitConfirm,
    HelpTitle,
    PaletteTitle,
    OnboardingTitle,
    OnboardingBody,
    DisconnectBanner,
    SizeFloor,
    DirectTitle,
    DirectHint,
    DirectNoSession,
    DirectQueued,
    DirectResumed,
    DirectRefused,
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
        (Msg::FleetLoading, Lang::Es) => "consultando la flota...",
        (Msg::FleetLoading, Lang::En) => "querying the fleet...",
        (Msg::FleetByoHint, Lang::Es) => {
            "trae tu propio agente: instala su CLI oficial (con su propia auth) \
             o declara uno en config con [[fleet.custom]]"
        }
        (Msg::FleetByoHint, Lang::En) => {
            "bring your own agent: install its official CLI (with its own auth) \
             or declare one in config with [[fleet.custom]]"
        }
        (Msg::TrayHint, Lang::Es) => "Enter aprueba | d deniega | r crear regla | j/k mover",
        (Msg::TrayHint, Lang::En) => "Enter approves | d denies | r make rule | j/k move",
        (Msg::TrayFatigueHint, Lang::Es) => {
            "· sugerencia: r crea una regla para no volver a preguntar esto"
        }
        (Msg::TrayFatigueHint, Lang::En) => {
            "· suggestion: r makes a rule so this is never asked again"
        }
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
        (Msg::OnboardingTitle, Lang::Es) => "Bienvenido a meltemi",
        (Msg::OnboardingTitle, Lang::En) => "Welcome to meltemi",
        (Msg::OnboardingBody, Lang::Es) => {
            "meltemi dirige tus agentes de codificacion; no es un editor ni un agente.\n\n\
             Teclas: 1-4 vistas | : paleta | ? ayuda | a permisos | q salir | Esc cierra overlays y campos de texto.\n\n\
             Pasos: daemon | proyecto .meltemi | agente (Flota) | propose (proximamente).\n\n\
             Esc o q para empezar."
        }
        (Msg::OnboardingBody, Lang::En) => {
            "meltemi drives your coding agents; it is not an editor nor an agent.\n\n\
             Keys: 1-4 views | : palette | ? help | a permissions | q quit | Esc closes overlays and text fields.\n\n\
             Steps: daemon | .meltemi project | agent (Fleet) | propose (coming soon).\n\n\
             Esc or q to start."
        }
        (Msg::DisconnectBanner, Lang::Es) => {
            "daemon inalcanzable - reconectando; los permisos pendientes se denegaran"
        }
        (Msg::DisconnectBanner, Lang::En) => {
            "daemon unreachable - reconnecting; pending permissions will be denied"
        }
        (Msg::SizeFloor, Lang::Es) => "terminal demasiado pequena; se requiere 80x24",
        (Msg::SizeFloor, Lang::En) => "terminal too small; 80x24 required",
        (Msg::DirectTitle, Lang::Es) => "Dirigir la sesión",
        (Msg::DirectTitle, Lang::En) => "Direct the session",
        (Msg::DirectHint, Lang::Es) => {
            "el texto viaja tal cual se escribe | Enter envía - Esc cancela"
        }
        (Msg::DirectHint, Lang::En) => {
            "the text travels exactly as typed | Enter sends - Esc cancels"
        }
        (Msg::DirectNoSession, Lang::Es) => {
            "ninguna sesión seleccionada: elige una en Sesiones y vuelve a dirigir"
        }
        (Msg::DirectNoSession, Lang::En) => {
            "no session selected: pick one in Sessions and direct it again"
        }
        // The turn in flight is not interrupted: the instruction waits its turn,
        // and the position says how long the queue is.
        (Msg::DirectQueued, Lang::Es) => "encolada, sin interrumpir el turno en curso: turno",
        (Msg::DirectQueued, Lang::En) => "queued without interrupting the turn in flight: turn",
        (Msg::DirectResumed, Lang::Es) => "sesión reanudada con la instrucción",
        (Msg::DirectResumed, Lang::En) => "session resumed with the instruction",
        (Msg::DirectRefused, Lang::Es) => "la sesión no admite la instrucción",
        (Msg::DirectRefused, Lang::En) => "the session does not accept the instruction",
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
    Msg::FleetLoading,
    Msg::FleetByoHint,
    Msg::TrayHint,
    Msg::TrayFatigueHint,
    Msg::HintKeys,
    Msg::HintExitField,
    Msg::QuitConfirm,
    Msg::HelpTitle,
    Msg::PaletteTitle,
    Msg::OnboardingTitle,
    Msg::OnboardingBody,
    Msg::DisconnectBanner,
    Msg::SizeFloor,
    Msg::DirectTitle,
    Msg::DirectHint,
    Msg::DirectNoSession,
    Msg::DirectQueued,
    Msg::DirectResumed,
    Msg::DirectRefused,
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
