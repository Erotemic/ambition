# Unused dependency declarations: a compiler-verified census

**Status:** the `[dependencies]` census is complete for all 78 workspace members
with a `src/lib.rs` (66 in `crates/`, 12 in `game/`), measured 2026-09-18.
`[dev-dependencies]` are covered by the standing guard
`scripts/check_dev_dependencies_are_used.py`. The per-crate tables are a
snapshot of one commit range. Re-run the detector at a fresh `HEAD` before you
trust a row. The method and the classes are expected to stay true.

## The instrument

Do not answer "does this crate use this dependency" with `grep`. Measured grep
failures: dependencies used only in the crate's `tests/` (4 false positives of
6); a name that appears only in an intra-doc link; third-party dependencies
missed by an `ambition_*` filter; `bevy::image::` read as the standalone
`image` crate; `insta` matching `install`.

Use the compiler, in stages:

1. Detector, per crate, on a warm cache (seconds; does not invalidate the
   shared target cache):

   ```
   cargo rustc -p <crate> --lib -- -W unused_crate_dependencies
   ```

2. Confirmer, on the hits only. `--lib` uses default features, so a use behind
   a non-default `#[cfg(feature = ...)]` reads as unused:

   ```
   cargo rustc -p <crate> --lib --all-features -- -W unused_crate_dependencies
   ```

3. All targets, for dependencies used only under `tests/` or `#[cfg(test)]`.
   Do not use `cargo rustc --all-targets -- <args>`: with more than one build
   target it fails with "extra arguments to `rustc` can only be passed to one
   target", and a grep of that log finds no warning and looks like a clean
   result. Use:

   ```
   RUSTFLAGS="-W unused_crate_dependencies" cargo check -p <crate> --all-targets
   ```

   This changes the fingerprint of every dependency and rebuilds the workspace
   graph (minutes to tens of minutes). Before you trust an absence, run a
   positive control on a crate with a known hit. Grep for the exact
   `` is unused in crate `<name>` `` string: the log also holds other crates'
   warnings, and a warning can belong to a different target of the same
   package (for example, `(lib test)` when a `[[test]]` binary needs the
   dependency).

4. Delete the line and build, at default features and at `--all-features`.
   This is the only settling test for a dependency that is never named. A
   wider build can supply a feature from another crate and hide the need:
   `ambition_input`'s `bevy_input` is unused by name in every stage, but
   removing it fails the default build on `KeyCode: serde::Serialize`, because
   only `leafwing-input-manager` turns on `bevy_input/serialize` under
   `--all-features`.

A `[dev-dependencies]` entry never enters a `--lib` build. Only stage 3 or a
`--tests` run can see it.

## What a hit means

| class | what it is | what to do |
|---|---|---|
| STRANDED | no occurrence anywhere in the crate | remove the line |
| REDUNDANT-UMBRELLA | never named, but the umbrella re-export is used (`bevy_input` declared, `bevy::input` used) | the direct line can go; this does not mean the feature is unused |
| DOC-ONLY | named only in a doc comment or intra-doc link | keep (ruling below). A backticked path in prose is not a link |
| MISFILED | used only in test code | move to `[dev-dependencies]`. If it is also doc-linked, check `cargo doc -p <crate>`, because the lib's rustdoc does not get dev-dependencies |
| FEATURE-GATED | named in the crate's own `[features]` table (`dep:x`) | not a delete: it is public feature surface |
| FEATURE-ACTIVATION | declared to turn a feature on (`features = ["serialize"]`), never named | not a delete unless stage 4 passes at default features |

**Maintainer ruling, 2026-09-03, DOC-ONLY:** keep the dependency and keep the
doc link. The link serves a reader; the cost is one manifest line.

Nothing on this page permits deleting documentation support or public feature
surface to reduce a count.

## Manifest changes made (2026-09-18)

| crate | dependency | change |
|---|---|---|
| `ambition_abilities` | `ambition_boss_encounter`, `ambition_gameplay_trace` | STRANDED, removed (left behind by a carve) |
| `ambition_damage` | `ambition_projectiles` | MISFILED, moved to dev |
| `ambition_encounter_features` | `ambition_interaction`, `ron` | MISFILED, moved to dev |
| `ambition_app` | `serde`, `serde_json` | MISFILED, moved to dev |
| `ambition_app` | `ron`, `image` | STRANDED, removed |
| `ambition_content` | `serde_json` | MISFILED, moved to dev |
| `ambition_content` | `insta` (dev) | STRANDED, removed |
| `ambition_demo_smash` | `serde` | STRANDED, removed |
| `ambition_platformer2d_actor_monolith` | `bevy_math`, `bevy_common_assets` | STRANDED, removed after stage 4 |
| `ambition_platformer2d_actor_monolith` | `parry2d`, `petgraph` | STRANDED, removed |
| `ambition_platformer2d_ldtk` | `ron` | STRANDED, removed |
| `ambition_persistence` | `tempfile` (dev) | STRANDED, removed |
| `ambition_platformer2d_actor_monolith` | `insta`, `proptest`, `tempfile` (dev) | STRANDED, removed |
| `ambition_app` | `insta`, `proptest` (dev) | STRANDED, removed |
| `fixtures/content_builder` | `ron` (dev) | STRANDED, removed. The fixture's dependency list is its assertion that authoring needs no engine |

## Hits that are real use (keep)

| crate | dependency | why it stays |
|---|---|---|
| `ambition_content_pack` | `thiserror` | `derive(thiserror::Error)` in `artifact.rs` |
| `ambition_abilities` | `ambition_items` | DOC-ONLY, ruled keep |
| `ambition_input` | `bevy_input` | FEATURE-ACTIVATION; removal fails the default build |
| `ambition_input` | `ambition_entity_catalog`, `bevy_window` | feature-gated real use |
| `ambition_encounter` | `ron` | real use behind `content_pack` |
| `ambition_dialog` | `ambition_persistence`, `ambition_input` | real use behind `ui` / `input` |
| `ambition_game_shell` | `ambition_persistence` | real use, partly ungated (`plugin.rs`), partly behind `basic_presentation` |
| `ambition_load_presentation` | `ambition_input`, `ambition_platformer2d_shared_tangle` | real use behind `basic_presentation` |
| `ambition_platformer2d_host` | `ambition_input`, `ambition_menu` | real use behind features |
| `ambition_platformer2d_host` | `ambition_characters`, `ambition_platformer2d_provider` (dev) | used by `tests/demo_shell_smoke.rs`; the `(lib test)` target does not need them |
| `ambition_platformer2d_actor_monolith` | `bevy_inspector_egui`, `virtual_joystick`, `bevy_framepace`, `ambition_platformer2d_ldtk` | FEATURE-GATED public surface |
| `ambition_platformer2d_ldtk` | `bevy_asset_loader` | FEATURE-GATED public surface |
| `ambition_content` | self, `ambition_content_cli`, `yarnspinner` (dev) | used in `tests/content_it.rs` submodules |
| `ambition_sim_view` | `leafwing_input_manager` | real use behind `input`, and test use |
| `ambition_touch_input` | `ambition_geometry`, `ambition_input`, `ambition_platformer2d_shared_tangle`, `serde` | real use behind `input` / `mobile_touch` |

The other 61 crates had zero hits under the default-features detector. That
means the detector found nothing to sort, not that every edge is proven
necessary: a FEATURE-ACTIVATION dependency can hide in a zero-hit crate.

## Blind spots

- `[target.'cfg(...)'.*]` tables. A tool that reads manifests must parse all
  four table shapes. At the census date these held six edges, none in the hit
  list: `ambition_dev_tools`/`libc` (unix); `ambition_persistence`/`web-sys`,
  `ambition_app`/`getrandom_03`, `ambition_app`/`getrandom_04` (wasm32);
  `ambition_app`/`mimalloc`, `ambition_app`/`oboe` (android).
- Code under `#[cfg(target_arch = "wasm32")]` cannot be checked from a native
  host with any feature flag. `ambition_app`'s `console_error_panic_hook` and
  `wasm_bindgen` are "feature-gated and not verifiable from this host".
- `fixtures/content_builder` is outside the workspace, so
  `check_no_warnings.py` does not build it.

## Procedure for the next change

- After a carve, check the source crate's manifest. A carve moves code out and
  leaves the declaration behind. A grep over the whole crate (not `src/` only)
  is a cheap first pass; send its hits through the confirmer.
- Every dependency move changes the lockfile of each independent sub-workspace
  that holds the crate (`examples/capability_demo`, `fixtures/headless_profile`,
  `fixtures/minimal_game`). Run `cargo update --workspace --offline` there and
  `python3 -m pytest scripts/tests/test_sub_workspace_lockfiles_are_current.py`
  after each manifest edit.
- Usage, feature activation, closure, and linked size are separate questions. A
  used dependency can still be wrong for a render-absent profile; see A9 in the
  [frontier](../engine/actor-monolith-work-frontier.md).
