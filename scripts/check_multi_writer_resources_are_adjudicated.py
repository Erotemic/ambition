#!/usr/bin/env python3
"""A resource written from a NEW second file must be adjudicated, not merely land.

⛔⛔ **THE CENSUS EXISTED, PRINTED ITS SHORTLIST, AND NO LANE RAN IT.**
`multi_writer_resource_census.py` answers *"which `Resource`s are written from
more than one file"* — the writer-side shape of a duplicated authority — and its
own tests only ever ran it over hand-built `tmp_path` corpora. So the number it
prints was a number somebody had to go and look at, and nothing noticed when a
resource joined the list. MEASURED 2026-09-17: **85 types** across 1,294
production files.

⚠ **THIS IS A RATCHET ON THE POPULATION, NOT A VERDICT ON IT.** The census
docstring is emphatic that multi-writer is NOT a defect by count, with two
measured cases that came out opposite ways, and nothing here contradicts that. 72
of the 75 are UNADJUDICATED and this check says so on every green run: what it
enforces is that the set and the per-type writer counts cannot move without
somebody editing this file.

⭐ **WHERE TO SPEND AN ADJUDICATION FIRST, MEASURED RATHER THAN GUESSED.** Of the
85, **18 are rollback-registered**, and those are the ones where a second writer
is a divergence rather than a design smell. Joining the writers' WRITE TARGETS
narrows it again — fields or methods touched by two or more of a type's writer
files:

    AmbitionGameSave        14 writers  ->  data() / data_mut()
    OwnedItems               4 writers  ->  grant()
    QuestRegistry            6 writers  ->  push_event()
    PendingLifecycleCommit   3 writers  ->  record()
    SlotInteractionState     4 writers  ->  get_mut() / primary_mut()
    ActiveConversation       2 writers  ->  close()
    ClockState               2 writers  ->  time_scale
    PossessionState          2 writers  ->  home
    BaseGravity, RoomTransitionCooldown, VersusMatch  ->  nothing shared

⚠ **AND THE OBVIOUS READING OF THAT TABLE IS WRONG.** *"They all go through one
method, so it is one authority"* holds for `grant`, which clamps unique items —
the policy is INSIDE the method. It does not hold for `data_mut()` or `get_mut()`,
which hand out `&mut` to everything: that is fourteen authorities with one door.
And it does not hold for a thin setter either: `close()` is `self.live = None`,
so what makes `ActiveConversation` correct is that its two callers fire on
DIFFERENT EVENTS and each has its own witness — which is the
`CutRopeBossArenaState` test, not the accessor count.

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
    "OwnedItems": 10,
    "ClassBRemapLog": 8,
    "FeatureEcsWorldOverlay": 8,
    "LoadCoordinator": 8,
    "UserSettings": 8,
    "CausalRecording": 7,
    "DeveloperRuntimeState": 7,
    "QuestRegistry": 6,
    "CaptureProgress": 5,
    "DeveloperTools": 5,
    "HudReadouts": 5,
    "PendingMechanicalEdits": 5,
    "SeatRawFrames": 5,
    "SlotControls": 5,
    "ActiveAudioSelection": 4,
    "DialogState": 4,
    "MapMenuState": 4,
    "MenuControlFrame": 4,
    "PendingLifecycleCommit": 4,
    "RoomTransitionLoadState": 4,
    "SlotInteractionState": 4,
    "ActiveSessionScope": 3,
    "ActiveUiCues": 3,
    "AudioLibrary": 3,
    "AuthoredOccurrences": 3,
    "GameplayTraceBuffer": 3,
    "KaleidoscopeCursor": 3,
    "KaleidoscopeScroll": 3,
    "KaleidoscopeSystemNav": 3,
    "PreparedSessionRegistry": 3,
    "SmashSelect": 3,
    "Warmup": 3,
    "AcceptedCheckpointRestore": 2,
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
    "LeaveRequested": 2,
    "LocalSeatOffer": 2,
    "MintedItemBaseline": 2,
    "MusicDirectorState": 2,
    "MusicIntent": 2,
    "MusicPlaybackState": 2,
    "NarrativeMusicRequest": 2,
    "OccurrenceBaseline": 2,
    "OwnedItemsBaseline": 2,
    "PortalTuning": 2,
    "PortalViewer": 2,
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
}

#: ⛔ ANTI-VACUITY. Every finding below is a set difference, and two empty sets
#: agree perfectly. These floors are an order of magnitude below the measured
#: 1,294 files / 333 types and far above the zero a broken scan produces.
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
    # invisible is an amnesty, and this one is mostly unread.
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
