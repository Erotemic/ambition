# Test-suite consolidation: action plan

**Status:** selected under [TEST-LANES](queue.md#test-lanes--keep-required-test-lanes-executable).
**Date:** 2026-10-26. **Scope:** Ambition Rust tests, first in `ambition_app`.
Current repository instructions govern implementation. Do not add meta-test
machinery or a source-count acceptance gate to carry out this plan.

This document gives a new agent a start point for reducing test maintenance cost.
It is a plan, not a claim that the tests named here are safe to delete.
Do not expand the test corpus to complete this plan.

## Why this work matters

Ambition is a Bevy game host and engine with several shipped game modes,
shared input and rendering, multi-room world state, live content reload, and
rollback simulation. Some long tests check failures at those integration
boundaries. Those tests cannot be replaced by a unit test of one component.

Ambition has a large, agent-written regression suite. Agents tend to add a new test
for each change and seldom remove older tests. This can leave repeated setup,
obsolete diagnostic code, tests of incidental details, and multiple proofs of
the same contract. Each extra test carries review, compile, and update cost.

The goal is **less code to maintain for the same or better defect detection**.
Do not optimize for a target test/source LOC ratio or a target test count.
A test is useful when it can fail for a meaningful defect that another retained
test would not catch. A test is costly when it repeats setup or asserts an
incidental detail that does not matter to the product.

The engine has real hazards at cross-system boundaries. Removing those witnesses
because they are long would make the suite smaller but less trustworthy.
This campaign must therefore distinguish repeated *implementation* from
repeated *behavioral evidence*.

## Read before editing

- [Static audit findings](test-corpus-static-audit-findings-2026-10-26.md): measured inventory, duplicates, limitations, and protected examples.
- [Initial audit and tool notes](test-corpus-static-audit-phase2.md): why the analyzer exists and what it can detect.
- [Test placement](../concepts/test-placement.md): tests belong at the lowest layer that owns their invariant; cross-crate behavior belongs in app acceptance tests.
- [Cheapest sufficient check](../recipes/cheapest-sufficient-check.md): the focused build and test commands; do not start with the full test lane.
- [Heavy app test lane](../recipes/running-the-heavy-app-it-lane.md): why a green headless or demo-app test does not always prove a shipped-host contract.
- [Reviewer guide](../reviewer-guide.md): assess runtime paths and independent authority, not a green test count or a comment.

The source files named below are under `game/ambition_app/tests/` unless a full
path is given. `game/ambition_app/Cargo.toml` sets `autotests = false` and uses
one `app_it` test binary. `tests/app_it.rs` lists its modules;
`tests/app_it_sync.rs` checks that this list stays complete. Never remove or
rename a module without checking both files.

## Evidence baseline: do not mistake it for a savings estimate

The audit data was collected on 2026-10-26. The source review used repository
commit `8e27113c7ec79b8c628d3d0a483fccc61286e425`; check the live HEAD before
making a change. The first workspace-wide scan included four alternate checkouts
under `.worktrees/`. **Do not use its 8,487-file or 51,021-test counts.**

| Baseline measure | Corrected workspace | `ambition_app` |
|---|---:|---:|
| Rust files that contain test code | 1,881 | 319 |
| Test functions found by the analyzer | 11,052 | 1,673 |
| Ignored test functions | 80 | 56 |

For the app, 283 of these files are under `tests/`; 36 are source files with
test code. The 314 repeated 10-line block groups contain overlapping windows.
Their union covers 3,826 line positions in 59 files. **These are not removable
LOC.** Only six high-similarity app test pairs were identified, and their input
differences matter. The 56 ignored app tests together have about 2,909 body
lines, so deleting all diagnostics would not resolve most of the bloat.

The numbers above are historical measurements for comparison, not a live count.
File LOC in the audit report includes non-test code when a source file also
contains tests. Do not compare that measure with a dedicated test LOC counter
without using the same classification rule.

## Step 0: establish an accurate, repeatable inventory

The analyzer is `scripts/measure/audit_test_corpus.py`. The companion tests are
`scripts/tests/test_audit_test_corpus_worktrees.py`. This campaign fixes the
original traversal: it now prunes `.worktrees/`, `worktrees/`, nested Git
checkouts, symlinked directories, `target/`, and Git metadata before visiting
the files. Filesystem mode includes untracked work. Use `--tracked-only` to
count just files in the current Git index (which excludes untracked tests and
submodules). Do not mix the two modes in a trend graph.

Run from the repository root:

```bash
cd ~/code/ambition
git status --short
git rev-parse HEAD

python3 -m unittest discover -s scripts/tests -p 'test_audit_test_corpus_worktrees.py' -v
python3 scripts/measure/audit_test_corpus.py \
  --scope game/ambition_app --out /tmp/ambition-app-test-audit
python3 scripts/measure/audit_test_corpus.py \
  --scope . --out /tmp/ambition-workspace-test-audit
```

Inspect the scan boundary and results before proposing deletions:

```bash
python3 - <<'PY'
import json
from pathlib import Path
for folder in ('ambition-app-test-audit', 'ambition-workspace-test-audit'):
    report = json.loads((Path('/tmp') / folder / 'audit.json').read_text())
    paths = [f['path'] for f in report['files']]
    assert not any('.worktrees' in p.split('/') or 'worktrees' in p.split('/') for p in paths)
    print(folder, report['summary'])
    print('Excluded:', report['excluded_tree_paths'][:12])
PY
```

If counts differ, check HEAD, new tests, feature-gated modules, scan mode,
and tool version. Do not interpret a changed count as code reduction without
comparing the actual tests.

## Step 1: remove repeated Smash match setup, keep all cases

**First edit:** `game/ambition_app/tests/smash_ride.rs` (3,259 lines, 14 tests at
baseline). The analyzer found 882 line positions in repeated code. Fourteen
tests construct the shipped Bevy app, and 13 share match-ready polling logic.

Build small, private helpers in that file first. Consider a constructor for
an independent, shipped admiral match and a bounded wait for a live round.
Only move a helper into `tests/common/` when another file needs it.

**Preserve:** one fresh `App` per test, real shipped composition, roster setup,
the human-controlled seat, and the two parts of the live-round premise:
there are seated fighters **and** none remain under `ControlHolds`. Preserve
checks on `DrivingParticipant`, `MatchSeat`, `MountReservedFor`, and `RidingOn`.
Do not replace readiness conditions with a fixed count of frames.

**Stop condition:** all 14 test cases and their distinct assertions remain.
The source file gets shorter and the tests still run in the focused lane.
Record actual before/after LOC; do not claim that all 882 matching positions
were removed.

```bash
scripts/setup/target_bindmount.sh --status
cargo test -p ambition_app --test app_it -- smash_ride::
```

Follow the bind-mount setup recipe if the target mount is not ready. Do not
run an exhaustive workspace test as the default response to a fixture-only edit.

## Step 2: simplify Versus setup without hiding phase boundaries

**Next edit:** `game/ambition_app/tests/versus_stage.rs` (3,168 lines and 32
tests at baseline). About 495 line positions are covered by repeated blocks.
The route-launch and bounded wait sequences appear at least 12 times.

Extract setup by the *state observed*, not by a single generic "ready" flag.
The following are different facts: the roster exists, the room activated, both
seats exist, and the round reached `MatchPhase::Fighting`. A test for a
transition at an earlier stage must still observe that earlier stage.

**Preserve:** controller-to-seat routing, countdown, KO freeze, score and
round reset, roster cleanup, and the shipped-host composition.

**Stop condition:** all 32 test cases keep their distinct transition and
ownership assertions, with less duplicate construction and polling code.

```bash
cargo test -p ambition_app --test app_it -- versus_stage::
```

**Done 2026-11-21.** `versus_stage.rs` is 2,906 lines (from 3,181), still 32
tests. Each helper waits on one named fact:
- `go_to_versus`: the launcher's command.
- `enter_the_arena`: the arena is the live room.
- `wait_for_seats(n)`: n bodies are seated.
- `enter_a_live_round`: the arena, then `MatchPhase::Fighting`.
- `seat(n)`: the body in seat n.

Twelve tests waited on the room, twelve on a seat count; each keeps the fact
it waited on. The tests that observe the roster, or the moment before the
round, still poll for those facts themselves. The copied waits broke out of
their loops without asserting, so a route that never activated would have
failed later as a countdown that never ended. The helpers name the fact
they never saw. `versus_stage::`: 32 passed before and after.

## Step 3: classify and retire obsolete diagnostics

This is a separate change from Steps 1 and 2. The highest-confidence review
queue is `enemy_body_scale.rs` (three ignored print-oriented probes) and
`sky_census.rs` (one ignored measurement). Then review
`hall_redecode_census.rs` and the wide fighter-matrix measurement in
`smash_cpus_damage_each_other.rs`.

For **each** ignored test, record which of these roles it has:

| Role | Action |
|---|---|
| Active regression, or unique anti-vacuity control | Keep as a test |
| Diagnostic needed for an active defect | Keep available as an explicit opt-in probe |
| Repeatable performance measurement with an owner | Keep in a named measurement lane or move to a tool |
| Superseded experiment with no remaining question | Delete after checking references and callers |
| Test that needs process isolation | Keep the isolation contract and the required runner |

Check current issue and planning state before touching the 14 ignored probes
in `a_bag_changed_mid_window_reaches_the_save.rs` (599 test-body lines). Some
investigated the rollback/save issue called Q135 in the audit. The old report
does not prove that the issue remains open **today**. Likewise, retain the
failure-bisection helpers in `rollback_exit_oracle.rs` and the full collision
oracle sweep unless an equivalent tool has taken their place.

**Stop condition:** for each deletion, name the old question, why it is closed,
and where an agent should now go for diagnosis. Do not delete probes solely
because `#[ignore]` appears above them.

**Classified 2026-11-21 (first review queue):**

| Ignored test | Role | Action |
|---|---|---|
| `sky_census::pirate_sky_sfx_census` | superseded | Deleted. Question: why was `pirate_sky_lookout` loud while its pirates were inert? Closed: the inert pirates were posted outside the player's view, fixed by the crew call-out and pinned in `pirate_sky_crew`. The sound question goes to the `room_census` tool (`ambition_app_tools`), which attributes every `SfxMessage` kind to a body; this census counted only `Play`. |
| `enemy_body_scale::print_enemy_bodies_against_the_player` and its control `the_enemy_body_report_is_actually_measuring_something` | superseded | Deleted. Question: are enemies too big against the player? The module doc said it could not answer: it compared sheet-pixel boxes across sheets of different densities. The control existed only to keep that report honest. Diagnosis now: `every_characters_drawing_is_the_size_of_the_body_it_collides_with`. |
| `enemy_body_scale::print_the_two_render_size_publishers` | superseded | Deleted. Question: do the two render-size publishers agree? Now asserted over every character by `every_characters_drawing_is_the_size_of_the_body_it_collides_with`, which reads the same `publishers()` helper. |
| `enemy_body_scale::print_the_height_each_legacy_row_already_has` | superseded | Deleted. Question: what `standing_height` should each of nineteen legacy-road rows author? Closed: all nineteen author one (checked in `character_catalog.ron`). |
| `hall_redecode_census::the_halls_entry_is_counted_for_art_it_decodes_twice` | needs process isolation | Kept, with its runner `scripts/measure/measure_hall_redecodes.sh`. |
| `smash_cpus_damage_each_other::every_fighter_on_the_grid_can_fight_its_mirror` | measurement with a stated use (validating a scoring change) | Kept. |
| `smash_cpus_damage_each_other::probe_where_the_goblin_pca_hit_events_come_from` | diagnostic | Kept as an opt-in probe. The sibling test asserts only the per-tick burst ceiling, not the hit rate this probe attributes. The rate question could not be rechecked here: on this VM the Smash start waits for music. |

`enemy_body_scale::` passes (1 test).

**`a_bag_changed_mid_window_reaches_the_save.rs` (2026-11-21).** It has 14
tests, and 6 were ignored, each with its role stated. All 6 were run, and
all pass.
- Five are the `SetFlagsRequested` straddle investigation (premise,
  controls, the desync mechanism, the gate/derivation split). They are
  kept as opt-in probes, as their ignore reasons say.
- The sixth tracked Q136: a developer's gravity cycle written into the
  channel from outside the simulation was lost under rollback. It passed
  because the defect was still present, so it was fixed:
  - the request is a host intent now;
  - the applier is ordered after the ledger's release (unordered, the two
    simulations of the stamped tick disagreed: a sync-test mismatch);
  - the arm was inverted into a gate,
    `an_ambient_gravity_request_raised_outside_the_simulation_survives_the_rewind`.
  The file runs 14 of 14.

## Step 4: consolidate repeated inputs, not the behaviors

- `carried_item_crosses_rooms.rs`: share custody/identity observations; keep
  each room and ownership transition as a separate witnessed case.
- `repro_walls.rs`: share geometry setup; keep the 0.01-pixel penetration case
  and each other start state.
- `dash_stability.rs`: share the feet-stability measurement; keep both normal
  dash and dash-plus-down.
- `movement_axis.rs`: share the movement check; keep both left and right.
- `hall_transition_cover.rs`: share state capture; preserve the loading,
  readiness, visibility, and teardown boundaries.
- `an_edit_reaches_the_shipped_game.rs`: share content edit/reject setup;
  preserve tests for each different content reference validator.

If parameterization makes error messages less clear, keep the separate named
tests and share only their setup. A similarity score of 1.0 is **not** proof
that two inputs expose the same failure mode.

**Done 2026-11-21.** Only setup is shared. No test was merged or removed, and
the 30 names are unchanged. Consolidated implementation: 8,056 lines to 7,330
across the six files and `common`.
- `dash_stability.rs` (111 → 72 lines): `max_feet_dy_over_a_grounded_dash(move_y)`.
  Both tests keep their own assertion and message.
- `movement_axis.rs` (75 → 54): `travel_holding(move_x)`; left and right stay
  separate tests.
- `repro_walls.rs` (798 → 467) shares:
  - the square-arena subset, the lock wall and `sandbox_room(id)` (the LDtk
    room, raw and with its moving platforms);
  - a clinging body, the one-step press into the wall, the 40-frame budget
    drive and its jump input.
  Each start state stays its own test, including the 0.01 px penetration.
  The two goblin-encounter tests now validate the project first, as the
  square-arena ones did.
- `hall_transition_cover.rs` (1,158 → 948) shares:
  - `boot_into_gameplay`;
  - `record_transition_through(zone)`: the room-graph transition and the
    avatar's live identity, written out three times before;
  - `room_set`, `live_room_id`, `sprite_setting` and `npc_character_ids`.
  The loading, readiness, visibility and teardown boundaries stay in their
  own tests.
- `carried_item_crosses_rooms.rs` (1,600 → 1,466) shares:
  - `held_by`, `lying_in_world`, `apart`, `settle` and `ledger_rows`;
  - `the_authored_enemy` and `make_invulnerable`;
  - `drive_through_the_door_to`.
  The hold-until-possessed loop, written four times here and once in
  `common`, is `common::possess_by_holding_on`, and `primary_possessed` reads
  the seat. Each room and ownership transition stays a separately witnessed
  case.
- `an_edit_reaches_the_shipped_game.rs`: its edit/reject setup was already
  shared (`watch_an_edited_copy`, `name_an_unknown_character`,
  `press_apply_reload`). The launcher prologue, written five times, is
  `the_shipped_launcher`. Each validator keeps its own test.

Lane: `dash_stability::`, `movement_axis::`, `repro_walls::`,
`carried_item_crosses_rooms::`, `hall_transition_cover::`,
`an_edit_reaches_the_shipped_game::`, and the two other users of the shared
possession helper (`a_save_taken_mid_possession_does_not_delete_the_enemy_in_a_fresh_process`
and `a_replay_follows_the_body_you_are_actually_driving`): 46 passed, 1 failed.
The failure, `a_reload_that_moves_a_tracks_file_plays_the_new_file`, stops on
its premise: the library holds no track on this VM, which has no music
registry. It is an environment red, not this change.

## Step 5: do a behavioral coverage review before deleting active tests

Build a small per-area matrix with these columns: test name, observed failure,
input or initial state, owning layer, production path used, other tests that
would fail for the same defect, and proposed disposition.

Focus first on the 76 hosted-Smash tests and the larger reload and two-room
modules. Use an explicit local break on a **scratch branch or disposable worktree**
when two tests appear equivalent. Verify that the retained test catches the
same defect. Restore the break before publishing any change. Mutation checks
must be focused and temporary; do not add a broad new mutation-test framework.

Do not merge Mary-O and Sanic acceptance tests just because their code is
identical. Each standalone app must prove that it wires the common behavior.
Shared fixtures are fine; one app's passing test is not evidence for the other.

### Step 5 record: `two_players_two_live_rooms.rs` (2026-11-21)

71 tests, 4,367 lines, the largest two-room module. Read test by test against
its assertions; no local break was run (Jon's rule since 2026-11-11: no
poison runs unless he asks), so a disposition rests on what each test
asserts and which production path it enters, not on a measured overlap.
Every test boots the real `Platformer2dSimHarness` with Alice on seat 0 and
Bob on slot 1, through the shared `alice_leaves_bob*` / `bob_beside_alice`
setup.

| Family | Tests | Production path | The defect each catches | Overlap examined | Disposition |
|---|---:|---|---|---|---|
| A. A producer acts in its own live room | 16 | each producer's own system, with a second room live | that one producer reading "the sole live room": gravity well, wave zoom, gravity switch, heavy-hit rule, GNU-ton and FSM hall measurement, apple rain, the gnu's back, overflow flood and swoop, blink walls, safe point, dev traces, portal shot, gallery barks, cut-rope music claim | none: each names a different system, and a fix to one leaves the others wrong | keep all |
| B. The crossing transaction | 15 | the room-transition commit (accept, commit, retire, join) | the other room stays live; it keeps its time; Bob's own door; a return joins; the crossing keeps its accepted participant (open, join); only the crossing seat waits, with its cooldown control; only what is left is reset; only the departing claim is released; continuity refused before any retirement; prefetch reuse; live occurrences over the unused plan; clock scale kept; dead bodies refused | `a_door_crossed_by_one_player_leaves_the_other_players_room_live` against `a_door_one_player_takes_does_not_stop_the_other_players_room`: the first asserts two live rooms, the second that Bob's room keeps running. A room can be live and paused, so they are distinct | keep all |
| C. An encounter in one of two rooms | 4 | the boss, wave and cut-rope encounter systems | the chest drops, mobs spawn, the fight runs and a replay rehangs, all in their own room | none | keep all |
| D. A player's lifecycle beside another room | 11 | replay, checkpoint reset, level exit, hazard respawn, death, cutscene, map, conversation, module retirement | each served in the player's own room, keeping the world clock and the other room | the two clock tests (`a_replay_keeps_…`, `a_hazard_respawn_keeps_…`) enter different reset roads | keep all |
| E. Views and presentation | 8 | the split-view and HUD systems and the effect producers | a second view opens and closes; the purse per view (split) and on a shared view (`SharedViewHudFacts`); framing; dust, hit, arrival and reset effects drawn in the right room | split purse against shared purse: different resources and systems | keep all |
| F. Rollback | 2 | GGRS sync test | resimulation across a crossing's commit and rebase (control: load count) / the checksum covering the non-primary room (control: an outside nudge must mismatch) | the two sync tests: different subjects and different controls | keep both |
| G. Custody | 1 | item custody across a crossing | an item carried out crosses whole and belongs to the room it is put down in | the one-room control lives in another module | keep |
| H. Native roster and party travel | 14 | `RoomTransitionIntent` companions, `PartyTravel` | relative arrivals (resolved, reference position, missing or moved reference, walls, cycles), the three party modes under rollback, follower walls, the leader's room on return, preferences under rollback | already share `native_roster_arrival`, `prepared_relative_return`, `policy_crossing` | keep all |

**Result:** 0 removals proposed, 0 merges. Two candidate pairs were read and
are distinct. The module's setup is already shared, so step 4 has nothing
left here. Next in step 5: `a_room_rewinds_alone.rs` (32 tests), then the
reload modules; the hosted-Smash half waits on the music environment.

## Protect these behaviors throughout the campaign

- Rollback registration coverage and poison controls in `rollback_coverage.rs`.
- Changed state across a real rewind in `rollback_exit_oracle.rs`,
  `rollback_populated_timeline.rs`, and the active save-mirror tests.
- Room re-entry, replay, and saved state in `canonical_reconstitution.rs`.
- Checkpoint death and item custody in `death_restores_the_checkpoint.rs` and
  `carried_item_crosses_rooms.rs`.
- Cross-game session cleanup in `shell_host_lifecycle.rs` and
  `shell_host_rendered.rs`.
- Actual input-to-action and composition paths in `smash_in_the_host.rs` and
  `an_edit_reaches_the_shipped_game.rs`.
- Scheduler order and cycle detection in `update_schedule_census.rs`.
- Two peers and two live rooms in `two_peers.rs` and
  `two_players_two_live_rooms.rs`.
- The test discovery guard in `app_it_sync.rs`.

Protect the **failure mechanisms**, not the current file size or helper design.
An extracted helper must not remove a test's premise or make its assertion
vacuous. A passing test that never enters its intended world is not evidence.

## Acceptance evidence after each edit

1. Check `git status` and record the current HEAD. Do not modify files outside
   the current task. Do not overwrite planning documents owned by other work.
2. Capture the names and ignored status of the tests affected by the change.
   For pure fixture extraction, the list should not change. For a deliberate
   removal, record each deleted test and the coverage that remains.
3. Check the changed file's real source LOC and repeated-block footprint.
   Count only actual removed or shared code; do not add overlapping matches.
4. Run the **focused** `app_it` module checks. Check `app_it_sync` when modules
   or file layout changes. Follow the feature-specific rows in the cheapest
   sufficient check recipe. Do not let lint-only failures block evidence.
5. For an actual behavioral deletion, use a targeted test break when practical.
   Check that the retained test fails for the defect. Remove the break and
   rerun the retained test.
6. Report the result with three labels: **consolidated implementation LOC**,
   **retired diagnostic LOC**, and **removed behavioral tests**. Do not combine
   them into one claimed coverage gain.

Do not make the next agent add a test simply because it changed code. Before
adding one, find the nearest existing test and say which new failure mechanism
the new test covers. If an old test has become redundant, retire it in the
same change with evidence. If nothing is wrong, make no additional tests.

## Execution

TEST-LANES owns current status and order. Recheck the corrected scanner and
live fixtures, then start with repeated Smash setup. Keep production APIs and
real phase boundaries intact. The findings are evidence, not a deletion quota.
