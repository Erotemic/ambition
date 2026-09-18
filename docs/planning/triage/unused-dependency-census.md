# Unused dependency declarations — a compiler-verified census

> **Partial, and the coverage is stated on purpose. Run 2026-09-03 at
> `e26a8e412` (branch `calculex-no-gpu`), covering 34 of the 75 workspace
> members that have a `src/lib.rs`.** The 41 unscanned crates are listed at the
> bottom. This page exists because the queue row it re-measures
> (`queue.md`, "SIX WORKSPACE DEPENDENCY DECLARATIONS ARE NEVER NAMED IN THEIR
> CRATE'S SOURCE") was measured with a text search, and every number that search
> produced was wrong in at least one direction.

## The instrument, and why the obvious one does not work

⛔ **Do not answer this question with `grep`.** The question is "does this crate
use this dependency", which is a question about a resolved crate graph, and a
text search answers a different one. Measured failures of the grep version, all
on this row:

| the grep measured | what it got wrong |
|---|---|
| occurrences in `src/` | **4 false positives of 6** — deps used in the crate's own `tests/` |
| occurrences of TEXT | **a false negative** — `ambition_abilities -> ambition_items` looked live because the name appears once, in an intra-doc link inside a `//!` comment |
| `ambition_*` names only | missed every third-party dep — `thiserror`, `ron`, `bevy_input` |

⇒ `rustc` has answered this since 1.44. **The detector:**

```
cargo rustc -p <crate> --lib -- -W unused_crate_dependencies
```

Prefer `cargo rustc` over a `RUSTFLAGS=` run: the flag then applies to that one
crate, so the shared target cache is not invalidated and no dependency is
rebuilt. On a warm cache a crate costs seconds.

⚠ **THE DETECTOR OVER-REPORTS, AND THE CONFIRMER IS NOT OPTIONAL.** `--lib`
compiles with DEFAULT features, so a dependency whose only production use sits
behind a non-default `#[cfg(feature = …)]` is genuinely unused *in that build*
and the lint says so — correctly — while the dep is not remotely removable.
**Six of the first sixteen hits were this.** The confirmer:

```
cargo rustc -p <crate> --lib --all-features -- -W unused_crate_dependencies
```

⇒ **Only a dependency reported unused under BOTH is unused.** Run the confirmer
over the HITS, not over every crate: two runs on the hits, not two runs on 75.

⛔⛤ **AND THAT RULE HAS AN EXCEPTION THAT POINTS THE OTHER WAY — MEASURED
2026-09-17 BY DELETING THE LINE.** The confirmer assumes a WIDER build can only
REVEAL a use the narrow one hid. For a dependency declared to activate a feature,
the wider build can SUPPLY that feature from somewhere else and hide the need
instead. `ambition_input`'s `bevy_input` is the worked case: unused by NAME in
both stages, so this page's rule calls it removable — and removing it fails the
DEFAULT build with `the trait bound KeyCode: serde::Serialize is not satisfied`,
the exact error its manifest comment predicts. Under `--all-features` it compiles
without the line, because `leafwing-input-manager` turns on `bevy_input/serialize`
itself.

⇒ **THE ONLY SETTLING INSTRUMENT FOR A ZERO-NAME DEPENDENCY IS DELETING THE LINE
AND BUILDING — at default features, not only at `--all-features`.** A lint that
answers "is this crate named" cannot answer "does this crate's feature have to be
on", and a confirmer that only widens can be wrong in the removable direction.

⛔⛤ **A THIRD CONFIRMER STAGE IS NEEDED, AND ITS FIRST VERSION SILENTLY DID
NOTHING — MEASURED 2026-09-18.** `--lib` reports per-BUILD-TARGET, so a
dependency used only under `tests/` or `#[cfg(test)]` reads as unused on `--lib`
alone even after the `--all-features` confirmer. The natural third stage —

```
cargo rustc -p <crate> --all-targets -- -W unused_crate_dependencies
```

— is **invalid** the moment a crate has more than one build target (a `[[bin]]`,
or more than one file under `tests/`): rustc hard-errors with "extra arguments
to `rustc` can only be passed to one target", suggesting a single-target filter
flag instead, and cargo writes that error to the log in place of any warning.
Every one of the first 9 crates run this way —
`ambition_app` included — produced that error, and a grep for
`"is unused in crate"` against an errored log correctly finds nothing, which
reads exactly like a real zero-hit confirmation. **An absence-based check must
prove the run happened before it may report nothing**; this one didn't, for
9/14 hit-crates, before it was caught.

⇒ Fixed by setting the lint through `RUSTFLAGS` instead of `-- <args>` — cargo
forwards an env var identically to every rustc invocation it spawns, regardless
of how many targets the crate has:

```
RUSTFLAGS="-W unused_crate_dependencies" cargo check -p <crate> --all-targets
```

**Positive-controlled before trusting it**: `ambition_abilities` has a known
real hit (`ambition_items`, confirmed at both earlier stages); re-run under this
invocation, its `--all-targets` log reproduced the same warning for both the
`(lib)` and `(lib test)` copies. An instrument whose pass condition is an
absent string needs a case where the string is present, or it cannot tell a
clean result from a run that never happened.

⚠ **One parsing wrinkle**: `RUSTFLAGS` applies to *every* rustc invocation in
that cargo command, not only the target crate's — a crate's `--all-targets` log
can carry another crate's own unused-dependency warnings from lower in the
build graph (e.g. `ambition_input`'s warnings appeared inside
`ambition_abilities`'s log, since `ambition_abilities` pulls it in
transitively). Grep for the exact `` is unused in crate `<name>` `` string, not
any warning line in the file.

⚠ **And it has a real cost the plain `cargo rustc -p <crate> -- <args>` form
does not**: because the flag is part of the environment rather than scoped to
one package, cargo's fingerprint for every dependency in the graph changes the
first time it sees this `RUSTFLAGS` value, which invalidates the shared target
cache for the whole workspace, not just the one crate under test. Each crate's
`--all-targets` confirmer pass in this sweep paid a several-minute-to-tens-of-
minutes rebuild of common dependencies (bevy, wgpu, etc.) rather than the
seconds a warm-cache `--lib`/`--all-features` run costs. Budget for it; do not
mistake a stuck-looking sweep for a hang.

## What a hit means — five outcomes, not one

A confirmed hit is not automatically a deletion. Sorting them is the actual work:

| class | what it is | what to do |
|---|---|---|
| **STRANDED** | no occurrence anywhere in the crate | remove the line |
| **REDUNDANT-UMBRELLA** | never named, but the umbrella re-export is used (`bevy_input` declared, `bevy::input` used 11×) | removing the direct dep is safe — but it does **not** mean the feature is unused, and the table must not imply that |
| **DOC-ONLY** | named only in a doc comment / intra-doc link | **two edits**, the dep line and the link — and see the ruling below |
| **MISFILED** | used only in test code | move to `[dev-dependencies]`; if it is ALSO doc-linked, the lib's rustdoc is not given dev-deps, so expect the link to break — confirm with `cargo doc -p <crate>` |
| **FEATURE-GATED** | named in the crate's own `[features]` table (`causal = ["dep:ambition_causal"]`) | **not a delete** — it is public feature surface, and removing it changes what downstream crates can enable |
| **FEATURE-ACTIVATION** | declared in order to TURN A FEATURE ON, never to be named (`bevy_input = { features = ["serialize"] }`) | ⛔ **not a delete, and the detector cannot see it in either stage** — the lint reports whether a crate is NAMED, and this dependency exists so that somebody else's derive exists |

⭐ **Maintainer ruling, 2026-09-03 (coordinator), on DOC-ONLY:** *keep the
dependency and keep the doc link.* A dep whose only use is an intra-doc
cross-reference is not debt — the link is a real service to a reader, the cost is
one manifest line, and deleting both to satisfy a lint trades documentation for
tidiness.

## Confirmed results, 34 crates

**FIVE confirmed under default AND `--all-features`** — these are the only rows
that have been through both stages:

| crate | dependency | class |
|---|---|---|
| `ambition_abilities` | `ambition_boss_encounter` | STRANDED — remove |
| `ambition_abilities` | `ambition_gameplay_trace` | STRANDED — remove |
| `ambition_content_pack` | `thiserror` | STRANDED — remove (0 occurrences, and the crate has no `derive(…Error)` at all) |
| `ambition_abilities` | `ambition_items` | DOC-ONLY — **ruled: keep both** |
| `ambition_damage` | `ambition_projectiles` | MISFILED — **fixed 2026-09-18**, moved to `[dev-dependencies]`. Re-read: `crates/ambition_damage/src/lib.rs:1099` is backtick prose (`` `ambition_projectiles::kind::ProjectileKind::spec` ``), not an intra-doc `[link]` — no rustdoc risk, so this needed only the one edit, not the two the DOC-ONLY class calls for. `cargo check`/`cargo test --no-run -p ambition_damage --lib` both clean afterward |

✔ **THE TWO DETECTOR-ONLY ROWS ARE SETTLED, 2026-09-17 — and they settled in
OPPOSITE directions, which is why the sentence below them was right to refuse
"both look safe":**

| crate | dependency | provisional class | settled |
|---|---|---|---|
| `ambition_input` | `bevy_input` | REDUNDANT-UMBRELLA — never named, `bevy::input` used 11× | ⛔ **WRONG, AND NOT REMOVABLE.** It is **FEATURE-ACTIVATION**: `features = ["serialize"]` is why the line exists. Deleting it and running `cargo check -p ambition_input` (DEFAULT features) fails on `KeyCode: serde::Serialize`. At `--all-features` it compiles, because leafwing turns the feature on |
| `ambition_encounter_features` | `ambition_interaction` | MISFILED — both uses in `tests.rs` | ✔ **CONFIRMED** unused under default AND `--all-features`; the two uses are `PickupKind` and `Chest` in `src/tests.rs`. Moved to `[dev-dependencies]` |

⇒ **"Looks safe" is what the default-features detector said about `ron` too**, and
one of these two was a wrong class rather than a wrong confidence. The confirmer
settled the second; only a DELETION settled the first.

⭐ **AND THE MOVE IS VISIBLE IN A SHIPPED PROFILE'S CLOSURE, which is the argument
for doing it at all.** `fixtures/minimal_game`'s sentinel lockfile — the one
`capability-footprint-sentinel-lockfile-is-stale` exists to keep honest — lost
`ambition_interaction` from `ambition_encounter_features`'s dependency list when
the line moved. A misfiled dev-dependency is not tidiness: it is a crate the
minimal profile linked in order to run nobody's tests.

⭐ **`ambition_abilities` is the carve-strandage case and it is worth its own
sentence.** Its two stranded deps are in the crate carved that same night, it
contains **zero** `cfg(feature` in its entire `src/` (so no conditional path can
hide a use), and both `--all-targets` configurations check clean — default in
17.06 s and `--features test-support` in 1.72 s. ⇒ **A carve moves code out and
leaves the source crate's DECLARATION behind, because nothing fails when a
dependency stops being named.** That is the process finding; the count is not.

**Reported by the detector, then cleared by the confirmer — production code
behind a non-default feature. These are NOT removable:**

⛔ **ONE ROW IN THIS TABLE WAS WRONG, AND ITS SHAPE IS WHAT GAVE IT AWAY.**
`ambition_encounter_features` / `ron` sat here justified by a bare pointer to
`crates/ambition_encounter_features/src/loading.rs:22` — while every other row states an EVIDENCE CLASS: a named <!-- cite-test: a `#[cfg(test)]` line cited ON PURPOSE — the row's claim IS that this line is a test, so a production-role citation here would mean the opposite of what it says. Triaged individually 2026-09-12, not swept. -->
feature gate, or a count of non-test references. ⇒ In a table whose
justification column is otherwise consistent, **the outlier row is the one to
check first, and the check is cheap because the neighbours define what a
sufficient answer looks like.** That line is inside
`ENCOUNTER_WAVE_BOOK_FIXTURE`, which carries `#[cfg(test)]` and whose own doc
says production embeds no encounter wave data. The dependency moved to
`[dev-dependencies]`; the row is gone rather than reworded.

⚠ The other five rows were audited at the same time and are sound.

| crate | dependency | where it is really used |
|---|---|---|
| `ambition_encounter` | `ron` | `crates/ambition_encounter/src/content_schema.rs:56`, behind `#[cfg(feature = "content_pack")]` at `crates/ambition_encounter/src/lib.rs:11`. ⚠ Was `:48` and the parsed type was `EncounterWaveBook`; it is `AuthoredWaveTimelines` now, renamed because the crate held TWO types called `EncounterWaveBook` — the bare map this schema lowers and the Bevy `Resource` newtype an App owns |
| `ambition_dialog` | `ambition_persistence` | `crates/ambition_dialog/src/systems.rs:18` and `:410`, behind that crate's `#[cfg(feature = "ui")]` modules. ⚠ **RE-READ 2026-09-17: the use NARROWED and the row's second site is gone.** This said `bridge.rs:26` too, for `use ambition_persistence::save::AmbitionGameSave` — `ambition_dialog` no longer names `AmbitionGameSave` anywhere, and every remaining reference is `persistence::settings` (`MenuTapMode`, `UserSettings`). The dependency is still real, so the row's verdict is unchanged; what moved is WHICH part of the dependency is load-bearing, which is the thing a later delete-list would be decided on |
| `ambition_dialog` | `ambition_input` | Same shape, found 2026-09-18 in the full-population re-sweep: `crates/ambition_dialog/src/systems.rs:16` (`ActiveDevice`, `MenuControlFrame`, `SeatActiveDevices`), behind `#[cfg(feature = "input")]`, non-default. Cleared by the `--all-features` confirmer |
| `ambition_game_shell` | `ambition_persistence` | 11 non-test references |
| `ambition_input` | `ambition_entity_catalog` | 5 non-test references |
| `ambition_input` | `bevy_window` | 1 non-test reference (`active_input.rs:19`, `use bevy_window::CursorMoved;`) |
| `ambition_load_presentation` | `ambition_input` | `crates/ambition_load_presentation/src/basic_presentation.rs:49`, `crates/ambition_load_presentation/src/deterministic_activity.rs:77,134` — behind `#[cfg(feature = "basic_presentation")]` at `crates/ambition_load_presentation/src/lib.rs:14`, non-default (`default = []`) |
| `ambition_load_presentation` | `ambition_platformer2d_shared_tangle` | `crates/ambition_load_presentation/src/basic_presentation.rs:4` — same `#[cfg(feature = "basic_presentation")]` gate |

✔ **Re-confirmed unchanged 2026-09-18 by the full-population re-sweep**: `ambition_input`'s two rows above and `ambition_encounter`'s `ron` row (further up this page) all reproduced identically under fresh `--lib`/`--all-features`/`--all-targets` runs — nothing regressed since the original per-row dates.

⇒ **The detector alone would have produced a delete list with real production
code on it.** That is the whole argument for the confirmer.

## `ambition_app`, fully settled 2026-09-18

The detector flagged six: `bevy_common_assets`, `bevy_yarnspinner`, `image`,
`ron`, `serde`, `serde_json`. All six are settled — three shapes, not one:

| dependency | class | evidence |
|---|---|---|
| `serde` | MISFILED | only `tests/replay_fixture_regression.rs` (a submodule of the aggregated `tests/app_it.rs` binary, see its own header comment) names it. Moved to `[dev-dependencies]` |
| `serde_json` | MISFILED | only `tests/gravity_symmetry_room.rs` (same aggregate binary) names it. Moved to `[dev-dependencies]` |
| `ron` | **STRANDED — removed** | zero occurrences anywhere in the crate (`lib`, `bin`, `tests/`, `examples/`), and it is a plain non-optional dependency — not wired through any `dep:ron` entry in the crate's own `[features]` table, so it is not FEATURE-ACTIVATION either. Delete-and-build clean at BOTH default features (5m51s) and `--all-features` (5m26s) |
| `image` | **REDUNDANT — removed** | zero occurrences of the standalone crate anywhere (every `image::` grep hit was `bevy::image::ImagePlugin`, the umbrella's own module, a false positive of the same shape the detector's own limitations table already names for `grep`). Its exact edge — `{ version = "0.25", default-features = false, features = ["png"] }` — is *already* declared identically by `ambition_platformer2d_actor_monolith` (a real dependency of `ambition_app`), `ambition_render`, and `ambition_app_tools`, so removing the redundant copy changes no effective feature unification, not just "still compiles" |
| `bevy_common_assets`, `bevy_yarnspinner` | FEATURE-GATED — kept | both `optional = true`, both wired through `dep:` in `ambition_app`'s own `[features]` table (`ui = [..., "dep:bevy_yarnspinner"]`; `bevy_common_assets` activates its `ron` feature for the RON asset loader consumed by `ambition_platformer2d_runtime`). Public feature surface, not a default-build finding |
| `console_error_panic_hook`, `wasm_bindgen` | FEATURE-GATED, and ALSO target-unverifiable | both `optional = true` behind `dep:` in `web_platform`; both additionally live only under `#[cfg(target_arch = "wasm32")]` call sites, so no `--all-features` run on this (native) host can ever exercise them regardless of which Cargo features are on — two independent reasons the sweep cannot see them, not one |

All four manifest edits (`ron` removed, `image` removed, `serde`/`serde_json`
moved) verified together: `cargo check -p ambition_app --lib` and
`cargo test -p ambition_app --no-run` both clean, the latter linking the full
182-module aggregate `tests/app_it.rs` binary — the one place `serde`/`serde_json`
now need to resolve from `[dev-dependencies]`.
`test_sub_workspace_lockfiles_are_current.py` stayed green (`ambition_app` is
not reachable from any of the three independent sub-workspaces).

## `ambition_content`, fully settled 2026-09-18

The `--lib` detector flagged only `serde_json`, but `--all-targets` surfaced two
more findings the `--lib`-only detector structurally cannot see: a
`[dev-dependencies]` entry never compiles under `--lib` at all, default or
`--all-features`, since dev-dependencies only enter the build for a test
target.

| dependency | class | evidence |
|---|---|---|
| `serde_json` | MISFILED | every use (`src/encounters/tests.rs`, `src/intro/tests.rs`, `src/intro/route_state/tests.rs`) is inside a `#[cfg(test)]` module. Moved `[dependencies]` → `[dev-dependencies]` |
| `insta` (dev) | **STRANDED — removed** | zero occurrences anywhere in the crate — checked the lib, and all 12 files aggregated into `tests/content_it.rs` (`aerial_authoring`, `boss_fight_validator`, `boss_presentation`, `boss_seeds`, `content_pack_registry`, `dialogue_lint`, `fighter_brain_ladder`, `intro_sprite_catalog`, `puppy_slug_forced_seat`, `summoned_minions_resolve`, `yarn_compile`, `yarn_condition_aliases`) |
| `ambition_content` (self, dev), `ambition_content_cli` (dev), `yarnspinner` (dev) | not a finding | each shows "unused" only in the `(lib test)` target (the library's own inline `#[cfg(test)]` modules) while genuinely used in `tests/content_it.rs` submodules (`ambition_content_cli` in `content_pack_registry.rs`, `yarnspinner` in `yarn_compile.rs`, self-reference across all 12). A dev-dependency not needed by every test target is normal, not removable — the per-target split that makes `--all-targets` necessary also means one target's "unused" verdict does not apply to the whole crate |

⇒ **The population needing the confirmer is bigger than "crates the `--lib`
detector flagged."** A `[dev-dependencies]` entry can be STRANDED with zero
signal from any `--lib` run, default or `--all-features` — the only instrument
that can see it at all is `--all-targets` (or a direct `--tests` run), because
dev-dependencies are invisible to every build that isn't compiling a test
target. This page's "run the confirmer over the hits, not over every crate"
shortcut therefore undercounts real findings for crates whose dev-dependencies
were never named-by-text-search in the first place — the exact grep-era
failure mode this page exists to end, recurring one layer down.

Verified: `cargo check -p ambition_content --lib` and
`cargo test -p ambition_content --no-run` (linking `unittests src/lib.rs` and
`tests/content_it.rs`) both clean. `test_sub_workspace_lockfiles_are_current.py`
stayed green.

## `ambition_demo_smash`, fully settled 2026-09-18

| dependency | class | evidence |
|---|---|---|
| `serde` | **STRANDED — removed** | zero occurrences anywhere in the crate (word-boundary grep, not just `serde::`, to rule out a bare `#[derive(Serialize)]` reached via `use serde::{Serialize}`) — a plain non-optional dependency, not wired through any `dep:serde` feature gate |

Verified: `cargo check -p ambition_demo_smash --lib` (default features, 8m29s —
first cache-cold run against the fixed `[dependencies]` set),
`--all-features` (1m15s), and `cargo test -p ambition_demo_smash --no-run`
all clean. `test_sub_workspace_lockfiles_are_current.py` needed
`cargo update --workspace --offline` in `examples/capability_demo`,
`fixtures/headless_profile`, and `fixtures/minimal_game` afterward (same
shape as the `ambition_damage` move), now green.

## `ambition_platformer2d_host` and `ambition_sim_view`, fully settled 2026-09-18

Neither needs a manifest edit — both hits are already-correct instances of
patterns this page has already named, re-confirmed by the full-population
sweep:

| crate | dependency | why it's not a finding |
|---|---|---|
| `ambition_platformer2d_host` | `ambition_input` | extensive real production use in `src/lib.rs`, cleared by `--all-features` (empty confirmer log) |
| `ambition_platformer2d_host` | `ambition_menu` | `optional = true`, wired through `dep:ambition_menu` in the `render` feature; real use at `src/lib.rs:622,630`, cleared by `--all-features` |
| `ambition_platformer2d_host` | `ambition_characters`, `ambition_platformer2d_provider` | **already correctly in `[dev-dependencies]`** — both used only by `tests/demo_shell_smoke.rs`. `--all-targets` flags them because that specific warning is scoped to a DIFFERENT compiled target within the same package (the same per-target split documented in the `ambition_content` section above), not because the crate as a whole is wrong |
| `ambition_sim_view` | `leafwing_input_manager` | two uses: `src/facts.rs:779` behind `#[cfg(feature = "input")]` (non-default), and `src/control_prompt.rs:815` inside a `#[test]` fn. Cleared by `--all-features` |

## `ambition_touch_input`, fully settled 2026-09-18

All four detector hits are real production use behind `ambition_touch_input`'s
own non-default features (`default = []`; `input`/`mobile_touch` gate them),
matching the same pattern as every other feature-gated row above. Cleared by
`--all-features`:

| dependency | evidence |
|---|---|
| `ambition_geometry` | `src/bevy_plugin.rs:529,547` |
| `ambition_input` | `src/bevy_plugin.rs:27,296` and elsewhere |
| `ambition_platformer2d_shared_tangle` | `src/bevy_plugin.rs:525,531` |
| `serde` | `src/layout.rs:27-28`, a `#[derive(Serialize, Deserialize)]` |

No manifest edit needed.

## `ambition_platformer2d_actor_monolith`, fully settled 2026-09-18

The detector flagged eight; four are FEATURE-GATED (`dep:` entries in this
crate's own `[features]` table — `dev_tools`, `mobile_touch`, `frame_pacing`,
and the ldtk `portal`/`portal_ldtk` group), kept. The other four had zero
occurrences anywhere in the crate and are not wired through any `dep:` feature
entry:

| dependency | class | evidence |
|---|---|---|
| `bevy_inspector_egui` | FEATURE-GATED — kept | `dev_tools = ["dep:bevy-inspector-egui"]` |
| `virtual_joystick` | FEATURE-GATED — kept | `mobile_touch = ["input", "dep:virtual_joystick"]` |
| `bevy_framepace` | FEATURE-GATED — kept | `frame_pacing = ["dep:bevy_framepace"]` |
| `ambition_platformer2d_ldtk` (`[dependencies]`, optional) | FEATURE-GATED — kept | `dep:ambition_platformer2d_ldtk` in `portal`/`portal_ldtk`. Zero production (`src/`) usage outside `#[cfg(test)]`, but public feature surface — a SEPARATE, deliberate `[dev-dependencies]` copy (undocumented duplicate name, same crate) already exists for the tests, with its own comment explaining exactly this split |
| `bevy_math` | **STRANDED — removed** | `features = ["serialize"]`, zero direct usage — a feature-activation SHAPE, but the delete-and-build test (the only settling instrument the doc's own rule allows) compiled clean at both default and `--all-features`, so nothing else in this crate's own build needed that feature through this edge |
| `bevy_common_assets` | **STRANDED — removed** | `features = ["ron"]`, named only in one backtick (not intra-doc-link) prose comment at `src/session/data.rs:4`. Same delete-and-build clearance |
| `parry2d` | **STRANDED — removed** | plain, non-optional, zero occurrences |
| `petgraph` | **STRANDED — removed** | plain, non-optional, zero occurrences |

⭐ All four removed together, verified with one delete-and-build pass:
`cargo check -p ambition_platformer2d_actor_monolith --lib` (default, 10m03s)
and `--all-features` (3m58s) both clean.

## `ambition_platformer2d_ldtk`, fully settled 2026-09-18

| dependency | class | evidence |
|---|---|---|
| `bevy_asset_loader` | FEATURE-GATED — kept | `optional = true`, `dep:bevy_asset_loader` in `ldtk_runtime`, which is in `default = ["ldtk_runtime", "portal_ldtk"]`. Zero direct usage, but public feature surface — same rule as `ambition_platformer2d_actor_monolith`'s ldtk edge above |
| `ron` | **STRANDED — removed** | plain (`{ workspace = true }`), not optional, not wired through any `dep:` entry, zero occurrences anywhere. Delete-and-build clean at default (8m01s) and `--all-features` (2m44s) |

## What is owed

▢ **33 crates unscanned** (`ambition_app`, `ambition_content`,
`ambition_demo_smash`, `ambition_platformer2d_actor_monolith`,
`ambition_platformer2d_host`, `ambition_platformer2d_ldtk`,
`ambition_sim_view`, and `ambition_touch_input` settled 2026-09-18, above),
including one more the original grep row named — `ambition_platformer2d`.
⛔ **The grep-era claims about it are therefore still unverified**, and the
four "misfiled" edges reported from the text search have NOT been through the
compiler. Do not act on them from this page.

Unscanned: `ambition_demo_mary_o`,
`ambition_demo_mary_o_app`, `ambition_demo_pocket`, `ambition_demo_sanic`,
`ambition_demo_sanic_app`, `ambition_demo_smash_app`,
`ambition_demo_twintrack`, `ambition_demo_twintrack_app`,
`ambition_menu_kaleidoscope`, `ambition_platformer2d`,
`ambition_platformer2d_core`,
`ambition_platformer2d_provider`, `ambition_platformer2d_rollback_ggrs`,
`ambition_platformer2d_runtime`, `ambition_platformer2d_shared_tangle`,
`ambition_platformer2d_world`, `ambition_portal2d`,
`ambition_portal2d_presentation`, `ambition_projectile_spec`,
`ambition_projectiles`, `ambition_registry_core`, `ambition_relativity`,
`ambition_relativity2d`, `ambition_render`, `ambition_settings_menu`,
`ambition_sfx`, `ambition_sfx_bank`, `ambition_sim_harness`,
`ambition_sprite_sheet`, `ambition_time`,
`ambition_ui_nav`, `ambition_vfx`, `ambition_world_items`.

▢ **A post-carve checklist step.** The grep that started this is still worth
running as a cheap smoke test at carve time — but over the WHOLE crate, never
`src/` alone, and understood as a detector whose hits go through the confirmer.

▢ **Every dependency MOVE is a lockfile change in every independent workspace
that transitively holds the crate — not a surprise found later.** Measured
2026-09-18: moving `ambition_damage`'s `ambition_projectiles` to
`[dev-dependencies]` broke `cargo tree --locked` in THREE unrelated sub-workspaces
(`examples/capability_demo`, `fixtures/headless_profile`,
`fixtures/minimal_game`) — each just needed `cargo update --workspace --offline`
to drop the one stale line. Run `python3 -m pytest
scripts/tests/test_sub_workspace_lockfiles_are_current.py` after each manifest
edit in this campaign, not once at the end where a batch of failures reads as
noise instead of one edit at a time.

⛔⛤ **AND THE COVERAGE LINE CANNOT BE RE-DERIVED FROM THIS PAGE — MEASURED
2026-09-17.** The workspace now holds **78 members with a `src/lib.rs`** (66 in
`crates/`, 12 in `game/`), not 75. The `Unscanned:` list above is the half that
IS re-derivable: it names 41 crates and all 41 still exist. The other half is
not — the 34 SCANNED crates are nowhere enumerated, only the ones that produced
hits, so "34 of 75" cannot be checked against the tree and 34 + 41 is three short
of the population either way. **22 live lib crates are named nowhere on this
page at all**, among them `ambition_combat`, `ambition_characters`,
`ambition_match`, `ambition_conversation` and `ambition_menu`.

⇒ The repair is not a bigger number. It is to make the SCANNED set the recorded
one, or to treat the unscanned list as the authority and derive coverage from it
— a coverage claim whose complement cannot be listed is a claim nobody can
falsify, which is the failure this page was written to end one level up.

⚠ **Why this page states its coverage in the first line.** The row it replaces
said "six", and six was neither the number of unused declarations nor a number
any single instrument had produced. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md)
— *"the error is not a bad tool, it is a claim wider than the tool's scope"*.

## Relationship to the architecture review

The counts and compiler runs above are historical receipts for their stated
source, not a fresh all-workspace unused-dependency verdict. The current review
inventoried 79 workspace packages but had no Rust toolchain. Re-run the detector
and compiler confirmer for a concrete migration rather than subtracting old
scanned counts from the new package total.

A9 in the [frontier](../engine/actor-monolith-work-frontier.md) asks a different
question: what does an advertised external profile actually require? A dependency
can be genuinely used and still be wrong for a promised render-absent profile.
Conversely, deleting a direct edge may leave the same package reachable through
another crate. Keep usage, feature activation, closure and linked size separate.

Retain the recorded doc-only dependency ruling unless the maintainer changes it;
show its profile cost explicitly rather than treating it as unused code. The A9
plan does not grant permission to delete documentation support or public feature
names to get a smaller count.
