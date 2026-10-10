# The queue — live execution order

This file is the current executable engineering queue. It is not a work log, a
review transcript or an archive. Git history keeps completed investigations.
Durable design and measurements belong in the linked owner document. Product
decisions belong in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

Each row states the current failure, the owner, the next action and the
acceptance. A row stays here only while an engineer can act on it. When a row
closes, delete it in the same change. Keep a receipt of 1 to 5 lines (what was
wrong, what fixed it, the guard, any standing prohibition) only when an open
row, a script or an inbound link depends on it. Receipts are at the end.

Re-measure a row before you implement it. Check a row's owner against who is
actually running before you wait on them. When a row converges, rewrite it
around its result. Do not append to it.

Scripts read this file:

- `scripts/check_blocking_set_names_every_gate.py` reads each `**Blocked by:**`
  field to the next blank line, and each `**Acceptance:**` clause.
- A heading that contains `✅`, `DONE` or `CLOSED` marks its row closed for
  that script, for `scripts/check_discharged_holds_are_rewritten.py` and for the
  two consolidation-ledger checks. The ledger holds work on `ID-PEER` by its
  row id, and a ledger test needs `A10` to stay a closed row.
- Other documents and source comments link to row headings. Do not rename a
  heading without fixing its inbound links (`rg -n "queue.md#" .`).

Measure the per-row mass with:

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

## Priority order (Jon, 2026-10-09)

The risk now is improving components one at a time without exercising the
whole engine. Work in this order, and fix an authoritative-state defect found on
the way in its own slice:

1. **P0:** A4, BAG-RECORD-HORIZON and AUTHORITY-POLISH are receipts
   (BAG-RECORD-HORIZON's open line is Q161). C03 stops as a
   campaign after its family 7: a remaining session resource moves only when a
   two-session witness shows it leaks
   ([consolidation plan §3](consolidation/consolidation-plan.md#3-c03--consolidate-session-owned-state-and-reduce-reset-only-app-globals)).
2. [WORLD-ACCEPTANCE](#world-acceptance--one-headless-playthrough-of-the-persistent-world).
3. [SDK-GAME](#sdk-game--a-small-independent-game-on-the-supported-api): acceptance met 2026-10-09.
4. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable).
5. [NAVIGATION](#navigation--a-character-reaches-an-item-in-another-room).

Short slice at any point: [PUBLICATION-FAULT](#publication-fault--a-refused-later-room-must-not-leave-a-mixed-world).
The two owner pages the 2026-10-08 review found stale (the fingerprint and the
second-seat join) were corrected against the tree on 2026-10-09.

Deferred, not queued: a 3D migration, the cube menu's second camera, crate
renames, a full monolith decomposition, online netplay, a plugin marketplace, a
universal planner and the Bevy 0.20 upgrade. Q157 is Jon's decision; build
neither side of it.

## P0 — architecture and correctness

### WORLD-ACCEPTANCE — one headless playthrough of the persistent world

**Owner:** [open-world roadmap](game/open-world-roadmap.md) (P4), with the
[residency plan](engine/open-world-runtime-and-residency.md) and
[item custody](engine/item-custody-and-accounting.md).

**Current failure:** each mechanism has its own passing arm (room transitions,
item custody, persisted switches, spawned actors, saves), but no single run of
the real game proves they compose. A set of passing arms does not prove the
game.

**Measured 2026-10-09, the content it can run on:** the intro world authors the
route as `intro_cartography_route` (`data/quests.ron`): Alice's sealed note in
`alice_relay`, Bob's field survey in `bob_relay`, then the first system boss and
the P5 route-memory pickup. The note has two roads: Alice's dialogue
(`dialogue/sandbox/intro.yarn`) puts a `sealednote` quantity in the bag and
Bob's takes it (`sell_item "sealednote" 0`, then `give_item "fieldsurvey"`), but
the quest steps read flags that separate `PickupSpawn`s (`flag:<name>`) set. So
the quest can advance with no note handed over, and no persistent object or
custody is exercised. The first slice gives the fact one road (the hand-over
the quest reads) and makes Bob's reward a world consequence rather than a
flag.

**WA1 done 2026-10-09:** the note has one road. The two flag `PickupSpawn`s
are deleted (`intro.ldtk` and the two room specs); Alice's choice writes
`alice_route_note_carried` as it gives the note, and Bob's hand-over is the one
writer of `bob_field_survey_received`. The content validator now counts a Yarn
`world.set_flag` as an authored flag (`dialogue::dialogue_set_flags`). Witness:
`the_note_travels_from_alice_to_bob` (real key presses in the shell session;
the old map makes it red at "the note's flag was set with no note handed
over"). Bob's reward is already a world consequence: the flag opens the two
`LockWall`s gated by it.

**WA2 done 2026-10-09:** `a_playthrough_of_the_persistent_world` runs the
shipped shell session from the hub to Alice through six real exits (doors by
Interact, edge exits by standing in them), the note to Bob, back to a wall
that is now open, out to another room, a save, a NEW process loaded from the
file (bag, flags, quest step), and the walk back to the open wall. Poison
(the survey flag cleared from the file): red at the after-load assertion.
Found: a death before any checkpoint takes the survey back and keeps the
facts it made, two horizons in one death; filed as
[Q164](awaiting-maintainer-decision.md).

**WA3 done 2026-10-09:** the playthrough first takes the Blink from the hub
basement. Attack moves the player 0 px before the take and 170 px after it,
and the one `ground_blink` is `Held` by the player at Alice and after the
load. Poisons: no take, red at the capability assertion (0 px and 0 px); a
stow before the route, red at "did not come along to Alice".

**WA4 done 2026-10-09:** a second participant.
`the_hand_over_opens_the_wall_in_the_other_participants_room` starts the shell
session with two pads, so it holds two seats (`declare_ambition_seating`).
Seat 1 joins at Alice and stays there while the player hands Bob the note in
`bob_relay`. With the two rooms live at once, Alice's return lock is open in
the second participant's room. Poison (gated walls re-evaluated only in the
primary's live room): red at the after-hand-over assertion, with the lock
still standing in `alice_relay`. Control: with no pads, the same Jump presses
give seat 1 no body in 120 frames, because the session opened one handle.

**WA5 done 2026-10-09:** the death step, the part Q164 does not decide.
`a_death_sends_the_blink_back_to_where_it_lay`: the player dies at Alice with
the Blink in hand and no checkpoint. After the death no Blink is in the live
world (not in the hand, not dropped there), back in the hub the one Blink lies
where it was found and can be taken again, and Alice's return lock agrees
with the survey flag. Poison (the restore keeps the dying player's held
objects as a spared room's): red at "a Blink is in the live world", the Blink
still `Held`. Next: the fact horizon of a death, when Q164 is ruled.

**WA6 done 2026-10-09 (review of f0409fcc):** the reload goes through the
file. It used to clone `AmbitionGameSaveData` into a new App in the same
process while the docs said "a new process loads the file". Now the first App's
autosave writes the save under its `PersistenceRoot`, the test reads the file
back and requires it to equal the live save, and a new App on that root loads
it at Startup (`load_save_at_startup`). Poison (the new App without the root):
red at "the new App's save is not the file the first App wrote". The words now
say what runs: one OS process. The route is walked since 2026-10-10 (WA8).

**WA7 done 2026-10-09:** the playthrough runs in the composition that draws
it. `the_playthrough_runs_in_the_composition_that_draws_it` builds the shipped
game with `OffscreenGpu` (a wgpu device, no window), waits for its plugins,
and plays the Blink, the route to Alice, the note, Bob and the way back. In
the end Alice's return lock is open. Poison (`NoWindow`): red at "this
composition has no wgpu device" in 5 s. Found on the way: Bob's published
sheet had no `air_dodge` row, which panicked neighbour-room preparation in
the drawn game until the sheets were published again; `sprites.sh --check`
now says so (`0d5c17f20`). The pixels of the open lock are `capture_scene`'s
question and are not asserted here.

**Next action:** the fact horizon of a death, when Q164 is ruled. Everything
else in the scenario has a witness (WA1-WA5), and the workspace lane runs it.
What the scenario does not exercise, measured 2026-10-09: the note and the
survey are `KeyItem` bag quantities (`items.ron`, no `held_item_id`), so they
are accounted by count and have no instance identity. The object with an
identity and a custody is the Blink. A note as an object is a content
decision, not a defect.

**WA8 done 2026-10-10: the playthrough is walked.** Each arm of
`a_playthrough_of_the_persistent_world` walks by keys in the shipped App
(`common::walk`): the in-room `NavGraph` for the player's own body, the input
`follow_leg` gives pressed as the arrows-and-ZXC keys, Interact at a door,
confirm on a cutscene's beats. The player walks to the Blink and through each
exit, back from Alice after a death too, and no step puts the body anywhere.
`a_walked_route_of_the_persistent_world` walks the hub to Bob through the sim
harness (an analog stick, the agent's input). Found and fixed on the way:
a leg through a hazard or another exit was kept (the rollout now fails on the
kernel's reset, and `build_avoiding` takes the exits); `drain_alley` had no
road down to the under-town door (the grate in the street is open); the graph
did not see a standing lock wall; the hub basement has no road up but flight
(a 512 px void; its pogo orb and moving platform are not in the graph), so
flight is a leg now, with starts beside block ends so that it goes up
through a grate. Measured, not changed: a body with one jump has no road from
Alice to Bob before the survey, and none out of the hub basement. Poisons: no
fly leg, red at the hub basement ("no route"); the graph over the authored
blocks only, red at `alice_relay`. `walked_route_census` (ignored) routes 9
of 9 crossings.

**Acceptance:** the scenario runs headless in a standing lane and is playable in
the rendered game; each step asserts its fact against the authority that owns
it; a poison at each step reddens that step's assertion.

### SDK-GAME — a small independent game on the supported API

**Owner:** [Public SDK 1.0](engine/public-sdk-1.0.md) and
[capability and runtime composition](engine/capability-and-runtime-composition.md)
(P3); release artifact per
[build and distribution](engine/project-build-and-distribution.md).

**State 2026-10-09: acceptance met (SG1–SG3 below).** What is left is the
deferred monolith carve: through the monolith, Outlander's build still links
menu, items, encounter, boss_encounter, cutscene, conversation, held_items and
audio, which its profile does not install. That is not a next action for this
priority.

**SG1 done 2026-10-09:** Outlander (`fixtures/external_consumer`) is the SDK
game: its own workspace and lockfile, the facade only, no `shared_tangle`,
monolith or Ambition content named in its source. Measured: its windowed build
linked `ambition_portal2d` and `ambition_platformer2d_ldtk`, which it never
selects, because the monolith's `visible` turned on `portal`, `portal_render`,
`portal_ldtk` and `ldtk_runtime`. They are out of `visible` now; a game that
uses them selects them (the Ambition app and sanic already did). Outlander's
windowed closure went from 61 to 59 ambition crates. New contract
`the-sdk-game-links-no-capability-it-omits` (`check_absence_contracts.py`)
resolves both builds and forbids the eight facade capabilities Outlander
omits, with a floor. Poison (the old `visible`): red, naming exactly
`visible: ambition_portal2d` and `visible: ambition_platformer2d_ldtk`. The
lane now compiles `outlander_visible` (`--features visible --bins`). Measured
on the way and not from this change (red on the uncut manifest too):
mary_o_app `her_spark_leaves_her_hand` (3 tests, the fire sheet's published
rows) and smash_app `nothing_a_match_created_survives_into_the_next_one`
under `--features visible`. Still linked through the monolith and not used by
Outlander: menu, items, encounter, boss_encounter, cutscene, conversation,
held_items, audio (the deferred monolith carve).

**SG2 done 2026-10-09:** the release artifact. `python3
scripts/package_outlander.py` builds `outlander_visible` (release,
`--features visible`) and writes `fixtures/external_consumer/dist/outlander/`
(binary, `run.sh`, `assets/`, `MANIFEST.json` with the engine revision and a
sha256 per file) and `outlander.tar.gz` (72.5 MB; 37 engine assets, 1.5 MB).
It then runs `run.sh --smoke 300` from the artifact under strace and refuses
it when an asset load failed or a file was read outside the artifact. Exit 2
is a missing prerequisite, 3 a failed build, 1 a failed artifact. The full lane
runs it (unrunnable without strace). Measured on the way:
`outlander_asset_root` baked `CARGO_MANIFEST_DIR` in, so it now follows the
engine's rule (`BEVY_ASSET_ROOT` → `assets`). Outlander's three windowed tests
were red and no lane compiled them (`#![cfg(feature = "visible")]`): they
stepped an app whose plugins were never finished, so the trail gizmo groups
were never registered. `stepped_windowed_app` finishes it, and the full lane
runs `cargo test --features visible`. The shrine sheet path was wrong in the
engine (180f89923). Poisons: an asset dropped from `package_assets.txt`, red
naming that file inside the artifact; a launcher without `BEVY_ASSET_ROOT`,
red naming the checkout files read. `--measure` writes the asset list again.

**SG3 done 2026-10-09:** reduced profiles through the shipped composition.
`PlatformerApp::profile(EngineProfile)` (profiles at `app::profile`) installs
`PlatformerEnginePlugins::for_profile`, or `RollbackProfilePlugin` under
`.rollback`. A module's `ModuleDraft::content_pack` is admitted at declaration
against `engine_schemas_without(profile.omitted_content_capabilities())`, so a
pack that requires an omitted capability is refused there, naming the profile.
Outlander composes `COMBAT_WITHOUT_INVENTORY_BOSS_DIALOGUE` in all three
builders. Witnesses: `a_reduced_profile_omits_what_it_names` (Outlander,
control: cutscenes stay installed; poison, the headless builder without
`.profile`: red at the omits assertion, naming inventory, held-use,
boss-encounters, dialogue) and the facade's
`a_profile_refuses_a_pack_that_needs_a_capability_it_omits` (poison, admission
against the full registry: red at the stage assertion). Measured on the way: a
windowed game without dialogue panicked on `ResMut<DialogState>`, because the
host's `dialog_pointer_input` ran unconditionally; it now runs only when the
dialogue state exists. Outlander's full suite under `--features visible` is
green (29 tests). The absence contract now prints cargo's error and the
`cargo fetch` remedy when the fixture's dependencies were never fetched.
The content CLI takes `--without <capability>` (repeatable) and checks a pack
against that reduced registry; witness `a_pack_is_refused_for_a_composition_without_its_capability`
(control: no flag admits; poison, the flag ignored: red at the exit code).

**Acceptance:** a dependency check shows the omitted capability crates absent
from the game's graph; its headless test and its windowed build both run in a
lane; a release artifact is produced by a documented command.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane. Operational rules and what a
green lane does not clear:
[`running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).
How a check can fail to run: [`checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).

**Current state:** the `app_it` lane runs (re-run 2026-10-08; read the count
from a fresh run, not from here). Cargo diagnostics are read through
`scripts/lib/cargo_output.py`, which disables colour and strips ANSI codes, and
`scripts/tests/test_cargo_diagnostics_are_read_plain.py` holds every script that
reads cargo output to it.

**Open items:**

1. **The compile-cost ratchet reports and does not fail the gate** (re-measured 2026-10-09: 8 findings, 0 gating, exit 0). Since `a614327fe` (2026-09-25, Jon: "line-count should not be a gate") only PATH and GONE gate; the size and seconds rows are reports. The critical path of 15 crates is banked (2026-10-09, `--adopt-wins`). The carve was `d56c46de9`, which removed `ambition_items`'s dead dependency on `ambition_combat`; the old 16-crate chain went through that edge. Two `--adopt-wins` defects found on the way are fixed: a held row kept its old `depth` (path 15 beside a row of depth 16), and `carried_from` dropped the older commit of a chain (`scripts/tests/test_compile_ratchet.py`, each arm red under its poison). Still open: the baseline disagrees with itself in three places (`ambition_geometry`, the monolith, `ambition_platformer2d_core`). The held numbers are from `11ef33c5b5a5`, and their table rows were already different, so no adopt can repair this; only a deliberate re-freeze can. Reported: the monolith's largest unit (100,742 → 132,359 lines). ⛔ Do not re-freeze with `--update` as bookkeeping: it banks the regressions, and that is a judgement to state in its own commit.
2. **An arm fails only in company** (see [the triage page](triage/a-composition-acceptance-that-only-fails-in-company.md)): `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps` failed once on 2026-09-10, and its assertion was never captured. Three other instances of the signature were per-arm measurements reading process-global state (`app_it` runs arms as threads of one process); they are fixed, and `scripts/a_test_static_is_a_channel_between_arms.py` guards the class. Next: capture this arm's assertion. `hall_redecode_census.rs` asserts over a delta of a process-wide counter, and it is not a candidate: it is `#[ignore]`d and run alone by `scripts/measure_hall_redecodes.sh` (read 2026-10-08). Counted 2026-10-09 from this machine's cost ledger (`run_tests_cost.jsonl`) since 2026-09-10: 56 job runs that run `app_it` in company passed and 35 failed, and no failed row says which arm failed: the evidence was only in a status file that the next run writes over. The ledger row of a failed job now keeps its `failure_evidence` (`test_a_failed_job_keeps_its_evidence_in_the_cost_ledger_row`), so a recurrence can be counted from here on. The A9 probe found and repaired two couplings that fail a composition without `BossEncounters` (`simulation_world` required `BossCatalog`; the progression plugin registered `populate_boss_encounter_registry`); whether either was this failure is not known. ⛔ Do not add a retry.
3. **The reload family failed under load, and its waits for the first activation now wait on the condition (2026-10-09).** The family is `an_edit_reaches_the_shipped_game`. Measured with the family alone at 30 test threads, five runs for each state: on main `0e2c22f9e`, five of five runs were red (two or three arms for each run, ten different arms, 13 failures), and each failure was a premise that the first session is live. With the waits changed, five of five runs were green. The cause: the gameplay route becomes active after work on other threads, so a fixed 240 frames is not a wait. Alone, the route was active on frame 3; in company it was seen on frame 83, and the red runs are the cases later than frame 240. The waits are in `game/ambition_app/tests/common/mod.rs`: `step_until_route_is_active`, `step_until_route_is_active_and_settled` and `ActivationWindow`. Each waits on the route under a ceiling of 120 seconds of wall time and then steps the frames the old loop stepped after frame 3. Two arms in that file hold the ceiling (`a_wait_for_a_route_that_never_activates_ends_at_its_ceiling` and the one for the window); with a ceiling check removed, its arm does not end. Converted 2026-10-09 (CalculexAmbition, `4cfe4e476` and the next commit): 16 candidate arms wait until `PendingGeneration` is gone and first assert that one was pending; two arms refused at request time assert that none is pending; two handoff waits wait on the activation id and on the handoff, under the same ceiling. Measured: a generation settled in 2-3 frames alone and at 30 test threads, so these arms were not short of frames; the change is that a negative arm now fails if the refusal was never decided. Left: the census probe, one handoff whose premise check fails loudly when its transaction has not run, three loops that assert on each frame, and each other family (63 loops of 240 frames outside this file). No failure of those was seen in the ten runs after the change; that is not a proof that they hold. One older session-root handoff failure did not reproduce in four full runs, and its assertion was never captured. One of its two candidate arms (`the_shipped_app_never_holds_two_session_roots_across_a_handoff`) had this window; whether that was the failure is not known. ⛔ Do not add a retry, and do not make the 240 larger: wait on the condition.

4. **`NOT RUN` is a first-class receipt state** (Q59 ruling, 2026-10-03). A
   ledger or receipt must tell PASS, FAIL, "not run, not required now" and "not
   run, required at this boundary" apart; a gate blocks only where its policy
   requires it at the current boundary. Two collapses repaired (2026-10-08):
   `last_test_run.py` read a `--only-job` status as the lane's PASS (it now
   says `NOT RUN: n of the lane's m job(s)` and exits 2; a FAIL in the
   selection still exits 1), and a full gate on a machine without
   `wasm32-unknown-unknown` dropped the web check from the plan and wrote
   `done` (it is now planned as unrunnable, `Job.missing`, so the run is
   `incomplete`). The receipt for a push exists (2026-10-09):
   `scripts/required_checks.py` prints, for each check the change requires,
   `passed on this change`, `FAILED on this change`, `NOT RUN`, `NOT RUN:
   <remedy>` or `ran only on a tree before this change`. A push does not wait
   for it (Q166, ruled no 2026-10-09): it reports. Each boundary's lane is
   listed in
   [testing and validation](../concepts/testing-and-validation.md#validation-states-and-cadence)
   (2026-10-09), pointing at the owner of each rule. Next: the commit
   messages and queue rows that quote a lane by hand.

5. **A sync test did not see an effect that only the first run of a frame
   has; the rollback host now does (2026-10-05).** GGRS never saves the state
   that the first run of a frame leaves, so each of its compares is between
   two resimulations. `first_run_witness` (in the GGRS host crate) takes the
   checksum of the first-run state and compares it with the first save of
   that frame; `the_sync_test_sees_a_first_run_only_effect` holds it, with
   the measurement before the repair in its doc. The full app lane found no
   first-run-only effect in the tree. Named limits, not built: only a sync
   test with a check distance above zero; a peer session is not covered (GGRS
   saves its first run when no rollback is owed). The witness covers the last
   advance of a host tick only, and that limit is closed for the harnesses by
   measurement (2026-10-05, one mark for each host tick of a sync test that
   rewinds, over the full `app_it` lane and the four demo host binaries):
   23,372 ticks advanced one frame, 236 advanced none, none advanced two or
   more. A pinned host adds one frame for each update. The proof pulse of
   the rollback observatory (a developer affordance) runs a sync test under
   the real clock, and it is the one host where a long render frame advances
   twice; the first of those two advances has no witness.
6. **The demo host apps' own integration tests were in no standing lane;
   the recipe now names their lane and when it is required (2026-10-05).**
   `mary_o_it`, `sanic_it`, `smash_it` and `twintrack_it` run only under the
   whole-workspace lane, so a change that both agents' standing lanes
   (`app_it`, pytest) pass can leave them red. Found that way: four death and
   room-replay arms were red on main from `f7ecfc019` (P1) until `69d29caa0`:
   instruments counted the message a restore no longer writes. The behaviour
   held; only the counters were blind. The command, the changes that require
   it, and the measurement that it needs no waiver are in
   [the heavy lane recipe](../recipes/running-the-heavy-app-it-lane.md#the-demo-host-apps-have-their-own-lane);
   [the check matrix](../recipes/cheapest-sufficient-check.md#the-matrix) has
   the row. **Reported 2026-10-09:** `run_tests.py` records each finished job
   against the tree it tested (`target/lane_ledger.jsonl`), and
   `scripts/required_checks.py` names the checks the change since
   `origin/main` requires (a changed crate's own tests, the demo host lane for
   its paths, the repo tooling job for `scripts/`) and says for each one
   whether it passed on a tree with the change in it. It reports and does not
   refuse a push (Q166). Measured: the SG3 commit
   `1485939ec` changed the rollback host and was pushed without the demo
   lane; the gate names that lane for it. Witness
   `scripts/tests/test_required_checks.py` (poisons: freshness ignored, red at
   `test_a_pass_before_a_later_edit_is_old`; untracked files left out of the
   tested tree, red at `test_a_new_untracked_source_file_is_part_of_the_tested_tree`).
   A row holds the tree at both ends of its job, and a check counts when
   both hold the change's paths (`36eb555ee`): a file the host of a shared
   folder edited during a 14-job run had voided two passing jobs. A
   checked-out submodule is recorded at the commit it has checked out
   (`8ac789621`): the index entry had made a change that moved a pointer
   impossible to certify. Only the
   run that writes the default status records evidence, so a test's fake
   jobs do not. Not held: the demo rule's fourth case (an instrument a demo
   test reads), the external-consumer fixtures, the matrix rows without a
   path, and a peer's change merged after the run. Two more rows are held
   (2026-10-09): a change to `game/ambition_content/assets` or the map
   assets requires the content arms (`declared_art_resolves`,
   `registered_character_art`), and a rollback registration or
   `sim_phase_pins.rs` requires the `rollback_` arms and the repo tooling
   job. A check named by test name counts for a run whose filter is part of
   that name. Found on the way: a filtered nextest run certified the whole
   crate, because `run_tests.py` gives nextest its filter as a bare word and
   the rule looked for it after `--` (witness: the nextest rows of
   `test_only_a_default_feature_run_of_every_target_covers_a_package`).
   A doc (`.md`) requires the repo tooling job, whose guards read docs, and
   counts for no other check: a doc committed while a crate's lane ran
   voided that crate's pass (`738d7c3da`, witness
   `test_a_doc_edited_after_a_crate_ran_does_not_void_the_crate`).

7. **The repo tooling job runs on several cores (2026-10-10).** It ran
   `scripts/tests` serially, and each push waits on it. Measured on 14 cores:
   1160 s serial; 232 s and 236 s with 8 workers, the same 1776 passed and 19
   skipped; 224-231 s through `run_tests.sh` with 14. `run_tests.py` adds
   `-n auto` when pytest-xdist is importable (`pytest_worker_args`), and
   `scripts/setup/python_tools.sh` installs it. The count is the `-j` cap, or
   at most 6 with none (`PYTEST_WORKERS_UNCAPPED`: the shared machine's rule
   is at most 6 parallel jobs and no pool that takes every CPU by default).
   Without xdist the job runs serially and prints why: the same tests run.
   Witness `scripts/tests/test_the_repo_tooling_job_runs_on_every_core.py`
   (poisons: no workers, red at "the job runs serially"; no default cap, red
   at "with no -j the pool is not capped"). The pre-push measurement is in
   [extension evidence M0](engine/extension-iteration-evidence.md#m0-results):
   `app_it`'s 890-913 s is now the largest part of a push.

8. **The change selects its checks, and one command runs them (2026-10-10).**
   `python3 scripts/required_checks.py --run` runs the `run_tests.sh` command
   of each required check that is not certified, then judges again from the
   ledger the runs wrote. Witness
   `test_run_runs_only_the_checks_that_are_not_certified_and_judges_again`
   (control: all certified, nothing runs; poison: run every required check,
   red at the commands assertion).

The published-sheet floor in `ambition_sprite_sheet` (780 below a floor of 800
on one checkout) is machine state. ⛔ Do not lower the floor.

A test that needs a sheet row a commit just added is red on each machine until
that machine publishes the sheet again: the sheets are untracked. Measured
2026-10-09, the default gate at `19e44c13`: 14 of 16 jobs green. The workspace
job ran 9730 tests, 9728 passed, and the 2 that failed
(`her_spark_leaves_her_hand::*` in the Mary-O app) said "her fire sheet has no
`shoot` row" on a machine that had not regenerated `mary_o_v2_fire`. The other
red job was the wasm check (a native-only call, fixed by its owner). The same
day Bob's sheet had no `air_dodge` row, and the rendered game panicked in
neighbour-room preparation. `scripts/regen/sprites.sh --check` now says
whether the published sheets are older than the renderer (the fingerprint
saved by the last full publish) and prints the command to run (`0d5c17f20`).
It cannot tell which row a test needs; a red test still reads as a defect
until someone runs the check.

A full regen took more than 2.5 h on one process, and now takes 74 min on 7
(2026-10-09, `38865ba48`). `AMBITION_SPRITE_JOBS` defaults to half the cores,
at most 8. A batch with one failure keeps the cache key of each target that
published. The run's own "render cost" line: 3392 s of the 4436 s in 7
renderer processes (tack-ons 1374 s, draw-review 893 s, draw-all 700 s, late
targets 287 s, factions 136 s). The other ~1044 s are stages outside the
renderer (ultrapack, quality variants, LDtk). The tack-on batch was at 143 of
144 after 1145 s, so its last target held about 230 s alone. Since renderer
`890c457` a batch starts the slowest known targets first (each target's last
publish seconds, kept in `.cache/publish_seconds.json`); its effect on a full
regen is not measured yet. Still serial: the faction-leader
lineup, the review canonical gallery, the ultrapack and the quality
variants. Also found in that regen:
- a run through the `/home/agent/code/ambition` alias wrote `relPath`s that
  climb to `/` into five worlds. Fixed in `rel_to_ldtk`, which now keeps both
  paths in one spelling (`test_a_repo_reached_through_a_symlinked_alias_keeps_the_virtual_mount`).
- the regen rewrites `mary_o.ldtk`'s editor-art tile ids: 22 rule tiles
  moved by +256 and 4 by +3, with their auto-layer tiles. These are editor
  visuals; why the atlas moved is not measured, and the change was not
  committed (Q62: a delta needs a domain-aware reading first).

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

### NAVIGATION — a character reaches an item in another room

**Owner:** [navigation and reachability](engine/platformer-navigation-and-reachability.md),
with the [agentic character runtime](engine/agentic-character-runtime.md) (P6).
The cross-room items (MISSING 1-4) are claimed by this session
(ClaudeAmbition) from ToothbrushAmbition, 2026-10-09; the in-room legs stay
theirs. Inside Jon's scope: a companion or NPC crosses rooms. Not built:
enemy navigation and a baked graph (both Jon's open decisions).

**Current failure:** typed actions, world facts, memory and combat policy exist.
In-room navigation exists since 2026-10-09 (a surface graph, checked in the
kernel, that a brain follows; the owner doc has "The first slice"). A route to
another room and an open custom-brain policy interface do not exist, so the
world-fact architecture has no consumer.

**Next action:** a character observes that an item exists in another room,
decides whether it can reach it with its real movement capabilities, moves there
and performs a typed action. Deterministic; no model call inside the
simulation.

What the in-room slice gives this row, and what it does not (2026-10-09):

- HAVE: "can this body reach that point with its real capabilities" in one
  room (`NavGraph::next`: a leg, `Arrived` or `Unreachable`), and a body that
  goes there (`NavFollower`). A door is a point, so a route to a door in the
  body's room exists today. The acceptance's third arm holds in one room: the
  same body with no jump cannot reach what needs one
  (`a_body_with_no_jump_cannot_reach_what_needs_one`).
- MISSING, in the order they block: (1) a non-player body that goes through a
  `LoadingZone` into another live room. Measured 2026-10-09: a body crosses
  only as a crossing's subject (a body a player slot drives, a possessed one
  too) or in its custody (a ridden mount, a limb, a held item); detection
  (`detect_room_transition_system`) reads driven bodies only. A body left in
  another room is rebuilt there from its `Placed` whereabouts row
  (`a_character_left_elsewhere_stays_there`). (2) A route over rooms at run
  time: done 2026-10-09, `RoomSet::route` (fewest rooms, by the authored
  zones; witness `a_route_over_rooms_goes_by_zones_that_cross_where_it_says`,
  which holds each hop and the route's length against the crossing rule;
  it reaches 75 of the 76 shipped rooms from the hub, and `sanic_sandbox` has
  no zone that leads into it).
  (3) A goal a brain can be given from outside ("fetch that") and (4) the
  typed action at the goal: done in one room, 2026-10-09 (`Errand`; the
  acceptance arm holds in one room, `a_dog_sent_for_an_item`; see the owner
  page's cross-room slices). (1) for a live room: done 2026-10-09, slice 2
  (a body on an errand goes through a zone into another live room, by the
  second-seat road, and keeps durable whereabouts there). Slice 3, a
  crossing into a room that is not live: done 2026-10-10 (the body leaves
  through the ledger and is despawned; the room builds it there). It was
  blocked because a rewind spawned the body again without its derived
  components; the body now requires them (owner page).

**Acceptance:** a headless arm with a reachable and an unreachable item: the
character fetches the first and refuses the second, and removing a movement
capability it needs turns the first into a refusal.

### PUBLICATION-FAULT — a refused later room must not leave a mixed world

**Owner:** [residency plan](engine/open-world-runtime-and-residency.md), the
multi-room hot reload row.

**Done 2026-10-09: a multi-room reload publishes all its rooms or none.** It
used to publish one room at a time, so a later room refused after the first
had published left a mixed world (the status said `THE WORLD IS MIXED`). Now
each room is held after its check (`replace_live_world_held`,
`HeldForOwner`): nothing is promoted and its effects wait on the
publication. Each later room is checked against the projected set and
counter. Then every room is committed (`commit_held_publication`) or every
room is refused by the refusal road (`refuse_held_publication`), so no undo
is needed. The design is in
[the residency plan](engine/open-world-runtime-and-residency.md#design-a-multi-room-publication-checks-every-room-then-commits-every-room).
Deleted: `republish_live_room`, the `THE WORLD IS MIXED` status and
`kept_the_old_generation`.

- Witness `a_later_room_refused_after_the_first_room_passed_rebuilds_none`.
  The fault is injected while the first room is built; each live room keeps
  its instance and generation, the epoch does not move, no receipt or
  candidate is left, and with the fault gone the reload applies.
- Poison (commit the first room before the later rooms are checked): red at
  "the first room was published although a later room was refused".
- Control: the rest of the reload family passed (48 passed), including
  `a_world_reload_rebuilds_every_live_room`.
- The guard before staging stays (`DescribableRooms`,
  `a_world_reload_with_a_live_room_that_cannot_be_described_rebuilds_none`):
  it refuses before anything is built.

Not done: an apply-time refusal after every room passed is logged
(`apply_world_replacement`), not failed loudly. With a correct projection it
cannot happen.

### AP14 — semantic actor-monolith SCC decomposition (continuous; deferred 2026-10-09)

**Owner:** [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).
Jon's 2026-10-09 order defers a full decomposition: take a cut only when a slice
above already moves the state it names.

| # | Item | Next step |
|---|---|---|
| AP14 | Semantic actor-monolith SCC decomposition (continuous, Jon 2026-09-24) | Measured 2026-10-08: the SCC is 9 modules (`abilities avatar character_runtime construction features items projectile session world`); `avatar`, `character_runtime` and `construction` joined it after 2026-09-27. One leg was a spelling: 17 readers named `SessionCast` through a `session::mechanics` re-export of `ambition_characters::prepared::SessionCast`; they name the owner now and the re-export is deleted. The single-edge cuts left are genuine reads: `avatar→session` (1 ref, `RulesOf`, the per-room rules authority; removing it would leave 7), `construction→session` (1 ref, `CommitFactsSource::AfterTheRestore` names a checkpoint operation), `features→projectile` (1 ref, perception reads `ProjectileAllegiance`, which cannot sink below `ambition_combat`'s `MatchTeam`), `character_runtime→avatar` (1), `session→world` (3). The 2026-09-27 kernel was 6 modules; its legs were walked and judged genuine; the one misplaced leg, `abilities→features` (the puppy-slug gun asks the actor domain to spawn a minion), is not inverted only for the graph. Open question: is the runtime-mint description in `session→items` (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) item knowledge or occurrence-lifecycle knowledge? When an owner is clear, move state, behavior and installation together and delete the old edge (no callbacks, no compatibility re-exports). After each migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`. See [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md). |

**Acceptance:** each cut moves state, behaviour and installation together and
deletes the old edge; `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`
is run after it.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners. Implementation state:
[I3](engine/fast-iteration-implementation.md#i3---coordinated-generation-publication-and-local-reload)
and [I4](engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference).

**Current state:** a prebuilt host loads edited content without Cargo. Reload
has explicit outcomes, a no-op revision does not advance the generation, and a
stale attempt carries the generation it was prepared against. A build that reads
content off disk watches the pack's sources (`ambition_content::content_watch`)
and reloads on save. Participating families include moves, boss profiles and
encounters, the character catalog, fighter facets, the boss seed library,
validator bands, items and audio registries. Content ownership is App-scoped
(I3 step 1): every install reads the App's `SelectedContentPack`, and the
process boot pack (`pack::shipped()`) is for inspection of the shipped product
only. The adaptive music-cue catalog (admitted with
`AdaptiveMusicCatalogRegistry::with_replaced`) and the cutscene library
(`publish_cutscene_library`, which replaces only the rows the selected pack
owns) do, since 2026-10-07, each with witnesses in
`an_edit_reaches_the_shipped_game.rs`; so does the quest book, as session-derived
state (it publishes nothing at the commit; `candidate_quest_book` admits or
refuses against the saved progress). The procedural tier (I4) runs
technique, boss-special and wielded-item modules on the linked and WASM roads,
with hot reload through the mechanical-edit protocol.

**Rulings (2026-09-19):** `Q110`: mechanical registry changes use explicit
lifecycle/replacement semantics; do not invent a universal silent overwrite.
`Q104`: content-authored movesets are the long-term authority, and duplicate
Rust move tables are migration scaffolding.

**Open work:**

- The three families that did not reload (music cues, cutscenes, quests) all take part since 2026-10-07. The references a candidate can break (a room naming a removed cutscene; a quest step naming a boss or room that does not exist) are judged at request time by the startup validator itself (`ContentGraphRefused`, 2026-10-07). Supersession of an in-flight generation is done (2026-10-07), and the quest book is asked again at the activation gate with the save as it is then, so a save that moves while a generation waits cancels it instead of being clamped.
- I4: save eligibility; ports for body motion so the remaining wielded items (dive, blink, grapple, mark/recall) can become modules; GNU-ton's conductor as a module.

**Blocked by:** nothing.

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### CONTENT-AUTHORITY-DUPLICATES — one authoritative reader per authored fact

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and
[`engine/participant-action-system.md`](engine/participant-action-system.md).

**Current state:** some authored facts have two readers:

- ✅ 2026-10-03: `boss_ron_target` no longer strips `_body`/`_hands` to map
  two files to one record; the key is the file stem. A probe on the strip
  fired 0 times over app_it (949 tests) and the content and boss lanes, with a
  positive control that fired. The `tools` generator still writes
  `gnu_ton_boss_{body,hands}` and `giant_gnu_{body,hands}` sheets into the
  published (gitignored) `gnu_ton_boss/` folder, and nothing loads them.
  The rows, the animation row and the hurtbox sample row of each boss attack
  are authored in `boss_art_keys.ron`; no other boss animation or sprite map
  was searched for after those moved.
- Yarn dialogue has its reader, and
  `game/ambition_content/src/content_validation.rs` checks dialogue references
  again. ✅ 2026-10-03, the `__` root fold: `known_dialogue_ids` also accepted
  the root of every `root__x` title. Four roots exist only as `__N` jump
  targets, so the validator accepted NpcSpawn ids the runtime cannot start
  (`a_spawn_naming_a_root_that_exists_only_as_variants_is_refused`). The ids
  are exact titles now. Open: the ids still come from `yarn_title_ids`, a
  `title:` line scan beside the Yarn compiler. The compiler is an optional
  dependency (`ui`), so replacing the scan is a dependency decision.
- LDtk/world cross-reference rules in `content_validation.rs` repeat rules that
  a world owner already checks. ✅ 2026-10-03, the LoadingZone target rule:
  `validate_ldtk_room_links` refused every zone without both targets, which <!-- cite-ok: the function's name on 2026-10-03; it is `validate_room_links` now -->
  refused a landing pad that `LdtkProject::validate` allows and reported half a
  target twice. It now checks only that a complete target names a room and a
  zone that exist (`a_landing_pad_is_allowed_and_half_a_target_is_refused_once`).
  ✅ 2026-10-03, the rest of the LoadingZone rules, compared with their owners:
  - Measured first: the validator had its own scan of the `LoadingZone`
    fields, and it trimmed a target that the converter did not trim. With
    `target_room = "scroll_lab "` the validator reported nothing and the room
    set dropped the link: a dead door that validation accepted.
  - The LDtk owner has the one text rule now (`field_text`: trimmed, blank is
    absent). `LdtkProject::validate` and `collect_room_links` both use it.
  - A blank zone id and a second zone with one id in an area are the LDtk
    owner's errors now. `validate` gates each room set, so each LDtk game gets
    them, not only this validator's game.
  - `rooms::unresolved_links` is the one judge of "this link names a room and
    a zone that exist". `RoomSet::try_from_parts` warns of an unknown room and
    `layout_warnings` of an unknown zone, both from it. A room set cannot
    refuse such a link: measured, a partial set keeps the exits of its rooms
    (one room alone, and the engine's 59-room world, whose 4 links name rooms
    that only the full game adds).
  - The validator holds the complete game, so there each unresolved link is
    an error. It reads the rooms and links that the runtime builds
    (`to_room_parts`), and its own scan is deleted.
  ✅ 2026-10-03, the NpcSpawn ids, same class. The validator judged
  `character_id`, `brain_override` and `dialogue_id` trimmed, and
  `convert_npc_spawn` gave the runtime the raw values (measured:
  `"npc_ai_slop "`, `" guard "`). An unknown character is a body with no
  identity, and an unknown preset is a panic when the room loads. The
  converter reads the three with `field_text` now
  (`an_npc_spawn_carries_its_trimmed_ids`).
  ✅ 2026-10-04, the quest checks' boss and encounter ids. Measured on the
  shipped world: the validator slugged the display name of each `BossSpawn`,
  and for 9 of the 11 bosses that is not the id that the boss reports when it
  is defeated (`system_boss` against `clockwork_warden`, `t_rex` against
  `trex_boss`). A quest that named the reported id was refused, and a quest
  that named the slug was accepted and could not complete. The two shipped
  boss quests name the only two bosses for which the ids agree.
  - `ambition_boss_encounter::behavior::authored_boss_behavior` is the one
    resolution from a placement to its behaviour. The boss constructor, the
    room boss-art keys and the validator use it.
  - The boss looted flag is keyed by the placement id, as
    `ambition_boss_encounter::rewards` keys the chest. The validator derived
    it from the slug.
  - The encounter ids and their looted flags are those of the loader
    (`load_encounter_specs_from_rooms`), which builds only the first trigger
    of a room. The validator's scan named each trigger.
  - Witnesses: `a_quest_names_a_boss_by_the_id_its_defeat_reports`,
    `a_boss_looted_flag_is_keyed_by_its_placement`. `check_quest_steps` takes
    the quests as an argument, so a test can plant a step.
  ✅ 2026-10-04, the rest of the quest targets. The flags of NPC talk,
  switches and `flag:` pickups, and the ids of NPCs, pickups and rooms, are
  read from the composed rooms (`QuestTargets::of`), each made by the function
  the runtime makes it with (`npc_talk_dialogue_id`, `SwitchActivation::
  parse_custom`, `PickupKind::StoryFlag`). Measured on the shipped world before
  the change: the sets were equal (198 flags, 172 NPCs, 38 pickups, 72 rooms)
  but for one flag that the scan did not know, `npc_generic_npc_talked`, which
  a talk to an NPC with no dialogue sets. The validator has no scan of the
  LDtk entities for a quest target now; `authored_flag_ids`, <!-- cite-ok: records deleted functions -->
  `authored_npc_ids`, `authored_pickup_ids` and `authored_entity_iids` are
  deleted. <!-- cite-ok: records deleted functions -->
  ✅ 2026-10-08, `QuestStepCondition::ItemCollected` had no producer of its
  event, so the validator accepted a step on a pickup that could not advance.
  `collect_ecs_pickups` writes `ItemCollected(pickup id)`, the placement id
  the validator reads (`a_collected_pickup_reports_its_id_to_the_quests`).
  No shipped quest uses it. Open: the cutscene bindings still read `active_area_ids`
  from the project; that is the level-by-level read the two-per-room rule
  needs.
  ✅ 2026-10-07, `scripts/check_world_graph_is_navigable.py` read the zone
  targets untrimmed, so it was stricter than the engine (a `"vault "` target
  resolves in the game and was reported as dangling). It reads them with the
  engine's `field_text` rule now (`test_a_door_target_is_read_with_the_engines_text_rule`,
  red before). It is still a second reader of the target fields, and it owns the
  trap analysis the Rust side does not have; the id of a zone is read as the
  engine reads it, untrimmed.

**Open work:**

- Remove each duplicate. Do not wrap Yarn in a schema unless that removes an
  authority. Do not move worlds into a content pack only for uniformity.
- External-capability witness: one capability outside the actor monolith uses a
  provider schema, a provider semantic action with a real device binding
  (`ProviderBindings`), and a causal fact, through public APIs only. It needs no
  new central enum variant and no private reader.
  State 2026-10-07: `examples/capability_demo` does all three (a registered
  schema, a `ProviderBindings` action that returns as `SemanticActionPressed`,
  causal facts), and its rollback test compiles and fails when the cooldown's
  registration is removed.
  **Gate C5 is read (review ruling, 2026-10-07):** an extension may name the
  narrow public crates it extends (`ambition_content_pack`, `ambition_causal`,
  `ambition_input`, `ambition_platformer2d_core`); it is NOT forced through the
  umbrella facade, because [`public-sdk-1.0.md`](engine/public-sdk-1.0.md) says the
  facade is not the dependency boundary for independent builders. What it may
  not name is engine-internal topology. The demo named one such crate,
  `ambition_platformer2d_shared_tangle`, for two items (`SimScheduleExt` and a
  phase set); those moved to a narrow crate, `ambition_sim_schedule` (depends only
  on `bevy`; `shared_tangle::schedule` re-exports them, so no engine path moved),
  and `scripts/tests/test_capability_demo_names_no_engine_topology_crate.py`
  keeps the demo off the tangle crate. **The demo is now the external-capability
  witness**: its normal closure is eight narrow crates. The phase set's topology name was dropped in the same sitting
  (`Platformer2dSimulationPhaseMonolith` is now `Platformer2dSimulationPhase`; 378 uses, <!-- cite-ok: records the old name -->
  114 files, compile-verified; the 0019 ADR keeps the old name as a record).

**Blocked by:** nothing. The external-capability witness is
`examples/capability_demo` (Gate C5 read above).

**Acceptance:** each fact has one authoritative read; diagnostics name the
authored source; the old reader is deleted.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling measure
what a profile links and installs. The three composition probes in
`composes_through_the_sdk` pin the fixed step and assert that `FixedUpdate` ran.
A profile witness must assert that it stepped, not only that it built.

**Ruling (2026-09-19, `Q100`, `Q106`, `Q108`, `Q97`, with `Q146` and `Q144`):**
capabilities are optional and composable. A capability that authored production
content requires and the composition lacks must refuse that content or its
admission. Reduced tools and tests may omit capabilities explicitly. The ruling
says: implement this architecture rather than continuing to census hypothetical
composition variants.

**Done 2026-10-08 (first set):** five named profiles are a registry
(`ambition_platformer2d_runtime::profile`): `headless-body-world`,
`windowed-body-world`, `combat-without-inventory-boss-dialogue`,
`collection-without-held-use`, `encounters-without-named-bosses`. Each constructs
and steps a real body, installs none of what it omits, and has its session-edge
parameters validated; a control arm proves the probe can say yes
(`ambition_platformer2d_host/tests/supported_profiles.rs`,
`scripts/check_engine_profiles.py`). The probe found eight couplings that made a
promised omission fail on its first tick; all are repaired in source, and the table
is in [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md#supported-profiles).

**Done 2026-10-08 (admission):** a profile admits content with
`ambition_engine_schemas::engine_schemas_without(profile.omitted_content_capabilities())`;
`Capability::content_capability` names the pack capability each runtime
capability interprets (`BossEncounters` → `boss_pattern`, `Cutscenes` →
`cutscene`). A pack whose manifest requires an omitted one is refused with
`MissingCapability`; the full schemas admit the same pack. Witness:
`supported_profiles.rs` `a_profile_refuses_content_that_needs_a_capability_it_omits`
(poison: the omission filter off gives no refusals).

**Open:** the claim is *not installed*, not *not linked* (`Q106`: the crates behind
these capabilities are unconditional dependencies); `Dialogue` has no pack
capability (Yarn is not a pack schema), so a Yarn script asked of a profile
without dialogue is not refused at admission; no shipped host admits content
under a reduced profile (the refusal is witnessed at the profile's schema set);
`world-without-cutscenes` (2026-10-08) omits `Cutscenes` and steps; a pack
that requires `cutscene` is refused under it. Re-entry is witnessed for
the headless profiles (`a_second_session_in_one_process_steps_as_the_first`:
a second session in one process puts the body at the same position bit for
bit; poison: a process-global spawn offset, red).

**Blocked by:** nothing.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md), which holds the
modeling rules and the evaluation-rig contract.

**Current state:** the option layer prices what a move can do from one combat
model:

- A move's hittable region (`MoveFrameData::coverage`) and its push region (`push_coverage`) are separate. A pure shove is priced by `Features::displacement_value` (push coverage × the foe's proximity to a blast line × whether the push sends them that way).
- Admission is absolute: the opponent must lie inside the move's own region, with `ADMISSION_SLACK_PX` on each side. Admission leads the foe over `Perceived::staleness_s()` plus startup. Scoring stays on the observed position, so `reaction_ms` stays the difficulty axis.
- The brain and the hit resolver spend one launch law, `ambition_entity_catalog::launch::launch_speed`. A hazard is a value (`MoveHazard`, `ThreatTravel` in `ambition_entity_catalog/src/hazard.rs`), and an unresolvable ranged request is no offer.
- The shipped ladder's weights still order the rungs at least as well as any single-weight change tried (2026-09-21 refit sweep), so do not refit them.

Instrument: the `#[ignore]`d sweep `every_fighter_on_the_grid_can_fight_its_mirror`
(21 mirror matches of 3600 ticks; `AMBITION_GRID_TRACE` for one fighter). Run it
before and after a change, on one binary, and report both columns.

**Open roads, in order:**

1. **A brain cannot decline to attack.** The decision takes `options.attacks.first()` whenever the body is free, so the move that survives at range is thrown until the world changes. The brain does not remember its own last move. A decline gate on decision ticks was built and was a no-op, because presses are bounded by move duration (24 ticks), not the decision cadence (5). Next: a refusal that lasts as long as the move would have (`frames.total_s`), which needs one more piece of brain state and its snapshot projection.
2. **A counter and a buff cannot be chosen between.** Pricing them needs a defensive feature ("is the opponent committed to a swing"), not a wider admission rule.
3. **`sanic` @5 leaves the stage early** (about 640–780 ticks, both seats airborne). That is a stage defect, not the one-move lock. The guard for it is a bout that ends early, not move variety.
4. **A ranged move scores `launch: 0`.** A projectile hit writes a dimensionless `HitKnockbackMagnitude::FeelScale(0.85)`, not `LaunchSpeed`. Settle whether the feel reference belongs on `LaunchConditions`, whether `0.85` leaves the projectile stepper, and what `max_knockback` means for a launcher. A launcher also needs a hazard-coverage feature (its `reach_fit` is zero at every range).
5. **A placed trap has a position, and `reach` is a radius.** Carry the dangerous region relative to the body at the resolved-offer seam, not another reach scalar.
6. **A teleport's destination does not reach the brain** (`TeleportParams`: `behind_nearest_foe`, `behind_gap`, aim). It belongs in the same resolved offer.
7. **The ladder's step per rung.** All shipped rungs author `rollout_depth: 0`, so the habit read is off at every rung (`read_weight` is deleted, Q90) and the L3 step the engine ladder takes at level 6 is missing. Measure `--rungs 3,4,6,7,8` to separate step size from one pair.

**Standing prohibitions:**

- Do not fix the lock with a second, softer admission rule or a `reach_fit` floor.
- Do not feed push coverage back into `reach_fit`.
- Do not patch the evaluator with a fighter-specific exception. Keep press generation separate from move utility.
- A two-move fixture cannot price a move against a kit. Pin a feature against the kit `attack_kit_of` builds.
- A fixture that shares an app with a live CPU measures the CPU too.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** product rows named by the inventory. (Q62
and Q89 were ruled on 2026-10-04: Q62 is the LDTK-SEMANTIC-DIFF task, and a
stand-in may keep an incomplete kit.)

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Current state (2026-10-08):** no known residual. The last one, the Smash
stand-in's hand-written verb list, is content in the form every fighter
authors (owner document, closed slices).

**Next implementation:** search for a new duplicated authored/runtime value
with the owner document's two questions (A1). For each one found, choose one
authoring owner and make every runtime representation a projection or admitted
prepared value. Prefer deleting the second truth to synchronizing it. Exit
criterion 5 (another experience consumes a character without irrelevant
facets) is not yet measured.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### BAKED-SHEET-IDENTITY — a body's collision box is outside the content identity

**Owner:** `ambition_sprite_sheet` (the baked sheet index) and
`ambition_platformer2d_provider` (`MechanicalRegistries`).

**Current failure (read 2026-10-05, not poisoned):** a sprite-authored body
takes its collision box, its per-pose boxes and its attack polygons from the
baked sheet index (`record_for_sheet_key`: `sprite_body_collision_for_sheet`,
`posed_body_geometry`). The index is `BAKED_SHEET_RONS`, compiled in and held
behind a process `OnceLock`. No content-identity section reads it:
`characters.authored-sheets` is `AuthoredSheets::deterministic_dump`, which
holds only the sheets a provider registers (one production caller,
`room_transition_assets.rs`). Published sprites are not in version control,
so two machines at one revision can hold different body metrics under one
content fingerprint. A rollback timeline contract that compares the
fingerprint then accepts a peer whose bodies are a different size.

**Blocked by:** [Q157](awaiting-maintainer-decision.md#q157--when-two-machines-hold-different-published-sprite-metrics-may-they-play-together)
for what a difference does (refuse, warn, or nothing). The projection itself
is not blocked.

**Not the fix:** a digest of each sheet's text. It also changes when only the
atlas packing changes.

**The projection is built (2026-10-08):**
`ambition_sprite_sheet::sheet_mechanics` encodes each record's mechanics (key,
target, frame size, body metrics with every box, part, polygon and feet,
per-animation hurtboxes, hitboxes and frame durations, tuning, drawn facing,
and each row's animation, frame count, durations and mirror) and not its
packing (images, pages, `label_width`, `y_offset`, `row_index`, `rects`).
`character::sheets::baked_sheet_mechanics_digest` is that digest over the baked
index in key order, computed once at runtime. Witnesses:
`a_sheet_whose_packing_alone_differs_has_the_same_digest` (poison: hash the
rects; red), `a_sheet_whose_mechanics_differ_has_another_digest`,
`the_baked_digest_is_this_builds_records_in_key_order`.

**What is left is the ruling.** The digest is not in the content fingerprint,
because Q157's default in force is (c), leave it out. Ruling (a) is one
section, `characters.baked-sheets`, beside `characters.baked-landmarks` in
`prepare_platformer_content` (a field on `MechanicalRegistries` filled by both
provider roads, as `baked_landmarks` is). Ruling (b) needs a weaker class of
identity section.

**Acceptance:** a baked sheet whose body box differs gives a different content
fingerprint; a sheet whose packing alone differs gives the same one.

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `features/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`features/ecs/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Current state:** `MountedBrainCache` has no production constructor, so
`ControlClaimant::Mount` has no production writer. Nothing resumes the mount's
recorded brain when a ride ends; a mount death rebuilds a solo brain from
config.

**Next action:** author the customer, then specify and repair, each with a
witness: control transfer, the mount's previous brain, dismount and its
restoration, body and room lifetime, damage and death, participant ownership,
animation composition.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

### REACH-VIEW — AI reads reach from the move's geometry

**Owner:** the moveset; `ambition_entity_catalog::MoveFrameData::reach` for
fighters and `ambition_characters::brain::action_set` (`reach_px`) for action
sets.

**Ruling:** Q35 (2026-10-03): the moveset/attack definition owns prospective
mechanical reach, also during startup. AI reads a derived view of the move
geometry, never sprite bounds. This extends Q107.

**Next action:** find each AI read of reach, and make sure that each one comes
from the move geometry by phase. `MoveFrameData::reach` is already derived from
the Active volumes.

**Acceptance:** a test changes a move's hitbox and sees the AI's reach change,
with no sprite change.

**Measured first (2026-10-03, the shipped prepared cast, 29 rows with an
autonomous profile):** 15 Smash rows have an attack move, and for 14 of them the
authored `smash_hit_band` was more than 1 px from the reach of the move. Examples:
goblin 32 against 38, the automata 36 against 44, Carl Stargan 36 against 22,
the goblin brute and the pirate heavies 36 against 48.4. Also measured: the
moveset derived from an action set reaches `1.1 × reach_px` (28 gives 30.8), so
`reach_px` is not the hit extent. The moveset geometry is the one answer.

**Slice 1, DONE 2026-10-03: the Smash brain.**

- `BrainSnapshot::melee_reach` is the reach of the move that the body's forward
  attack press starts, in the real posture of the body. The snapshot builder
  fills it with `melee_reach_of`, which asks `move_for_attack`, the resolver of
  the press road. A running body thus reads its dash attack.
- `tick_smash` sets its three distance bands from that reach each tick
  (`SmashCfg::with_hit_band`). The bands in a `SmashCfg` are only those of a
  body with no attack move (`NO_ATTACK_MOVE_HIT_BAND`, 36 px).
- `BrainProfile::smash_hit_band` is deleted, with its four authored rows. <!-- cite-ok: records a deleted field -->
- Witnesses: `the_hit_band_is_the_reach_the_snapshot_states` (combat),
  `a_smash_brain_swings_where_the_hitbox_of_its_move_reaches` (the acceptance
  test: two bodies that differ only in one hitbox) and
  `the_reach_is_that_of_the_move_the_press_starts` (monolith), and
  `a_smash_enemy_swings_from_the_reach_of_its_own_move` (app_it: the goblin
  brute decides its first press at 48.3 px; with the snapshot line removed it
  closes to 35.4 px).

This changes how the Smash enemies space themselves: each now stops and swings
where its own move reaches.

**Slice 2, DONE 2026-10-03: MeleeBrute, the hostile Aerial bird, the aggressive
Patrol.** Each read `cfg.attack_range`, an authored distance. Each now reads
`BrainSnapshot::melee_reach`, and its cfg distance is only for a body with no
attack move. The snapshot builder derives the reach for every body with an
attack move (review 2026-10-04: the reach is a fact of the moveset, so no
brain gate decides if it is derived). Each brain decides if it reads it: a
peaceful patroller does not, because its `attack_range` is where it stops to
talk (`a_peaceful_patroller_keeps_its_own_distance_when_told_a_reach`; poison:
it reads the reach, and it stops turning to the foe at 40 px). The old gate,
`StateMachineCfg::closes_to_its_melee_reach`, is deleted. <!-- cite-ok: a deleted name --> The shipped MeleeBrute users are
the provoked pirate heavies (reach 48.4, authored 53 to 59 with the 56 px
floor). The parrot read 60 and fsm_noodling 50; both pecks reach 52.8.
Witnesses: `melee_reach_tests` in `brain/state_machine/tests.rs`. Each read
was poisoned alone, and only its own test failed.

**Remaining readers (not changed):**

- `ChargeCrashCfg::bite_range` (the shark 200 against 46.2). It is not a
  reach. The shark presses its bite and then charges, and the charge carries
  the hitbox to the foe. A view from geometry must add the travel of the
  charge to the reach of the bite. This needs a measurement of how far the
  charge moves the hitbox while the Active window is open.
- The fighter's `assumed_foe_reach` (60 px): since 2026-10-08 it is only
  the reach of an attack the shadow predicts. A foe the view sees swinging
  carries the reach of the move it plays (`PerceivedActor::attack_reach`,
  from `MoveFrameData::reach`), and the shadow lands that swing from there.
  Witnesses: `a_watcher_reads_a_swings_reach_from_its_hitbox` (monolith;
  two hitboxes 20 px apart read 36 and 56) and
  `a_foes_swing_reaches_as_far_as_the_move_it_plays` (combat; a 100 px move
  lands from 90 px, a 40 px move and the 60 px assumption do not). Each
  poisoned.
- A body with no attack move keeps the authored distance of its brain. One
  case looks incorrect and is not measured in play: a dismounted rider with no
  ranged item gets a MeleeBrute brain whose distance is the profile's
  `attack_range` (1100 px for the pirate raider), so it can stop and press
  nothing from far away.
- `MoveFrameData::reach` is the reach of the volumes in the body frame. It
  does not include the motion of the move (a dash attack moves the body), so a
  running Smash enemy reads 40 px for a dash attack that travels farther.

### CPU-LADDER — the brain owns the knobs, Smash owns the ladder

**Owner:** `ambition_combat::brain::fighter` (mechanism) and the Smash
rules/content (ladder tuning).

**Ruling:** Q88 (2026-10-03): the generic fighter brain exposes reusable
controls (reaction, tactics/aggression, prediction/evaluation, execution/error);
Smash owns the mapping from "CPU level N" to them. Q90 (2026-10-03): remove the
inert `read_weight`; a correctly named parameter returns with an implementation
that gives it meaning.

**Current state:** `fighter_brain_ladder.ron` is in
`game/ambition_content/assets/data/`, and its schema is registered by
`ambition_combat`. ✅ Q90 is built (2026-10-08): `read_weight` is deleted from
the profile, the ladder and every fixture; the habit decay is the constant
`HABIT_DECAY` (0.9); the rollout reads the habit whenever the read is genuine.
No ladder rung changes (none rolls out). `for_level` brains at levels 6 to 9
roll out, so their habit counts now decay at 0.9, not `t × 0.6`.

**Acceptance:** the ladder data and its level vocabulary live with the Smash
rules/content; another game can build the brain with no ladder; no authored
ladder field is inert.

**Measured 2026-10-08 (NamekAmbition):** the Smash demo app installs no
`AuthoredFighterLadder` (`the_ladder_the_demo_runs.rs` pins the floor).
`ambition_content` `plugin.rs` and `reload.rs` (`publish_fighter_ladder`)
are the only installers. A Smash pack source may name a file outside its
root, as George's facet does.

**Next (2026-10-08):** the engine still carries a level curve,
`FighterBrainProfile::for_level` (13 production fallback sites), and the demo
plays it. The order of the move is in
[`fighter-brain.md` F1](engine/fighter-brain.md#f1--the-authored-ladder-is-the-authority-q88-2026-10-03):
the ladder becomes a rule scoped to the rooms it governs (`ambition_app`
composes Smash, so a second `AuthoredFighterLadder` installer would make two
authorities), Smash declares its pack's rows for its mode, and then the floor
becomes one level-free default.

### LANDMARK-CLIP-TIME — a published landmark clip loops or holds as the row it describes

**Owner:** `ambition_sprite_sheet` (`baked_landmarks`) and
`ambition_characters::actor::landmarks`. Part of RIG-LANDMARKS. Review of
2026-10-05, finding 3.

**Current failure (read 2026-10-05):** three sources answer one landmark
question and they keep time in three ways. A rig clip states `looping` and
holds the last frame of a one-shot (`RigClip::frame_at_time`). The visual
animator holds a row whose pose is in a code table
(`ambition_sprite_sheet::character::non_looping`), and holds every row a move
asks for by name. The package landmark clip (`LandmarkClip::frame_at_time`)
always wraps, and its published schema has no loop statement. So after a
one-shot row ends, the sprite and the rig hold the last frame and the package
landmark goes back to frame 0: a hand or a head jumps while the art does not.
The pet reads this query, and so do the shots and the held items.

**Built 2026-10-05:** `LandmarkClip` carries `looping`, and
`LandmarkClip::frame_at_time` holds a one-shot on its last frame, as
`RigClip::frame_at_time` does. The table's source states the bit from the one
rule the animator uses for a row name
(`ambition_sprite_sheet::character::row_loops`); the query (`BodyLandmarks`)
infers nothing. Witnesses: the real `CharacterAnimator` against the table of
`player_robot_v3`, 2.3 clip lengths in, for `idle` (loops) and `shoot`
(holds): before the change the table was on frame 2 while the sprite held
frame 5 (`a_landmark_clip_loops_or_holds_as_the_animator_shows_its_row`); and
a rig against a table of the same clips, for a looping and a one-shot clip
(`a_rig_and_a_package_keep_one_time_for_a_looping_and_a_one_shot_clip`).

**Open, measured 2026-10-05:** the rigs and the animator do not agree with
each other. No published sheet states which rows loop. Each rig target states
it by hand (Mary-O: `idle`, `walk`, `climb`, `swim`, `crouch_walk` loop; every
other row holds), and the animator's code table loops five rows that the
eight published rigs hold: `crouch`, `crouch_jump`, `jump`, `skid`, `taunt`
(`the_rows_a_rig_and_the_animator_time_differently_are_these` holds the set).
A rig is not admitted in a shipping game, so no player sees this today. The
repair is ONE loop statement for each row, published in the sheet, which the
animator, the rig and the landmark table all read; then the code table and
the rig's own bit go. The renderer owns the publish (Toothbrush), and every
sheet is published again.

**Named limits:** the bit is of the row and not of the pose. The animator
times a pose by the pose it was asked for, so a one-shot pose that falls back
to a looping row (a `hurt` pose on a sheet with no `hurt` row shows `idle`)
is held by the animator and wrapped by the table and by a rig. The bit is set
when the table is decoded, from code, so the landmark digest does not hold
it: two builds of one revision agree.

**Acceptance:** ✅ for a looping row and for a one-shot row, more than one
duration in, the package frame is the frame the animator shows; ✅ the rows on
which the rig and the animator disagree are a named, counted set that a test
holds. Open: one published loop statement.

### MOUNT-SEAT-LANDMARK — a mount's seat is the saddle its art states

**Owner:** `ambition_mount` and the landmark query; the renderer for the
publish (Toothbrush). Part of RIG-LANDMARKS. Review of 2026-10-05, finding 6.

**Current failure (read 2026-10-05):** the shark's catalog row holds
`saddle: Some((28.4, -10.2))`, a number measured from the art's `saddle`
socket and typed into the gameplay catalog (`d1589237c`). The art moved once
and the number did not, which seated the rider off the saddle. A second
redraw repeats it. The socket itself is placed by hand in the renderer target
(`burning_flying_shark.py`, `_px(396.0, 152.0)`), is a rest-pose point in
`<target>_actor.ron`, and is not a track of the part flipbook: it does not
move with the fly or bob frames (Toothbrush, measured 2026-10-05).

**The fix:** gameplay names the seat as a landmark and the art package
supplies its place for each pose: a `saddle` track in the part flipbook,
projected into the landmark table as `head` is. An authored seat stays for a
mount whose art publishes none, as the named fallback. Do not write an
importer for the shark alone.

**Blocked on:** the renderer publishing the `saddle` track (Toothbrush).

**2026-10-06 (review P2, the T-rex jaw):** the seam for a named point that a
body rides exists for a body with a BODY RIG: an attachment the art states on
a joint (`RigAttachment`), placed by `resolve_body_rig_poses`, read by the
capture relation (`CapturedBy::hold_attachment`) and offered to a module
(`ambition.body.attachments`). The T-rex's jaw uses it. The shark does not:
it publishes no body rig and no per-pose saddle, so this row stays blocked on
the publish. When the shark has a rig, its saddle is one `ATTACHMENTS` row in
its renderer target and the mount reads the attachment as the capture relation
does; do not add a second road. One known difference to close then:
`body_landmarks::feet_of` places a rig from the bottom of the body's box and
does not read `RigFeetOffset`, which `rig_feet_from_centre` (hurt parts, hold
points) does.

**Acceptance:** the shark's row holds no saddle number; a redraw that moves
the saddle moves the rider with no gameplay edit; a mount with no published
saddle still seats its rider at its authored seat.

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**The invariant:** every pixel a composited body draws lies in its cell. One
camera draws a whole page with no per-cell scissor, so a part past its cell
is cut from its own body and drawn into its neighbour's.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_margin`,
`posed_reach`), `ambition_sprite_sheet::character::rigged` (`art_overhang`)
and `scripts/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Scope since 37497a444 (2026-10-05):** a part-drawn body draws its parts in
the world, and uses an impostor cell only while something composes it
(`ComposedBodyDemand`: hit flash, portal pieces, deep dream, quasar) or its
frame fades as one picture. So the invariant applies to that composed road
only.

**Solved, by kind of draw:**

- A published frame (2026-10-05). The cell is the frame plus
  `impostor_margin`, which covers the flipbook's stated `art_overhang`. The
  oni leader's banner is drawn whole (`a_cell_holds_every_draw_of_its_body`).
- A tween's in-between (2026-10-06, review P2). `art_overhang` is a
  conservative envelope of every draw `tween_into` can make, taken when the
  flipbook is read: each moving draw is sampled finely enough that no corner
  moves more than half a pixel between samples, and that slack is added
  (`tween_overhang`). A part that turns between two frames reaches where
  neither frame does (`the_overhang_of_a_tweened_clip_covers_its_in_betweens`;
  the published corpus: `no_in_between_of_a_published_flipbook_reaches_past_its_overhang`).
  Measured on the 123 published flipbooks with a tweened clip: the in-betweens
  move the stated overhang of 12 of them, by 0.21 px at most; no cell changed.
- Parts placed by a `PartPose` (2026-10-06, review P2). A pose is in no file,
  so its reach is measured each frame (`posed_reach`). While it is more than
  the margin, the body is drawn directly, as a body whose frame fits no cell
  is: whole, and its readers see no image, with one warning. The hold that
  smooths composition demand smooths this too
  (`a_part_pose_that_reaches_past_its_cell_is_drawn_directly`).

**Open:**

1. `scripts/measure_rigged_parity.py` still clips its oracle to the cell, so
   a cut part cannot read as a parity failure there. The clip was added when
   the cell was the frame plus 16 px (the oni leader's banner read as 1153
   wrong pixels of a correct draw). With the margin from `art_overhang` the
   clip should remove nothing. Remove it and run the harness on a GPU
   (`rigged_sprite_parity`); acceptance: the oni leader passes unclipped, and
   with `impostor_margin` poisoned back to 16 px it fails.
2. A posed body that reaches past its cell loses its composed readers (its
   hit flash overlay, its portal pieces) while it does. If a ragdoll must keep
   them, give the presentation a cell chosen from the pose's reach, or let a
   pose declare an envelope. No shipped body carries a `PartPose` today.
3. A two-body shared-page test that reads the pixels of both cells needs a
   GPU; the three proofs above are on the draw geometry.

### DENSE-MELEE-ROOM — author the dense-melee development room

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Current state:** no such room; the density script fakes density by widening
the viewport over the Hall's `stand_still` cast.

**Next action:** after the P1 correctness rows above, author one room (LDtk or
a generated spec) with many tactical-brain fighters in close melee, reachable
by an ordinary route and by `measure_perception_density.sh`.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

### LDTK-SEMANTIC-DIFF — review content diffs by meaning, and find the rewrite

**Owner:** `tools/ambition_ldtk_tools` (`edit/semantic_diff.py`). Plan:
[`engine/authoring-and-tools.md`](engine/authoring-and-tools.md#content-diffs-need-domain-aware-comparison-q62-q78).

**Ruling:** Q62 and Q78 (2026-10-04) are engineering and evidence tasks, not
maintainer choices.

**Current state:** the comparison is done: `diff semantic` (file or
`REV:PATH`), `diff range` and `diff normalize`, with verdict, per-level
summary, ambiguities and a noise section (tests:
`tests/test_semantic_diff.py`). It prints the per-level summary for the
post-fix rewrite commits `576a8fd`, `c6df2b7` and `056079f`. `48f8e26` is not
in this clone, so `48f8e26 → cb7062a` is not re-measured. LDtk files still
rewrite far beyond their edits after `54d99e7fb` and `2e69e81b9`; the writer
is not identified.

**Measured 2026-10-08:** each whole-file writer ran twice on an unchanged
copy. `compact`, `policy fix`, `asset editor-art` and `repair` write the same
bytes on a second run. The rewriter was `generate hall-of-characters`, through
`area create --replace-existing`: it allocated every iid and uid again, so each
regen renumbered 145 iids and 5 uids (139 of them runtime ids of entities with
no `id` field). Fixed: a replaced level carries the iid, uid and seed of the
level, each layer's iid, and each entity's iid matched by type and position
(`carry_identities`), and a built entity sits on its policy layer
(`place_on_policy_layers`). A regen of the committed hall now writes it byte
for byte (`tests/test_area_regen_keeps_identities.py`; poisons red).
`camera auto-cover --create` writes the same bytes on a second run. Not
measured: the spec-driven `entity` writers (move, set-field) and `level clone`
(which mints fresh ids by design).

**Acceptance:** a second run of each writer on its own output changes no
byte.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings; the packager ships every file, including the
98.8% of the 449 MB ultrapack that no runtime road reaches.

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change; then cut the ultrapack's
runtime product by runtime unit.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

### PORTRAIT-TIERS — portraits scale with quality and stay usable

**Owner:** `ambition_sprite_sheet` (`bake_portrait_manifests`) and the dialog
portrait consumer. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#quality-is-a-presentation-policy-q84).

**Ruling:** Q84 (2026-10-04).

**Current state:** reduced portrait tiers are generated and shipped, and
nothing loads them.

**Acceptance:** each quality tier loads its own portrait product, sized from
the dialog draw size; potato portraits are smaller than full and still fill
their box.

## Receipts

Closed rows that an open row, a script or an inbound link still names.

### A4 — separate control authority from body execution on the real schedule — ✅ DONE 2026-10-09

`sim_phase_pins::every_control_writer_is_ordered_against_the_gate_and_the_gate_before_integration`
reads the declared access of every system in the shipped `GgrsSchedule`
(2026-10-08: 22 `ActorControl` writers, none unordered against
`PlayerInputSet::ControlGate`). The four that produced intent after the gate
moved before it: `shark_ride::tick_departures` and the boss brain
(`tick_boss_brains_system`) run after `ActorDecisionSet::Publish`; `BossSteerSlot`
runs in the gate after `drive_commanded_moves`. `WRITES_CONTROL_AFTER_THE_GATE`
lists only systems that spend a press, integrate, or derive from a gated frame.
Witness: `boss_motion_parity::the_frame_a_held_boss_integrates_under_is_the_gated_one`.
One commanded-move road walks every body, a boss too
(`a_lured_boss_walks_to_its_mark_and_starts_no_attack`, `a_dead_body_is_not_walked`).
The Mockingbird's path is its conductor's
(`its_path_is_its_conductors_and_a_hold_on_its_brain_changes_nothing`). Its
brain keeps its movement write: the body road declines locomotion for a held
body in one place (`integrate_actor_body`), for a rider and a conducted boss
alike.

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel — ✅ DONE 2026-10-09

Closed on a fresh census, as its acceptance asked (2026-10-09): every
multi-writer App resource and session-world component carries a verdict
(`check_multi_writer_resources_are_adjudicated.py`: 8 of 23 session-world
components are multi-writer, all adjudicated; the rollback-registered queue is
spent), every optional read of a session authority says what its `None` means
(`check_session_authority_none_arms.py`: 141 sites, 99 rows), and no resource
crosses the rewind boundary unclassified (`resources_crossing_the_rewind_boundary.py`).
Its continuous items: AP9 (stale docs) is ordinary hygiene; AP14 is its own row
under P1. C11 in [`consolidation-plan.md`](consolidation/consolidation-plan.md)
owns the history.

### BAG-RECORD-HORIZON — a bag record is owned by what is left of its consequence — ✅ DONE 2026-10-09

A restore keeps the grant and spend records it keeps beside the bag
(`ItemCheckpointRestoreInputs.grants`/`.spends`), an occurrence that ends in a
live room becomes `Consumed` with its room's participants as owners, an ended
mint's row is compacted at the next checkpoint, and an object in a spared room
stays where it is. Witnesses are in `death_restores_the_checkpoint.rs`. The one
open line, whether a dormant row is owned by participants, is Q161 (default (a),
today's behaviour, in force); nothing is buildable before its answer.

### SETTINGS-ROLLBACK — a settings change reaches simulation only at an admitted rebase — ✅ DONE 2026-10-08

The frame-mode half was closed earlier (`ControlFrame` carries
`control_frame_modes`). The damage half: `PlayerDamagePolicy` was written from
`UserSettings` in `Update` and read by three simulation systems, so a
resimulated frame read the current difficulty. It is now a `Q120` mechanical
domain. `propose_player_damage_policy` writes `ProposedPlayerDamagePolicy` and
proposes; `publish_player_damage_policy`, the only writer of the policy, copies
it in `MechanicalEditSet::Publish` when the timeline admits it. A local
timeline is stopped and rebased; a foreign or unhealthy one refuses, and the
proposal waits. No simulation system reads `UserSettings` or the proposal.
Witness: `a_settings_change_reaches_the_simulated_policy_only_when_the_timeline_admits_it`
(poisons: a publisher that ignores the admission; a proposer that never
proposes). `Q127` (match-wide or per participant) changes the shape of the
policy, not this road.

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation — ✅ DONE 2026-10-08

Owner: [`engine/extension-model.md`](engine/extension-model.md) and
`crates/ambition_platformer2d_provider/src/lifecycle.rs`. Candidate B is built
from transaction-local N+1 (`PendingGenerationInputs`, keyed by `load_id`):
the cast, bosses, character catalog, audio and adaptive cue providers. The
fighter ladder and the encounter waves are stated standing projections.
`AuthoredSheets` cannot change across the window (a reload refuses it), and
`forced_brains`, `population_cap` and `perception_extent` are inserted once at
plugin build (re-read 2026-10-08). Witnesses:
`the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`,
`a_boss_tuning_saved_while_the_game_runs_is_played`,
`a_candidate_that_drops_a_providers_audio_is_refused_and_the_live_audio_survives`
(control `a_candidate_that_edits_a_providers_audio_activates_and_publishes_it`).

### MUSIC-CANDIDATES — music is chosen from scoped, prioritized candidates — ✅ DONE 2026-10-08

Ruling Q72, Q150. Owner: `ambition_encounter::music` (`EncounterMusicRequest`,
`MusicSource`) and `ambition_platformer2d_actor_monolith/src/music/intent.rs`.
Each source owns one candidate per room; a source is a kind and an instance
(each encounter script is its own source, keyed by its `SimId`); the latest
claim plays, ties by source order; the request is in the peer checksum by
value (schema 331). Witnesses:
`a_participants_boss_outranks_the_primary_seats_room_music`,
`the_heard_room_is_the_highest_priority_then_the_primary_then_the_lowest`,
`a_release_leaves_the_claim_of_another_source_in_the_room`,
`the_claim_that_plays_does_not_depend_on_the_order_of_the_claims`,
`two_scripts_in_one_room_are_two_music_candidates`,
`the_checksum_sees_each_part_of_a_claim_and_not_its_order` and
`a_boss_music_claim_is_the_same_after_a_rewind` (poison: a resimulated tick
that writes another track; a checksum mismatch with the value probe, none
with the old presence probe). Deferred to its customer: a priority authored
per candidate, when content needs a value between the tiers.
`EncounterEffect::SetMusic` is kept with no shipped customer (Q74: usage is
not worth).

### MIRROR-SYMMETRY — mirrored CPUs stay mirrored per tick — ✅ DONE 2026-10-08

Q49: symmetry is a correctness property. Plan:
[`engine/fighter-brain.md`](engine/fighter-brain.md#mirror-symmetry-is-a-correctness-property-q49).
Two Emmys are one fighter reflected, compared per tick over the full state
(`two_emmys_are_one_fighter_reflected_until_the_first_grab`), until frame 883,
when one grab takes the other: the grab tie is an authored rule
(`two_bodies_grabbing_each_other_on_one_tick_make_one_hold`), and inverting it
swaps each fighter's split exactly. The decision layer has its reflection test.
Each poison of plan item 5 is red: placement and an unmirrored stream in the
match; the zero-lateral `signum`, the floor list order and the left-first
`nearest_support` in their unit witnesses, which the match does not reach.

### LEDGE-OCCUPANCY — two fighters can hold one ledge — ✅ DONE 2026-10-08

Q43 follow-up: Ultimate-like occupancy and trump, deterministic and
rollback-compatible. Plan:
[`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md#ledge-occupancy).
One corner is one edge whatever the bodies' sizes
(`two_fighters_of_different_sizes_on_one_corner_are_one_edge`); occupancy is
derived each tick from the hang, so release and knockoff free the edge, and a
body out of play holds none (`a_body_out_of_play_holds_no_edge`). A rewind
across a trump gives the same holder
(`a_rewind_across_a_ledge_trump_gives_the_same_holder`). The regrab limit
(six per airtime, decaying intangibility, reset by landing or a hit) and the
trumped body's lockout (`CombatRules::ledge_trump_lockout`, Smash 0.5 s) are
built. Not built: Ultimate's "the trumper cannot let go for about 20 frames"
(unverified source).

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence — ✅ DONE 2026-10-08

Q63, Q105. A one-time pickup stays taken through its `Consumed` occurrence
row, and an opened chest through its `Spent` row; both survive a rebuild of
the room, and the save carries them across a load
(`a_one_time_heart_stays_gone_when_its_room_is_built_again`,
`a_load_does_not_build_a_one_time_heart_the_file_remembers_taken`,
`an_opened_chest_is_built_opened_when_its_room_is_built_again`,
`a_load_builds_opened_a_chest_the_file_remembers_spent`, and end to end
through a written save file,
`a_chest_opened_in_play_is_opened_after_its_save_file_is_loaded`). Two
bodies that open one chest on one tick are paid once
(`two_bodies_on_one_chest_in_one_tick_are_paid_once`). A chest authored
open (`ChestSpec::opened`) lowers into the `Opened` marker that play sets.
`InteractableSpec::requires_facing` lowers into `RequiresFacing`, and the
interact road refuses a person or switch the body does not face
(`a_facing_gated_switch_refuses_a_body_that_faces_away`). LDtk authors both
(`opened` on `ChestSpawn`, `requires_facing` on `NpcSpawn` and `Switch`:
contract rows, converter reads, editor definitions in `sandbox.ldtk`;
`an_author_writes_an_opened_chest_and_a_facing_gate`). The inert
`InteractableSpec::enabled` is deleted. Deferred by Q63 itself: the
per-breakable debris cue.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update` — ✅ DONE 2026-10-08

Q129: shared durable world state is peer state, compared by its canonical
form. Every writer of the hashed save runs in the simulation schedule (the
three `persist_*_to_save` mirrors, the dialogue-visit count and the restore
chain, BODY-BORN-ON-THE-TIMELINE). The bag (`OwnedItems`) is in the peer
checksum by its own value (schema 327,
`the_bag_is_compared_by_its_own_value`). Deferred to P6, not before:
`dialog_visits` moves out of the shared save to the participant with the
reactive-character memory model (Q134).

### CAST-FRAMING-TARGET — framing asks for a composition, not only a floor — ✅ DONE 2026-10-08

Q86. A framed cast asks for a view (`camera_snapshot::cast_view_target`):
the authored view scaled so the eased cast box and its margin fill it in the
tighter axis, in both directions. The hard bound
(`CAST_FRAMING_MIN_VIEW_SCALE`, 0.75 of the authored view) clamps that
target; the eased box is the smoothing and the room clamp is downstream.
Witness: `a_cast_that_closes_in_zooms_the_view_in_down_to_the_hard_bound`
(the control is a wide cast, which grows the view as before).

### WEAPON-READINESS — a refused trigger is visible as "not ready" — ✅ DONE 2026-10-08

Q33. `WeaponReadiness` (`Ready`, `Recharging { progress }`, `NoRoom`,
`NoAmmunition`) is the read model of the weapon a body's ranged press
reaches: the fire-rate floor, which keeps the length it was armed with (a
held item arms it with its own spec), or a charge body's fireball spawner,
which keeps the length of its cooldown. Each refusal on either road writes
`RangedFireRefused` with that readiness, and the prompt reads the model.
Plan and witnesses:
[`engine/participant-action-system.md`](engine/participant-action-system.md#p5--weapon-readiness-is-a-semantic-state-q33).

### GATE-PER-ACTOR — a body/capability gate is solid or open for each actor — ✅ DONE 2026-10-03

Q54. A gated wall with a subject form writes a `GatePass` for each body that
satisfies it; `RoomCollision::gates_open_for` is the one rule, and body steps
and the brain's movement and floor queries read the per-body walls. The design
is in [world gating](engine/capability-progression-and-world-gating.md#gate-subject-per-actor-q54).
Witnesses: `a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`,
`a_gate_open_for_a_badnik_does_not_change_the_ground_it_plans_on`,
`a_fighter_over_a_floor_open_for_it_plays_as_over_the_void`. Not ruled:
awareness through an open gate stays on the shared walls.

### BOSS-REPLAY-RETRACTION — a replay that un-defeats a boss un-defeats it for every family — ✅ DONE 2026-10-03

Q51/Q56. One generic retraction for every boss family takes back the
placement, the chest, the mints, the bounty, the quest steps and their payout,
and on the death road every defeat since the checkpoint. The consequence list
is in [boss system](engine/boss-system.md). Witnesses:
`game/ambition_app/tests/boss_replay_retraction.rs`.

### MENU-OVER-DIALOGUE — an overlay opened during a conversation must not end it — ✅ DONE 2026-10-02

Q75. Start no longer ends a conversation (`back` alone closes it); pause, map
and inventory open over it, capture its input (`INVENTORY_CONTEXT` at 160,
above `DIALOGUE` at 150) and close back to `Dialogue`. Map and inventory are two
faces of one overlay (`menu::model::open_overlay_key`). Witnesses:
`start_never_ends_a_conversation_and_back_alone_does`,
`an_overlay_above_the_conversation_captures_its_input`, the per-backend
`start_during_a_conversation_opens_the_{menu,cube}_and_closes_back_to_it`, and
`update_schedule_census::the_conversation_reads_its_input_after_the_inventory_claims_it`.
Residual, read and not measured: under the Grid backend (the web build),
`menu.map` also toggles the standalone map panel
(`handle_map_menu_hotkeys`), which declares no input context.

### ROLLBACK-DEAD-SESSION — an invalidated GGRS session stops the clock in silence — ✅ DONE 2026-10-02

Q138. The harness refuses to step an unhealthy session
(`the_harness_refuses_to_step_an_unhealthy_session`); the rule is in
[headless verification](engine/headless-verification.md#drive-the-real-sim).

### SAVE-DIVERGES-AFTER-RELEASE — a pickup and release in `blink_run` desyncs the sync test on the save — ✅ DONE 2026-10-02

The save mirror read `InCustodyOf` before the projection that derives it.
`DurableHorizonSet` now runs after `ResidencyStep::Project`. Guard:
`every_reader_of_in_custody_of_runs_after_both_derivers`.

### BREAKABLE-SOLIDITY — a solid breakable is a barrier, not a blink wall — ✅ DONE 2026-10-03

Q102. `BlockKind::Barrier` and one solidity mechanism
(`is_full_collision_surface`, 21 readers); a moving platform stays a soft blink
wall. The rule is in [collision](engine/collision-and-ccd.md). Witnesses:
`the_hard_blink_upgrade_does_not_pass_an_unbroken_solid_breakable`,
`the_soft_blink_upgrade_passes_a_moving_platform`.

### BARK-CARDINALITY — a singular bark role has one owner — ✅ DONE 2026-10-03

Q52. A character's barks are the `barks` field of its one catalog row; catalog
assembly refuses a second provider's row for the same id with
`DuplicateCharacter`. Witness:
`a_second_provider_cannot_author_the_barks_of_one_character`. No plural bark
collection exists, because no content asks for one.

### SESSION-EDGE-STATE — a session starts from nothing the last one left — ✅ DONE 2026-10-05

A session that follows another equals a fresh host's first session with the
same save, on every tick from 0: run-time spawns carry the session scope,
channels and per-session state reset at the activation, a session is built
from its own experience's save (and a stale candidate is discarded), and it
begins with its experience's bag. The rules, the witnesses and five named
limits are in [the session edge](engine/construction-and-reconstitution.md#the-session-edge).

### LEVEL-BOX-READERS — a reader of a body's box outside the kernel asks the level box — ✅ DONE 2026-10-06

Every class of level-box read that was a footprint question asks the turned
box now, each with an arm red in turned gravity first; every remaining read
has a class and a reason in `scripts/baselines/level-box-readers.json`, held
by `scripts/check_level_box_readers.py`. The rule, two rejected approaches and
nine open items are in
[one box for a turned body](engine/controlled-character-actor-kernel.md#one-box-for-a-turned-body).

### CRAWLER-HAZARD-FOOTPRINT — a body tests hazards with the footprint it collides with — ✅ DONE 2026-10-06

A body that is not square tested hazards, water, ladders, a ledge carry and a
rebound pad with its level box while its step moved the turned box. Each arm
now states the box it moved and every world read in the step uses it. Witnesses:
`movement/tests/hazard_footprint.rs`, `step_box_world_reads.rs`,
`integration/body_box_tests.rs`. Q160 (water in turned gravity) and the view's
inverse footprint are open in
[one box for a turned body](engine/controlled-character-actor-kernel.md#one-box-for-a-turned-body).

### RIG-LANDMARKS — interactions read authored landmarks — ✅ DONE 2026-10-06

Q41. One landmark query (`BodyLandmarks`), answered by the rig or by the
tables built from each part flipbook (in the content fingerprint); the pet's
hand meets the petted body's authored contact point; the player's shot, a hand
weapon, a held item and its prop read one hand
(`ambition_held_items::holding_hand_world`; `gesture_hand` on the row names it);
a match seat and a re-worn body state the quad their art is drawn at.
Witnesses: `a_pet_hand_meets_the_contact_point.rs`,
`a_fireball_leaves_the_hand.rs`, `a_hand_muzzle_fires_from_the_drawn_hand.rs`,
`a_seat_with_no_sheet_states_no_drawn_quad`,
`a_reworn_body_states_the_quad_it_is_drawn_at`. Open questions: Q158 (the
fireball's flight from the hand) and Q159 (fitting a worn character's art). The
design and its open populations are in
[semantic landmarks](engine/runtime-rigged-sprite-animation.md#semantic-landmarks-q41).

### CALIBRATION-LAB-SHOT — a shot born at chest height in the calibration lab is gone on its first tick — ✅ CLOSED 2026-10-08

The cause is the room. The raider stands 4 px past the right end of a solid
in the lab's collision layer (144..272 by 688..704, the base of the rebound
pad). A shot born 26.6 ahead at chest height has a 24 by 18 box that starts
3.4 px inside that solid's lower corner; the sweep reports a start overlap
(time of impact 0) and the world-hit branch ends the shot on its first step.
The hip-height shot passes under the solid, and `mockingbird_arena` has no
such block. Not a defect of the spawn or of the instrument: a muzzle inside a
wall fires into the wall. Measured with a probe of the world-hit branch (the
hit block's name, box and kind); the comment in
`a_hand_muzzle_fires_from_the_drawn_hand.rs` now says why that room is not
used.

### CHARGE-SPEC-NAME — `SmashChargeSpec` is a generic mechanism — ✅ DONE 2026-10-08 <!-- cite-ok: the retired name this row records -->

Q44: no leaf-game name on a generic API. `SmashChargeSpec` is <!-- cite-ok: the retired name this row records -->
`MoveChargeSpec`, the field `move_charge`, and the payoff multiplier
`charge_mult`, in every source, table (the renderer submodule's George table
too), the exporter's JSON and the moveset inspector (`web/app.js`,
`check_bundle_contract.mjs`). The smash GESTURE (`ChargeGesture::Smash`), the
`smash_charge` animation clip and its SFX ids name the gesture and stay. The
content fingerprint moved; no rollback row did.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity — ✅ DONE 2026-10-03

Two Apps that burned different numbers of local session activations reach the
same canonical identities and checksums. Closed roads: the smash roster seed,
`SessionScopedEntity` in the checksum, the match ordinal, `random_context`,
checkpoint operation keys, the session root's `SimId`, `TransactionId`
provenance, `GameplayElapsed`, the startup-resume checksum, perception's
`Entity` fallback, the GGRS carrier order, the schema fingerprint over
mechanical facts (Q122, `rewording_a_row_leaves_the_fingerprint_alone`), the
session-relative `SimTick` (Q128), the unchecksummed float rows
(`two_peers.rs`, two GGRS peers in one process) and the save handed to its
experience (Q129, `SaveOwner`). Witnesses:
`two_local_histories_compute_the_same_ggrs_component_checksums`,
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
and the arms in `id_peer_audit.rs`. The standing rules are in
[netcode identity rules](engine/netcode.md#identity-rules).

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state — ✅ DONE 2026-10-03

`scripts/check_rollback_mutators_run_in_sim.py` reads every rollback
registration in every supported parameter spelling, including exclusive-world
bodies and one helper hop (`inherited_mutations`). 494 systems, 0 acknowledged
offenders. The save restore chain left the scan with BODY-BORN-ON-THE-TIMELINE;
`reconcile_roster_with_frozen_topology` left when the unread
`ActiveMatch` copy was deleted (schema 302). The guard's limits (`Transform`
excluded by name, Q139; one hop; a run condition is not reachability) are
stated in the script (`PRESENTATION_SHARED`, `BLIND_SPOTS`). Tests:
`scripts/tests/test_rollback_mutators_run_in_sim.py`.

### BODY-BORN-ON-THE-TIMELINE — the simulation applies the save — ✅ DONE 2026-10-03

The restore chain (`adopt_occurrence_checkpoint_from_save`,
`restore_inventory_from_save`, `complete_durable_restore`) runs at the head of
the gameplay root in the simulation schedule, so a rewind past it applies it
again on the same tick. Deleted with the `Update` window: the Q135
session-start gate, `refuse_a_restore_over_a_live_timeline`, the fixture <!-- cite-ok: records a deleted check -->
declaration `TheBodyIsBornOnTheTimeline` and three mutator-guard waivers. Q135
is reopened in the awaiting file. Witnesses:
`the_save_is_applied_by_the_simulation`,
`a_startup_load_is_applied_on_the_timeline_and_resimulates_identically`,
`a_mid_session_load_does_not_reach_back_across_the_rewind`.

### WEAR-REFUSES-UNPREPARED — a character outside the prepared generation is never worn — ✅ DONE 2026-10-03

Q103: refuse. The kit compiler takes a prepared definition (`WornKit::of`);
`WornKit::resolve` and the peaceful-kit fallback are deleted. A re-wear to an
unprepared id writes nothing and puts `WornCharacter` back to
`PersonaBaseline::id`. Witnesses
(`avatar::starting_character::tests`):
`a_rewear_to_an_unprepared_id_keeps_the_previous_character`,
`an_unprepared_id_writes_nothing_on_the_body`. The two remainders (a generation
that drops a worn id; the home body of an unprepared starting id) are open work
in [content generations](engine/content-generation-and-reload.md#open-work).

### THROW-MODIFIERS — route throws through rage and staleness policy — ✅ DONE 2026-10-03

Q133: follow Smash. A set launch declines rage
(`ambition_entity_catalog::launch::launch_speed`). A throw stales the throw
move: `CaptureThrowRequested` carries the use's `move_instance`, damage stales
through `hitbox::staled_damage` and the percent term through
`knockback_stale_scale`; a set throw stales its damage, not its launch.
Witnesses: `a_hurt_captor_throws_farther_and_a_set_throw_is_immune`,
`capture::a_repeated_throw_stales_and_a_neutral_ruleset_leaves_it_whole`,
`a_set_throw_stales_its_damage_and_not_its_launch`,
`a_throw_that_no_playing_use_claims_is_not_staled_or_recorded`.

### CHECKPOINT-ADMISSION-IS-NOT-COMMIT — an accepted restore changes nothing until it commits — ✅ BUILT 2026-10-05

An accepted restore pins its replay and changes no live state; its room is
built from the prospective facts, and its consequences run at the commit
(`RestoreConsequences`). One terminal rule gives every uncommitted operation
`Cancelled { NotCommitted }`; two primary bodies give `AmbiguousSubject`.
Schema 313. Witness: `a_cancelled_restore_changes_nothing` (three refusal
arms, and a committed control). The design, the witnesses and the one open
witness are in
[checkpoint restoration](engine/checkpoint-restoration-protocol.md).

### DEATH-IS-ROOM-LOCAL — a participant's death rewinds its own horizon, not the session's — ✅ DONE 2026-10-07

Q151. Each consequence since the checkpoint names the participants whose
horizons own it (a boss defeat's `present`, `GrantSource::Authored { owners }`,
`WorldTimeSchedule` owners, `ConsumedSinceCheckpoint`; a bag spend is owned by
its object), and a restore takes the dying participant out of each. A New Game
retires every other live room (`retires_beside`). Witnesses:
`boss_replay_retraction.rs`, `death_restores_the_checkpoint.rs`,
`pickup_regrowth_across_rooms.rs`, `a_second_seat_joins_the_session.rs`. The
design summary is in [open-world runtime](engine/open-world-runtime-and-residency.md)
("Death horizon (Q151)"); BAG-RECORD-HORIZON continues it. ⛔ No global
durable rewind followed by reconciliation of surviving rooms; no new uses of
`RoomReplayAdmitted::spared` / `spared_participants` as the model; a
`CheckpointDomainApply` reducer does not write back a row the acceptance pins.

### NPC-UNREGISTERED-CHARACTER — a person who names an unregistered character stopped the game — ✅ DONE 2026-10-05

**Owner:** `construction::preflight_planned_bodies`
(`ambition_platformer2d_actor_monolith`).

**Found** 2026-10-04 on the world reload road: an `NpcSpawn` whose
`character_id` is in no cast and no catalog passed LDtk validation and plan
preparation, and the recipe panicked when the room was built
(`report_unprepared_character`, `actor_spawn/character_spawn_plan.rs`). A
person is a placement row, and the preflight read no placement row. An
`EnemySpawn` with the same fault was already refused when its room was planned.

**Done:** the preflight takes the character catalog and refuses a person in
the one case the NPC road cannot build: the cast does not have the character,
a cast is published, and no catalog row can give it a body. It is the enemy's
refusal (`BodyCharacterNotRegistered`), and the room error names the room and
the character. The three cases the NPC road can build stay plans: a character
that only the catalog has (a borrowed kit), a person who names nobody, and a
composition that published no cast (a warning).

**Witnesses:**
`construction::tests::a_person_who_names_an_unregistered_character_is_refused_when_the_room_is_planned`
(the refusal and the four controls), and on the shipped session
`a_world_reload_that_cannot_rebuild_one_live_room_rebuilds_none`
(`an_edit_reaches_the_shipped_game.rs`): an enemy in the other live room (the
control, refused before and after), a person in the other live room, and a
person in the one live room. Each reload is refused with the room and the
character named, and no live room and no generation changes. Poison: the
refusal removed fails the unit arm, and the two person arms panic as before.

**Not done:** the assertion in `report_unprepared_character` stays. A road
that builds a person with no plan (none is known) would still reach it.

### NEW-GAME-RESYNC — a New Game after durable hydration fails the sync test — ✅ DONE 2026-09-29

A New Game is a checkpoint resume to the fresh baseline at the start room, on
the confirmed-frame road the death restore takes (`resume_at_checkpoint_on_reset`
reads `NewGameRequested`; the fresh-run reducers run inside
`CheckpointDomainApply`). ⛔ Do not register the four `derived` body types for
rollback to hide a rebuild on a speculative frame. Witness:
`a_new_game_asked_for_by_the_host_commits_once_under_a_rewind`.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

A failed candidate world leaves world N playable and unchanged, and N+1 is
verified before N is retired. Every road that changes the authoritative world
crosses its own publication verdict. Owner and witnesses:
[construction and reconstitution](engine/construction-and-reconstitution.md).
⛔ Not part of A10: peer-stable identity (ID-PEER), the defensive
`DepartureAuthority::Custodian` fallback, and the refused-door player signal.

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision — ✅ DONE 2026-09-28

The cutscene dismiss and skip are seat intents on the `ControlFrame`, which
`tick_active_cutscene` reads from `SlotControls`, and `CutsceneSkipHold` is
rollback state. The host-side `Update` request is deleted. Guards:
`a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once` and
`a_cutscene_skip_is_a_hold_of_the_seats_cancel`.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

`ambition_platformer2d_core::rollback_kind::spelling` holds every (kind,
sentence) pair once, and both registration roads reference it. Guard:
`scripts/check_rollback_kind_spelled_once.py`.

### A12 — finish move-contact attribution and reflection identity

✅ DONE 2026-09-19. A late projectile or melee outcome, a reflected shot and an
ability contact cannot credit the wrong move occurrence. An outcome that names
another occurrence, or none, credits no move (ruled: an ability contact is
independent by default). Witnesses: `an_outcome_naming_another_occurrence_credits_no_move`,
`an_unclaimed_outcome_credits_no_move`,
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind`. ⛔ The
frontier's `A12` (flow validation) is a different subject.

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay — ✅ REPAIRED 2026-09-16, acceptance MET; the authority/representation split is DEFERRED

The three live→save mirrors ran in `Update` and wrote a hashed resource that a
replay did not re-derive. They register through `app.sim_schedule()` now. Guards:
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` and
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at`. ⛔ Do not remove
`AmbitionGameSave` from the checksum: most of its writers run inside rewinding
schedules. Deferred: the authority/representation split. Q129 (decided
2026-10-03) keeps shared durable state in what peers compare, by its canonical
semantic form rather than the save's serialization. The bag itself is compared
by its own value (schema 327).

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update` — CLOSED 2026-09-19

Menu presses reach the simulation through `HostIntentLedger<M>`
(`crates/ambition_platformer2d_actor_monolith/src/session/host_intents.rs`),
released at the head of the stamped tick in every pass. Ruled (`Q140`): one frame of stale UI is acceptable. ⛔ Do not add
duplicate authoritative inventory state or optimistic reconciliation to hide it.
Witness: `a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell`.
