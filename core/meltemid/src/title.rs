// SPDX-License-Identifier: Apache-2.0
//! Session titles, derived locally from the instruction that opens a session.
//!
//! Local and deterministic on purpose (titulo-de-sesion design D1): Meltemi has
//! no model of its own (§5), and asking the user's paid session to name itself
//! would spend their tokens on something they did not ask for and put words in
//! their log that they never wrote — and the log is the truth.

/// How long a title may be before it is cut, counted in characters.
///
/// Wide enough for a sentence that says what the work is, short enough that a
/// tab strip of six can still show the beginning of each.
pub const TITLE_MAX_CHARS: usize = 64;

/// The character appended when a title is cut.
const ELLIPSIS: char = '…';

/// Derives a session's title from the instruction that opens it.
///
/// The first non-empty line, whitespace collapsed, cut to [`TITLE_MAX_CHARS`].
/// Returns `None` when there is nothing to name the session after: a session
/// with no user instruction gets no title rather than a fabricated one.
///
/// Takes the instruction **as the user typed it**, before `@` references are
/// expanded: a title that quoted the contents of a file would not be what they
/// wrote.
pub fn derive(instruction: &str) -> Option<String> {
    let line = instruction.lines().find(|line| !line.trim().is_empty())?;
    let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    Some(truncate(&collapsed))
}

/// Cuts to [`TITLE_MAX_CHARS`], counting characters.
///
/// Never bytes: slicing a `&str` by byte index splits a multi-byte character
/// and hands back a broken title. In a project written in Spanish, an accent
/// arrives long before character 64.
fn truncate(text: &str) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(TITLE_MAX_CHARS).collect();
    if chars.next().is_none() {
        head
    } else {
        // The ellipsis replaces the last character rather than being added to
        // it, so the cut title never exceeds the budget it declares.
        let mut cut: String = head.chars().take(TITLE_MAX_CHARS - 1).collect();
        cut.push(ELLIPSIS);
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: Título derivado de la primera instrucción
    #[test]
    fn the_first_non_empty_line_names_the_session() {
        assert_eq!(
            derive("Corregir el login\ny luego revisar el resto"),
            Some("Corregir el login".to_owned())
        );
        assert_eq!(
            derive("\n\n   \nEmpieza aquí\n"),
            Some("Empieza aquí".to_owned())
        );
        // Collapsed, and cut with an ellipsis without splitting a character.
        assert_eq!(
            derive("  Dos   espacios  "),
            Some("Dos espacios".to_owned())
        );
        let long = derive(&"á".repeat(TITLE_MAX_CHARS + 3)).expect("a title");
        assert_eq!(long.chars().count(), TITLE_MAX_CHARS);
        assert!(long.ends_with(ELLIPSIS));
    }

    #[test]
    fn whitespace_is_collapsed_not_preserved() {
        assert_eq!(
            derive("  Corregir    el   login  "),
            Some("Corregir el login".to_owned())
        );
        assert_eq!(derive("\tuna\tpalabra"), Some("una palabra".to_owned()));
    }

    #[test]
    fn an_instruction_with_nothing_in_it_names_nothing() {
        assert_eq!(derive(""), None);
        assert_eq!(derive("   \n\t\n  "), None);
    }

    #[test]
    fn one_word_is_a_title() {
        assert_eq!(derive("refactor"), Some("refactor".to_owned()));
    }

    #[test]
    fn a_long_title_is_cut_with_an_ellipsis_and_never_grows_past_its_budget() {
        let long = "a".repeat(TITLE_MAX_CHARS + 20);
        let title = derive(&long).expect("a title");
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS);
        assert!(title.ends_with(ELLIPSIS));
    }

    #[test]
    fn a_title_exactly_at_the_budget_is_not_cut() {
        let exact = "b".repeat(TITLE_MAX_CHARS);
        let title = derive(&exact).expect("a title");
        assert_eq!(title, exact, "nothing to cut means no ellipsis");
        assert!(!title.ends_with(ELLIPSIS));
    }

    #[test]
    fn cutting_counts_characters_and_never_bytes() {
        // Every character here is multi-byte: cutting by byte index would split
        // one and produce a broken string — or panic.
        let accented = "ñ".repeat(TITLE_MAX_CHARS + 5);
        let title = derive(&accented).expect("a title");
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS);
        assert!(title.starts_with('ñ'));

        // And a sentence a Spanish-speaking user would actually write.
        let sentence = format!(
            "Corregir la validación de la sesión {}",
            "ñandú ".repeat(12)
        );
        let title = derive(&sentence).expect("a title");
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS);
        assert!(title.starts_with("Corregir la validación"));
    }
}
