# Unused dependency declarations — a compiler-verified census

> **Complete FOR `[dependencies]`. REOPENED 2026-09-18 for
> `[dev-dependencies]`, which the detector below cannot see.** All 78
> workspace members with a `src/lib.rs` (66 in `crates/`, 12 in `game/`) ran
> through the detector on 2026-09-18 at `455b35876`+. Reproducible: `find
> crates game -name lib.rs -path '*/src/lib.rs'` names exactly this
> population — verified zero-diff against the sweep's own crate list the day
> this page was finished. 61 crates never produced a hit under the
> default-features detector, and 17 had at least one (one of those 17,
> `ambition_encounter_features`, is now ALSO clean after its fix — see
> "Clean crates" below for why it is listed in both places on purpose).
>
> ⛔⛤ **AND "NO FURTHER WORK" IS WHAT THIS PAGE GOT WRONG.** The section "an
> `--all-targets` run answers a different question" below states the gap in
> this page's own words — *a `[dev-dependencies]` entry can be STRANDED with
> zero signal from any `--lib` run* — and the page then declared the census
> complete anyway. A zero from an instrument that cannot see a table is not a
> zero about that table. The 61 were never asked the dev-dependency question,
> and neither were the 17 called "fully settled": both halves are scoped to
> `[dependencies]`. Seven stranded dev-dependencies were found the moment the
> question was actually asked — see "Stranded dev-dependencies" below. The other 17 are
> each fully classified below: every hit is either a compiler-and-deletion-
> confirmed manifest fix (applied) or a confirmed real use this page names
> the evidence for (kept, unchanged). This page replaces an earlier partial
> run (2026-09-03, 34 of 75 members) whose own coverage claim could not be
> checked against the tree — see "What is owed" for that history and why it
> matters more than the number.

## The instrument, and why the obvious one does not work

⛔ **Do not answer this question with `grep`.** The question is "does this crate
use this dependency", which is a question about a resolved crate graph, and a
text search answers a different one. Measured failures of the grep version:

| the grep measured | what it got wrong |
|---|---|
| occurrences in `src/` | **4 false positives of 6** — deps used in the crate's own `tests/` |
| occurrences of TEXT | **a false negative** — `ambition_abilities -> ambition_items` looked live because the name appears once, in an intra-doc link inside a `//!` comment |
| `ambition_*` names only | missed every third-party dep — `thiserror`, `ron`, `bevy_input` |
| `image::` / `bevy::image::` | a false positive — the umbrella's own re-exported module reads identically to the standalone crate in a text search |

⇒ `rustc` has answered this since 1.44. **The detector:**

```
cargo rustc -p <crate> --lib -- -W unused_crate_dependencies
```

Prefer `cargo rustc` over a `RUSTFLAGS=` run for this stage: the flag then
applies to that one crate, so the shared target cache is not invalidated and
no dependency is rebuilt. On a warm cache a crate costs seconds.

⚠ **THE DETECTOR OVER-REPORTS, AND THE CONFIRMER IS NOT OPTIONAL.** `--lib`
compiles with DEFAULT features, so a dependency whose only production use sits
behind a non-default `#[cfg(feature = …)]` is genuinely unused *in that build*
and the lint says so — correctly — while the dep is not remotely removable.
Roughly half of this page's hits were this shape. The confirmer:

```
cargo rustc -p <crate> --lib --all-features -- -W unused_crate_dependencies
```

⇒ Run the confirmer over the HITS, not over every crate: two runs on the hits,
not two runs on 78.

⛔⛤ **AND THAT RULE HAS AN EXCEPTION THAT POINTS THE OTHER WAY — MEASURED
2026-09-17 BY DELETING THE LINE.** The confirmer assumes a WIDER build can only
REVEAL a use the narrow one hid. For a dependency declared to activate a
feature, the wider build can SUPPLY that feature from somewhere else and hide
the need instead. `ambition_input`'s `bevy_input` is the worked case: unused
by NAME in both stages, so the "unused under both = unused" rule calls it
removable — and removing it fails the DEFAULT build with `the trait bound
KeyCode: serde::Serialize is not satisfied`, the exact error its manifest
comment predicts. Under `--all-features` it compiles without the line, because
`leafwing-input-manager` turns on `bevy_input/serialize` itself.

⇒ **THE ONLY SETTLING INSTRUMENT FOR A ZERO-NAME DEPENDENCY IS DELETING THE
LINE AND BUILDING — at default features, not only at `--all-features`.** A
lint that answers "is this crate named" cannot answer "does this crate's
feature have to be on", and a confirmer that only widens can be wrong in the
removable direction. Every STRANDED verdict on this page that had a
`features = [...]` entry on its own line went through this delete-and-build
step before being called removable, not the lint alone.

⛔⛤ **A THIRD CONFIRMER STAGE IS NEEDED, AND ITS FIRST VERSION SILENTLY DID
NOTHING — MEASURED 2026-09-18.** `--lib` reports per-BUILD-TARGET, so a
dependency used only under `tests/` or `#[cfg(test)]` reads as unused on
`--lib` alone even after the `--all-features` confirmer. The natural third
stage —

```
cargo rustc -p <crate> --all-targets -- -W unused_crate_dependencies
```

— is **invalid** the moment a crate has more than one build target (a
`[[bin]]`, or more than one file under `tests/`): rustc hard-errors with
"extra arguments to `rustc` can only be passed to one target", suggesting a
single-target filter flag instead, and cargo writes that error to the log in
place of any warning. Every one of the first 9 crates run this way —
`ambition_app` included — produced that error, and a grep for
`"is unused in crate"` against an errored log correctly finds nothing, which
reads exactly like a real zero-hit confirmation. **An absence-based check must
prove the run happened before it may report nothing**; this one didn't, for
9/14 hit-crates, before it was caught.

⇒ Fixed by setting the lint through `RUSTFLAGS` instead of `-- <args>` — cargo
forwards an env var identically to every rustc invocation it spawns,
regardless of how many targets the crate has:

```
RUSTFLAGS="-W unused_crate_dependencies" cargo check -p <crate> --all-targets
```

**Positive-controlled before trusting it**: `ambition_abilities` has a known
real hit (`ambition_items`, confirmed at both earlier stages); re-run under
this invocation, its `--all-targets` log reproduced the same warning for both
the `(lib)` and `(lib test)` copies. An instrument whose pass condition is an
absent string needs a case where the string is present, or it cannot tell a
clean result from a run that never happened.

⚠ **One parsing wrinkle**: `RUSTFLAGS` applies to *every* rustc invocation in
that cargo command, not only the target crate's — a crate's `--all-targets`
log can carry another crate's own unused-dependency warnings from lower in
the build graph (e.g. `ambition_input`'s warnings appeared inside
`ambition_abilities`'s log, since `ambition_abilities` pulls it in
transitively). Grep for the exact `` is unused in crate `<name>` `` string,
not any warning line in the file. A second wrinkle inside the same log: a
warning attributed to `<name>` can belong to a DIFFERENT compiled target
within that same package (an example, a `(lib test)` unit-test build, a
second `[[test]]` binary) than the one where a dependency is actually needed
— `ambition_content`'s and `ambition_platformer2d_host`'s dev-dependencies
below are exactly this shape, and it is normal, not a finding, when one
target's "unused" verdict does not hold for the crate's other targets.

⚠ **And it has a real cost the plain `cargo rustc -p <crate> -- <args>` form
does not**: because the flag is part of the environment rather than scoped to
one package, cargo's fingerprint for every dependency in the graph changes
the first time it sees this `RUSTFLAGS` value, which invalidates the shared
target cache for the whole workspace, not just the one crate under test. Each
crate's `--all-targets` confirmer pass in this sweep paid a several-minute-
to-tens-of-minutes rebuild of common dependencies (bevy, wgpu, etc.) rather
than the seconds a warm-cache `--lib`/`--all-features` run costs. Budget for
it; do not mistake a stuck-looking sweep for a hang.

⇒ **A `[dev-dependencies]` entry can be STRANDED with zero signal from any
`--lib` run, default or `--all-features`.** Dev-dependencies never enter a
non-test build, so the only instrument that can see one at all is
`--all-targets` (or a direct `--tests` run). `ambition_content`'s `insta`
below was found exactly this way — invisible to the first two stages, a real
STRANDED finding under the third.

## What a hit means — six outcomes, not one

A confirmed hit is not automatically a deletion. Sorting them is the actual work:

| class | what it is | what to do |
|---|---|---|
| **STRANDED** | no occurrence anywhere in the crate | remove the line |
| **REDUNDANT-UMBRELLA** | never named, but the umbrella re-export is used (`bevy_input` declared, `bevy::input` used 11×), or an identical edge is already declared elsewhere in the same build graph | removing the direct dep is safe — but it does **not** mean the feature or configuration is unused, and the table must not imply that |
| **DOC-ONLY** | named only in a doc comment / intra-doc link | **two edits**, the dep line and the link — and see the ruling below. A backtick reference to a path (`` `crate::path` ``) is prose, not a link, and needs only the one edit |
| **MISFILED** | used only in test code (`tests/`, or `#[cfg(test)]` inside `src/`) | move to `[dev-dependencies]`; if it is ALSO doc-linked, the lib's rustdoc is not given dev-deps, so expect the link to break — confirm with `cargo doc -p <crate>` |
| **FEATURE-GATED** | named in the crate's own `[features]` table (`causal = ["dep:ambition_causal"]`) | **not a delete** — it is public feature surface, and removing it changes what downstream crates can enable, even with zero direct code usage |
| **FEATURE-ACTIVATION** | declared in order to TURN A FEATURE ON, never to be named (`bevy_input = { features = ["serialize"] }`) | ⛔ **not a delete unless the delete-and-build test at DEFAULT features clears it** — the lint reports whether a crate is NAMED, and this shape exists so that somebody else's derive exists. Some feature-activation-shaped deps turn out removable (the feature was live-but-unneeded); the shape is a warning to test, not a verdict |

⭐ **Maintainer ruling, 2026-09-03 (coordinator), on DOC-ONLY:** *keep the
dependency and keep the doc link.* A dep whose only use is an intra-doc
cross-reference is not debt — the link is a real service to a reader, the cost
is one manifest line, and deleting both to satisfy a lint trades documentation
for tidiness.

## Confirmed results — all 17 hit-crates, fully settled

⚠ This heading and the paragraph below it read **"14 hit-crates"** and
**"64 of 78... needed no further work"** until 2026-09-18. Counted by
parsing both tables below rather than by re-reading them: the union of
crate names across "Manifest changed" (9) and "Confirmed real use... kept"
(12, four crates shared with the first table) is **17**, not 14, and the
"Clean crates" list beneath carried an eighteenth name —
`ambition_game_shell` — with no annotation explaining the overlap, even
though its own row above (`ambition_persistence`, FEATURE-GATED, real use)
means it is not zero-hit. Removed from that list; it belongs only here.
`ambition_encounter_features` is the one crate legitimately in both places,
because its one stranded edge was fixed and the crate is now clean under a
fresh run — see its note in "Clean crates".

61 of 78 crates produced zero hits under the default-features detector and
needed no further work (listed at the bottom, "Clean crates"). These 17 each
had at least one hit; every hit below has been through the detector, the
`--all-features` confirmer, and (where the finding wasn't already settled by
those two) the `--all-targets` confirmer or a direct delete-and-build test.

### Manifest changed (16 edges, across 8 crates)

⚠ This heading read *"11 edges, across 7 crates"* until 2026-09-18 while the
table under it held 17 rows across 9 crates. A count of a list that sits
directly above the list is the cheapest of all numbers to check and was never
checked. Recounted by parsing the table rather than by re-reading it — and
recounted AGAIN the same day, down to 16/8, once `ambition_content_pack`
turned out not to belong here at all (see the note on its row, moved below).

| crate | dependency | class | evidence |
|---|---|---|---|
| `ambition_abilities` | `ambition_boss_encounter` | STRANDED — removed | 0 occurrences |
| `ambition_abilities` | `ambition_gameplay_trace` | STRANDED — removed | 0 occurrences. Both this and the row above are the carve-strandage case: the crate carved that same night has **zero** `cfg(feature` in its entire `src/` (no conditional path can hide a use), and `--all-targets` checks clean in both default (17.06s) and `--features test-support` (1.72s) configurations — a carve moves code out and leaves the source crate's declaration behind, because nothing fails when a dependency stops being named |
| `ambition_damage` | `ambition_projectiles` | MISFILED — moved to `[dev-dependencies]` | only `crates/ambition_damage/src/tests.rs` names `ProjectileKind`; `crates/ambition_damage/src/lib.rs:1098` is backtick prose (`` `ambition_projectiles::kind::ProjectileKind::spec` ``), not an intra-doc link — no rustdoc risk, so this needed only the one edit |
| `ambition_encounter_features` | `ambition_interaction` | MISFILED — moved to `[dev-dependencies]` | both uses (`PickupKind`, `Chest`) in `src/tests.rs`. Visible in `fixtures/minimal_game`'s sentinel lockfile: the crate dropped out of the minimal profile's closure once the edge moved — a misfiled dev-dependency is not tidiness, it is a crate a shipped profile linked in order to run nobody's tests |
| `ambition_app` | `serde` | MISFILED — moved to `[dev-dependencies]` | only `tests/replay_fixture_regression.rs` (a submodule of the aggregated `tests/app_it.rs` binary) names it |
| `ambition_app` | `serde_json` | MISFILED — moved to `[dev-dependencies]` | only `tests/gravity_symmetry_room.rs` (same aggregate binary) names it |
| `ambition_app` | `ron` | STRANDED — removed | 0 occurrences anywhere (`lib`, `bin`, `tests/`, `examples/`); plain, not optional, not wired through any `dep:ron` feature entry. Delete-and-build clean at default (5m51s) and `--all-features` (5m26s) |
| `ambition_app` | `image` | STRANDED — removed | 0 occurrences of the standalone crate (every `image::` hit was `bevy::image::ImagePlugin`, the umbrella's own module). ⚠ **THE ORIGINAL EVIDENCE HERE WAS WRONG AND IS CORRECTED, 2026-09-18.** It said the identical edge is "already declared identically by `ambition_platformer2d_actor_monolith` (a real dependency of `ambition_app`), `ambition_render`, and `ambition_app_tools`, so removing the redundant copy changes no effective feature unification". None of those three supplies `image/png` to `ambition_app`: actor-monolith's is a `[dev-dependencies]` entry, `ambition_render`'s is optional behind its `capture` feature, and `ambition_app_tools` is a separate package, not a dependency of this one. The REMOVAL still stands — `ambition_app` names the crate nowhere, and PNG support reaches the visible compositions through Bevy's own presentation stack — but it stands on the zero use, not on a unification argument that was never established. Classed REDUNDANT on that bad argument; it is plain STRANDED |
| `ambition_content` | `serde_json` | MISFILED — moved to `[dev-dependencies]` | every use (`src/encounters/tests.rs`, `src/intro/tests.rs`, `src/intro/route_state/tests.rs`) is inside a `#[cfg(test)]` module |
| `ambition_content` | `insta` (dev) | STRANDED — removed | 0 occurrences anywhere in the crate, including all 12 files aggregated into `tests/content_it.rs`. Found only via `--all-targets` — invisible to any `--lib` run, since dev-dependencies never enter one |
| `ambition_demo_smash` | `serde` | STRANDED — removed | 0 occurrences (word-boundary grep, not just `serde::`, ruling out a bare derive reached through `use serde::{Serialize}`); plain, not feature-wired |
| `ambition_platformer2d_actor_monolith` | `bevy_math` | STRANDED — removed | `features = ["serialize"]`, 0 direct usage — feature-activation SHAPE, but the delete-and-build test (the only settling instrument for this shape) compiled clean at both default (10m03s) and `--all-features` (3m58s) |
| `ambition_platformer2d_actor_monolith` | `bevy_common_assets` | STRANDED — removed | `features = ["ron"]`, named only in one backtick prose comment at `src/session/data.rs:4`. Same delete-and-build clearance |
| `ambition_platformer2d_actor_monolith` | `parry2d` | STRANDED — removed | plain, not optional, 0 occurrences |
| `ambition_platformer2d_actor_monolith` | `petgraph` | STRANDED — removed | plain, not optional, 0 occurrences |
| `ambition_platformer2d_ldtk` | `ron` | STRANDED — removed | plain (`{ workspace = true }`), not optional, not feature-wired, 0 occurrences. Delete-and-build clean at default (8m01s) and `--all-features` (2m44s) |

Verified per crate: `cargo check -p <crate> --lib` (default and
`--all-features`) and `cargo test -p <crate> --no-run` all clean after each
edit; `test_sub_workspace_lockfiles_are_current.py` re-run after every
manifest change (see "What is owed" — a MOVE is a lockfile change in every
independent sub-workspace that transitively holds the crate, not a surprise
found later). Every sub-workspace break this campaign caused
(`examples/capability_demo`, `fixtures/headless_profile`,
`fixtures/minimal_game`, at different points for different edges) was fixed
with `cargo update --workspace --offline` in that sub-directory and is green
as of the commits this page cites.

### Confirmed real use, or already correct — no edit (kept)

| crate | dependency | class | evidence |
|---|---|---|---|
| `ambition_content_pack` | `thiserror` | real use, no edit | ⛔⛤ **THIS ROW READ "STRANDED — removed, 0 occurrences, no `derive(…Error)` at all" UNTIL 2026-09-18, AND IT WAS NEVER TRUE.** `crates/ambition_content_pack/src/artifact.rs:35` has carried `#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]` since `fd50efde7`, 2026-09-11 — a week before this page's own dated measurement. The manifest edge was never actually deleted (checked: still a plain `[dependencies]` entry, unconditional, no `[features]` table in this crate at all), so nothing shipped broken; only the write-up was wrong, and it said "removed" about an edit that was never made. Found re-verifying this table against the tree it describes, not by a bug report. Moved out of "Manifest changed" into this table, where it always belonged |
| `ambition_abilities` | `ambition_items` | DOC-ONLY | named once, in an intra-doc link inside a `//!` comment. **Ruled 2026-09-03: keep both** |
| `ambition_input` | `bevy_input` | FEATURE-ACTIVATION | `features = ["serialize"]` is why the line exists. Deleting it and building DEFAULT features fails on `KeyCode: serde::Serialize` — the ambition_input/bevy_input worked trap this page's methodology section cites throughout |
| `ambition_input` | `ambition_entity_catalog` | FEATURE-GATED, real use | 5 non-test references |
| `ambition_input` | `bevy_window` | FEATURE-GATED, real use | 1 non-test reference (`active_input.rs:19`, `use bevy_window::CursorMoved;`) |
| `ambition_encounter` | `ron` | FEATURE-GATED, real use | `crates/ambition_encounter/src/content_schema.rs:56`, behind `#[cfg(feature = "content_pack")]` at `crates/ambition_encounter/src/lib.rs:11` |
| `ambition_dialog` | `ambition_persistence` | FEATURE-GATED, real use | `crates/ambition_dialog/src/systems.rs:18,410`, behind `#[cfg(feature = "ui")]`. Re-read 2026-09-17: the load-bearing symbol narrowed from `save::AmbitionGameSave` to `persistence::settings::{MenuTapMode, UserSettings}`; the dependency itself is unchanged |
| `ambition_dialog` | `ambition_input` | FEATURE-GATED, real use | `crates/ambition_dialog/src/systems.rs:16` (`ActiveDevice`, `MenuControlFrame`, `SeatActiveDevices`), behind `#[cfg(feature = "input")]` |
| `ambition_game_shell` | `ambition_persistence` | real use, mixed | 15 non-test references (was 11 when this row was written; grown by unrelated feature work since, re-measured 2026-09-18). Not one shape: `plugin.rs`'s 2 are unconditional (its `mod plugin;` is ungated on purpose, and consumes `audio_controls`, itself deliberately ungated too — see that file's own header comment), while `pause_menu.rs`/`basic_presentation.rs`'s 9 sit behind `#[cfg(feature = "basic_presentation")]` at the `mod` declaration in `lib.rs`. "FEATURE-GATED" alone overstated it; this dependency is plainly real in the default build, feature-gating is only part of its use |
| `ambition_load_presentation` | `ambition_input` | FEATURE-GATED, real use | `crates/ambition_load_presentation/src/basic_presentation.rs:49`, `crates/ambition_load_presentation/src/deterministic_activity.rs:77,134` — behind `#[cfg(feature = "basic_presentation")]` at `crates/ambition_load_presentation/src/lib.rs:14`, non-default (`default = []`) |
| `ambition_load_presentation` | `ambition_platformer2d_shared_tangle` | FEATURE-GATED, real use | `crates/ambition_load_presentation/src/basic_presentation.rs:4` — same gate |
| `ambition_platformer2d_host` | `ambition_input` | FEATURE-GATED, real use | extensive use in `crates/ambition_platformer2d_host/src/lib.rs`, cleared by `--all-features` |
| `ambition_platformer2d_host` | `ambition_menu` | FEATURE-GATED, real use | `optional = true`, `dep:ambition_menu` in the `render` feature; use at `crates/ambition_platformer2d_host/src/lib.rs:622,630` |
| `ambition_platformer2d_host` | `ambition_characters`, `ambition_platformer2d_provider` | already correct | already in `[dev-dependencies]`, used only by `tests/demo_shell_smoke.rs`. `--all-targets` flags them because the warning is scoped to a DIFFERENT compiled target within the same package (the `(lib test)` unit tests, which don't happen to need them) — not because the crate as a whole is wrong |
| `ambition_platformer2d_actor_monolith` | `bevy_inspector_egui`, `virtual_joystick`, `bevy_framepace` | FEATURE-GATED | `dep:` entries in `dev_tools`, `mobile_touch`, `frame_pacing` respectively; 0 direct usage but public feature surface |
| `ambition_platformer2d_actor_monolith` | `ambition_platformer2d_ldtk` (optional, `[dependencies]`) | FEATURE-GATED | `dep:ambition_platformer2d_ldtk` in `portal`/`portal_ldtk`. 0 production usage outside `#[cfg(test)]`, but public feature surface — a separate, deliberate `[dev-dependencies]` copy of the same crate name already exists for the tests, with its own comment explaining the split |
| `ambition_platformer2d_ldtk` | `bevy_asset_loader` | FEATURE-GATED | `optional = true`, `dep:bevy_asset_loader` in `ldtk_runtime`, which is in `default = ["ldtk_runtime", "portal_ldtk"]`. 0 direct usage but public feature surface |
| `ambition_content` | `ambition_content` (self, dev), `ambition_content_cli` (dev), `yarnspinner` (dev) | already correct | each shows "unused" only in the `(lib test)` target while genuinely used in `tests/content_it.rs` submodules — a dev-dependency not needed by every test target is normal |
| `ambition_sim_view` | `leafwing_input_manager` | FEATURE-GATED + test use | `src/facts.rs:779` behind `#[cfg(feature = "input")]`, and `src/control_prompt.rs:815` <!-- cite-test: the cell's own evidence class is "test use" --> inside a `#[test]` fn |
| `ambition_touch_input` | `ambition_geometry`, `ambition_input`, `ambition_platformer2d_shared_tangle`, `serde` | FEATURE-GATED, real use | all four gated behind `input`/`mobile_touch` (non-default; `default = []`) in `src/bevy_plugin.rs` and `src/layout.rs` (a `#[derive(Serialize, Deserialize)]`) |

⭐ **`ambition_encounter_features`/`ron` — a since-corrected row.** It sat here
once justified by a bare pointer to
`crates/ambition_encounter_features/src/loading.rs:22` <!-- cite-test: the row records the bare pointer that turned out to be test-only -->, the only row in this
table's history without a stated evidence class. That line was inside
`ENCOUNTER_WAVE_BOOK_FIXTURE`, `#[cfg(test)]`, and the crate's own docs say
production embeds no encounter wave data — the dependency moved to
`[dev-dependencies]` (see the manifest-changed table) and this row is gone
rather than reworded. In a table whose justification column is otherwise
consistent, the one row that doesn't match the shape of its neighbours is the
one to check first.

## Stranded dev-dependencies — the half the detector cannot see

⛔⛤ **THIS SECTION EXISTS BECAUSE THE PAGE CLOSED WITHOUT IT.** The detector is
a `--lib` run, and a `[dev-dependencies]` entry never enters one. The page said
so, in the section above, and then reported 61 crates as needing "no further
work" on the strength of a measurement that was never pointed at their
dev-dependency tables. Raised in an outside review, 2026-09-18.

**Method, and why it is not the detector.** There is no compiler flag here that
is both authoritative and affordable: `-W unused_crate_dependencies` is part of
cargo's fingerprint, so turning it on rebuilds the workspace — which is how this
target directory filled the disk three times, and why `check_no_warnings.py`
refuses to reach for `RUSTFLAGS`. So this sweep is textual, and its weakness is
named rather than hidden: it parses each manifest with `tomllib` (NOT by
splitting on the string `[dev-dependencies]`, which matches a MENTION of the
table in a comment — the first version of this sweep did exactly that and
reported five deps from the wrong crate's `[dependencies]` block), collects both
the plain table and every `[target.'cfg(..)'.dev-dependencies]`, and asks whether
any `.rs` under `src/`, `tests/`, `benches/` or `examples/` contains a
`name::`/`name!` reference, following a `package = ` rename.

⇒ **This is no longer a one-off sweep: `scripts/check_dev_dependencies_are_used.py`
(landed `9bfc20cab`) is the same method as a standing guard**, and it covers a
WIDER population than this page's 78 — every `Cargo.toml` in the tree (88
manifests, 18 with a `[dev-dependencies]` table), not only crates with a
`src/lib.rs`. Re-run against HEAD while writing this correction:
`ok: every dev-dependency is named by its own crate (18 crate(s) with a
[dev-dependencies] table, 88 manifest(s) scanned)` — the seven fixes below are
what took it from red to that green, and it is the instrument that now backs
this section's completeness claim, not a re-reading of this table.

⚠ **A substring is not a reference, and this is where the sweep would have
lied.** A loose grep for `insta` in `game/ambition_app` returns 562 hits and in
actor-monolith 443 — every one of them inside the words `install`, `installs`,
`installed`. Requiring `insta::` or `insta!` is what makes the zero real. A
finding here is a CANDIDATE; each of the seven below was then settled by
deleting the line and building the crate's `--all-targets`, which is the same
delete-and-build instrument the `[dependencies]` half used.

| crate | dev-dependency | evidence | settled by |
|---|---|---|---|
| `ambition_persistence` | `tempfile` | 0 `tempfile::` references in the crate | `cargo check -p ambition_persistence --all-targets`, clean |
| `ambition_platformer2d_actor_monolith` | `insta` | 0 `insta::`/`insta!`; the 443 raw hits are all `install*` | `cargo check -p ambition_platformer2d_actor_monolith --all-targets`, clean |
| `ambition_platformer2d_actor_monolith` | `proptest` | 0 references | same run |
| `ambition_platformer2d_actor_monolith` | `tempfile` | 0 references. ⚠ Its comment CLAIMED a use — "Isolated scratch dirs for the asset-publish fixture tests" — and no such use exists; the comment went with the line | same run |
| `game/ambition_app` | `insta` | 0 `insta::`/`insta!`; the 562 raw hits are all `install*` | `cargo check -p ambition_app --all-targets`, clean |
| `game/ambition_app` | `proptest` | 0 references | same run |
| `fixtures/content_builder` | `ron` | 0 `ron::`; the two raw hits are this crate's own `.to_ron()` method and one comment | built standalone (it has its own `[workspace]` and lockfile), `cargo check --all-targets` + `cargo test`, clean; `test_authoring_needs_no_engine.py` 10/10 |

⭐ **AND THE FIXTURE IS AN INSTRUMENT, so tightening it is the point rather than
tidiness.** `fixtures/content_builder`'s own header says its dependency list IS
the assertion — that authoring a move needs no engine. An unused dev-dependency
sitting in that list is a line the assertion does not mean.

⚠ **ONE WARNING CAME WITH IT, and it was invisible for a structural reason worth
recording beside the others on this page.** `fixtures/content_builder` is
OUTSIDE the workspace on purpose, so `cargo check --workspace` never builds it
and `check_no_warnings.py` cannot see it at all: `WindowTag` was imported at
module scope, used only inside `#[cfg(test)]`, and warned on every lib build
nobody ran. Moved into the test module. That is the third distinct way this
repository's warning gate reads clean over code that warns — the other two
(non-default `cfg(feature)`, and workspace feature unification) are disclosed in
the gate's own output.

## Clean crates — 62, zero hits under the default-features detector

⚠ Read 64 until 2026-09-18 and carried `ambition_game_shell` with no
annotation — it has a real, kept `ambition_persistence` hit above and is not
zero-hit; moved out (see "Confirmed results" for the recount). 61 of these
62 never produced a hit; `ambition_encounter_features` is the other one,
listed here because its hit was fixed rather than kept.

`ambition_asset_manager`, `ambition_audio`, `ambition_binding`,
`ambition_body_seed`, `ambition_boss_encounter`, `ambition_causal`,
`ambition_character_sprites`, `ambition_characters`, `ambition_combat`,
`ambition_content_cli`, `ambition_conversation`, `ambition_cutscene`,
`ambition_demo_mary_o`, `ambition_demo_mary_o_app`, `ambition_demo_pocket`,
`ambition_demo_sanic`, `ambition_demo_sanic_app`, `ambition_demo_smash_app`,
`ambition_demo_twintrack`, `ambition_demo_twintrack_app`,
`ambition_dev_tools`, `ambition_encounter_features` (after its move above),
`ambition_engine_schemas`, `ambition_entity_catalog`,
`ambition_gameplay_trace`, `ambition_geometry`, `ambition_held_items`,
`ambition_interaction`, `ambition_inventory_ui`, `ambition_items`,
`ambition_load`, `ambition_match`, `ambition_menu`,
`ambition_menu_kaleidoscope`, `ambition_mount`, `ambition_persistence`,
`ambition_platformer2d`, `ambition_platformer2d_actor_spawn`,
`ambition_platformer2d_core`, `ambition_platformer2d_provider`,
`ambition_platformer2d_rollback_ggrs`, `ambition_platformer2d_runtime`,
`ambition_platformer2d_shared_tangle`, `ambition_platformer2d_world`,
`ambition_portal2d`, `ambition_portal2d_presentation`,
`ambition_projectile_spec`, `ambition_projectiles`, `ambition_registry_core`,
`ambition_relativity`, `ambition_relativity2d`, `ambition_render`,
`ambition_settings_menu`, `ambition_sfx`, `ambition_sfx_bank`,
`ambition_sim_harness`, `ambition_sprite_fx`, `ambition_sprite_sheet`,
`ambition_time`, `ambition_ui_nav`, `ambition_vfx`, `ambition_world_items`.

⚠ **"Zero hits" is not a stronger guarantee than the methodology above
states.** A crate here could still hold a FEATURE-ACTIVATION-shaped
dependency that is genuinely load-bearing and simply never showed up as a
hit in either stage — that is exactly `ambition_input`/`bevy_input`'s shape,
and it was caught by a maintainer's suspicion of a `features = [...]` line,
not by the sweep. This list means "the detector found nothing to sort", not
"every edge in these manifests is proven necessary."

## What is owed

▢ **Two structural blind spots this page's own tooling cannot close, stated
rather than silently assumed clean:**

1. **`[target.'cfg(...)'.dependencies]` tables.** A naive manifest read (this
   page's own crate/feature extraction included) only sees the top-level
   `[dependencies]`/`[dev-dependencies]`/`[build-dependencies]` tables. ⚠
   **THIS BULLET NAMED THREE CRATES AND MISSED AN EDGE, WHICH IS EXACTLY THE
   "LUCK, NOT VERIFIED" IT WARNED ABOUT.** Recounted 2026-09-18 by parsing
   every `[target.'cfg(...)'.*]` table with `tomllib` (all three kinds, not by
   re-reading this sentence): three crates, six edges —
   `ambition_dev_tools`/`libc` under `cfg(unix)`;
   `ambition_persistence`/`web-sys` under `cfg(target_arch = "wasm32")`;
   `ambition_app`/`mimalloc` and `ambition_app`/`oboe` under
   `cfg(target_os = "android")` (`oboe` is the one this bullet never named);
   `ambition_app`/`getrandom_03` and `ambition_app`/`getrandom_04` under
   `cfg(target_arch = "wasm32")`. None overlap this page's `[dependencies]`
   hit list — now a checked property of all six, not an assumption about
   three. A tool that reads manifests for this purpose again should parse all
   four table shapes, not the three most common ones.
2. **`#[cfg(target_arch = "wasm32")]`-gated code is unverifiable from this
   host, structurally, not by omission.** `ambition_app`'s
   `console_error_panic_hook` and `wasm_bindgen` are `dep:`-gated behind
   `web_platform` AND live only under wasm32 `cfg` — no `--all-features` run
   on a native machine can ever exercise that code, however many flags are
   set, because the gate is the TARGET, not a feature. The honest verdict for
   both is "feature-gated and unverifiable from this host", not "verified
   clean" — a `--all-features` pass that reports no warning for a
   wasm32-only symbol proves nothing about whether it is truly used, only
   that this host never compiled the code path that would use it.

▢ **A post-carve checklist step.** The grep that started this page's
predecessor row is still worth running as a cheap smoke test at carve time —
but over the WHOLE crate, never `src/` alone, and understood as a detector
whose hits go through the confirmer, never as a verdict on its own.

▢ **Every dependency MOVE is a lockfile change in every independent
workspace that transitively holds the crate — not a surprise found later.**
Measured repeatedly this campaign (`ambition_damage`→`ambition_projectiles`,
`ambition_content`, `ambition_demo_smash`, `ambition_platformer2d_actor_monolith`,
`ambition_platformer2d_ldtk`): a manifest edit broke `cargo tree --locked` in
one or more of `examples/capability_demo`, `fixtures/headless_profile`, and
`fixtures/minimal_game` almost every time, each needing only
`cargo update --workspace --offline` in that sub-directory to drop the stale
line. Run `python3 -m pytest
scripts/tests/test_sub_workspace_lockfiles_are_current.py` after each
manifest edit, not once at the end where a batch of failures reads as noise
instead of one edit at a time.

▢ **The next re-derivation should re-run the detector against a fresh `HEAD`
before trusting any row on this page**, including the settled ones — this is
a compiler-verified snapshot of one commit range, not a standing guarantee.
The methodology section above is the part expected to still be true; the
per-crate table is a receipt.

## Relationship to the architecture review

The counts and compiler runs above are historical receipts for their stated
source, not a permanent verdict independent of future manifest changes. A
prior architecture review inventoried 79 workspace packages with no Rust
toolchain available; this page's 78 is a compiler-verified count from the
same-era tree and the two are not in tension — re-run the detector and
confirmer for any future concrete migration rather than subtracting old
scanned counts from a new package total.

A9 in the [frontier](../engine/actor-monolith-work-frontier.md) asks a
different question: what does an advertised external profile actually
require? A dependency can be genuinely used and still be wrong for a
promised render-absent profile. Conversely, deleting a direct edge may leave
the same package reachable through another crate. Keep usage, feature
activation, closure, and linked size separate questions.

Retain the recorded DOC-ONLY dependency ruling unless the maintainer changes
it; show its profile cost explicitly rather than treating it as unused code.
Nothing on this page grants permission to delete documentation support or
public feature surface (the FEATURE-GATED class) to shrink a count — every
removal above is a STRANDED, REDUNDANT, or MISFILED edge the compiler and a
deletion experiment both confirmed, not a minimization campaign.
