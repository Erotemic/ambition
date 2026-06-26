//! `ui`-gated guard: every `.yarn` under `assets/dialogue/` must compile.
//!
//! Yarn is compiled by `bevy_yarnspinner` at game *startup* (the asset loader),
//! so a grammar mistake — a file-level `//` comment, a node missing its `===`,
//! an unclosed `<<if>>` — surfaces only as a runtime `ERROR bevy_asset` and the
//! dialogue silently never loads. That exact class of bug (a `//` header block on
//! `hall.yarn`) reached the running game.
//!
//! This test runs the same `yarnspinner` compiler bevy uses, over exactly the
//! files [`crate::dialog::YARN_SOURCES`] registers, compiled as one project
//! (matching startup's `add_files`) so cross-file references and duplicate node
//! names are caught the same way they would be at runtime. Globbing the folder
//! instead would falsely fail on intentionally-unloaded files.
//!
//! It is gated on the `ui` feature because that is the only configuration where
//! the `yarnspinner` compiler crate is built (it ships with the dialogue
//! runtime). Lean/headless test configs skip it — the static arity lint in
//! `dialog_lint` (no Yarn runtime) still runs everywhere.

use std::path::PathBuf;
use yarnspinner::compiler::{Compiler, File};

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[test]
fn every_yarn_file_compiles() {
    let root = asset_root();

    // Compile exactly the registered sources as one project, the way the game
    // does at startup (`YarnCompiler::new().add_files(...).compile()`).
    let mut compiler = Compiler::new();
    for rel in crate::dialog::YARN_SOURCES {
        let path = root.join(rel);
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "registered Yarn source {rel} is missing or unreadable ({e}) — \
                 fix crate::dialog::YARN_SOURCES or restore the file"
            )
        });
        compiler.add_file(File {
            file_name: (*rel).to_string(),
            source,
        });
    }

    if let Err(err) = compiler.compile() {
        // `CompilerError` Displays its joined diagnostics; surface them so the
        // failure points at the offending file:line.
        panic!(
            "registered Yarn sources failed to compile (the game's asset loader \
             would reject these at startup):\n{err}"
        );
    }
}
