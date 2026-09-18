#!/usr/bin/env python3
r"""A resource written from a NEW second file must be adjudicated, not merely land.

⛔⛔ **THE CENSUS EXISTED, PRINTED ITS SHORTLIST, AND NO LANE RAN IT.**
`multi_writer_resource_census.py` answers *"which `Resource`s are written from
more than one file"* — the writer-side shape of a duplicated authority — and its
own tests only ever ran it over hand-built `tmp_path` corpora. So the number it
prints was a number somebody had to go and look at, and nothing noticed when a
resource joined the list. MEASURED 2026-09-17: **102 types** across 1,294
production files.

⚠ **THIS IS A RATCHET ON THE POPULATION, NOT A VERDICT ON IT.** The census
docstring is emphatic that multi-writer is NOT a defect by count, with two
measured cases that came out opposite ways, and nothing here contradicts that. 94
of the 102 are UNADJUDICATED and this check says so on every green run — the split
is printed by the run itself, so read it there rather than from this paragraph.
What the check enforces is that the set and the per-type writer counts cannot
move without somebody editing this file.

⛔⛤ **85 -> 82 ON 2026-09-17, BECAUSE THE CENSUS WAS READING PROSE.** `ResMut<T>`
in a comment is not a writer, and three of the 85 were ENTIRELY an artefact of
one: `AcceptedCheckpointRestore`, `PortalTuning` and `PortalViewer` each had a
single real writer plus a paragraph SAYING SO — *"it was `ResMut<PortalTuning>`
registered"*, *"took `ResMut<AcceptedCheckpointRestore>` and
`ResMut<PendingLifecycleCommit>`"*. Nine more shed a phantom writer file, and the
type population held a `R` and a `_`, from `Res<R>/ResMut<R>` in a doc comment
and `Option<ResMut<_>>` in a line comment. A census parsing prose does not fail;
it invents. ⇒ The worst of it landed in exactly the class this docstring sends
people to FIRST: `AcceptedCheckpointRestore` is rollback-registered, so the
highest-priority shortlist was pointing at a duplication that did not exist.

⚠ **AND THE OTHER HALF WAS THREE WAYS OF NOT SEEING A TEST MODULE AT ALL.** The
shared stripper matched `#[cfg(test)]` then `\s*mod NAME {`, which misses a `///`
between the two (2 sites), a visibility (`pub(crate) mod tests`, 1 site) and any
cfg PREDICATE rather than the bare attribute (`all(test, …)`, 16 sites) — 18
test-only modules read as production. That is how `Captured`, a type the census's
own docstring lists as multi-writer ONLY because a fixture wrote it, got a second
writer, and it is why `CausalRecording` was ratcheted at 7 writers when two of
them were fixtures. ⇒ `#[cfg(any(test, feature = "test-support"))]` is
deliberately still NOT stripped: that module ships when the feature is on, and
over-cutting hides production facts from a census that exists to find them. Both
fixes are in `scripts/lib/`, one owner each: `rust_source.strip_comments` and
`test_paths.strip_test_modules`.

⛔⛔ **82 -> 102 THE SAME DAY, BECAUSE THE CENSUS KNEW ONE OF THE TWO WAYS BEVY
HANDS OUT A MUTABLE RESOURCE.** It read `ResMut<T>` parameters and not
`world.resource_mut::<T>()`. Nineteen types were not on the shortlist at all
(`LocalSessionOwnership`, `ShellRouteCatalog`, `ConstructionSchemaCatalog`, …)
and twenty-one gained writers. ⭐ And the missing files were not a random sample,
which is why this mattered more than the count: an exclusive-world system is what
a COMMIT EXECUTOR is, so the road this shape hid was the destructive one —
`rollback_ggrs/lifecycle_commit.rs` clears `PendingLifecycleCommit` and
`RoomTransitionLoadState`, `session/reset/mod.rs` reaches `AmbitionGameSave`,
`AuthoredOccurrences`, `QuestRegistry` and `GameplayBanner`. **The census was
blind to the systems that SPEND the state it was auditing.**

⚠ A third shape, a `&mut T` PARAMETER, is measured and deliberately NOT counted:
it would take the shortlist to 109, and unlike the other two it is ambiguous —
`fn grant(items: &mut OwnedItems, ..)` may be a second authority or the one
owner's helper, and only the call sites say which. Check it by hand when
adjudicating; the census's `writers` docstring carries the numbers.

⭐ **WHERE TO SPEND AN ADJUDICATION FIRST, AND THE TABLE IS NO LONGER CARRIED BY
HAND.** **At least 37 of the 121 are rollback-registered, and EIGHT of
those are still unadjudicated** — `AuthoredOccurrences` (5 files), `BaseGravity`
(6), `MovingPlatformSet` (4), `OwnedItems` (10), `QuestRegistry` (8),
`RoomTransitionCooldown` (7), `SlotControls` (6), `VersusMatch` (2). That is the
whole remaining shortlist and it is short enough to name, which is the point of
spending verdicts here first. ⚠ This line said "31 of them unadjudicated" for
part of one afternoon and "21 of the 102" the day before; the 37 did not move,
the verdicts did — 23 of the 25 written on 2026-09-18 landed inside this
intersection on purpose. Those are the ones where a second writer
is a divergence rather than a design smell. The 37 is a LOWER BOUND and carries
its instrument: it is the intersection with the type names in `rollback_*::<T>` /
`declare_rollback_derived_*::<T>` turbofish calls across `crates/` and `game/`,
which parses 396 such names where the registry itself holds 491 rows — every row
registered through a non-turbofish form is invisible to it. ⚠ Both numbers moved
(95 -> 396 parsed, 21 -> 37 intersecting) and neither move is growth in the tree:
the earlier parse was narrower. ⇒ Re-derive rather than quote, with:

    python3 - <<'EOF'
    import sys, re, subprocess, pathlib
    sys.path.insert(0, "scripts")
    import check_multi_writer_resources_are_adjudicated as guard
    files = subprocess.run(["git","ls-files","crates/**/*.rs","game/**/*.rs"],
                           capture_output=True, text=True).stdout.split()
    pat = re.compile(r"\b(?:rollback_[a-z_]+|declare_rollback_derived_[a-z_]+)"
                     r"::<\s*(?:[A-Za-z0-9_]+::)*([A-Za-z_][A-Za-z0-9_]*)")
    names = {m.group(1) for f in files
             for m in pat.finditer(pathlib.Path(f).read_text(errors="replace"))}
    print(sorted((set(guard.BASELINE) & names) - set(guard.ADJUDICATED)))
    EOF Joining the writers' WRITE TARGETS narrows it again — which field or
method do two of a type's writer files both reach for:

    python3 scripts/multi_writer_resource_census.py --shared-targets

⛔⛤ **THAT COMMAND EXISTS BECAUSE THE HAND-WRITTEN VERSION OF THIS TABLE WAS
WRONG IN FOUR PLACES, DISCOVERED 2026-09-17 WHILE CITING IT.** It read
`AmbitionGameSave 14 writers` beside a baseline of 17 — the column was a
per-target count wearing the word "writers", so the same word carried two
reference points on one screen. `OwnedItems 4 writers -> grant()` is 2 of 10.
`BaseGravity, RoomTransitionCooldown, VersusMatch -> nothing shared` was true of
one of the three: `RoomTransitionCooldown` shares `remaining`, and both of
`VersusMatch`'s writers replace THE WHOLE RESOURCE with `*state = ..`, which is
the least separable shape there is. What the run prints today, for the types that
already carry a verdict or are named below:

    AmbitionGameSave        18 files  data_mut 14 (14 certain), data 11  ROUTED
    OwnedItems               9 files  grant 2
    QuestRegistry            7 files  push_event 4, quests 2
    PendingLifecycleCommit   7 files  record 2, take 2                   ADJUDICATED
    RoomTransitionCooldown   5 files  remaining 2
    RoomTransitionLoadState  5 files  active 4 (3 certain)
    SlotInteractionState     4 files  primary_mut 2 (2 certain)          ADJUDICATED
    BaseGravity              4 files  nothing shared
    ActiveConversation       2 files  close 2                            ADJUDICATED
    ClockState               2 files  time_scale 2 (2 certain)           ADJUDICATED
    PossessionState          2 files  home 2, possessed 2                ADJUDICATED
    VersusMatch              2 files  *<the resource> 2 (2 certain)

⚠ **"CERTAIN" IS THE HONEST HALF OF A COUNT NO REGEX CAN FINISH.** A call through
a `ResMut` binding may read (`save.data()`) or write (`save.data_mut()`), so the
mode reports both the files that TOUCH a target and the subset whose access is
mutation-shaped. Read the wide number as *where to look* and the narrow one as
*where a write is certain*. Neither is the type's writer count.

⚠ **AND THE OBVIOUS READING OF THE TABLE IS WRONG.** *"They all go through one
method, so it is one authority"* holds for `grant`, which clamps unique items —
the policy is INSIDE the method. It does not hold for `data_mut()`, which hands
out `&mut` to the whole save: that is thirteen authorities with one door. And it
does not hold for a thin setter either: `close()` is `self.live = None`, so what
makes `ActiveConversation` correct is that its two callers fire on DIFFERENT
EVENTS and each has its own witness — which is the `CutRopeBossArenaState` test,
not the accessor count.

⭐ **THE DISCRIMINATOR THAT ACTUALLY SETTLED ONE WAS NEITHER: ASK WHO CAN ARM THE
FACT.** `SlotInteractionState` has four writers over one door, and it is correct
because exactly ONE of them can make a gesture live and the other three can only
clear — so no ordering among them changes what a reader sees. That question
(*which writers can move the value AWAY from its resting state?*) separates the
verdicts below better than the accessor, the field, or the count does.

⇒ **WHAT TO DO WHEN IT REDDENS**, in the census's own words: ask what READS the
fact and whether an ambiguity in it can reach a DECISION; then poison one writer
and run the test that should care. A green poison means nothing tests the fact OR
another writer is covering, and only the second is a finding. If the new writer is
correct, add it here with the reason. If it is not, it is a duplicate authority
that was one commit old when you found it — which is the cheapest moment this
repository ever gets.

⛔ A count that DROPS fails too, and on purpose: a baseline nobody has to lower is
a baseline that stops describing the tree. Lowering it in the commit that removed
the writer is how the number keeps meaning something.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import multi_writer_resource_census as census  # noqa: E402

#: `{type: how many production files write it}`, MEASURED 2026-09-17 at
#: `fb2cde38f` with the corrected test-region cut. The count is part of the baseline because a set alone cannot see
#: a 15-writer resource becoming a 16-writer one, which is the growth that
#: matters most.
#:
#: ⭐ One row has moved since: `SeatRawFrames` 7 -> 6 on 2026-09-18, when the two
#: `drive_slot_frame` bodies (`rollback_ggrs/src/session.rs` and
#: `runtime/src/input_drive.rs`, picked apart by `#[cfg(feature = "rollback")]`)
#: were collapsed onto the lower one.
#:
#: ⚠⛤ **AND THAT IS WHAT A COLLAPSE LOOKS LIKE THROUGH A FILE-GRANULAR
#: INSTRUMENT: SMALLER THAN IT WAS.** Two duplicated arms were removed and
#: exactly ONE row moved. `SlotControls` and `SlotControlLatches` did not,
#: because the same `session.rs` still writes them from `publish_ggrs_input` and
#: `capture_latched_local_input` — the backend's own roads, which were never the
#: duplicate. ⇒ A ratchet on file counts cannot score a de-duplication, and a
#: flat row here is not evidence that nothing was collapsed. The warning this
#: script prints about one-other-FILE versus one-other-WRITER is the same
#: sentence read from the other end.
BASELINE: dict[str, int] = {
    "AmbitionGameSave": 18,
    "GameAssets": 13,
    "GameplayBanner": 11,
    "FeatureEcsWorldOverlay": 10,
    "OwnedItems": 10,
    "CausalRecording": 9,
    "ClassBRemapLog": 9,
    "DeveloperRuntimeState": 9,
    "LoadCoordinator": 9,
    "UserSettings": 9,
    "PendingLifecycleCommit": 8,
    "QuestRegistry": 8,
    "RoomTransitionCooldown": 7,
    "SeatRawFrames": 6,
    "BaseGravity": 6,
    "DeveloperTools": 6,
    "SlotControls": 6,
    "SlotInteractionState": 6,
    "ActiveConversation": 5,
    "AuthoredOccurrences": 5,
    "CaptureProgress": 5,
    "DialogState": 5,
    "HudReadouts": 5,
    "PendingMechanicalEdits": 5,
    "RoomTransitionLoadState": 5,
    "ActiveAudioSelection": 4,
    "AudioLibrary": 4,
    "CharacterLoadDemand": 4,
    "MapMenuState": 4,
    "MenuControlFrame": 4,
    "MintedItemBaseline": 4,
    "MovingPlatformSet": 4,
    "SessionSeatingSource": 4,
    "ShellHostConfiguration": 4,
    "ShellRouteCatalog": 4,
    "ShellRouteHolds": 4,
    "YarnContentBindings": 4,
    "ActiveSessionScope": 3,
    "ActiveUiCues": 3,
    "BossEncounterRegistry": 3,
    "CharacterLoadStates": 3,
    "ConstructionSchemaCatalog": 3,
    "CustodyBaseline": 3,
    "CutsceneAdvanceRequest": 3,
    "CutsceneTriggerQueue": 3,
    "EncounterRegistry": 3,
    "GameplayTraceBuffer": 3,
    "KaleidoscopeCursor": 3,
    "LocalSessionOwnership": 3,
    "MusicPlaybackState": 3,
    "NarrativeMusicRequest": 3,
    "OccurrenceBaseline": 3,
    "PossessionState": 3,
    "PreparedSessionRegistry": 3,
    "SelectCursors": 3,
    "SlotControlLatches": 3,
    "SwitchActivationQueue": 3,
    "Warmup": 3,
    "WorldSourceHotReload": 3,
    "AbandonedCheckpointOperation": 2,
    "ActiveCutscene": 2,
    "ActiveGameplaySession": 2,
    "BodyClocksView": 2,
    "CameraShakeState": 2,
    "ClockState": 2,
    "ContentEpochSequence": 2,
    "ControlFrame": 2,
    "ControlledSubject": 2,
    "CutsceneSkipHold": 2,
    "DefaultMusicStarted": 2,
    "EditablePortalTuning": 2,
    "EncounterView": 2,
    "FallingSandRoomState": 2,
    "FixedStepsTaken": 2,
    "GameplayElapsed": 2,
    "InventoryUiState": 2,
    "KaleidoscopeOpenState": 2,
    "KaleidoscopeScroll": 2,
    "KaleidoscopeSystemNav": 2,
    "LastCutsceneRoom": 2,
    "LastQuestRoom": 2,
    "LeaveRequested": 2,
    "LiveMatchTicks": 2,
    "LocalSeatOffer": 2,
    "LocalSeatTopology": 2,
    "MobileTouchState": 2,
    "MusicDirectorState": 2,
    "MusicIntent": 2,
    "NewGameResetRequested": 2,
    "OwnedItemsBaseline": 2,
    "PortalCameraContinuitySelection": 2,
    "PortalCameraContinuityState": 2,
    "PortalEffectSelection": 2,
    "PortalViewConeDebugDumpRequest": 2,
    "PresentationPhase": 2,
    "ProjectileSeqCounter": 2,
    "RadioStationState": 2,
    "ReservedGameplayScopes": 2,
    "RoomConstructionPlanPrefetch": 2,
    "RoomContentStagingRegistry": 2,
    "SaveRestored": 2,
    "ScrollbarDragState": 2,
    "SeatActiveDevices": 2,
    "SeatControlFrameModes": 2,
    "SeatMenuFrames": 2,
    "SelectPage": 2,
    "SessionMatchOrdinal": 2,
    "SfxBankRegistry": 2,
    "SfxPlaybackState": 2,
    "ShellActivationGates": 2,
    "ShellRouter": 2,
    "ShellSequenceCatalog": 2,
    "ShrineActivationPulse": 2,
    "SmashSelect": 2,
    "StartRequested": 2,
    "StocksMatchSettled": 2,
    "SuddenDeathEntered": 2,
    "VersusMatch": 2,
    "VisualQualityConfirmState": 2,
    "WorldlineHistoryView2d": 2,
    "YarnPresentationCue": 2,
}

#: The ones somebody has actually read. ⚠ An entry here is a CITATION, not an
#: opinion: it names the row or the source contract that owns the answer.
ADJUDICATED: dict[str, str] = {
    "ControlFrame": (
        "CORRECT — TWO HOSTS, ONE DERIVATION, AND THE SOURCE CALLS IT AN OUTPUT. "
        "`mirror_primary_slot_to_control_frame` "
        "(`actor_monolith/src/schedule/input_systems.rs`) is `*frame = "
        "slots.get(PRIMARY)` and nothing else, registered *\"once, for every "
        "host\"* in `PlayerInputSet::Device`; `publish_ggrs_input` "
        "(`rollback_ggrs/src/session.rs`) writes the SAME expression from the "
        "session's confirmed inputs on every GGRS advance. ⇒ Both derive one "
        "value from `SlotControls::PRIMARY`, so they cannot disagree and their "
        "relative order does not matter. ⛔ AND THE THIRD WRITER IS FORBIDDEN "
        "WITH A WITNESS, which is what makes this a verdict rather than a "
        "coincidence: `session.rs` asserts *\"a driver must not write the output "
        "mirror on ANY host\"* under both hosts, because under GGRS this is an "
        "OUTPUT and a driver writing it would feed resimulated input back in as "
        "new input. The input side there is handle zero of `PendingSeatInputs`."
    ),
    "SlotControlLatches": (
        "CORRECT — FIVE SITES, FIVE ROLES, AND THE TWO DESTRUCTIVE ONES CANNOT "
        "BOTH BE INSTALLED. Three files, five sites by `write_sites`. ONE "
        "accumulator from the shaped table: `commit_seat_raw_frames` "
        "(`actor_monolith/src/schedule/input_systems.rs`), which folds every "
        "seat's `SeatRawFrames` row in *\"once the shaping stages have all "
        "run\"*. ONE out-of-band accumulator: "
        "`ambition_platformer2d_runtime::input_drive::drive_slot_frame`, the "
        "driver seam, and since 2026-09-18 the only copy of it. ONE clear that "
        "contributes no value: `populate_seat_control_frames` reaches the table "
        "mutably but its single use is `latches.reset(slot)` on the PAUSED "
        "branch — *\"a seat that has stopped being driven must not hand a held "
        "direction to the tick after the pause\"*. Then TWO destructive drains, "
        "both `latches.take(slot)`: `publish_latched_slot_controls` into "
        "`SlotControls`, and `capture_latched_local_input` "
        "(`rollback_ggrs/src/session.rs`) into `PendingSeatInputs` at "
        "`ReadInputs`.\n"
        "    ⇒ The two drains are the whole question, because whichever ran "
        "first would leave the other a neutral table. They are mutually "
        "exclusive: `install_latched_slot_publication` opens with `if "
        "!app.sim_is_fixed_tick() { return; }`, and a rollback host's sim "
        "schedule is `GgrsSchedule`. ENFORCED BY A SCHEDULE-LABEL COMPARISON, "
        "not by convention.\n"
        "    ⛔⛤ AND THE PREDICATE'S NAME NEARLY COST THIS VERDICT. "
        "`sim_is_fixed_tick()` is `self.is(FixedUpdate)` "
        "(`shared_tangle/src/schedule.rs`), NOT *\"the host steps on a fixed "
        "tick\"* — and a rollback host IS fixed-tick in that second sense. "
        "`latched_input_reaches_the_tick.rs`'s own header says so in as many "
        "words: *\"Fixed60Hz, NOT Rollback. Both are fixed-tick.\"* Read the "
        "way the name reads, both drains install under rollback, "
        "`capture_latched_local_input` empties the table at `ReadInputs`, and "
        "`publish_latched_slot_controls` then writes NEUTRAL over every seat in "
        "`Platformer2dSimulationPhaseMonolith::PlayerInput` — which is "
        "`.in_set(CoreSimulation)`, while `publish_ggrs_input` is "
        "`.before(CoreSimulation)`, so nothing would put the confirmed input "
        "back. That reading predicts total input loss under rollback. It is "
        "wrong, and it is one line of code away from being right.\n"
        "    ⭐ WITNESSED, and the witness had to be built because the shipped "
        "suite could not see it: "
        "`latched_input_reaches_the_tick.rs` accumulates into the latch on the "
        "FRAME clock and never calls the driver helper, so the drain is the only "
        "road. Its header records the poison — all 602 `app_it` tests survived "
        "deleting the installer outright, because `drive_slot_frame` writes "
        "`SlotControls` directly when the composition has no latch, so a test "
        "that introduces input that way cannot witness the seam either way."
    ),
    "EncounterRegistry": (
        "CORRECT — ONE BUILDER AND TWO LIFECYCLE WIPES, one per lifecycle fact. "
        "`populate_encounter_registry` (`ambition_encounter_features/src/systems.rs`) "
        "is the only system that fills the `id -> Entity` index; "
        "`process_new_game_reset_request` clears it on New Game from inside the "
        "rewinding schedule's `ResetProcessing`; `SESSION_SCOPE_RESET` clears it at "
        "the session edge. ⭐ The crate's own plugin doc states the ownership this "
        "rests on — *\"the crate owns its `id -> Entity` index ... Live encounter "
        "state stays on the encounter ENTITIES\"* — so the registry is an index "
        "rather than authority, and an index with one builder is not a duplicated "
        "authority."
    ),
    "WorldlineHistoryView2d": (
        "CORRECT — A TELEMETRY VIEW WITH ONE PUBLISHER, ONE PRUNER AND ONE SESSION "
        "SEED. `publish_worldline_history` and "
        "`clear_worldlines_without_live_spacetime` are both in "
        "`ambition_relativity2d/src/telemetry.rs` — publish and prune of one view, "
        "in the crate that owns it — and `install_twintrack_session` "
        "(`game/ambition_demo_twintrack`) seeds it when the demo installs its "
        "session. ⇒ A view is not authority: nothing simulates from it. ⚠ Read as "
        "TWO files by the census and THREE sites by `write_sites`, which is the "
        "distinction that matters here — the two telemetry systems are the pair, "
        "and they are in one module by design."
    ),
    "SwitchActivationQueue": (
        "CORRECT — ONE PRODUCER, ONE CONSUMER, AND IT IS A CROSS-TICK CHANNEL BY "
        "CONSTRUCTION. `apply_switch_effects` (`features/ecs/effect_bus.rs`) "
        "PUSHES, inside `Platformer2dSimulationPhaseMonolith::GameplayEffects`; "
        "`drain_switch_activations` (`ambition_encounter/src/switches.rs`) "
        "`std::mem::take`s the whole queue, `.in_set(SwitchActivationDrained)`; the "
        "third writer is `SESSION_SCOPE_RESET`. One pusher, one taker, one "
        "session-edge clear.\n"
        "    ⚠ `SwitchActivationDrained` is never `configure_sets`'d into any "
        "phase, so *\"nothing orders the drain against the producer\"* is literally "
        "true — and I first wrote that down as *\"executor order, stable per build "
        "and arbitrary\"*, which is WRONG and is the correction worth keeping. The "
        "drain's position is pinned by its CONSUMERS: `drive_wave_encounters` is "
        "`.in_set(EncounterSimulation).after(SwitchActivationDrained)`, and the "
        "phase chain is `... EncounterSimulation -> Cutscene -> GameplayEffects -> "
        "Progression`. ⇒ The drain must precede a system two phases BEFORE the "
        "producer, so an activation is always resolved on the FOLLOWING tick. "
        "Deterministic, peer-stable, and forced.\n"
        "    ⛔ AND THE OBVIOUS REPAIR IS A SCHEDULE CYCLE. Adding "
        "`.after(apply_switch_effects)` would put the drain after "
        "`GameplayEffects` and before `drive_wave_encounters` in "
        "`EncounterSimulation`, which is earlier in the same frame. The one-tick "
        "delay is the price of the phase order, not a missing edge.\n"
        "    ⭐ MEASURED AND PINNED: "
        "`a_switch_activation_is_drained_on_the_tick_after_it_was_pushed` "
        "(`game/ambition_app/tests/symmetry_attunement.rs`) reads "
        "`(queued, resolved) == (1, 0)` one step after a real `SwitchActivated` "
        "through the shipped composition, and `(0, 1)` the step after — so the "
        "delay is a DELAY and not a loss. ⚠ This is NOT the ordering gap "
        "`switches.rs` records at line 437: that one is two writers of the SAVE's "
        "switch family (`drain_switch_activations` vs "
        "`content/src/falling_sand_sim.rs`), routed to "
        "`world-facts-observations-and-memory.md`, and it is still open."
    ),
    "NewGameResetRequested": (
        "OPEN — `queue.md`'s MENU-RESET-MIDSESSION row owns it and the row's "
        "measurement is the verdict: a New Game asked for from outside the "
        "simulation commits ZERO times, because this resource is "
        "`resource-canonical` and the restore returns the flag to `false` before "
        "`process_new_game_reset_request` (which runs INSIDE the rewinding "
        "schedule) ever sees it. ⇒ The two writers are the row's subject: "
        "`process_new_game_reset_request` consumes it in-schedule, and the menu "
        "arms it from `Update` through `SystemMenuParams::request_reset`. ⚠ The "
        "census reports the menu side as `<param bundle: SystemMenuParams>` rather "
        "than as a system, and that is the honest answer — the writers are "
        "whichever systems take that bundle, which is why the bundle's own doc "
        "names its two consumers. Blocked on Q136 (how does a local menu intent "
        "enter the synchronised timeline), not on anything this guard can settle."
    ),
    "OccurrenceBaseline": (
        "CORRECT — ONE WRITER PER LIFECYCLE EVENT, AND THE EVENTS ARE DISJOINT. "
        "Measured per system 2026-09-18: CAPTURE is `capture_occurrence_baseline` "
        "on `CheckpointCommitted` (`shared_tangle/src/lifecycle/continuity.rs`); "
        "ADOPT is `DurableHorizon::install`, whose doc says *\"Called by ADOPTION "
        "and by nothing else\"*, reached from `adopt_the_ledger` / "
        "`adopt_occurrence_checkpoint_from_save`; RESET is "
        "`reset_occurrence_horizon_on_new_game` on New Game plus "
        "`SESSION_SCOPE_RESET` at the session edge. ⇒ Four writer functions, four "
        "different lifecycle facts, none of them able to fire on another's event. "
        "That is not two owners of one fact; it is one fact with four stated "
        "transitions. ⚠ The `Update` placement of the adopt road is a separate "
        "and OPEN question — `queue.md`'s DURABLE-HORIZON-CHECKSUM row and Q135 — "
        "and this verdict is about authority, not about schedule."
    ),
    "CustodyBaseline": (
        "CORRECT — THE SAME FOUR TRANSITIONS AS `OccurrenceBaseline`, WITH ITS OWN "
        "CAPTURER. `capture_custody_baseline` "
        "(`shared_tangle/src/lifecycle/custody_horizon.rs`) on `CheckpointCommitted`; "
        "`DurableHorizon::install` on adoption; "
        "`reset_occurrence_horizon_on_new_game` on New Game; `SESSION_SCOPE_RESET` "
        "at the session edge. ⭐ One reducer per mechanical domain is the rule the "
        "source states at `reset_occurrence_horizon_on_new_game`, which records "
        "that the INVENTORY subsystem briefly reset both of these baselines — *\"six "
        "domains' reset details known to one subsystem\"* — and was corrected. ⚠ "
        "Its `Update` adopt road is Q135's, as above."
    ),
    "MintedItemBaseline": (
        "CORRECT — THE ITEM DOMAIN'S COPY OF THE SAME FOUR TRANSITIONS, ADOPTED BY "
        "ITS OWN DOMAIN ON PURPOSE. `capture_minted_item_baseline` on "
        "`CheckpointCommitted`; `DurableHorizon::install` on adoption; "
        "`restore_inventory_from_save` and `reset_inventory_on_new_game` in "
        "`items/persist.rs`; `SESSION_SCOPE_RESET` at the session edge. ⭐ The "
        "domain-local adoption is a RECORDED correction, not an inconsistency: "
        "`minted_horizon.rs` says the item baselines are adopted in one function "
        "because `OwnedItemsBaseline` *\"once joined capture, restore and rollback "
        "but silently missed durable adoption\"*, and keeping them together makes "
        "that omission local to the domain rather than a fifth cross-crate census."
    ),
    "OwnedItemsBaseline": (
        "CORRECT ON AUTHORITY, AND IT SURFACED A CHECKSUM ASYMMETRY THAT IS NOT "
        "THIS GUARD'S TO RULE ON. Authority first: three writer functions, three "
        "events — `capture_owned_items_baseline` on `CheckpointCommitted`, and "
        "`restore_inventory_from_save` / `reset_inventory_on_new_game` in "
        "`items/persist.rs` on the load and New Game roads.\n"
        "    ⚠ IT IS THE ONE CHECKPOINT BASELINE OF FOUR THAT IS **NOT** IN "
        "`SessionScopedResources`, and that is consistent rather than an omission: "
        "`OwnedItems` itself is not session-scoped either (measured — the bag does "
        "not appear in `session/teardown.rs` at all), so the baseline travels with "
        "the value it baselines. The three that ARE reset describe WORLD PLACEMENT, "
        "and the teardown's own comment gives that reason: *\"a checkpoint baseline "
        "from the previous session is a baseline for a world that no longer "
        "exists\"*. ⛔ But that reason is written for *\"the same three facts\"* and "
        "says nothing about the fourth, so the exclusion is currently a DEFAULT "
        "rather than a decision.\n"
        "    ⛔⛤ **AND THE ASYMMETRY WORTH A RULING IS THE CHECKSUM ONE.** "
        "`OwnedItems` is `rollback_resource_clone` — restored, NOT in the peer "
        "checksum, and unhashed by KIND rather than by any stated decision (its "
        "registration in `ambition_items/src/rollback_registration.rs` carries no "
        "reason). `OwnedItemsBaseline` wraps that same `OwnedItems` and is "
        "`rollback_resource_clone_checksum`, projecting `to_persisted()` rows. ⇒ "
        "The player's stored quantities are OUT of the peer contract as the bag and "
        "IN as its baseline, and the first `CheckpointCommitted` copies the live "
        "value across that line. Nothing can observe it today because only "
        "`SyncTestSession` is ever constructed — one peer replaying itself, whose "
        "two save files are the same file. Routed to "
        "`docs/planning/awaiting-maintainer-decision.md`'s Q129, which asks "
        "exactly whether a save file belongs in what two peers agree on."
    ),
    "FeatureEcsWorldOverlay": (
        "CORRECT — ONE REBUILDER, EIGHT CONTRIBUTORS AND A FIELD SPLIT THE "
        "COMPILER ENFORCES. `rebuild_feature_ecs_world_overlay` "
        "(`actor_monolith/src/world/overlay.rs`) calls "
        "`clear_engine_contributions` in `FeatureWorldOverlaySet`, and every "
        "contributor carries an explicit `.after(FeatureWorldOverlaySet)` edge — "
        "verified per registration 2026-09-18: `contribute_encounter_lock_walls` "
        "and `sync_authored_gated_lock_walls` (`runtime/src/world_gating.rs`), "
        "`gate_gnu_ton_arena_ladder`, `project_particles_to_movement_world`, "
        "`project_settled_sand`, `contribute_broken_bricks_to_overlay`, "
        "`contribute_discovered_hidden_blocks_to_overlay`, "
        "`contribute_broken_monitors_to_overlay`. ⇒ Without that edge a "
        "contribution is wiped by the clear depending on set order, so the edge "
        "IS the authority argument, and all eight state it.\n"
        "    ⭐ THE NINTH WRITER IS THE INTERESTING ONE AND IT NEEDS NO SUCH "
        "EDGE. `bridge_portal_carves` owns `portal_carves`, the ONE field "
        "`clear_engine_contributions` deliberately does not clear — its body says "
        "*\"NOT OURS ... clearing it here would race that and blink the "
        "aperture depending on system order\"*. Two owners over DISJOINT field "
        "sets, both single-authority.\n"
        "    ⭐⭐ AND THE SPLIT IS MECHANICAL, WHICH IS WHY THIS IS A VERDICT AND "
        "NOT A HOPE: `clear_engine_contributions` destructures `Self` with NO "
        "`..`, so a seventh field fails to compile (E0027) and lands its author "
        "at the question *\"engine-owned or contributor-owned?\"*. That is the "
        "shape every other many-writer resource here should be measured against."
    ),
    "ClassBRemapLog": (
        "CORRECT — AND IT IS THE CASE WHERE MANY WRITERS ARE THE DESIGN, ENFORCED BY "
        "THE TYPE. Nine files write it and that is the contract: "
        "`ambition_platformer2d_shared_tangle/src/class_b.rs` holds a PRIVATE "
        "`entries: Vec<ClassBRemapEntry>` whose only `&mut self` methods are "
        "`record(body, kind)` and `clear()`, so a Class-B writer cannot reach the "
        "ledger any other way — measured 2026-09-18, the impl has exactly those two. "
        "⇒ Nine appenders and one clearer is not nine authorities; it is one "
        "append-only ledger with nine reporters. ⭐ AND THE MULTIPLICITY IS THE "
        "SUBJECT OF ITS OWN ORACLE: `contentions()` reports every body that took two "
        "or more remaps in one frame, which is §6.1 invariant 5's violation shape, "
        "and it scans the append-ordered `Vec` rather than a hash container (ADR "
        "0023). The one clearer, `clear_class_b_remap_log`, is registered "
        "`.before(Platformer2dSimulationPhaseMonolith::CoreSimulation)` in "
        "`GameplaySimulationRoot` (`ambition_platformer2d_runtime/src/lib.rs`), which "
        "is what makes the frame scope real. ⭐ ITS `declare_rollback_derived_resource` "
        "REASON IS TRUE, which is worth saying because `AuthoredOccurrences`'s was "
        "not: *\"frame-local diagnostic ledger cleared before every simulation "
        "step\"* — the clear is upstream of every writer, so a replayed frame "
        "rebuilds the whole value. ⚠ One writer reads as a bundle rather than a "
        "system, `<param bundle: TransitBodies>` in `room_transition/commit.rs`; "
        "that is the census reporting honestly, not a hidden writer."
    ),
    "ControlledSubject": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `resolve_controlled_subject` (`abilities/traversal/possession.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "CutsceneSkipHold": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `apply_menu_frame_to_cutscene_request` (`schedule/input_systems.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "EncounterView": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `apply_wave_encounter_effects` (`ambition_encounter_features/src/systems.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "GameplayElapsed": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `advance_gameplay_elapsed` (`features/mod.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "LastCutsceneRoom": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `auto_trigger_room_cutscenes` (`cutscene.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "LastQuestRoom": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `push_room_entered_quest_events` (`quest/mod.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "LiveMatchTicks": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `count_the_live_match_ticks` (`character_runtime/live_match_clock.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "SaveRestored": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `complete_durable_restore` (`session/durable_horizon.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "SessionMatchOrdinal": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `activate_the_prepared_match` (`character_runtime/match_activation.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "StocksMatchSettled": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `decide_stocks_match` (`features/stocks_match.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "SuddenDeathEntered": (
        "CORRECT — ONE IN-SESSION OWNER PLUS THE SESSION BOUNDARY, and the second "
        "\"writer\" is not an authority. `decide_stocks_match` (`features/stocks_match.rs`) is the "
        "only production system that writes it inside a session; the other file is "
        "`SESSION_SCOPE_RESET`, where `SessionScopedResources::reset` returns it to "
        "its default at the session edge. MEASURED 2026-09-18 per SYSTEM rather than "
        "per file: exactly one `ResMut`/`resource_mut` site in that file, in that "
        "one function, with comments and test modules stripped. ⇒ Nothing here is "
        "two owners of one fact."
    ),
    "ProjectileSeqCounter": (
        "CORRECT — ONE MINTING IMPLEMENTATION REACHED BY TWO REGISTRATIONS, AND THE "
        "FILE-GRANULAR CENSUS CANNOT SEE THAT. It reports 2 files; per SYSTEM there "
        "are THREE sites in `ambition_projectiles/src/materialize.rs` — "
        "`materialize_projectiles_for_this_tick`, `materialize_projectiles_for_next_tick` "
        "and the private `materialize_matching` both delegate to, which is the one "
        "place a sequence number is minted. ⭐ AND THE ORDER IS EXPLICIT: both "
        "registrations sit in one `.chain()` in `CombatSet::Materialize` "
        "(`ambition_platformer2d_runtime/src/combat_schedule.rs`), `this_tick` before "
        "`next_tick`. ⛔ THE DETERMINISM IS A FREE RIDER ON THAT CHAIN AND THE CHAIN "
        "DOES NOT SAY SO: every comment justifying it argues about the despawn window "
        "and `step_projectiles`, not about the counter. This resource is "
        "`rollback_resource_canonical` and its own registration calls it *\"the "
        "deterministic id source\"*, minting `ProjectileSeq` on every bolt — so "
        "unchaining these two for an unrelated reason makes two peers mint different "
        "ids for the same shots. That is the `RollbackOrdered` defect class. The third "
        "writer file is `SESSION_SCOPE_RESET`."
    ),
    "ActiveCutscene": (
        "CORRECT — AN OPENER AND AN ADVANCER WITH DISJOINT PRECONDITIONS AND AN "
        "EXPLICIT CHAIN. Per SYSTEM, not per file: `drain_cutscene_triggers` starts a "
        "cutscene and returns early on `active.is_playing()`; `tick_active_cutscene` "
        "advances one and does nothing when none is playing. Both are in "
        "`(auto_trigger_room_cutscenes, drain_cutscene_triggers, tick_active_cutscene)"
        "`.chain()` in the `Cutscene` phase (`cutscene.rs`), so the tick a freshly "
        "started cutscene first advances on is stated rather than left to set order. "
        "The other writer file is `SESSION_SCOPE_RESET`. ⚠ Owed a poison: no arm has "
        "been shown to fail on removing either write, and "
        "`ending_a_cutscene_records_that_it_was_seen` exists because deleting a "
        "different cutscene write left every cutscene test green."
    ),
    "AmbitionGameSave": (
        "OPEN — AND THE REASON IT IS OPEN CHANGED, 2026-09-18. The verdict here "
        "used to be \"the save mirrors disagree with their own rollback replay\", "
        "and that defect is REPAIRED: the three `persist_*_to_save` mirrors and "
        "`count_the_dialogue_visit_when_a_conversation_opens` all register "
        "through `app.sim_schedule()`, the divergence set is empty, and "
        "`resources_crossing_the_rewind_boundary.py` reports this type DOES NOT "
        "CROSS the rewind boundary. ⇒ The 18 writer FILES are no longer a "
        "rollback finding at all; what keeps this unadjudicated is the split "
        "`queue.md`'s ROLLBACK-BAG-DESYNC row records as deliberately deferred — "
        "is `AmbitionGameSave` both simulation authority and disk "
        "representation? Until that is answered, 18 files writing one resource "
        "is the shape of the question, not the answer. ⛔ Do NOT read this as "
        "settled because the checksum is quiet: 19 `ResMut<AmbitionGameSave>` "
        "parameters in 17 production files is the widest shared write in the "
        "tree."
    ),
    "CutsceneAdvanceRequest": (
        "OPEN — CUTSCENE-ROLLBACK-DECISION item 1: produced on the HOST side in "
        "`Update` and consumed with `std::mem::take` inside the sim schedule, so "
        "a dismiss press is lost across a rewind. Held by a failing-by-design "
        "witness."
    ),
    "ActiveConversation": (
        "CORRECT — ONE OPENER AND FOUR END CONDITIONS, each on a different event "
        "and each with its own witness. `conversation/opening.rs` is the only "
        "writer that STARTS one. `rules.rs` closes on knockback, separation or a "
        "despawned participant; `ui_bridge.rs` closes on a stamped "
        "`ConversationEnded` input whose instance matches; "
        "`runtime/room_transition/commit.rs` closes when the room the "
        "conversation lives in is replaced; `session/teardown.rs` replaces the "
        "whole resource as part of `SessionScopedResources::reset`. POISON-"
        "VERIFIED 2026-09-17 in both directions for the first pair: removing "
        "`rules.rs`'s close fails "
        "`a_conversation_breaks_on_knockback_or_on_the_bodies_separating`, "
        "removing `ui_bridge.rs`'s fails three arms including "
        "`a_rewind_past_the_end_replays_it_at_the_same_tick`. Neither hides the "
        "other's absence, which is the `CutRopeBossArenaState` test. "
        "⛔⛤ **AND THIS ROW SAID \"TWO END CONDITIONS\" UNTIL A REVIEW FOUND THE "
        "CENSUS BLIND TO `ResMut<'w, T>`.** The opener, the room-transition close "
        "and the teardown reset were never examined, because a `DialogueDispatch` "
        "`SystemParam` bundle is how three of the five spell their access — and a "
        "bundle is precisely the shape shared access takes.\n"
        "    ⛔⛤ THE ROOM-TRANSITION CLOSE IS POISONED AND THE POISON PASSES — "
        "MEASURED 2026-09-18. Deleting `self.conversation.close()` from "
        "`RoomTransitionFinalize::apply_crossing` "
        "(`ambition_platformer2d_runtime/src/room_transition/commit.rs:557`) leaves "
        "**810 arms green**: `app_it` 717 passed / 0 failed / 47 ignored in 580s, "
        "plus 93 in `ambition_conversation` and `ambition_platformer2d_runtime`. ⇒ "
        "Nothing witnesses it, and a poison that PASSES is a finding about the "
        "ARM, not a clean result.\n"
        "    ⭐ WHY IT PASSES IS THE ARCHITECTURAL PART: **THREE ROADS END A "
        "CONVERSATION WHEN ITS ROOM IS REPLACED, AND ONLY ONE IS IMMEDIATE.** (1) "
        "this direct `close()`, on the crossing tick; (2) "
        "`break_dialogue_on_hit_or_separation`, whose own comment calls a room "
        "swapping under a conversation *\"a separation of the most literal "
        "kind\"*; (3) `publish_the_narrative_end` -> "
        "(⛔ this cited \"stamp conversation end when the box closes\", which is that "
        "function's DOC SENTENCE and not an identifier — caught by "
        "`test_every_system_a_verdict_names_exists`) "
        "`close_conversation_on_narrative_end`, because the line ABOVE the poison "
        "closes `DialogState`. Roads 2 and 3 are reactive and land a tick or more "
        "later. ⇒ Do NOT read the passing poison as licence to delete the close: "
        "it is the only one that closes on the same tick, and the window it "
        "removes is one in which the authority names two despawned bodies and "
        "`HeldByConversation` still holds them.\n"
        "    ⛔ AND THE NAIVE WITNESS MEASURES ROAD 3, NOT ROAD 1. Seating a "
        "conversation from a test and walking into a REFUSED room finds it closed "
        "every third frame — one sim tick — with `apply_crossing` never running "
        "(its first line sets `preset_flash = 1.0`, and the probe watched that "
        "value DECAY). A seated conversation has no Yarn node, so its box never "
        "opens, so road 3 ends it. Held as "
        "`probe_what_closes_a_seated_conversation_while_the_room_transaction_is_refused` "
        "in `game/ambition_app/tests/walking_into_a_loading_zone.rs`, print-only, "
        "with the frame table. ⇒ STILL OWED: a witness with two live bodies and a "
        "shipped Yarn node, so roads 2 and 3 are quiet while road 1 is under "
        "test. `commit.rs`'s close is uncovered until then, and now it is "
        "uncovered ON THE RECORD."
    ),
    "PossessionState": (
        "CORRECT — one protocol deliberately split across two systems, and the "
        "source says so at the seam: `release_possession` clears `possessed` and "
        "*\"`state.home` is deliberately NOT cleared here\"*, because the seat "
        "still has to travel back; `project_driving_participant` clears `home` "
        "once it has retracted the vacated seat and restored the home one. "
        "POISON-VERIFIED 2026-09-17: deleting `state.home = None` from the "
        "production half fails "
        "`releasing_the_possession_returns_the_seat_to_the_body_that_owns_it` "
        "and nothing else in 1,207 arms."
    ),
    "ClockState": (
        "CORRECT — three policies over one smoothed value, and the coupling is "
        "witnessed. `smooth_sim_clock_toward_target_system` ramps toward the "
        "target, `apply_clock_reset_requests` snaps to 1.0 through the permission "
        "table, and `apply_suspended_time_scale_system` forces 0.0 — and that "
        "third one writes `RequestedClockScale::sim_clock` TOO, precisely so the "
        "smoother cannot ramp back up underneath it. POISON-VERIFIED 2026-09-17: "
        "dropping the target write fails "
        "`suspended_frame_zeros_world_time_scaled_dt` "
        "(`actor_monolith/src/time/time_control/tests.rs`), which is the arm that "
        "makes this a verdict rather than an opinion."
    ),
    "PendingLifecycleCommit": (
        "CORRECT — ONE EARLIEST-STICKY SLOT WITH A STATED PRIORITY LADDER. Five "
        "production systems can ARM it and all five go through `record`, whose "
        "policy is inside the method (earliest wins, idempotent under resim) and "
        "whose `#[must_use]` says a refusal must not have its consequences run. "
        "Every armer honours that. ⇒ The resolution order is not ambiguous: it is "
        "a `.chain()`ed phase order plus one explicit edge, MEASURED 2026-09-17:\n"
        "      resume_at_checkpoint_on_reset         CheckpointRestore      PlayerInput\n"
        "      admit_room_replay                     RoomReplayAdmission    PlayerInput, "
        "after CheckpointRestore (`checkpoint_horizon.rs:45`)\n"
        "      restore_checkpoint_on_session_start   ItemPickupSet::CoreHeldItems  "
        "PlayerSimulation\n"
        "      detect_room_transition_system         RoomTransitionSet::Detect     "
        "RoomTransition\n"
        "    with `(PlayerInput, WorldPrep, PlayerSimulation, RoomTransition, ..)"
        "`.chain()` at `schedule/schedule.rs:91`. So a checkpoint resume outranks a "
        "replay admission outranks a session-start restore outranks a door, within "
        "one tick, deterministically and peer-stably. The clearers cannot conflict "
        "with it: `take()` from the two commit executors and "
        "`retract_transition_for_subject` from `open_death_interlude`, all after "
        "the operation they end, plus the `SessionScopedResources::reset` road in "
        "`session/teardown.rs` (an eighth writer, visible once the census learned "
        "`ResMut<'w, T>`). "
        "⛔⛤ AND THE DOOR ARMER WAS SPENDING THE PLAYER'S PRESS BEFORE ASKING: it "
        "cleared the interact buffer and then discarded the `Admission`, so a TAP "
        "was consumed on a crossing this slot REFUSED. Found by review 2026-09-17 "
        "and fixed — admit first, spend second — witnessed by "
        "`a_door_refused_the_lifecycle_slot_keeps_the_press_it_could_not_spend`. "
        "⇒ `#[must_use]` on `record` is not enough on its own: `let _ = "
        "pending.record(..)` compiles, and that is exactly what the door wrote. "
        "⭐ AND THE ONE CONTENTION THAT IS REACHABLE IN PRACTICE IS ALREADY "
        "WITNESSED, with its own recorded negative: "
        "`a_save_with_a_checkpoint_and_an_occurrence_lands_both` "
        "(`app/tests/canonical_reconstitution.rs`) pins the two checkpoint roads "
        "racing for this slot, and its docstring records that poisoning EITHER "
        "road alone leaves it green — they are redundant, not cooperating — so "
        "the poison that reddens it is both at once. "
        "⚠ WHAT IS OWED IS PROSE, NOT A FIX: the ladder above is EMERGENT, "
        "assembled from four `in_set` declarations in four crates, and no single "
        "page states it. A reader asking *what happens if a player dies in a "
        "doorway on the frame a checkpoint resumes* has to rebuild this table "
        "from the schedule to find out."
    ),
    "CutsceneTriggerQueue": (
        "CORRECT BY COINCIDENCE — CUTSCENE-ROLLBACK-DECISION item 2: every "
        "producer happens to sit inside the sim schedule, so a replay "
        "re-produces what a rewind dropped. The invariant is ratcheted by "
        "`check_sim_consumed_request_writers.py`, not by this baseline."
    ),
    "SlotInteractionState": (
        "CORRECT — ONE ARMER, FIVE CLEARERS. Every production call that can make "
        "a gesture live is in `control/input_systems.rs`: `buffered_interact`, "
        "`register_down_tap`, `register_up_tap`, `held_up_interact` and the one "
        "`double_tap_down_pending = true`. MEASURED 2026-09-17 by grepping the "
        "arming methods across `crates/` and `game/` — there are no other "
        "production call sites. Every other writer can only move a row TOWARD "
        "its resting state: `world/rooms/systems.rs` clears row zero once a "
        "crossing is ADMITTED; `control/acting.rs`'s `ActingParticipant::"
        "consume_interact` clears the ACTING body's slot for the chest and the "
        "NPC/switch loops; `body_mode/mechanics/mod.rs` `mem::take`s "
        "`double_tap_down_pending`, a different field; "
        "`runtime/src/sandbox_reset.rs` hands `primary_mut()` to `reset_sandbox`, "
        "which calls `SlotGestures::reset()`; and `session/teardown.rs` replaces "
        "the whole resource as part of `SessionScopedResources::reset`. ⇒ Row "
        "zero has three writers and they cannot disagree: a clear is idempotent "
        "and none of them can arm, so no ordering between them changes what a "
        "reader sees. "
        "⛔⛤ **THIS VERDICT WAS BANKED AT FOUR WRITERS AND CORRECTED TWICE THE "
        "SAME DAY, WHICH IS THE PART WORTH KEEPING.** First the narrowing table "
        "it cited was hand-carried and wrong about `get_mut()`; then a review "
        "found the census blind to `ResMut<'w, T>`, so `control/acting.rs` and "
        "`session/teardown.rs` had never been examined. Both turned out to be "
        "clearers and the argument survived — but it survived by luck, not by "
        "having been checked, and a verdict that names 4 of 6 writers is not an "
        "adjudication. ⇒ Re-read the writer list from the run before trusting any "
        "row here. "
        "⛔ POISONED 2026-09-17 AND THE POISON PASSED: deleting the row-zero clear "
        "left 1,207 crate arms and all 11 room-transition integration arms green, "
        "because every authored door arm HOLDS interact for thirty frames and the "
        "armer refills the buffer underneath it. Witnessed now by "
        "`a_door_crossing_consumes_the_buffered_press_rather_than_letting_it_"
        "decay` and, for the refusal case the first arm could not see, "
        "`a_door_refused_the_lifecycle_slot_keeps_the_press_it_could_not_spend` "
        "(both `actor_monolith/src/world/rooms/tests.rs`)."
    ),
}

#: `{session world component: production writer files}`, MEASURED 2026-09-17.
#:
#: ⛔⛤ **A SECOND POPULATION, BECAUSE A10 MOVED SESSION STATE OUT OF RESOURCES
#: AND THE INSTRUMENT DID NOT FOLLOW.** These are components on the session root,
#: reached through `SessionWorldMut<T>`, and none of them carries
#: `#[derive(Resource)]` — so the resource census above is silent about them by
#: construction. Four have more than one production writer and one has EIGHT.
#:
#: ⚠ It is ratcheted SEPARATELY rather than folded in, because the question is
#: not the same one: a session world component's lifetime is the SESSION's, so
#: two writers is a claim about one session's state and not about the App's. The
#: discriminator this census leads with — which writers can move the value away
#: from its resting state — still applies; what changes is that a session
#: boundary reclaims the whole thing, so a stale write cannot outlive it.
SESSION_WORLD_BASELINE: dict[str, int] = {
    "EncounterMusicRequest": 8,
    "LdtkRuntimeIndex": 2,
    "RoomGeometry": 2,
    "RoomSet": 2,
}

#: ⛔ Its own floor, for its own reason: this population is small enough that a
#: broken scan and a clean tree look identical. Eight types carry the accessor
#: today; a reading below four is the instrument, not the tree.
MIN_SESSION_WORLD_TYPES = 4

#: The session-scope reset, which is ONE road and a known one.
#:
#: ⭐ **NAMING IT IS NOT WAIVING IT.** `SessionScopedResources::reset` replaces
#: every session-scoped resource with its default at a session boundary, from one
#: function, so it appears in this census as a writer of thirty of the 121 —
#: MEASURED 2026-09-17 — and thirteen of those would be SINGLE-writer without it
#: (`ActiveCutscene`, `ControlledSubject`, `CutsceneSkipHold`, `EncounterView`,
#: `GameplayElapsed`, `LastCutsceneRoom`, `LastQuestRoom`, `LiveMatchTicks`,
#: `ProjectileSeqCounter`, `SaveRestored`, `SessionMatchOrdinal`,
#: `StocksMatchSettled`, `SuddenDeathEntered`). ⇒ Whether that MEMBERSHIP is right
#: is a live question with its own owner —
#: `check_session_owner_census_matches_source.py` ratchets the member list
#: against the source — so a verdict here should name this road and point at that
#: guard rather than re-argue it. What it must NOT do is treat the road as
#: absolution: a resource written by the reset AND by two in-session systems has
#: a question the reset says nothing about.
SESSION_SCOPE_RESET = (
    "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs"
)

#: ⭐⛤ **THE VERDICTS THAT SAY "ONE IN-SESSION OWNER", AS A CHECKED FACT RATHER
#: THAN A SENTENCE.** Each entry is `type -> the one production function that
#: writes it inside a session`; the other writer file must be
#: [`SESSION_SCOPE_RESET`]. [`main`] verifies both halves against
#: `census.write_sites`, so a second writing system arriving in that file reddens
#: the guard instead of quietly falsifying a paragraph.
#:
#: ⛔ WHY THIS IS NOT DECORATION. The green line's *"N are written from exactly
#: one OTHER file"* is FILE-granular, and two of the thirteen it counted on
#: 2026-09-18 had more than one writing function in that file — `ActiveCutscene`
#: two and `ProjectileSeqCounter` three. Those two carry their own verdicts,
#: argued per system, and are deliberately NOT in this table: it is for the ones
#: whose *"one owner"* claim is mechanical.
SOLE_IN_SESSION_OWNER: dict[str, str] = {
    "ControlledSubject": "resolve_controlled_subject",
    "CutsceneSkipHold": "apply_menu_frame_to_cutscene_request",
    "EncounterView": "apply_wave_encounter_effects",
    "GameplayElapsed": "advance_gameplay_elapsed",
    "LastCutsceneRoom": "auto_trigger_room_cutscenes",
    "LastQuestRoom": "push_room_entered_quest_events",
    "LiveMatchTicks": "count_the_live_match_ticks",
    "SaveRestored": "complete_durable_restore",
    "SessionMatchOrdinal": "activate_the_prepared_match",
    "StocksMatchSettled": "decide_stocks_match",
    "SuddenDeathEntered": "decide_stocks_match",
}


def sole_owner_shortfalls(
    multi: dict[str, list[str]], files: list[str]
) -> list[str]:
    """Every way a [`SOLE_IN_SESSION_OWNER`] claim can have stopped being true."""
    problems: list[str] = []
    for ty, owner in sorted(SOLE_IN_SESSION_OWNER.items()):
        if ty not in ADJUDICATED:
            problems.append(
                f"{ty} claims a sole in-session owner but carries no verdict; the "
                "table and ADJUDICATED must agree."
            )
            continue
        writers = multi.get(ty)
        if writers is None:
            problems.append(
                f"{ty} is no longer written from more than one file, so this claim "
                "has no subject. Remove it here and in ADJUDICATED."
            )
            continue
        if SESSION_SCOPE_RESET not in writers:
            problems.append(
                f"{ty} no longer includes `SessionScopedResources::reset` among its "
                f"writers ({', '.join(writers)}); the verdict's second half is gone."
            )
            continue
        others = [f for f in writers if f != SESSION_SCOPE_RESET]
        if len(others) != 1:
            problems.append(
                f"{ty} now has {len(others)} in-session writer FILE(s) "
                f"({', '.join(others)}), not one."
            )
            continue
        sites = census.write_sites(ty, others).get(others[0], [])
        if sorted(set(sites)) != [owner]:
            problems.append(
                f"{ty}'s in-session writes are in {sorted(set(sites))}, and the "
                f"verdict names `{owner}`. A second system reaching one owner's "
                "resource is exactly what this table exists to catch."
            )
    return problems


#: ⛔ ANTI-VACUITY. Every finding below is a set difference, and two empty sets
#: agree perfectly. These floors are an order of magnitude below the measured
#: 1,294 files / 329 types and far above the zero a broken scan produces.
MIN_FILES = 500
MIN_TYPES = 100


def main() -> int:
    files = census.production_files()
    if len(files) < MIN_FILES:
        print(
            f"⛔⛔ only {len(files)} production Rust file(s) found (expected "
            f"{MIN_FILES}+). The corpus is broken, not clean."
        )
        return 1
    found = census.writers(files)
    if len(found) < MIN_TYPES:
        print(
            f"⛔⛔ only {len(found)} mutably-reached resource type(s) parsed (expected "
            f"{MIN_TYPES}+); that is a claim about the regex."
        )
        return 1
    multi = {t: sorted(fs) for t, fs in found.items() if len(fs) > 1}

    # ⛔⛤ THE VERDICTS MUST NAME LIVE SUBJECTS, AND THIS RULE USED TO LIVE IN
    # `scripts/tests/` ONLY — which `--maintenance` does not run. The debt line
    # below prints `len(multi) - len(ADJUDICATED)`, and that subtraction is a
    # claim: a verdict on a type that is not on the shortlist would understate
    # the unread half while looking like one more thing settled.
    phantom = sorted(set(ADJUDICATED) - set(multi))
    if phantom:
        print(
            "an adjudication names a type that is not a multi-writer resource "
            "today:\n"
        )
        for ty in phantom:
            where = len(found.get(ty, ())) or 0
            print(
                f"  {ty} is adjudicated but has {where} production writer file(s). "
                "Either the name is misspelled, or the duplication is gone and the "
                "verdict should go with it."
            )
        return 1

    # ⭐ A VERDICT THAT SAYS "ONE OWNER" IS CHECKED, NOT TRUSTED.
    shortfalls = sole_owner_shortfalls(multi, files)
    if shortfalls:
        print("a sole-in-session-owner verdict no longer describes the tree:\n")
        for problem in shortfalls:
            print(f"  {problem}")
        return 1

    # ⭐ THE SECOND POPULATION, RATCHETED ON ITS OWN TERMS.
    world_all = census.session_world_writers(files)
    if len(world_all) < MIN_SESSION_WORLD_TYPES:
        print(
            f"⛔⛔ only {len(world_all)} type(s) reached through "
            f"`SessionWorldMut<T>` (expected {MIN_SESSION_WORLD_TYPES}+); that is "
            "a claim about the regex, not about the tree."
        )
        return 1
    world = {t: sorted(fs) for t, fs in world_all.items() if len(fs) > 1}
    world_moves = []
    for ty in sorted(set(world) - set(SESSION_WORLD_BASELINE)):
        world_moves.append(
            f"  NEW multi-writer SESSION WORLD component: {ty} "
            f"({len(world[ty])} files)\n"
            + "\n".join(f"      {f}" for f in world[ty])
        )
    for ty in sorted(set(SESSION_WORLD_BASELINE) - set(world)):
        world_moves.append(
            f"  {ty} is no longer written from more than one file. Remove it "
            "from SESSION_WORLD_BASELINE in this commit."
        )
    for ty in sorted(t for t in world if t in SESSION_WORLD_BASELINE):
        if len(world[ty]) != SESSION_WORLD_BASELINE[ty]:
            world_moves.append(
                f"  {ty} moved: {SESSION_WORLD_BASELINE[ty]} -> {len(world[ty])} "
                "writer file(s)."
            )
    if world_moves:
        print("the session-world component writer population moved:\n")
        for line in world_moves:
            print(line)
        print(
            "\n⇒ Same question, different lifetime: a session boundary reclaims "
            "these,\n  so ask what READS the fact WITHIN one session."
        )
        return 1

    arrived = sorted(set(multi) - set(BASELINE))
    left = sorted(set(BASELINE) - set(multi))
    grew = sorted(t for t in multi if t in BASELINE and len(multi[t]) > BASELINE[t])
    shrank = sorted(t for t in multi if t in BASELINE and len(multi[t]) < BASELINE[t])

    if arrived or left or grew or shrank:
        print("the multi-writer resource population moved:\n")
        for ty in arrived:
            print(f"  NEW multi-writer authority: {ty} ({len(multi[ty])} files)")
            for f in multi[ty]:
                print(f"      {f}")
        for ty in grew:
            print(f"  {ty} gained a writer: {BASELINE[ty]} -> {len(multi[ty])}")
            for f in multi[ty]:
                print(f"      {f}")
        for ty in shrank:
            print(
                f"  {ty} lost a writer: {BASELINE[ty]} -> {len(multi[ty])}. Lower it "
                "in this commit."
            )
        for ty in left:
            print(
                f"  {ty} is no longer written from more than one file. Remove it from "
                "the baseline in this commit."
            )
        print(
            "\n⇒ A NEW ENTRY IS NOT AUTOMATICALLY A DEFECT — see the module "
            "docstring.\n"
            "  Ask what READS the fact and whether an ambiguity in it can reach a\n"
            "  decision; then delete one writer and run the test that should care.\n"
            "  Record the answer in ADJUDICATED, or fix the second authority."
        )
        return 1

    print(
        f"ok: {len(multi)} resources are written from more than one production file, "
        f"the same set and the same per-type counts as the 2026-09-17 baseline "
        f"({len(files)} files, {len(found)} mutably-reached resource types)"
    )
    # ⛔ THE DEBT IS PRINTED, NOT IMPLIED. A baseline whose unread half is
    # invisible is an amnesty, and this one is mostly unread. The subtraction is
    # sound because the phantom rule above has already refused any verdict whose
    # subject is not in `multi`.
    print(
        f"  adjudicated: {len(ADJUDICATED)} "
        f"({', '.join(sorted(ADJUDICATED))}); UNADJUDICATED: "
        f"{len(multi) - len(ADJUDICATED)}"
    )
    # ⭐ THE SHAPE THAT EXPLAINS A QUARTER OF THE POPULATION, PRINTED SO NOBODY
    # POISONS IT AGAIN. It is a signpost, not a waiver — see `SESSION_SCOPE_RESET`.
    reset_writers = sorted(t for t, fs in multi.items() if SESSION_SCOPE_RESET in fs)
    reset_only = sorted(t for t in reset_writers if len(multi[t]) == 2)
    print(
        f"  ⭐ {len(reset_writers)} of them include `SessionScopedResources::reset` "
        f"among their writers, and {len(reset_only)} are written from exactly one "
        "OTHER file. That road's member list is owned by "
        "`check_session_owner_census_matches_source.py`, not by a verdict here."
    )
    # ⛔⛤ THAT SECOND NUMBER USED TO READ "would be single-writer without it", AND
    # IT WAS FALSE FOR TWO OF THEM. This census is FILE-granular: `ActiveCutscene`
    # has two writing systems in one file and `ProjectileSeqCounter` has three, so
    # "one other file" is not "one other writer". Both are adjudicated above, per
    # SYSTEM, and the difference is what the verdicts had to argue about.
    print(
        "  ⚠ ONE OTHER FILE IS NOT ONE OTHER WRITER — this census is file-granular. "
        "Re-read the file per SYSTEM before writing a verdict that says \"one owner\"."
    )
    print(
        f"  + {len(world)} session-world component(s) written from more than one "
        f"file, of {len(world_all)} carrying a `SessionWorldMut<T>` accessor — a "
        "population the resource census is silent about by construction."
    )
    print(
        "  ⚠ multi-writer is NOT a defect by count. This ratchets the population "
        "so a new one cannot land unnoticed."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
