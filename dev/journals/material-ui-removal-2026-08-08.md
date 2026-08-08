# `bevy_material_ui` removal — what actually made the title test flip

2026-08-08. Investigation before deletion. Raw observations first.

⭐ **What this campaign is, stated plainly up front:** removing a dependency
nothing imports, and deleting a `⛔` warning that was false. It also removes
160 s of rustc work from a clean build — a real but secondary benefit, worth
~20 s of wall clock (see the compile section for why it is not 160 s). **It is
not a compile-performance win**, and framing it as one would repeat the very
error that produced the comment being deleted.

## The claim under test

`game/ambition_app/src/app/plugins.rs` carried (since 2026-08-03) a comment saying
`bevy_material_ui::dialog::DialogPlugin` is "load-bearing": dropping it makes
`the_title_screen_says_choose_game_and_is_readable` read the title at 20.0px
"because menu typography stops resolving". Found by a six-step bisect. The bisect
proved a CORRELATION; the mechanism was never checked.

## Observation 1 — baseline is green, and the resolver is a no-op here

`cargo test -p ambition_app --test app_it -- shell_host_rendered::the_title_screen_says_choose_game_and_is_readable`
→ **ok** on the current tree (Material installed).

## Observation 2 — the full `Text` population, WITH Material

Probe `probe_dump_title_screen_text_population` (temporary) dumps every `Text`
entity after `settle()`: entity, font size, `MenuTextHeightFraction`, `Name`,
ancestor chain, and whether an ancestor is `BasicShellUiRoot`.

29 text entities. The launcher subtree (`1175v0[bevy_ui menu root]`):

```
1179v0 size= 20.00 frac=None       label="Choose Game"  < tab[0] < menu tab bar < menu panel < root
1181v0 size= 60.48 frac=Some(5.6)  label="Ambition"     < menu body < menu panel < root   name="text"
1183v0 size= 20.00 frac=None       label="Ambition"     < control  < menu body  < menu panel < root
1185v0 size= 20.00 frac=None       label="Sanic"        < control  < ...
1187v0 size= 20.00 frac=None       label="Mary-O"       < control  < ...
1189v0 size= 20.00 frac=None       label="Pocket"       < control  < ...
1191v0 size= 20.00 frac=None       label="TwinTrack"    < control  < ...
1193v0 size= 20.00 frac=None       label="Smash"        < control  < ...
1195v0 size= 20.00 frac=None       label="Versus"       < control  < ...
1197v0 size= 20.00 frac=None       label="Exit"         < control  < ...
1198v0 size= 23.76 frac=Some(2.2)  label="Arrow keys select · Enter launches"  name="text"
```

Plus 18 non-launcher texts (FPS overlay, GGRS proof HUD, frame-axis glyphs, the
touch overlay cluster). No debug HUD "Ambition" — there is no session.

```
=== 'Ambition' entities: [(1181v0, 60.48), (1183v0, 20.0)] ===
=== primary windows: 0 ===
=== UiPlugin present: true ===
```

**⭐ THERE ARE TWO "Ambition" TEXTS AND BOTH ARE THE LAUNCHER'S.** `1181v0` is
the title. `1183v0` is the *game-select row label* — "Ambition" is the first game
in the roster, so the row for the game and the title of the screen carry the
identical string. The test's `rendered.iter().find(|(label,_)| label == "Ambition")`
picks whichever the global query reaches first.

**And `resolve_menu_text_size` cannot be the story**: primary windows = 0, so the
resolver's `unwrap_or(MENU_REFERENCE_VIEWPORT_HEIGHT)` branch is taken and it
writes exactly the size the spawner already wrote. 5.6% of 1080 = 60.48 — the
observed value. Running it or not running it is indistinguishable in this
composition. **The comment's stated mechanism is false.**

Row labels are 20.0px because `spawn_control` spawns them with no `font_size` and
no `MenuTextHeightFraction` — `TextFont`'s Bevy default is 20.0. That is where the
"Bevy's default 20.0" in the old comment came from; it was always a real entity,
not a failed resolve.

## Observation 3 — the full `Text` population, WITHOUT Material

Both plugins commented out of `add_ui_plugins`. Same probe.

**29 text entities. Same labels. Same sizes. The title is still 60.48px.**
The only difference in the whole dump is that every entity index shifted by
one or two (Material's `Startup: setup_dialog_overlay` spawns an entity), and
`ArchetypeId`s renumbered.

`=== 'Ambition' entities: [(1180v0, 60.48), (1182v0, 20.0)] ===` — the same two.

And the test FAILED:

```
panicked at tests/shell_host_rendered.rs:801:
the title renders at 20.0px — this is the units bug, not a taste question
```

**The world did not change. The test's answer did.**

## Observation 4 — the mechanism, by archetype

The probe was widened to print the query's own iteration order (unsorted) with
each entity's `ArchetypeId`. The title lives in the archetype that carries
`MenuTextHeightFraction`; the row labels live in the one that does not.

WITH Material — walk order by archetype `133, 118, 89, 120, 131, 94, 127`:

```
[ 2] 1198v0 arch=118  23.76  "Arrow keys select · Enter launches"
[ 3] 1181v0 arch=118  60.48  "Ambition"        <-- TITLE reached first
...
[12] 1183v0 arch=120  20.00  "Ambition"
```

WITHOUT Material — walk order by archetype `88, 86, 108, 117, 132, 126, 130`:

```
[ 2] 1182v0 arch=108  20.00  "Ambition"        <-- ROW LABEL reached first
...
[12] 1180v0 arch=117  60.48  "Ambition"
```

⭐ **That is the entire cause.** `bevy_material_ui` was never touching typography.
Its plugins register components and spawn a startup entity, which shifts
archetype/table creation order, which flipped which of two identically-labelled
entities a global `find()` reached first. Five days of "load-bearing" came from a
query-order coincidence.

## Observation 5 — the reviewer's `TextFont` hypothesis, tested

Hypothesis offered mid-investigation: *"an `"Ambition"` text with NO explicit
`TextFont` would produce Bevy's default size and explain the 20px immediately."*

**Confirmed in substance, with one correction.** `TextFont::default().font_size`
is **20.0**, and the row label reads exactly that (`bevy_default_size=true`). But
it is not that the entity lacks a `TextFont` — it cannot, `Text` requires one.
`spawn_control` (`crates/ambition_menu/src/render/bevy_ui/spawn.rs:245`) inserts
`TextFont { font: font.cloned().unwrap_or_default(), ..default() }` — it sets the
font HANDLE and leaves the SIZE at `..default()`. Same outcome, different reason,
and the difference matters: the handle was deliberately fixed there (the TOFU
work), the size was simply never considered.

Every launcher row label, and the "Choose Game" tab head, is a
`bevy_default_size=true` / `frac=None` node. So **the launcher's rows do not
scale with the window while its title and footer do.** That is a real
inconsistency in our own text path, it is NOT what made this test flip, and it is
left alone here — changing row typography is a layout/design change, not a
dependency removal.

## Verdict

1. `bevy_material_ui` was never load-bearing. The comment in `plugins.rs` was
   wrong and has been deleted, with the true finding put in its place.
2. The test was wrong: it let CONTENT TEXT act as ENTITY IDENTITY. It now scopes
   to the `BasicShellUiRoot` subtree, selects the title by ROLE (a launcher text
   node carrying `MenuTextHeightFraction` — which a control's label never has),
   and asserts the match is UNIQUE so a future duplicate fails loudly.
3. **Poison check**: with Material removed, the archetype order still puts the
   20px row label first (probe index 2, title at index 12) — i.e. the exact
   condition that produced the old false failure is still present — and the fixed
   test passes. The guard defends the gap, not just the fix.

---

# Measurements after removal

## Tests

| job | result |
|---|---|
| `cargo test -p ambition_app` | **320 passed, 0 failed, 10 ignored** + 1 doc-test, 135.9 s |
| `python3 -m pytest scripts/tests -q` (repo tooling / contracts) | **251 passed** |
| `python3 scripts/check_absence_contracts.py --check` | **25 of 25 hold** |
| `cargo check -p ambition_app --all-targets` | clean, 23.1 s |

⚠ **the repo-tooling job failed on the first run and the reason is a standing
lesson biting again.** `test_sub_workspace_lockfiles_are_current` reported
`fixtures/external_consumer/Cargo.lock is STALE`. I had checked for sibling
lockfiles with `git ls-files '*Cargo.lock'`, which returned four paths and NOT
that one — because `fixtures/external_consumer/.gitignore:2` ignores it
deliberately (Outlander is an external consumer; its lock is generated locally,
not committed). **A git-aware search cannot see a gitignored file, and the
checker discovers sub-workspaces from the FILESYSTEM.** Fixed with
`cd fixtures/external_consumer && cargo update --workspace --offline`; nothing to
commit, since the file is ignored by design.

⭐ and that refresh reported something the main lock diff confirms: **five
packages left the graph, not two.** `bevy_material_ui`,
`google-material-design-icons-bin`, `hct-cam16`, `lz4_flex`, `png`.

## Update schedule census — Material's ACTUAL share

Measured with `update_schedule_census::census_of_how_much_of_update_is_inside_a_set`,
same App (`build_visible_app(NoWindow, true)`), same command, Material re-added
temporarily for the "before" and reverted by editing the bytes back.

| | `Update` total | in a set | in NONE |
|---|---|---|---|
| with Material | 482 | 231 | 251 (52%) |
| without | 456 | 231 | 225 (49%) |
| **delta** | **−26** | **0** | **−26** |

`GgrsSchedule` is unchanged at 534 either way. The independent
`[schedule-census]` instrument agrees exactly: `Update` 463 → 437, also −26.

⚠ **so the number in the deleted comment, "584 → 428", was not Material's
share** — it was the whole schedule before and after trimming the full
30-plugin `MaterialUiPlugin` bundle down to the core+dialog pair, which happened
on 2026-08-03 and is already landed. What the pair still cost was **26 systems**:
5.4% of `Update`, and 10.4% of its unsetted population. Every one of the 26 was
unsetted — the count of systems inside a set did not move by one.

## Compile — CPU WORK REMOVED, not wall time saved

One apples-to-apples build: commit `03878f81b`, **profile `test`**,
`collector: dev/first-party`, `dirty=false`, 533 units, rustc 1.95.0,
`max_concurrency: 7 (jobs=8 ncpu=8)`.

| unit | frontend | codegen | total |
|---|---|---|---|
| `bevy_material_ui` | 14.31 | 132.49 | **146.80 s** |
| `google-material-design-icons-bin` | 1.13 | 12.12 | **13.25 s** |
| **the two named units** | | | **160.05 s** |
| `png` (two units) | 1.70 | 3.13 | 4.83 s |
| `hct-cam16` | 0.13 | 0.34 | 0.47 s |
| `lz4_flex` | 0.14 | 0.03 | 0.17 s |
| with the three transitives that also left | | | 165.52 s |

**After: the units do not exist.** `cargo tree -i bevy_material_ui` and
`-i google-material-design-icons-bin` both answer *"did not match any
packages"*, and all five are gone from `Cargo.lock`,
`fixtures/minimal_game/Cargo.lock` and `fixtures/external_consumer/Cargo.lock`.
That is exact, not estimated — there is no "after" row to quote because there is
no unit. It is a correctness check, and it passes.

### ⛔ 160 s of rustc work is NOT 160 s off the build, and it is not even 160 s off a COLD build

Two independent reasons, both measured on that same build:

1. **The build is already saturated.** 4153.3 s of unit work completes in
   539.9 s of wall clock — average parallelism **7.69 of 8 cores**. Removing
   160.05 s is **3.9% of total work**, so the wall-clock order is
   `160 / 7.69 ≈ 20 s of 540 s`.
2. **`bevy_material_ui` is not on the critical path.** It runs 358.6 → 505.4 and
   finishes **19 s before** the tail even begins:
   `monolith (385.4 → 524.4) → ambition_app (524.4 → 539.9)`. Units in flight
   are 8/8 at t=500 s, 6 at t=515 s, 1 at t=530 s — the last 35 s are the serial
   chain this crate is not in.

⚠ **and ~20 s is below this measurement's noise floor.** Two clean dev builds
are on record at 539.9 s and 833.7 s — a 54% spread. A single before/after pair
cannot resolve a 20 s difference, so producing one would be noise presented as a
result. **No clean build was run for this campaign, deliberately.**

⛔ It is also not a REBUILD win at all: these are third-party units, compiled
once and cached for every subsequent build. The edit→test loop is unchanged.

⭐ **So the honest framing of this whole campaign: it removes a dependency
nothing imports and deletes a false `⛔` warning. The 160 s is CPU work that no
longer happens on a clean build — a real but secondary benefit, worth ~20 s of
wall clock and unresolvable against this build's run-to-run spread.** It is not
a compile-performance win, and calling it one would repeat the exact error that
produced the comment this campaign deleted.

⚠ **the error being avoided, named**: quoting a per-unit ledger number as if it
were a build number. `dev/journals/compile-cost-what-actually-drives-it-2026-08-08.md`
§5 already says a unit's `cargo --timings` duration is wall time *inside a real
build sharing 8 cores* — the input for PRIORITISING, never a subtractable build
cost. I wrote the pooled "160–236 s" version of this claim before checking, and
that range pooled dev with release across four commits.

## What was NOT done, deliberately

- Row-label typography. Every launcher row and the tab head are 20.0px with no
  `MenuTextHeightFraction`, so they do not scale with the window while the title
  and footer do. Real, unrelated to this, and a layout decision.
- The `crates/ambition_game_shell/src/launcher.rs:65-85` TOFU mystery ("why the
  default handle fails here is NOT settled"). Untouched. This investigation did
  produce one adjacent datum for whoever picks it up: `spawn_control` sets the
  font handle and leaves `font_size` at `..default()`, so the row labels take the
  resolved `MenuFont` handle and Bevy's default SIZE — the two halves of
  `TextFont` are set from different places, which is the kind of split that
  mystery lives in.
