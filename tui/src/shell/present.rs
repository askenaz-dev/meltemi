// SPDX-License-Identifier: Apache-2.0

//! Presentation policy: color and Unicode capability (design D4).
//!
//! [`Presentation::resolve`] is a pure function of the relevant environment and
//! flags, so the accessibility baseline (honor `NO_COLOR`, force ASCII) is
//! unit-testable without a terminal.

/// Whether color and Unicode are used when rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presentation {
    /// Whether ANSI color may be emitted at all.
    pub color: bool,
    /// Whether Unicode glyphs may be used (otherwise the ASCII twins).
    pub unicode: bool,
}

/// The inputs that decide presentation, read from the environment/flags.
#[derive(Debug, Clone, Default)]
pub struct PresentationEnv {
    /// `NO_COLOR` value (any non-empty value disables color).
    pub no_color: Option<String>,
    /// `--no-color` flag.
    pub no_color_flag: bool,
    /// `TERM` value (`dumb` forces monochrome ASCII).
    pub term: Option<String>,
    /// `MELTEMI_ASCII` value (any non-empty, non-`0` value forces ASCII).
    pub ascii_env: Option<String>,
    /// `--ascii` flag.
    pub ascii_flag: bool,
    /// `LANG`/`LC_*` value (used to detect UTF-8 capability).
    pub lang: Option<String>,
}

impl Presentation {
    /// Resolves presentation from explicit inputs. Pure.
    #[must_use]
    pub fn resolve(env: &PresentationEnv) -> Self {
        let dumb = env.term.as_deref() == Some("dumb");

        let color = !env.no_color_flag && !is_set(&env.no_color) && !dumb;

        let ascii_forced = env.ascii_flag
            || is_set_not_zero(&env.ascii_env)
            || dumb
            || lang_is_non_utf8(&env.lang);
        let unicode = !ascii_forced;

        Self { color, unicode }
    }

    /// Resolves presentation from the real process environment.
    #[must_use]
    pub fn from_env() -> Self {
        let get = |k: &str| std::env::var(k).ok();
        Self::resolve(&PresentationEnv {
            no_color: get("NO_COLOR"),
            no_color_flag: false,
            term: get("TERM"),
            ascii_env: get("MELTEMI_ASCII"),
            ascii_flag: false,
            lang: get("LC_ALL")
                .or_else(|| get("LC_CTYPE"))
                .or_else(|| get("LANG")),
        })
    }
}

/// Whether an optional string is present and non-empty (the `NO_COLOR` rule).
fn is_set(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(|s| !s.is_empty())
}

/// Whether an optional string is present, non-empty and not `"0"`.
fn is_set_not_zero(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(|s| !s.is_empty() && s != "0")
}

/// A `LANG` value that is explicitly non-UTF-8 disables Unicode; an unset or
/// UTF-8 locale keeps it (conservative but not hostile to modern terminals).
fn lang_is_non_utf8(lang: &Option<String>) -> bool {
    match lang.as_deref() {
        None | Some("") => false,
        Some(l) => {
            let l = l.to_ascii_lowercase();
            !(l.contains("utf-8") || l.contains("utf8"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> PresentationEnv {
        PresentationEnv::default()
    }

    #[test]
    fn defaults_to_color_and_unicode() {
        let p = Presentation::resolve(&PresentationEnv {
            lang: Some("en_US.UTF-8".into()),
            ..env()
        });
        assert!(p.color);
        assert!(p.unicode);
    }

    #[test]
    fn no_color_env_disables_color_only() {
        // Scenario: Render monocromo.
        let p = Presentation::resolve(&PresentationEnv {
            no_color: Some("1".into()),
            lang: Some("en_US.UTF-8".into()),
            ..env()
        });
        assert!(!p.color);
        assert!(p.unicode, "NO_COLOR must not disable Unicode");
    }

    #[test]
    fn empty_no_color_does_not_disable_color() {
        // Per the NO_COLOR spec, only a non-empty value counts.
        let p = Presentation::resolve(&PresentationEnv {
            no_color: Some(String::new()),
            lang: Some("C.UTF-8".into()),
            ..env()
        });
        assert!(p.color);
    }

    #[test]
    fn ascii_forced_by_flag_or_env() {
        // Scenario: Conmutación a ASCII.
        let by_flag = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..env()
        });
        assert!(!by_flag.unicode);
        let by_env = Presentation::resolve(&PresentationEnv {
            ascii_env: Some("1".into()),
            ..env()
        });
        assert!(!by_env.unicode);
        // MELTEMI_ASCII=0 does not force ASCII.
        let zero = Presentation::resolve(&PresentationEnv {
            ascii_env: Some("0".into()),
            lang: Some("en_US.UTF-8".into()),
            ..env()
        });
        assert!(zero.unicode);
    }

    #[test]
    fn term_dumb_forces_monochrome_ascii() {
        let p = Presentation::resolve(&PresentationEnv {
            term: Some("dumb".into()),
            ..env()
        });
        assert!(!p.color);
        assert!(!p.unicode);
    }

    #[test]
    fn non_utf8_locale_forces_ascii() {
        let p = Presentation::resolve(&PresentationEnv {
            lang: Some("en_US.ISO8859-1".into()),
            ..env()
        });
        assert!(!p.unicode);
    }
}
