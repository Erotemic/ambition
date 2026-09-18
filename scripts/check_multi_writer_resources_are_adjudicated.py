#!/usr/bin/env python3
r"""A resource written from a NEW second file must be adjudicated, not merely land.

⛔⛔ **THE CENSUS EXISTED, PRINTED ITS SHORTLIST, AND NO LANE RAN IT.**
`multi_writer_resource_census.py` answers *"which `Resource`s are written from
more than one file"* — the writer-side shape of a duplicated authority — and its
own tests only ever ran it over hand-built `tmp_path` corpora. So the number it
prints was a number somebody had to go and look at, and nothing noticed when a
resource joined the list. MEASURED 2026-09-17: **82 types** across 1,294
production files.

⚠ **THIS IS A RATCHET ON THE POPULATION, NOT A VERDICT ON IT.** The census
docstring is emphatic that multi-writer is NOT a defect by count, with two
measured cases that came out opposite ways, and nothing here contradicts that. 75
of the 82 are UNADJUDICATED and this check says so on every green run — the split
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

⭐ **WHERE TO SPEND AN ADJUDICATION FIRST, AND THE TABLE IS NO LONGER CARRIED BY
HAND.** **At least 20 of the 82 are rollback-registered**, and those are the ones
where a second writer is a divergence rather than a design smell. That 20 is a
LOWER BOUND and carries its instrument: it is the intersection with the type
names in `rollback_*::<T>` / `declare_rollback_derived_*::<T>` turbofish calls
across `crates/` and `game/`, which parses 95 names where the registry itself
holds 491 rows — every row registered through a non-turbofish form is invisible
to it. Joining the writers' WRITE TARGETS narrows it again — which field or
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

    AmbitionGameSave        17 files  data_mut 13 (13 certain), data 11   ROUTED
    OwnedItems               9 files  grant 2
    QuestRegistry            6 files  push_event 4, quests 2
    PendingLifecycleCommit   3 files  record 2
    SlotInteractionState     4 files  primary_mut 2 (2 certain)           ADJUDICATED
    ActiveConversation       2 files  close 2                             ADJUDICATED
    ClockState               2 files  time_scale 2 (2 certain)            ADJUDICATED
    PossessionState          2 files  home 2, possessed 2                 ADJUDICATED
    RoomTransitionCooldown   2 files  remaining 2
    VersusMatch              2 files  *<the resource> 2 (2 certain)
    BaseGravity              2 files  nothing shared

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
BASELINE: dict[str, int] = {
    "AmbitionGameSave": 17,
    "GameAssets": 12,
    "GameplayBanner": 10,
    "OwnedItems": 9,
    "ClassBRemapLog": 8,
    "FeatureEcsWorldOverlay": 8,
    "LoadCoordinator": 8,
    "UserSettings": 7,
    "QuestRegistry": 6,
    "CaptureProgress": 5,
    "CausalRecording": 5,
    "DeveloperRuntimeState": 5,
    "DeveloperTools": 5,
    "HudReadouts": 5,
    "PendingMechanicalEdits": 5,
    "SeatRawFrames": 5,
    "SlotControls": 5,
    "ActiveAudioSelection": 4,
    "MapMenuState": 4,
    "MenuControlFrame": 4,
    "RoomTransitionLoadState": 4,
    "SlotInteractionState": 4,
    "ActiveSessionScope": 3,
    "ActiveUiCues": 3,
    "AudioLibrary": 3,
    "AuthoredOccurrences": 3,
    "DialogState": 3,
    "GameplayTraceBuffer": 3,
    "KaleidoscopeCursor": 3,
    "PendingLifecycleCommit": 3,
    "PreparedSessionRegistry": 3,
    "Warmup": 3,
    "ActiveConversation": 2,
    "ActiveGameplaySession": 2,
    "BaseGravity": 2,
    "BodyClocksView": 2,
    "CameraShakeState": 2,
    "CharacterLoadDemand": 2,
    "CharacterLoadStates": 2,
    "ClockState": 2,
    "ControlFrame": 2,
    "CustodyBaseline": 2,
    "CutsceneAdvanceRequest": 2,
    "CutsceneTriggerQueue": 2,
    "DefaultMusicStarted": 2,
    "FallingSandRoomState": 2,
    "FixedStepsTaken": 2,
    "InventoryUiState": 2,
    "KaleidoscopeOpenState": 2,
    "KaleidoscopeScroll": 2,
    "KaleidoscopeSystemNav": 2,
    "LeaveRequested": 2,
    "LocalSeatOffer": 2,
    "MintedItemBaseline": 2,
    "MusicDirectorState": 2,
    "MusicIntent": 2,
    "MusicPlaybackState": 2,
    "NarrativeMusicRequest": 2,
    "OccurrenceBaseline": 2,
    "OwnedItemsBaseline": 2,
    "PossessionState": 2,
    "PresentationPhase": 2,
    "ReservedGameplayScopes": 2,
    "RoomConstructionPlanPrefetch": 2,
    "RoomTransitionCooldown": 2,
    "ScrollbarDragState": 2,
    "SeatActiveDevices": 2,
    "SelectCursors": 2,
    "SelectPage": 2,
    "SfxBankRegistry": 2,
    "ShellRouteHolds": 2,
    "ShellRouter": 2,
    "ShrineActivationPulse": 2,
    "SlotControlLatches": 2,
    "SmashSelect": 2,
    "StartRequested": 2,
    "SwitchActivationQueue": 2,
    "VersusMatch": 2,
    "VisualQualityConfirmState": 2,
    "WorldSourceHotReload": 2,
    "WorldlineHistoryView2d": 2,
    "YarnPresentationCue": 2,
}

#: The ones somebody has actually read. ⚠ An entry here is a CITATION, not an
#: opinion: it names the row or the source contract that owns the answer.
ADJUDICATED: dict[str, str] = {
    "AmbitionGameSave": (
        "OPEN — `queue.md`'s ROLLBACK-BAG-DESYNC row owns it: the save mirrors "
        "disagree with their own rollback replay. The 17 writers are that row's "
        "subject, not a separate finding."
    ),
    "CutsceneAdvanceRequest": (
        "OPEN — CUTSCENE-ROLLBACK-DECISION item 1: produced on the HOST side in "
        "`Update` and consumed with `std::mem::take` inside the sim schedule, so "
        "a dismiss press is lost across a rewind. Held by a failing-by-design "
        "witness."
    ),
    "ActiveConversation": (
        "CORRECT — two END CONDITIONS, not two retractors of one event. "
        "`break_dialogue_on_hit_or_separation` closes on knockback, separation "
        "or a despawned participant; `close_conversation_on_narrative_end` "
        "closes on a stamped `ConversationEnded` input whose instance matches. "
        "POISON-VERIFIED 2026-09-17 in both directions: removing the first fails "
        "`a_conversation_breaks_on_knockback_or_on_the_bodies_separating`, "
        "removing the second fails three arms including "
        "`a_rewind_past_the_end_replays_it_at_the_same_tick`. Neither hides the "
        "other's absence, which is the `CutRopeBossArenaState` test."
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
    "CutsceneTriggerQueue": (
        "CORRECT BY COINCIDENCE — CUTSCENE-ROLLBACK-DECISION item 2: every "
        "producer happens to sit inside the sim schedule, so a replay "
        "re-produces what a rewind dropped. The invariant is ratcheted by "
        "`check_sim_consumed_request_writers.py`, not by this baseline."
    ),
    "SlotInteractionState": (
        "CORRECT — ONE ARMER, THREE CLEARERS. Every production call that can make "
        "a gesture live is in `control/input_systems.rs`: `buffered_interact`, "
        "`register_down_tap`, `register_up_tap`, `held_up_interact` and the one "
        "`double_tap_down_pending = true`. MEASURED 2026-09-17 by grepping the "
        "arming methods across `crates/` and `game/` — there are no other "
        "production call sites. The other three writers can only move the row "
        "TOWARD its resting state: `world/rooms/systems.rs` reads "
        "`primary().buffered()` then calls `primary_mut().clear()`; "
        "`body_mode/mechanics/mod.rs` `mem::take`s `double_tap_down_pending` out "
        "of `get_mut(slot)`, a different field; `runtime/src/sandbox_reset.rs` "
        "hands `primary_mut()` to `reset_sandbox`, which calls "
        "`SlotGestures::reset()`. ⇒ So two of them DO share row zero — the rooms "
        "consumer and the reset boundary — and that is still one authority, "
        "because a clear is idempotent and neither can arm: no ordering between "
        "them changes what any reader sees. The rooms system reads row zero only "
        "and its subject genuinely IS the primary (`ControlledSubject`, else the "
        "`PrimaryPlayerOnly` single), not a leftover of the D175 *producer filled "
        "row zero* bug. "
        "⛔⛤ POISONED 2026-09-17 AND THE POISON PASSED: deleting the "
        "`slot_gestures.primary_mut().clear()` that the source calls *consuming "
        "the gesture* left 1,207 crate arms and all 11 room-transition "
        "integration arms green, because every authored door arm HOLDS interact "
        "for thirty frames and the armer refills the buffer underneath the clear. "
        "That is a finding about the arms: a tap is the only shape that can see a "
        "consumption. Witnessed now by "
        "`a_door_crossing_consumes_the_buffered_press_rather_than_letting_it_"
        "decay` (`actor_monolith/src/world/rooms/tests.rs`), which reddens on that "
        "deletion and — through its out-of-zone control — on hoisting the clear "
        "above the validation too."
    ),
}

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
            f"⛔⛔ only {len(found)} `ResMut<T>` type(s) parsed (expected "
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
        f"({len(files)} files, {len(found)} `ResMut<T>` types)"
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
    print(
        "  ⚠ multi-writer is NOT a defect by count. This ratchets the population "
        "so a new one cannot land unnoticed."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
