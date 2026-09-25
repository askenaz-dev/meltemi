// SPDX-License-Identifier: Apache-2.0

//! The local-models guide is verified by hand against a real agent, and dated
//! (guia-modelos-locales design D1, D8). What a test can hold is its FORM: every
//! configuration block parses, points only at local endpoints where it says
//! local, and carries no key in the clear — so the page cannot quietly teach a
//! secret in a file, or rot into something that no longer parses.

use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // core/meltemid → repository root.
    dir.pop();
    dir.pop();
    dir
}

fn guide() -> String {
    std::fs::read_to_string(repo_root().join("docs/local-models.md"))
        .expect("read docs/local-models.md")
}

/// The fenced blocks of one language, in order.
fn blocks(text: &str, lang: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if current.is_none() && trimmed.starts_with("```") {
            if trimmed.trim_start_matches('`').trim() == lang {
                current = Some(String::new());
            }
            continue;
        }
        if trimmed.starts_with("```") {
            if let Some(block) = current.take() {
                found.push(block);
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    found
}

/// Every string field named `field` anywhere in a JSON value.
fn fields<'a>(value: &'a Value, field: &str, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == field
                    && let Some(text) = inner.as_str()
                {
                    out.push(text);
                }
                fields(inner, field, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| fields(item, field, out)),
        _ => {}
    }
}

/// Whether a key is an environment reference in OpenCode's own form.
fn is_env_reference(value: &str) -> bool {
    value
        .strip_prefix("{env:")
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
}

// Scenario: Ninguna clave en claro
#[test]
fn every_recipe_parses_points_where_it_says_and_carries_no_key() {
    let guide = guide();
    let json = blocks(&guide, "json");
    assert!(json.len() >= 3, "three recipes, three files: {json:#?}");
    for block in &json {
        let value: Value = serde_json::from_str(block)
            .unwrap_or_else(|e| panic!("a recipe is not valid JSON: {e}\n{block}"));
        assert_eq!(
            value["$schema"], "https://opencode.ai/config.json",
            "each file declares the agent's own schema: {block}"
        );
        // Every key arrives by reference to the environment, never literally.
        let mut keys = Vec::new();
        fields(&value, "apiKey", &mut keys);
        for key in keys {
            assert!(
                is_env_reference(key),
                "a recipe carries a key in the clear: `{key}` in {block}"
            );
        }
        // A base URL here is a local endpoint: nothing leaves the machine by a
        // recipe that says it runs on it.
        let mut urls = Vec::new();
        fields(&value, "baseURL", &mut urls);
        for url in urls {
            assert!(
                url.starts_with("http://localhost:") || url.starts_with("http://127.0.0.1:"),
                "a base URL points off the machine: {url}"
            );
        }
    }

    let toml = blocks(&guide, "toml");
    assert!(
        toml.len() >= 4,
        "a profile per recipe, plus the first one: {toml:#?}"
    );
    for block in &toml {
        let value: toml::Value = toml::from_str(block)
            .unwrap_or_else(|e| panic!("a profile is not valid TOML: {e}\n{block}"));
        let profiles = value
            .get("fleet")
            .and_then(|fleet| fleet.get("profile"))
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| panic!("the block configures fleet profiles: {block}"));
        for profile in profiles {
            assert_eq!(
                profile.get("agent").and_then(toml::Value::as_str),
                Some("opencode"),
                "{block}"
            );
            let env = profile
                .get("env")
                .and_then(toml::Value::as_table)
                .unwrap_or_else(|| panic!("the profile names its configuration: {block}"));
            for (name, value) in env {
                let value = value.as_str().expect("an env value is a string");
                assert!(
                    !meltemid::config::looks_like_plaintext_secret(value),
                    "`{name}` looks like a secret in the clear: {block}"
                );
                // A reference is the whole value or none of it: one inside a
                // longer text is not expanded, and would launch the agent with
                // the variable empty.
                assert!(
                    !value.contains('$') || (value.starts_with("${") && value.ends_with('}')),
                    "`{name}` mixes a reference with text: {value}"
                );
            }
        }
    }
}

// Scenario: Tres recetas con su perfil
#[test]
fn three_recipes_each_with_its_file_and_its_profile() {
    let guide = guide();
    for recipe in ["## Recipe 1", "## Recipe 2", "## Recipe 3"] {
        let start = guide
            .find(recipe)
            .unwrap_or_else(|| panic!("the guide has `{recipe}`"));
        let body = &guide[start..];
        let body = body[3..].find("\n## ").map_or(body, |end| &body[..end + 3]);
        assert_eq!(
            blocks(body, "json").len(),
            1,
            "`{recipe}` shows the agent's file"
        );
        assert_eq!(
            blocks(body, "toml").len(),
            1,
            "`{recipe}` shows the profile that selects it"
        );
        assert!(
            body.contains("opencode models"),
            "`{recipe}` says how to check it"
        );
    }
}

// Scenario: La guía declara con qué se verificó
#[test]
fn the_guide_says_when_what_and_how_far_it_was_verified() {
    let guide = guide();
    let verified = guide
        .lines()
        .find(|line| line.contains("Verified:"))
        .expect("the guide declares its verification");
    // A date, and the agent's version.
    let date = verified
        .split_whitespace()
        .find(|word| {
            let word = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '-');
            word.len() == 10 && word.as_bytes()[4] == b'-' && word.as_bytes()[7] == b'-'
        })
        .is_some();
    assert!(date, "the verification is dated: {verified}");
    assert!(verified.contains("OpenCode 1."), "{verified}");
    // And how far: in form, and whether live.
    assert!(guide.contains("**in form**") && guide.contains("**live**"));
}
