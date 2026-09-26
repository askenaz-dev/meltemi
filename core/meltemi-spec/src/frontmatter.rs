// SPDX-License-Identifier: Apache-2.0

//! Minimal front-matter reader, shared by `rumbo/` files (design M4) and by the
//! harness's `RULE.md` files (harness-global-y-por-agente design D3).
//!
//! The front-matter both formats use is a small, fenced YAML block with exactly
//! two shapes: `key: value` and `key: [a, b, c]`. That is not an assumption —
//! the four real `RULE.md` files of the Forge Development Hub use those two and
//! nothing else. A ~120-line scanner covers them without pulling in a YAML
//! dependency (constitution §10), and a shape it does not cover produces a
//! diagnostic rather than a value read halfway: a `scope` truncated in silence
//! is a rule that applies where it should not.
//!
//! If a real file ever needs richer shapes, this is still the place to swap in
//! a real YAML parser.

use std::path::{Path, PathBuf};

use crate::model::{Inclusion, Ratification, RumboFile};

/// A front-matter value, in the two shapes that exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontValue {
    /// `key: value`
    Scalar(String),
    /// `key: [a, "b", 'c']`
    List(Vec<String>),
}

impl FrontValue {
    /// The scalar, when this is one.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(text) => Some(text),
            Self::List(_) => None,
        }
    }

    /// The list, when this is one.
    #[must_use]
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(items) => Some(items),
            Self::Scalar(_) => None,
        }
    }
}

/// A parsed front-matter block: its entries in the order they were written, and
/// whatever the scanner could not read.
///
/// Order is preserved because a caller that shows the block back to a human
/// should show it as its author wrote it, and because the entries a caller does
/// not know about travel through unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrontMatter {
    /// Every `key: value` of the block, in source order.
    pub entries: Vec<(String, FrontValue)>,
    /// Shapes the scanner does not cover, each naming its key and what was
    /// expected. A non-empty list means the block was NOT fully read.
    pub problems: Vec<String>,
}

impl FrontMatter {
    /// The value of a key, when present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&FrontValue> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The scalar value of a key, when present and scalar.
    #[must_use]
    pub fn scalar(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(FrontValue::as_scalar)
    }

    /// The list value of a key, when present and a list.
    #[must_use]
    pub fn list(&self, key: &str) -> Option<&[String]> {
        self.get(key).and_then(FrontValue::as_list)
    }
}

/// Reads a document's front-matter block and body.
///
/// Returns `None` when there is no leading `---` fence — a document without
/// front-matter, which is not an error for either caller.
#[must_use]
pub fn parse_front_matter(content: &str) -> Option<(FrontMatter, &str)> {
    let (front, body) = split_front_matter(content)?;
    let mut parsed = FrontMatter::default();
    let mut last_key: Option<String> = None;

    for line in front.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // A block list continues the key above it. It is the one shape a real
        // file could plausibly grow into, and reading only its first line would
        // hand back a value nobody wrote — so it is named, not guessed at.
        if let Some(item) = trimmed.strip_prefix("- ") {
            let key = last_key.as_deref().unwrap_or("(no key)");
            parsed.problems.push(format!(
                "`{key}` uses a block list (`- {}`), which this reader does not cover; \
                 write it inline as `{key}: [a, b]`",
                item.trim()
            ));
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = strip_comment(value.trim()).trim();
        // A key with nothing after it is what introduces a block list; the
        // lines below it are diagnosed above.
        if value.is_empty() {
            last_key = Some(key);
            continue;
        }
        let parsed_value = if value.starts_with('[') {
            FrontValue::List(parse_list(value))
        } else {
            FrontValue::Scalar(unquote(value).to_string())
        };
        parsed.entries.push((key.clone(), parsed_value));
        last_key = Some(key);
    }
    Some((parsed, body))
}

/// Reads and parses a rumbo file.
///
/// # Errors
///
/// Propagates the read error when the file cannot be read.
pub fn parse_rumbo_file(path: &Path) -> std::io::Result<RumboFile> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_rumbo(path.to_path_buf(), &content))
}

/// Parses a rumbo file's front-matter and body from its contents.
#[must_use]
pub fn parse_rumbo(path: PathBuf, content: &str) -> RumboFile {
    let Some((front, body)) = parse_front_matter(content) else {
        return RumboFile {
            path,
            inclusion: None,
            ratification: None,
            body: content.to_string(),
        };
    };

    let inclusion = match front.scalar("inclusion") {
        Some("siempre") => Some(Inclusion::Always),
        Some("manual") => Some(Inclusion::Manual),
        Some("por-patrón") => Some(Inclusion::OnMatch(
            front.list("fileMatch").unwrap_or_default().to_vec(),
        )),
        _ => None,
    };

    let ratification = match (front.scalar("ratificado"), front.scalar("ratificador")) {
        (Some(date), Some(ratifier)) => Some(Ratification {
            date: date.to_string(),
            ratifier: ratifier.to_string(),
        }),
        _ => None,
    };

    RumboFile {
        path,
        inclusion,
        ratification,
        body: body.to_string(),
    }
}

/// Splits a `---`-fenced front-matter block from the body. Returns
/// `(front_matter, body)` or `None` when there is no leading block.
fn split_front_matter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---")?;
    // The opening fence must be its own line.
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))?;
    // Find the closing fence at the start of a line.
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let front = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return Some((front, body));
        }
        offset += line.len();
    }
    None
}

/// Cuts a trailing `#` comment off a value.
///
/// Real files carry them — `version: 0.2.0 # x-release-please-version` is in
/// every `RULE.md` of the hub — and without this the value comes back with the
/// comment glued to it. YAML's own rule is used rather than a looser one: a `#`
/// counts as a comment only at the start or after whitespace, so `a#b` stays
/// whole, and one inside quotes is text.
fn strip_comment(value: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous: Option<char> = None;
    for (index, character) in value.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double && previous.is_none_or(char::is_whitespace) => {
                return &value[..index];
            }
            _ => {}
        }
        previous = Some(character);
    }
    value
}

/// Strips surrounding single or double quotes from a scalar value.
fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Parses a simple inline YAML list `[a, "b", 'c']` into its items.
///
/// Splits on commas OUTSIDE quotes, which is what YAML means and what the real
/// files need: `scope: ["**/*.{ts,tsx}"]` carries a comma inside the glob, and
/// a naive split tore that glob in half — leaving a rule that would apply to a
/// pattern nobody wrote. No `rumbo/` file could have caught it, because none of
/// their globs has ever carried a comma.
fn parse_list(value: &str) -> Vec<String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or("");
    let mut items = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    for (index, character) in inner.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ',' if !in_single && !in_double => {
                push_item(&mut items, &inner[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    push_item(&mut items, &inner[start..]);
    items
}

/// Trims, unquotes and keeps one item, dropping the empties a trailing comma
/// leaves behind.
fn push_item(items: &mut Vec<String>, raw: &str) {
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
        items.push(unquote(trimmed).to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inclusion_always_and_ratification() {
        let content = "---\ninclusion: siempre\nratificado: 2026-07-11\nratificador: Guillmar Ortiz\n---\n# cuerpo\n";
        let rf = parse_rumbo(PathBuf::from("rumbo/product.md"), content);
        assert_eq!(rf.inclusion, Some(Inclusion::Always));
        let r = rf.ratification.unwrap();
        assert_eq!(r.date, "2026-07-11");
        assert_eq!(r.ratifier, "Guillmar Ortiz");
        assert_eq!(rf.body.trim(), "# cuerpo");
    }

    #[test]
    fn parses_on_match_with_globs() {
        let content =
            "---\ninclusion: por-patrón\nfileMatch: [\"src/**\", 'docs/*.md']\n---\nbody\n";
        let rf = parse_rumbo(PathBuf::from("r.md"), content);
        assert_eq!(
            rf.inclusion,
            Some(Inclusion::OnMatch(vec![
                "src/**".to_string(),
                "docs/*.md".to_string()
            ]))
        );
    }

    #[test]
    fn missing_front_matter_yields_no_inclusion() {
        let rf = parse_rumbo(PathBuf::from("r.md"), "# just a body\n");
        assert_eq!(rf.inclusion, None);
        assert_eq!(rf.body, "# just a body\n");
    }

    // Scenario: Las claves que el núcleo no conoce se conservan
    #[test]
    fn every_key_is_kept_in_source_order_including_the_ones_nobody_asked_for() {
        let content = "---\nname: no-any-cast\nkind: rule\nowner_team: dx-platform\n---\nbody\n";
        let (front, body) = parse_front_matter(content).expect("a block is present");
        let keys: Vec<&str> = front.entries.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, ["name", "kind", "owner_team"]);
        assert_eq!(front.scalar("owner_team"), Some("dx-platform"));
        assert_eq!(body.trim(), "body");
        assert!(front.problems.is_empty());
    }

    #[test]
    fn a_trailing_comment_is_not_part_of_the_value() {
        // The exact line every RULE.md of the hub carries today.
        let content = "---\nversion: 0.2.0 # x-release-please-version\n---\n";
        let (front, _) = parse_front_matter(content).expect("a block is present");
        assert_eq!(front.scalar("version"), Some("0.2.0"));
    }

    #[test]
    fn a_hash_that_is_not_a_comment_stays_in_the_value() {
        // YAML's own rule, not a looser one: a `#` is a comment only at the
        // start or after whitespace, and one inside quotes is text.
        let content = "---\ntag: c#\nnote: \"pound # inside quotes\"\n---\n";
        let (front, _) = parse_front_matter(content).expect("a block is present");
        assert_eq!(front.scalar("tag"), Some("c#"));
        assert_eq!(front.scalar("note"), Some("pound # inside quotes"));
    }

    #[test]
    fn a_comma_inside_a_quoted_item_does_not_split_it() {
        // The real `scope` of the hub's rules. Splitting on every comma tore
        // this glob in half — into `"**/*.{ts` and `tsx}"` — leaving a rule
        // that would apply to a pattern nobody wrote. No `rumbo/` file could
        // have caught it: none of their globs has ever carried a comma.
        let content = "---
scope: [\"**/*.{ts,tsx}\", \"docs/*.md\"]
---
";
        let (front, _) = parse_front_matter(content).expect("a block is present");
        assert_eq!(
            front.list("scope"),
            Some(["**/*.{ts,tsx}".to_string(), "docs/*.md".to_string()].as_slice())
        );
    }

    // Scenario: Una forma no cubierta se diagnostica en vez de leerse a medias
    #[test]
    fn a_block_list_is_named_rather_than_read_halfway() {
        let content = "---\nname: r\nscope:\n  - \"src/**\"\n  - \"docs/**\"\n---\n";
        let (front, _) = parse_front_matter(content).expect("a block is present");
        assert_eq!(front.scalar("name"), Some("r"));
        // The key is NOT reported as an empty value that silently applies
        // everywhere: it is reported as unreadable, twice — once per item.
        assert_eq!(front.problems.len(), 2, "{:?}", front.problems);
        assert!(front.problems[0].contains("scope"), "{:?}", front.problems);
        assert!(
            front.problems[0].contains("inline"),
            "the diagnostic says what to write instead: {:?}",
            front.problems
        );
        assert!(front.get("scope").is_none(), "no half-read value survives");
    }
}
