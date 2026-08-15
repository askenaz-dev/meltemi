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
    RaceLoading,
    RaceEmpty,
    RaceUnknown,
    RaceCommitted,
    RaceUncommitted,
    RaceLaneHint,
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
    LinkTitle,
    LinkHint,
    LinkBadInput,
    DirectNoSession,
    DirectQueued,
    DirectResumed,
    DirectRefused,
    DirectWillQueue,
    DirectWillRelay,
    InterruptTitle,
    InterruptHint,
    DirectRelayed,
    DirectWillResume,
    DirectNotResumable,
    RegisterTitle,
    ForgetTitle,
    ProjectPathHint,
    ProjectRegistered,
    ProjectForgotten,
    ProjectNotListed,
    ProjectRegistryTitle,
    ProjectRegistryEmpty,
    ProjectRegistryHint,
    ProjectPresent,
    ProjectAbsent,
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
        (Msg::RaceLoading, Lang::Es) => "leyendo las calles de la carrera...",
        (Msg::RaceLoading, Lang::En) => "reading the race lanes...",
        (Msg::RaceEmpty, Lang::Es) => "sin competidores en esta tarea",
        (Msg::RaceEmpty, Lang::En) => "no competitors on this task",
        (Msg::RaceUnknown, Lang::Es) => "sin registro",
        (Msg::RaceUnknown, Lang::En) => "unrecorded",
        (Msg::RaceCommitted, Lang::Es) => "comiteada",
        (Msg::RaceCommitted, Lang::En) => "committed",
        (Msg::RaceUncommitted, Lang::Es) => "sin commit",
        (Msg::RaceUncommitted, Lang::En) => "uncommitted",
        (Msg::RaceLaneHint, Lang::Es) => "j/k calle | flechas panear | Esc volver",
        (Msg::RaceLaneHint, Lang::En) => "j/k lane | arrows pan | Esc back",
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
        (Msg::LinkTitle, Lang::Es) => "Vincular una suscripción",
        (Msg::LinkTitle, Lang::En) => "Link a subscription",
        (Msg::LinkHint, Lang::Es) => {
            "escribe `agente nombre` (el nombre viaja tal cual) | Enter enviar | Esc cancelar"
        }
        (Msg::LinkHint, Lang::En) => {
            "type `agent name` (the name travels verbatim) | Enter send | Esc cancel"
        }
        (Msg::LinkBadInput, Lang::Es) => "vincular necesita dos palabras: `agente nombre`",
        (Msg::LinkBadInput, Lang::En) => "linking needs two words: `agent name`",
        (Msg::DirectTitle, Lang::Es) => "Dirigir la sesión",
        (Msg::DirectTitle, Lang::En) => "Direct the session",
        (Msg::DirectHint, Lang::Es) => {
            "el texto viaja tal cual se escribe | Tab interrumpe el turno - Enter envía"
        }
        (Msg::DirectHint, Lang::En) => {
            "the text travels exactly as typed | Tab interrupts the turn - Enter sends"
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
        // What directing will do, said before the instruction is written.
        (Msg::DirectWillQueue, Lang::Es) => "se encolará como siguiente turno de esta sesión",
        (Msg::DirectWillQueue, Lang::En) => "it will be queued as this session's next turn",
        // Said in the order it happens, and naming what is lost: the turn in
        // flight stops where it is. Work already done stays done — the session
        // does not end and nothing is rolled back — but the turn does not finish.
        (Msg::DirectWillRelay, Lang::Es) => {
            "interrumpirá el turno en curso y lo relevará con esta instrucción;              la sesión sigue viva"
        }
        (Msg::DirectWillRelay, Lang::En) => {
            "it will interrupt the turn in flight and relay it with this instruction;              the session stays alive"
        }
        (Msg::InterruptTitle, Lang::Es) => "Interrumpir y relevar",
        (Msg::InterruptTitle, Lang::En) => "Interrupt and relay",
        (Msg::InterruptHint, Lang::Es) => {
            "Tab vuelve a encolar sin interrumpir | Enter envía - Esc cancela"
        }
        (Msg::InterruptHint, Lang::En) => {
            "Tab goes back to queueing without interrupting | Enter sends - Esc cancels"
        }
        (Msg::DirectRelayed, Lang::Es) => "turno interrumpido y relevado por la instrucción",
        (Msg::DirectRelayed, Lang::En) => "turn interrupted and relayed by the instruction",
        (Msg::DirectWillResume, Lang::Es) => "reanudará esta sesión con la instrucción",
        (Msg::DirectWillResume, Lang::En) => "it will resume this session with the instruction",
        (Msg::DirectNotResumable, Lang::Es) => {
            "esta sesión terminó y su agente no admite reanudación: no hay envío que ofrecer — \
             arranca una sesión sobre el mismo proyecto"
        }
        (Msg::DirectNotResumable, Lang::En) => {
            "this session ended and its agent cannot be resumed: there is no send to offer — \
             start a session on the same project"
        }
        (Msg::RegisterTitle, Lang::Es) => "Dar de alta un proyecto",
        (Msg::RegisterTitle, Lang::En) => "Register a project",
        (Msg::ForgetTitle, Lang::Es) => "Dar de baja un proyecto",
        (Msg::ForgetTitle, Lang::En) => "Forget a project",
        (Msg::ProjectPathHint, Lang::Es) => {
            "ruta absoluta, tal cual se escribe | Enter confirma - Esc cancela"
        }
        (Msg::ProjectPathHint, Lang::En) => {
            "absolute path, exactly as typed | Enter confirms - Esc cancels"
        }
        (Msg::ProjectRegistered, Lang::Es) => "proyecto dado de alta",
        (Msg::ProjectRegistered, Lang::En) => "project registered",
        // The whole point of the wording: forgetting governs the listing, and
        // nothing else. Sessions, logs and the tree itself stay where they are.
        (Msg::ProjectForgotten, Lang::Es) => {
            "fuera del listado; no se borró nada y reaparece al volver a usarlo"
        }
        (Msg::ProjectForgotten, Lang::En) => {
            "dropped from the listing; nothing was deleted and it returns when used again"
        }
        (Msg::ProjectNotListed, Lang::Es) => "no estaba en el listado: nada cambió",
        (Msg::ProjectNotListed, Lang::En) => "it was not in the listing: nothing changed",
        (Msg::ProjectRegistryTitle, Lang::Es) => "Registro de proyectos",
        (Msg::ProjectRegistryTitle, Lang::En) => "Project registry",
        (Msg::ProjectRegistryEmpty, Lang::Es) => "el registro no tiene proyectos todavía",
        (Msg::ProjectRegistryEmpty, Lang::En) => "the registry holds no project yet",
        (Msg::ProjectRegistryHint, Lang::Es) => {
            ": projects register <ruta> da de alta | : projects forget <ruta> da de baja"
        }
        (Msg::ProjectRegistryHint, Lang::En) => {
            ": projects register <path> adds | : projects forget <path> drops"
        }
        (Msg::ProjectPresent, Lang::Es) => "presente",
        (Msg::ProjectPresent, Lang::En) => "present",
        (Msg::ProjectAbsent, Lang::Es) => "ausente",
        (Msg::ProjectAbsent, Lang::En) => "missing",
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
    Msg::RaceLoading,
    Msg::RaceEmpty,
    Msg::RaceUnknown,
    Msg::RaceCommitted,
    Msg::RaceUncommitted,
    Msg::RaceLaneHint,
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
    Msg::LinkTitle,
    Msg::LinkHint,
    Msg::LinkBadInput,
    Msg::DirectNoSession,
    Msg::DirectQueued,
    Msg::DirectResumed,
    Msg::DirectRefused,
    Msg::DirectWillQueue,
    Msg::DirectWillResume,
    Msg::DirectNotResumable,
    Msg::RegisterTitle,
    Msg::ForgetTitle,
    Msg::ProjectPathHint,
    Msg::ProjectRegistered,
    Msg::ProjectForgotten,
    Msg::ProjectNotListed,
    Msg::ProjectRegistryTitle,
    Msg::ProjectRegistryEmpty,
    Msg::ProjectRegistryHint,
    Msg::ProjectPresent,
    Msg::ProjectAbsent,
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
