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

## What a hit means — five outcomes, not one

A confirmed hit is not automatically a deletion. Sorting them is the actual work:

| class | what it is | what to do |
|---|---|---|
| **STRANDED** | no occurrence anywhere in the crate | remove the line |
| **REDUNDANT-UMBRELLA** | never named, but the umbrella re-export is used (`bevy_input` declared, `bevy::input` used 11×) | removing the direct dep is safe — but it does **not** mean the feature is unused, and the table must not imply that |
| **DOC-ONLY** | named only in a doc comment / intra-doc link | **two edits**, the dep line and the link — and see the ruling below |
| **MISFILED** | used only in test code | move to `[dev-dependencies]`; if it is ALSO doc-linked, the lib's rustdoc is not given dev-deps, so expect the link to break — confirm with `cargo doc -p <crate>` |
| **FEATURE-GATED** | named in the crate's own `[features]` table (`causal = ["dep:ambition_causal"]`) | **not a delete** — it is public feature surface, and removing it changes what downstream crates can enable |

⭐ **Maintainer ruling, 2026-09-03 (coordinator), on DOC-ONLY:** *keep the
dependency and keep the doc link.* A dep whose only use is an intra-doc
cross-reference is not debt — the link is a real service to a reader, the cost is
one manifest line, and deleting both to satisfy a lint trades documentation for
tidiness.

## Confirmed results, 34 crates

**Genuinely unused, confirmed under default AND `--all-features`:**

| crate | dependency | class |
|---|---|---|
| `ambition_abilities` | `ambition_boss_encounter` | STRANDED — remove |
| `ambition_abilities` | `ambition_gameplay_trace` | STRANDED — remove |
| `ambition_content_pack` | `thiserror` | STRANDED — remove (0 occurrences, and the crate has no `derive(…Error)` at all) |
| `ambition_input` | `bevy_input` | REDUNDANT-UMBRELLA — `bevy::input` used 11× |
| `ambition_abilities` | `ambition_items` | DOC-ONLY — **ruled: keep both** |
| `ambition_damage` | `ambition_projectiles` | MISFILED **and** doc-linked (`crates/ambition_damage/src/lib.rs:1082`) |
| `ambition_encounter_features` | `ambition_interaction` | MISFILED — both uses in `tests.rs` |

⭐ **`ambition_abilities` is the carve-strandage case and it is worth its own
sentence.** Its two stranded deps are in the crate carved that same night, it
contains **zero** `cfg(feature` in its entire `src/` (so no conditional path can
hide a use), and both `--all-targets` configurations check clean — default in
17.06 s and `--features test-support` in 1.72 s. ⇒ **A carve moves code out and
leaves the source crate's DECLARATION behind, because nothing fails when a
dependency stops being named.** That is the process finding; the count is not.

**Reported by the detector, then cleared by the confirmer — production code
behind a non-default feature. These are NOT removable:**

| crate | dependency | where it is really used |
|---|---|---|
| `ambition_encounter` | `ron` | `crates/ambition_encounter/src/content_schema.rs:48`, behind `#[cfg(feature = "content_pack")]` at `crates/ambition_encounter/src/lib.rs:11` |
| `ambition_encounter_features` | `ron` | `crates/ambition_encounter_features/src/loading.rs:22` |
| `ambition_dialog` | `ambition_persistence` | `crates/ambition_dialog/src/bridge.rs:26`, `crates/ambition_dialog/src/systems.rs:18`, behind that crate's `#[cfg(feature = "ui")]` modules |
| `ambition_game_shell` | `ambition_persistence` | 11 non-test references |
| `ambition_input` | `ambition_entity_catalog` | 5 non-test references |
| `ambition_input` | `bevy_window` | 1 non-test reference |

⇒ **The detector alone would have produced a delete list with real production
code on it.** That is the whole argument for the confirmer.

## What is owed

▢ **41 crates unscanned**, including several the original grep row named —
`ambition_platformer2d_host`, `game/ambition_content`, `game/ambition_app`,
`ambition_platformer2d`, `ambition_sim_view`, `ambition_touch_input`. ⛔ **The
grep-era claims about those crates are therefore still unverified**, and the
four "misfiled" edges reported from the text search have NOT been through the
compiler. Do not act on them from this page.

Unscanned: `ambition_app`, `ambition_content`, `ambition_demo_mary_o`,
`ambition_demo_mary_o_app`, `ambition_demo_pocket`, `ambition_demo_sanic`,
`ambition_demo_sanic_app`, `ambition_demo_smash`, `ambition_demo_smash_app`,
`ambition_demo_twintrack`, `ambition_demo_twintrack_app`,
`ambition_menu_kaleidoscope`, `ambition_platformer2d`,
`ambition_platformer2d_actor_monolith`, `ambition_platformer2d_core`,
`ambition_platformer2d_host`, `ambition_platformer2d_ldtk`,
`ambition_platformer2d_provider`, `ambition_platformer2d_rollback_ggrs`,
`ambition_platformer2d_runtime`, `ambition_platformer2d_shared_tangle`,
`ambition_platformer2d_world`, `ambition_portal2d`,
`ambition_portal2d_presentation`, `ambition_projectile_spec`,
`ambition_projectiles`, `ambition_registry_core`, `ambition_relativity`,
`ambition_relativity2d`, `ambition_render`, `ambition_settings_menu`,
`ambition_sfx`, `ambition_sfx_bank`, `ambition_sim_harness`, `ambition_sim_view`,
`ambition_sprite_sheet`, `ambition_time`, `ambition_touch_input`,
`ambition_ui_nav`, `ambition_vfx`, `ambition_world_items`.

▢ **A post-carve checklist step.** The grep that started this is still worth
running as a cheap smoke test at carve time — but over the WHOLE crate, never
`src/` alone, and understood as a detector whose hits go through the confirmer.

⚠ **Why this page states its coverage in the first line.** The row it replaces
said "six", and six was neither the number of unused declarations nor a number
any single instrument had produced. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md)
— *"the error is not a bad tool, it is a claim wider than the tool's scope"*.
