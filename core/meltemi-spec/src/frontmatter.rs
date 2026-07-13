// SPDX-License-Identifier: Apache-2.0

//! Minimal front-matter parser for `rumbo/` files (design M4).
//!
//! The rumbo front-matter is a small, fenced YAML block with only simple
//! shapes: `key: value` and `key: [a, b, c]`. A ~50-line scanner covers
//! `inclusion`, `fileMatch`, `ratificado` and `ratificador` without pulling in
//! a YAML dependency (constitution §10). If the front-matter ever grows richer
//! shapes, this is the place to swap in a real YAML parser.

use std::path::{Path, PathBuf};

use crate::model::{Inclusion, Ratification, RumboFile};

/// Reads and parses a rumbo file.
pub fn parse_rumbo_file(path: &Path) -> std::io::Result<RumboFile> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_rumbo(path.to_path_buf(), &content))
}

/// Parses a rumbo file's front-matter and body from its contents.
#[must_use]
pub fn parse_rumbo(path: PathBuf, content: &str) -> RumboFile {
    let Some((front, body)) = split_front_matter(content) else {
        return RumboFile {
            path,
            inclusion: None,
            ratification: None,
            body: content.to_string(),
        };
    };

    let mut inclusion_word: Option<String> = None;
    let mut file_match: Vec<String> = Vec::new();
    let mut ratified: Option<String> = None;
    let mut ratifier: Option<String> = None;

    for line in front.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "inclusion" => inclusion_word = Some(unquote(value).to_string()),
            "fileMatch" => file_match = parse_list(value),
            "ratificado" => ratified = Some(unquote(value).to_string()),
            "ratificador" => ratifier = Some(unquote(value).to_string()),
            _ => {}
        }
    }

    let inclusion = match inclusion_word.as_deref() {
        Some("siempre") => Some(Inclusion::Always),
        Some("manual") => Some(Inclusion::Manual),
        Some("por-patrón") => Some(Inclusion::OnMatch(file_match)),
        _ => None,
    };

    let ratification = match (ratified, ratifier) {
        (Some(date), Some(ratifier)) => Some(Ratification { date, ratifier }),
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
fn parse_list(value: &str) -> Vec<String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or("");
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| unquote(s).to_string())
        .collect()
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

    #[test]
    fn unknown_inclusion_value_is_none() {
        let rf = parse_rumbo(PathBuf::from("r.md"), "---\ninclusion: whenever\n---\n");
        assert_eq!(rf.inclusion, None);
    }
}
