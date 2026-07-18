// SPDX-License-Identifier: Apache-2.0

//! Emits the generated CLI reference to stdout. Regenerate the committed doc
//! with: `cargo run -q --example gen_cli_ref > docs/referencia-cli.md`.

fn main() {
    print!("{}", meltemi::cli::reference());
}
