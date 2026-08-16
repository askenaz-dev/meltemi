// SPDX-License-Identifier: Apache-2.0

//! How output is chosen and dressed: the format (human, JSON, YAML), the
//! decision to paint or not, and the ANSI it paints with.
//!
//! Two rules govern everything here, and both are spec, not taste. Colour is
//! **decorative only** — every distinction it marks is marked as well by a
//! word, a glyph or a figure, so stripping the colour cannot remove
//! information. And a machine-readable format is **never** painted: a document
//! for a program does not get decorated (salida-que-se-lee D3, D4).

use serde_json::Value;

/// The single output-format choice. An enum rather than a second boolean:
/// `--json --yaml` is a state that means nothing, and with two flags every
/// call site would have to decide what it meant (design D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Prose for a person, possibly painted.
    #[default]
    Human,
    /// Exactly one JSON object on stdout, never painted.
    Json,
    /// Exactly one YAML document on stdout, never painted.
    Yaml,
}

impl Format {
    /// Whether this format is for a program rather than a person.
    #[must_use]
    pub fn is_machine(self) -> bool {
        matches!(self, Self::Json | Self::Yaml)
    }
}

/// Whether this invocation may paint.
///
/// Any one of four signals turns colour off, in this order: the explicit
/// `--no-color`, a non-empty `NO_COLOR`, `TERM=dumb`, and stdout not being a
/// terminal. The last one is what makes the change backwards compatible byte
/// for byte: a script piping this output sees exactly what it saw before.
#[must_use]
pub fn paints(format: Format, no_color_flag: bool, stdout_is_tty: bool) -> bool {
    if format.is_machine() || no_color_flag || !stdout_is_tty {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()) {
        return false;
    }
    !matches!(std::env::var("TERM").as_deref(), Ok("dumb"))
}

/// The eight ANSI colours this surface uses, plus the two attributes. Written
/// out rather than pulled from a crate: it is what the shell already emits and
/// it fits in a constant (design D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    Dim,
    Bold,
    Green,
    Yellow,
    Red,
    Blue,
    Cyan,
    Magenta,
}

impl Paint {
    const fn code(self) -> &'static str {
        match self {
            Self::Dim => "\x1b[2m",
            Self::Bold => "\x1b[1m",
            Self::Green => "\x1b[32m",
            Self::Yellow => "\x1b[33m",
            Self::Red => "\x1b[31m",
            Self::Blue => "\x1b[34m",
            Self::Cyan => "\x1b[36m",
            Self::Magenta => "\x1b[35m",
        }
    }
}

const RESET: &str = "\x1b[0m";

/// Wraps `text` in `paint` when painting is on, and returns it untouched when
/// it is off. The caller never branches, which is what keeps the painted and
/// unpainted outputs identical in everything but escapes.
#[must_use]
pub fn paint(on: bool, paint: Paint, text: &str) -> String {
    if on {
        format!("{}{text}{RESET}", paint.code())
    } else {
        text.to_string()
    }
}

/// Renders a JSON value as a YAML document.
///
/// No dependency: YAML 1.2 is a strict superset of JSON, so emitting every
/// string as a **double-quoted** scalar with JSON's own escaping is valid by
/// construction — which is what removes the edge cases a hand-rolled emitter is
/// usually feared for (multiline text, a key with a colon). Numbers, booleans
/// and null go bare; objects and arrays go in block style (design D2).
#[must_use]
pub fn to_yaml(value: &Value) -> String {
    let mut out = String::new();
    write_yaml(value, 0, &mut out);
    if out.is_empty() {
        out.push_str("{}\n");
    }
    out
}

/// A scalar that YAML reads back as the same value it came from.
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // Always quoted, with JSON's escaping: the one rule that makes this
        // emitter safe without a parser behind it.
        Value::String(s) => serde_json::to_string(s).expect("a string serializes"),
        _ => unreachable!("scalar() takes only scalars"),
    }
}

fn write_yaml(value: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match value {
        Value::Object(map) if map.is_empty() => out.push_str(&format!("{pad}{{}}\n")),
        Value::Array(items) if items.is_empty() => out.push_str(&format!("{pad}[]\n")),
        Value::Object(map) => {
            for (key, child) in map {
                let key = serde_json::to_string(key).expect("a key serializes");
                match child {
                    Value::Object(inner) if !inner.is_empty() => {
                        out.push_str(&format!("{pad}{key}:\n"));
                        write_yaml(child, depth + 1, out);
                    }
                    Value::Array(inner) if !inner.is_empty() => {
                        out.push_str(&format!("{pad}{key}:\n"));
                        write_yaml(child, depth, out);
                    }
                    Value::Object(_) | Value::Array(_) => {
                        // Empty containers stay on the key's own line.
                        let empty = if child.is_object() { "{}" } else { "[]" };
                        out.push_str(&format!("{pad}{key}: {empty}\n"));
                    }
                    scalar_child => {
                        out.push_str(&format!("{pad}{key}: {}\n", scalar(scalar_child)));
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::Object(inner) if !inner.is_empty() => {
                        // The dash owns the first key's line, so the item reads
                        // as one block instead of a dash floating above it.
                        let mut block = String::new();
                        write_yaml(item, depth + 1, &mut block);
                        let mut lines = block.lines();
                        if let Some(first) = lines.next() {
                            out.push_str(&format!("{pad}- {}\n", first.trim_start()));
                        }
                        for rest in lines {
                            out.push_str(&format!("{rest}\n"));
                        }
                    }
                    Value::Array(inner) if !inner.is_empty() => {
                        out.push_str(&format!("{pad}-\n"));
                        write_yaml(item, depth + 1, out);
                    }
                    Value::Object(_) | Value::Array(_) => {
                        let empty = if item.is_object() { "{}" } else { "[]" };
                        out.push_str(&format!("{pad}- {empty}\n"));
                    }
                    scalar_item => out.push_str(&format!("{pad}- {}\n", scalar(scalar_item))),
                }
            }
        }
        scalar_value => out.push_str(&format!("{pad}{}\n", scalar(scalar_value))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_machine_format_is_never_painted() {
        // Even on a terminal, with no flag and no environment saying otherwise.
        assert!(!paints(Format::Json, false, true));
        assert!(!paints(Format::Yaml, false, true));
    }

    // Scenario: La salida redirigida no lleva color
    #[test]
    fn without_a_terminal_nothing_is_painted() {
        assert!(!paints(Format::Human, false, false));
    }

    // Scenario: El usuario apaga el color
    #[test]
    fn the_explicit_flag_turns_colour_off() {
        assert!(!paints(Format::Human, true, true));
    }

    #[test]
    fn paint_adds_only_escapes() {
        let painted = paint(true, Paint::Green, "active");
        let plain = paint(false, Paint::Green, "active");
        assert_eq!(plain, "active");
        assert!(painted.contains("active"));
        assert!(painted.starts_with("\x1b["));
        assert!(painted.ends_with(RESET));
    }

    // Scenario: YAML emite un documento y nada más
    #[test]
    fn yaml_quotes_every_string_so_the_hard_cases_never_arise() {
        let value = json!({
            "plain": "meltemi",
            "with: colon": "a: b",
            "multiline": "one\ntwo",
            "quoted": "he said \"hi\"",
            "number": 42,
            "flag": true,
            "nothing": null,
        });
        let yaml = to_yaml(&value);
        // Every string — key and value — is double quoted, which is where YAML
        // and JSON agree, so nothing here needs a parser to be safe.
        assert!(yaml.contains("\"with: colon\": \"a: b\""));
        assert!(yaml.contains("\"multiline\": \"one\\ntwo\""));
        assert!(yaml.contains("\"quoted\": \"he said \\\"hi\\\"\""));
        // Scalars that are not strings stay bare, so they read back as
        // themselves.
        assert!(yaml.contains("\"number\": 42"));
        assert!(yaml.contains("\"flag\": true"));
        assert!(yaml.contains("\"nothing\": null"));
    }

    #[test]
    fn yaml_nests_objects_and_lists_in_block_style() {
        let value = json!({
            "changes": [
                { "name": "one", "active": true },
                { "name": "two", "active": false },
            ],
            "summary": { "total": 2 },
            "empty": [],
        });
        let yaml = to_yaml(&value);
        assert!(yaml.contains("\"changes\":\n"));
        // The dash carries the first key, so an item reads as one block.
        assert!(yaml.contains("- \"name\": \"one\""));
        assert!(yaml.contains("  \"active\": true"));
        assert!(yaml.contains("\"summary\":\n  \"total\": 2"));
        assert!(yaml.contains("\"empty\": []"));
    }

    #[test]
    fn an_empty_document_is_still_a_document() {
        assert_eq!(to_yaml(&json!({})), "{}\n");
    }
}
