// SPDX-License-Identifier: Apache-2.0

//! Scripted level-3 headless agent for Meltemi's conformance suite
//! (niveles-integracion-conformidad). It is NOT a real agent: it emits a fixed
//! JSONL progress stream to stdout — the common subset Meltemi maps to session
//! events (a text line and a done line), plus one line that does not map and
//! must be preserved raw — then exits 0. It reads no network and takes no
//! permissions (level 3 has no rich permission channel), which is exactly the
//! shape the conformance criteria for level 3 assert.

fn main() {
    // A structured line Meltemi maps to a streamed agent update.
    println!(r#"{{"type":"text","content":"headless working"}}"#);
    // A line outside the common subset: kept raw in the log, never dropped.
    println!(r#"{{"type":"tool_call","name":"grep","args":["foo"]}}"#);
    // A final structured line.
    println!(r#"{{"type":"done","ok":true}}"#);
}
