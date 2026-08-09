// SPDX-License-Identifier: Apache-2.0

//! The `titulo-de-sesion` scenarios that are about WHERE a title comes from
//! rather than what it says. Two of them are proven by order and by absence —
//! the title is taken before `@` expansion, and the paths with no user sentence
//! take none — which is why they are pinned against the code that decides it.
//! The conducted confirmation is task 5.2's smoke.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

// Scenario: El título sale del texto que se escribió
#[test]
fn the_title_is_taken_before_references_are_expanded() {
    let free = read("core/meltemid/src/free_session.rs");

    let derived_at = free
        .find("crate::title::derive(&params.instruction)")
        .expect("the free session derives its title from the raw instruction");
    let expanded_at = free
        .find("crate::repo_map::expand_refs(")
        .expect("and expands `@` references somewhere");
    assert!(
        derived_at < expanded_at,
        "the title is derived from the text as typed, before expansion replaces \
         a reference with the contents of a file"
    );

    // Proven by absence too: nothing derives a title from the expanded prompt.
    for expanded in ["title::derive(&prompt", "title::derive(&expanded"] {
        assert!(
            !free.contains(expanded),
            "no title is derived from expanded text: {expanded}"
        );
    }

    // And propose takes the idea the same way.
    let propose = read("core/meltemid/src/propose.rs");
    let idea_at = propose
        .find("crate::title::derive(&params.idea)")
        .expect("propose derives from the idea");
    let propose_expand = propose
        .find("crate::repo_map::expand_refs(")
        .expect("propose expands too");
    assert!(idea_at < propose_expand, "same order, same reason");
}

// Scenario: Sin instrucción de usuario no hay título inventado
#[test]
fn the_paths_without_a_user_sentence_take_no_title() {
    // A dispatched lane and `sdd/implement` are opened by a change and a task,
    // and the SDD authoring turns by a prompt the method composes. None of them
    // is a sentence somebody typed, so none of them is a name.
    let server = read("core/meltemid/src/server.rs");
    let sdd = read("core/meltemid/src/sdd_flow.rs");
    for source in [&server, &sdd] {
        assert!(
            !source.contains("crate::title::derive("),
            "these paths derive no title at all"
        );
    }

    // The dispatch record and its start event say so explicitly rather than by
    // omission, so a reader sees the decision instead of a missing field.
    let lane = server
        .split("let dispatch_record")
        .nth(1)
        .unwrap_or(&server)
        .split("};")
        .next()
        .unwrap_or("");
    assert!(
        lane.contains("title: None"),
        "the lane's record names nothing on purpose"
    );

    // And the only place a title enters `server.rs` is the resume, which
    // inherits one rather than deriving it.
    assert!(
        server.contains("title: record.title.clone()"),
        "the resume is the one path in server.rs that carries a title"
    );
}

// Scenario: Una sesión reanudada conserva el título
#[test]
fn a_resumed_session_keeps_the_name_of_the_conversation_it_continues() {
    let server = read("core/meltemid/src/server.rs");
    let resume = server
        .split("fn resume_with_instruction")
        .nth(1)
        .expect("the resume path");

    // It inherits, in the record and in the start event of the new session.
    assert_eq!(
        resume.matches("title: record.title.clone()").count(),
        2,
        "the resumed session carries the original title into both its record \
         and its first event"
    );
    // And never re-derives from the instruction that resumed it: two names for
    // one thread is what that would produce.
    assert!(
        !resume.contains("title::derive("),
        "a resume continues a conversation rather than naming a new one"
    );
    // The link that says they are the same history is still written beside it.
    assert!(
        resume.contains("resumed_from: Some(record.session_id.clone())"),
        "the resume still records what it continues"
    );
}
