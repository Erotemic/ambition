# Unused dependency declarations: a compiler-verified census

**Status:** the `[dependencies]` census of every workspace member with a
`src/lib.rs` was completed and applied on 2026-09-18. `[dev-dependencies]` are
covered by the standing guard `scripts/check_dev_dependencies_are_used.py`.
Manifest comments in `game/ambition_app`, `game/ambition_content` and
`crates/ambition_damage` cite this page as their evidence.

## Finding

A carve moves code out of a crate and leaves its dependency declarations
behind. `grep` cannot tell whether a crate uses a dependency: it misses
test-only use, doc-only names, re-exports (`bevy::image::` is not the `image`
crate) and substrings (`insta` matches `install`). Use the compiler.

What remains unchecked:

- `[target.'cfg(...)'.*]` dependency tables. A tool that reads manifests must
  parse all four table shapes.
- Code under `#[cfg(target_arch = "wasm32")]` cannot be checked from a native
  host with any feature flag.
- `fixtures/content_builder` is outside the workspace, so
  `check_no_warnings.py` does not build it.
- A crate with zero detector hits can still hold a feature-activation
  dependency.

## Evidence command

Run the stages in order, on the hits of the previous stage only:

1. Detector, on a warm cache (seconds):
   `cargo rustc -p <crate> --lib -- -W unused_crate_dependencies`
2. Confirmer (a use behind a non-default feature reads as unused in stage 1):
   `cargo rustc -p <crate> --lib --all-features -- -W unused_crate_dependencies`
3. All targets, for test-only use:
   `RUSTFLAGS="-W unused_crate_dependencies" cargo check -p <crate> --all-targets`.
   This rebuilds the workspace graph. Do not use
   `cargo rustc --all-targets -- <args>`: with more than one target it fails,
   and a grep of that log looks clean. Run a positive control first, and grep
   for the exact `` is unused in crate `<name>` `` string.
4. Delete the line and build at default features and at `--all-features`. This
   is the only settling test for a dependency that is never named. A wider build
   can supply a feature from another crate and hide the need (`ambition_input`'s
   `bevy_input` is unused by name but needed for `KeyCode: serde::Serialize` at
   default features).

| class | meaning | action |
|---|---|---|
| STRANDED | no occurrence in the crate | remove the line |
| REDUNDANT-UMBRELLA | never named, but the umbrella re-export is used | the direct line can go; the feature may still be used |
| DOC-ONLY | named only in a doc comment or intra-doc link | keep (maintainer ruling, 2026-09-03) |
| MISFILED | used only in test code | move to `[dev-dependencies]`; check `cargo doc -p <crate>` if it is also doc-linked |
| FEATURE-GATED | named in the crate's `[features]` (`dep:x`) | keep: public feature surface |
| FEATURE-ACTIVATION | declared to turn a feature on, never named | keep unless stage 4 passes at default features |

Do not delete documentation support or public feature surface to reduce a
count. After each manifest edit, run `cargo update --workspace --offline` in
each sub-workspace that holds the crate (`examples/capability_demo`,
`fixtures/headless_profile`, `fixtures/minimal_game`) and
`python3 -m pytest scripts/tests/test_sub_workspace_lockfiles_are_current.py`.

## Owner

None. Whoever carves a crate checks its source manifest.

## Trigger to promote

A carve or crate split. Re-run stages 1–2 on the source crate in the same
change. Usage, feature activation, closure and linked size are separate
questions: a used dependency can still be wrong for a render-absent profile
(A9 in [the work frontier](../engine/actor-monolith-work-frontier.md)).
