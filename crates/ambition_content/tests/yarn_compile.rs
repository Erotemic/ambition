//! `ui`-gated guard: every registered `.yarn` source must compile.
//!
//! Yarn is compiled by `bevy_yarnspinner` at game *startup* (the asset
//! loader), so a grammar mistake — a file-level `//` comment, a node missing
//! its `===`, an unclosed `<<if>>` — surfaces only as a runtime
//! `ERROR bevy_asset` and the dialogue silently never loads. This test runs
//! the same `yarnspinner` compiler over exactly the sources
//! `ambition_content::dialogue::YARN_SOURCES` registers, compiled as one
//! project (matching startup), so cross-file references and duplicate node
//! names are caught the same way they would be at runtime.
//!
//! Gated on the `ui` feature — the only configuration where the
//! `yarnspinner` compiler crate is built. The static arity lint in
//! `dialogue_lint.rs` (no Yarn runtime) still runs everywhere.
#![cfg(feature = "ui")]

use yarnspinner::compiler::{Compiler, File};

#[test]
fn every_yarn_source_compiles() {
    let mut compiler = Compiler::new();
    for (name, text) in ambition_content::dialogue::YARN_SOURCES {
        compiler.add_file(File {
            file_name: (*name).to_string(),
            source: (*text).to_string(),
        });
    }
    if let Err(err) = compiler.compile() {
        panic!(
            "registered Yarn sources failed to compile (the game's asset loader \
             would reject these at startup):\n{err}"
        );
    }
}
