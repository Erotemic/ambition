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
HAND.** **THE RUN PRINTS IT NOW — there is no number here to
go stale.** As of 2026-09-18 that queue is SPENT: every rollback-registered
multi-writer type the parse can see carries a verdict. ⚠ Which is not the same as
clean — the parse is a LOWER BOUND, so a row registered through any
non-turbofish form was never in the queue to begin with, and the remaining
unadjudicated majority is simply not rollback-registered. Every invocation ends with a line naming the multi-writer types that
are also rollback-registered and still carry no verdict; those are the ones where
a second writer is a DIVERGENCE rather than a design smell.

⛔⛤ **IT USED TO BE PROSE HERE AND IT WENT STALE FIVE TIMES ON 2026-09-18
ALONE** — "21 of the 102", then 31, eight, seven, six, five, four. Every
restatement was correct when written and wrong within the hour, because the
number counts VERDICTS and verdicts are exactly what this campaign spends. The
intersection itself never moved once. ⇒ A number that changes every time somebody
does the work cannot live in a docstring; see `rollback_registered_shortlist`.

What the intersection buys is that a second writer there is a DIVERGENCE rather
than a design smell. The 37 is a LOWER BOUND and carries
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
import re
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
#: ⛔⛤ **A COUNT IS NOT AN IDENTITY, AND THIS TABLE HELD ONLY COUNTS UNTIL
#: 2026-09-18.** `dict[str, int]` let a type lose one real writer and gain a
#: different, unrelated one in the same run — the count held, so the ratchet read
#: clean forever. DEMONSTRATED, not argued: replacing one of `AmbitionGameSave`'s
#: 18 writer files with a fabricated path left this guard green, printing the same
#: `ok` line and the same `UNADJUDICATED: 0`. That matters because EVERY verdict
#: in [`ADJUDICATED`] argues from the SPECIFIC files it names, so a silent
#: substitution at constant cardinality rots the citations with nothing to flag
#: it. The existing arms could not see it either: both change the COUNT
#: (add-without-remove, remove-without-add), and neither tests a same-cardinality
#: swap. Found by CalculexAmbition.
#:
#: ⭐ THE FILE SET IS NOW THE ONE OWNER and the count is `len()` of it, rather
#: than a second recorded fact that can disagree with the first — which is the
#: rule this whole census exists to enforce, applied to the census.
BASELINE: dict[str, tuple[str, ...]] = {
    "AmbitionGameSave": (
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_encounter/src/switches.rs",
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_menu/src/map/systems.rs",
        "crates/ambition_persistence/src/quest/registry.rs",
        "crates/ambition_persistence/src/save.rs",
        "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/encounter_rewards.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/persist.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/shrine.rs",
        "game/ambition_content/src/bosses/cut_rope/mod.rs",
        "game/ambition_content/src/encounters.rs",
        "game/ambition_content/src/falling_sand_sim.rs",
        "game/ambition_content/src/quest.rs",
    ),
    "GameAssets": (
        "crates/ambition_platformer2d/src/game_assets.rs",
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs",
        "crates/ambition_render/src/rendering/parallax.rs",
        "game/ambition_app/src/app/setup_systems.rs",
        "game/ambition_app/src/app/startup_loading.rs",
        "game/ambition_app/src/app/world_flow/parallax_residency.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
        "game/ambition_content/src/intro/plugin.rs",
        "game/ambition_demo_mary_o/src/ai_slop.rs",
        "game/ambition_demo_mary_o/src/plane.rs",
        "game/ambition_demo_mary_o/src/scenery.rs",
        "game/ambition_demo_mary_o/src/snake.rs",
        "game/ambition_demo_sanic/src/lib.rs",
    ),
    "GameplayBanner": (
        "crates/ambition_boss_encounter/src/encounter_script.rs",
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_combat/src/banner.rs",
        "crates/ambition_combat/src/breakables.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/chests.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/interact.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/pickups.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "game/ambition_content/src/bosses/cut_rope/arena.rs",
        "game/ambition_content/src/quest.rs",
    ),
    "FeatureEcsWorldOverlay": (
        "crates/ambition_encounter_features/src/lock_walls.rs",
        "crates/ambition_platformer2d_actor_monolith/src/world/gated_lock_walls.rs",
        "crates/ambition_platformer2d_actor_monolith/src/world/overlay.rs",
        "game/ambition_content/src/bosses/gnu_ton.rs",
        "game/ambition_content/src/falling_sand.rs",
        "game/ambition_content/src/falling_sand_sim.rs",
        "game/ambition_content/src/portal/carve_adapter.rs",
        "game/ambition_demo_mary_o/src/bricks.rs",
        "game/ambition_demo_mary_o/src/powerups.rs",
        "game/ambition_demo_sanic/src/monitors.rs",
    ),
    "OwnedItems": (
        "crates/ambition_held_items/src/lib.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/chests.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/pickups.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/narrative.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/persist.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs",
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_content/src/portal/inventory_adapter.rs",
        "game/ambition_content/src/quest.rs",
    ),
    "CausalRecording": (
        "crates/ambition_combat/src/causal.rs",
        "crates/ambition_combat/src/moveset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/causal.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/actors/update.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs",
        "crates/ambition_platformer2d_runtime/src/causal.rs",
        "game/ambition_app_tools/src/bin/moveset_takes.rs",
        "game/ambition_app_tools/src/bin/rl_random_walker.rs",
        "game/ambition_demo_smash_app/src/tools/ladder_probe.rs",
    ),
    "ClassBRemapLog": (
        "crates/ambition_abilities/src/traversal/blink.rs",
        "crates/ambition_abilities/src/traversal/dive.rs",
        "crates/ambition_abilities/src/traversal/mark_recall.rs",
        "crates/ambition_damage/src/lib.rs",
        "crates/ambition_platformer2d_actor_monolith/src/abilities/traversal/teleport.rs",
        "crates/ambition_platformer2d_actor_monolith/src/abilities/traversal/trapdoor.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_shared_tangle/src/class_b.rs",
        "crates/ambition_portal2d/src/transit.rs",
    ),
    "DeveloperRuntimeState": (
        "crates/ambition_dev_tools/src/lib.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_render/src/rendering/debug_viz.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
        "game/ambition_app/src/dev/frame_step.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_app_tools/src/bin/capture_scene.rs",
        "game/ambition_app_tools/src/bin/moveset_render.rs",
        "game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs",
    ),
    "LoadCoordinator": (
        "crates/ambition_game_shell/src/plugin.rs",
        "crates/ambition_game_shell/src/session.rs",
        "crates/ambition_load/src/plugin.rs",
        "crates/ambition_load_presentation/src/shell_adapter.rs",
        "crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/loading.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
        "game/ambition_app/src/app/world_flow/room_transition_presentation.rs",
    ),
    "UserSettings": (
        "crates/ambition_game_shell/src/pause_menu.rs",
        "crates/ambition_game_shell/src/plugin.rs",
        "crates/ambition_persistence/src/settings/persistence.rs",
        "crates/ambition_render/src/quality.rs",
        "crates/ambition_sim_harness/src/runtime.rs",
        "game/ambition_app/src/dev/fps_overlay.rs",
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_app_tools/src/bin/capture_scene.rs",
    ),
    "PendingLifecycleCommit": (
        "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/death.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_actor_monolith/src/world/rooms/systems.rs",
        "crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_runtime/src/sandbox_reset.rs",
        "game/ambition_demo_mary_o/src/lib.rs",
    ),
    "QuestRegistry": (
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_persistence/src/quest/registry.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs",
        "crates/ambition_platformer2d_actor_monolith/src/quest/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "game/ambition_content/src/quest.rs",
    ),
    "RoomTransitionCooldown": (
        "crates/ambition_platformer2d_actor_monolith/src/control/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_runtime/src/sandbox_reset.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
    ),
    "SeatRawFrames": (
        "crates/ambition_platformer2d/src/scripted_input.rs",
        "crates/ambition_platformer2d_actor_monolith/src/control/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_runtime/src/input_drive.rs",
        "game/ambition_app/src/app/sim_systems.rs",
        "game/ambition_content/src/portal/ability_adapter.rs",
    ),
    "BaseGravity": (
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/gravity/lifecycle.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_shared_tangle/src/gravity.rs",
        "crates/ambition_sim_harness/src/runtime.rs",
    ),
    "DeveloperTools": (
        "crates/ambition_dev_tools/src/persistence.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
        "game/ambition_app/src/dev/mod.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_app_tools/src/bin/capture_scene.rs",
        "game/ambition_app_tools/src/bin/moveset_render.rs",
    ),
    "SlotControls": (
        "crates/ambition_platformer2d_actor_monolith/src/control/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_rollback_ggrs/src/session.rs",
        "crates/ambition_platformer2d_runtime/src/input_drive.rs",
        "game/ambition_app/src/app/sim_systems.rs",
        "game/ambition_content/src/portal/ability_adapter.rs",
    ),
    "SlotInteractionState": (
        "crates/ambition_platformer2d_actor_monolith/src/body_mode/mechanics/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/control/acting.rs",
        "crates/ambition_platformer2d_actor_monolith/src/control/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_actor_monolith/src/world/rooms/systems.rs",
        "crates/ambition_platformer2d_runtime/src/sandbox_reset.rs",
    ),
    "ActiveConversation": (
        "crates/ambition_conversation/src/opening.rs",
        "crates/ambition_conversation/src/rules.rs",
        "crates/ambition_conversation/src/ui_bridge.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
    ),
    "AuthoredOccurrences": (
        "crates/ambition_held_items/src/lib.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs",
    ),
    "CaptureProgress": (
        "crates/ambition_render/src/capture.rs",
        "game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs",
        "game/ambition_demo_sanic_app/src/bin/capture_sanic.rs",
        "game/ambition_demo_smash_app/src/tools/match_shots.rs",
        "game/ambition_demo_twintrack_app/src/bin/capture_twintrack.rs",
    ),
    "DialogState": (
        "crates/ambition_conversation/src/ui_bridge.rs",
        "crates/ambition_dialog/src/bridge.rs",
        "crates/ambition_dialog/src/systems.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
    ),
    "HudReadouts": (
        "game/ambition_app/src/app/versus_rules.rs",
        "game/ambition_demo_mary_o/src/provider.rs",
        "game/ambition_demo_sanic/src/provider.rs",
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_twintrack/src/lib.rs",
    ),
    "PendingMechanicalEdits": (
        "crates/ambition_combat/src/feel.rs",
        "crates/ambition_dev_tools/src/dev_tools/editable.rs",
        "crates/ambition_dev_tools/src/lib.rs",
        "crates/ambition_portal2d/src/tuning.rs",
        "game/ambition_app/src/app/player_clone.rs",
        "game/ambition_content/src/portal/transit_body_adapter.rs",
    ),
    "RoomTransitionLoadState": (
        "crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/commit.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/loading.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
        "game/ambition_app/src/app/world_flow/room_transition_presentation.rs",
    ),
    "ActiveAudioSelection": (
        "crates/ambition_audio/src/bank_asset.rs",
        "crates/ambition_game_shell/src/session.rs",
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs",
        "game/ambition_app/src/app/setup_systems.rs",
    ),
    "AudioLibrary": (
        "crates/ambition_audio/src/library.rs",
        "crates/ambition_audio/src/music/director/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "CharacterLoadDemand": (
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs",
        "game/ambition_app/src/app/startup_loading.rs",
        "game/ambition_app/src/app/versus.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
    ),
    "MapMenuState": (
        "crates/ambition_menu/src/map/input.rs",
        "crates/ambition_menu/src/map/pointer.rs",
        "crates/ambition_menu/src/map/systems.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "MenuControlFrame": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_touch_input/src/menu_bridge.rs",
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "MintedItemBaseline": (
        "crates/ambition_platformer2d_actor_monolith/src/items/persist.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "MovingPlatformSet": (
        "crates/ambition_platformer2d_actor_monolith/src/avatar/body_integration.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_actor_monolith/src/world/rooms/transaction.rs",
        "game/ambition_demo_smash/src/lib.rs",
    ),
    "SessionSeatingSource": (
        "crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs",
        "game/ambition_app/src/app/versus.rs",
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_twintrack/src/participants.rs",
    ),
    "ShellHostConfiguration": (
        "crates/ambition_platformer2d/src/app.rs",
        "crates/ambition_platformer2d_provider/src/composition.rs",
        "game/ambition_app/src/app/shell_host.rs",
        "game/ambition_app_tools/src/bin/preview_vanity_card.rs",
    ),
    "ShellRouteCatalog": (
        "crates/ambition_platformer2d/src/app.rs",
        "crates/ambition_platformer2d_provider/src/composition.rs",
        "game/ambition_app/src/app/shell_host.rs",
        "game/ambition_app_tools/src/bin/preview_vanity_card.rs",
    ),
    "ShellRouteHolds": (
        "crates/ambition_game_shell/src/plugin.rs",
        "crates/ambition_load_presentation/src/shell_adapter.rs",
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
        "game/ambition_content/src/reload.rs",
    ),
    "YarnContentBindings": (
        "crates/ambition_conversation/src/dialog.rs",
        "crates/ambition_conversation/src/dialog/yarn_harness.rs",
        "game/ambition_content/src/bosses/mod.rs",
        "game/ambition_content/src/plugin.rs",
    ),
    "ActiveSessionScope": (
        "crates/ambition_game_shell/src/session.rs",
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
        "crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs",
    ),
    "ActiveUiCues": (
        "crates/ambition_game_shell/src/basic_presentation.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_demo_smash/src/lib.rs",
    ),
    "BossEncounterRegistry": (
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "CharacterLoadStates": (
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs",
        "game/ambition_app/src/app/startup_loading.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
    ),
    "ConstructionSchemaCatalog": (
        "crates/ambition_platformer2d_actor_monolith/src/gravity/plugin.rs",
        "crates/ambition_platformer2d_runtime/src/sim_core_resources.rs",
        "crates/ambition_portal2d/src/plugin.rs",
    ),
    "CustodyBaseline": (
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_shared_tangle/src/lifecycle/custody_horizon.rs",
    ),
    "CutsceneAdvanceRequest": (
        "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs",
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "CutsceneTriggerQueue": (
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "EncounterRegistry": (
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "GameplayTraceBuffer": (
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/dev/trace/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs",
    ),
    "KaleidoscopeCursor": (
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_app/src/menu/kaleidoscope_app/pointer.rs",
    ),
    "LocalSessionOwnership": (
        "crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
        "game/ambition_content/src/reload.rs",
    ),
    "MusicPlaybackState": (
        "crates/ambition_audio/src/music/director/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "NarrativeMusicRequest": (
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
        "crates/ambition_platformer2d_actor_monolith/src/music/intent.rs",
        "game/ambition_content/src/yarn_vocabulary.rs",
    ),
    "OccurrenceBaseline": (
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_platformer2d_shared_tangle/src/lifecycle/continuity.rs",
    ),
    "PossessionState": (
        "crates/ambition_platformer2d_actor_monolith/src/abilities/traversal/possession.rs",
        "crates/ambition_platformer2d_actor_monolith/src/control/authority.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "PreparedSessionRegistry": (
        "crates/ambition_game_shell/src/plugin.rs",
        "crates/ambition_load_presentation/src/shell_adapter.rs",
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
    ),
    "SelectCursors": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_smash/src/select_screen.rs",
        "game/ambition_demo_smash_app/src/tools/select_walkthrough.rs",
    ),
    "SlotControlLatches": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_rollback_ggrs/src/session.rs",
        "crates/ambition_platformer2d_runtime/src/input_drive.rs",
    ),
    "SwitchActivationQueue": (
        "crates/ambition_encounter/src/switches.rs",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "Warmup": (
        "game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs",
        "game/ambition_demo_sanic_app/src/bin/capture_sanic.rs",
        "game/ambition_demo_twintrack_app/src/bin/capture_twintrack.rs",
    ),
    "WorldSourceHotReload": (
        "crates/ambition_dev_tools/src/hot_reload.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "AbandonedCheckpointOperation": (
        "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
        "crates/ambition_platformer2d_runtime/src/room_transition/loading.rs",
    ),
    "ActiveCutscene": (
        "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "ActiveGameplaySession": (
        "crates/ambition_game_shell/src/session.rs",
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
    ),
    "BodyClocksView": (
        "crates/ambition_sim_view/src/facts.rs",
        "game/ambition_demo_smash/src/mark.rs",
    ),
    "CameraShakeState": (
        "crates/ambition_platformer2d_shared_tangle/src/camera_ease.rs",
        "game/ambition_app/src/app/player_tick.rs",
    ),
    "ClockState": (
        "crates/ambition_platformer2d_actor_monolith/src/time/time_control/mod.rs",
        "crates/ambition_time/src/time_control/mod.rs",
    ),
    "ContentEpochSequence": (
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
    ),
    "ControlFrame": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_rollback_ggrs/src/session.rs",
    ),
    "ControlledSubject": (
        "crates/ambition_platformer2d_actor_monolith/src/abilities/traversal/possession.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "CutsceneSkipHold": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "DefaultMusicStarted": (
        "crates/ambition_audio/src/library.rs",
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
    ),
    "EditablePortalTuning": (
        "game/ambition_app/src/dev/portal_inspector.rs",
        "game/ambition_content/src/portal/transit_body_adapter.rs",
    ),
    "EncounterView": (
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "FallingSandRoomState": (
        "game/ambition_content/src/falling_sand.rs",
        "game/ambition_content/src/falling_sand_sim.rs",
    ),
    "FixedStepsTaken": (
        "game/ambition_app/src/app/cli.rs",
        "game/ambition_app/src/headless.rs",
    ),
    "GameplayElapsed": (
        "crates/ambition_platformer2d_actor_monolith/src/features/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "InventoryUiState": (
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "KaleidoscopeOpenState": (
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_menu_kaleidoscope/src/lib.rs",
    ),
    "KaleidoscopeScroll": (
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "game/ambition_app/src/menu/kaleidoscope_app/scroll.rs",
    ),
    "KaleidoscopeSystemNav": (
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "LastCutsceneRoom": (
        "crates/ambition_platformer2d_actor_monolith/src/cutscene.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "LastQuestRoom": (
        "crates/ambition_platformer2d_actor_monolith/src/quest/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "LeaveRequested": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_smash/src/select_screen.rs",
    ),
    "LiveMatchTicks": (
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/live_match_clock.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "LocalSeatOffer": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_twintrack/src/participants.rs",
    ),
    "LocalSeatTopology": (
        "crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs",
        "crates/ambition_sim_harness/src/runtime.rs",
    ),
    "MobileTouchState": (
        "crates/ambition_touch_input/src/bevy_plugin.rs",
        "crates/ambition_touch_input/src/virtual_device.rs",
    ),
    "MusicDirectorState": (
        "crates/ambition_audio/src/music/director/mod.rs",
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
    ),
    "MusicIntent": (
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
        "crates/ambition_platformer2d_actor_monolith/src/music/intent.rs",
    ),
    "NewGameResetRequested": (
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "OwnedItemsBaseline": (
        "crates/ambition_platformer2d_actor_monolith/src/items/persist.rs",
        "crates/ambition_platformer2d_actor_monolith/src/items/pickup/minted_horizon.rs",
    ),
    "PortalCameraContinuitySelection": (
        "game/ambition_app/src/dev/portal_inspector.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "PortalCameraContinuityState": (
        "crates/ambition_platformer2d_host/src/portal.rs",
        "crates/ambition_render/src/rendering/camera.rs",
    ),
    "PortalEffectSelection": (
        "game/ambition_app/src/dev/portal_inspector.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "PortalViewConeDebugDumpRequest": (
        "crates/ambition_portal2d_presentation/src/view_cones/debug.rs",
        "game/ambition_app/src/dev/portal_inspector.rs",
    ),
    "PresentationPhase": (
        "crates/ambition_platformer2d_rollback_ggrs/src/lib.rs",
        "crates/ambition_sim_view/src/presented_pose.rs",
    ),
    "ProjectileSeqCounter": (
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
        "crates/ambition_projectiles/src/materialize.rs",
    ),
    "RadioStationState": (
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "ReservedGameplayScopes": (
        "crates/ambition_game_shell/src/session.rs",
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
    ),
    "RoomConstructionPlanPrefetch": (
        "crates/ambition_platformer2d_runtime/src/room_transition/loading.rs",
        "game/ambition_app/src/app/world_flow/room_transition_assets.rs",
    ),
    "RoomContentStagingRegistry": (
        "crates/ambition_sim_harness/src/runtime.rs",
        "game/ambition_content/src/plugin.rs",
    ),
    "SaveRestored": (
        "crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "ScrollbarDragState": (
        "crates/ambition_menu/src/render/bevy_ui/mod.rs",
        "game/ambition_menu_kaleidoscope/src/lib.rs",
    ),
    "SeatActiveDevices": (
        "crates/ambition_input/src/active_input.rs",
        "crates/ambition_touch_input/src/menu_bridge.rs",
    ),
    "SeatControlFrameModes": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "crates/ambition_sim_harness/src/runtime.rs",
    ),
    "SeatMenuFrames": (
        "crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
        "game/ambition_demo_smash_app/src/tools/select_walkthrough.rs",
    ),
    "SelectPage": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_smash/src/select_screen.rs",
    ),
    "SessionMatchOrdinal": (
        "crates/ambition_platformer2d_actor_monolith/src/character_runtime/match_activation.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "SfxBankRegistry": (
        "crates/ambition_audio/src/bank_asset.rs",
        "game/ambition_app/src/app/setup_systems.rs",
    ),
    "SfxPlaybackState": (
        "crates/ambition_audio/src/bank_asset.rs",
        "crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs",
    ),
    "ShellActivationGates": (
        "crates/ambition_platformer2d_provider/src/lifecycle.rs",
        "game/ambition_content/src/reload.rs",
    ),
    "ShellRouter": (
        "crates/ambition_game_shell/src/plugin.rs",
        "crates/ambition_load_presentation/src/shell_adapter.rs",
    ),
    "ShellSequenceCatalog": (
        "game/ambition_app/src/app/shell_host.rs",
        "game/ambition_app_tools/src/bin/preview_vanity_card.rs",
    ),
    "ShrineActivationPulse": (
        "crates/ambition_platformer2d_actor_monolith/src/shrine.rs",
        "crates/ambition_sim_view/src/facts.rs",
    ),
    "SmashSelect": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_smash/src/select_screen.rs",
    ),
    "StartRequested": (
        "game/ambition_demo_smash/src/lib.rs",
        "game/ambition_demo_smash/src/select_screen.rs",
    ),
    "StocksMatchSettled": (
        "crates/ambition_platformer2d_actor_monolith/src/features/stocks_match.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "SuddenDeathEntered": (
        "crates/ambition_platformer2d_actor_monolith/src/features/stocks_match.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    ),
    "VersusMatch": (
        "game/ambition_app/src/app/versus.rs",
        "game/ambition_app/src/app/versus_rules.rs",
    ),
    "VisualQualityConfirmState": (
        "game/ambition_app/src/menu/grid_backend.rs",
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
    ),
    "WorldlineHistoryView2d": (
        "crates/ambition_relativity2d/src/telemetry.rs",
        "game/ambition_demo_twintrack/src/lib.rs",
    ),
    "YarnPresentationCue": (
        "crates/ambition_dialog/src/bindings.rs",
        "crates/ambition_dialog/src/bridge.rs",
    ),
}

#: The ones somebody has actually read. ⚠ An entry here is a CITATION, not an
#: opinion: it names the row or the source contract that owns the answer.
ADJUDICATED: dict[str, str] = {
    "DeveloperTools": (
        "NO FIELD IS REACHED BY TWO FILES AT ALL — `census.shared_targets` "
        "returns the EMPTY SET over its six writers, which is the strongest "
        "answer this instrument can give and it is available without reading a "
        "line. The resource is roughly twenty-five independent debug bools and "
        "enums; each writer touches a disjoint field. ⚠ TWO THINGS THE COUNT "
        "MISREADS, both checked: `load_developer_at_startup` appears TWICE in "
        "`crates/ambition_dev_tools/src/persistence.rs` because there are two "
        "definitions of it — the native one at `:74` (a Startup-only "
        "whole-resource load from disk) and a `#[cfg(target_arch = \"wasm32\")]` "
        "no-op at `:113`; both run before `Update` and never in the same build. "
        "And the two capture-tool writers are separate `[[bin]]` targets that "
        "never coexist with `ambition_app`. ⭐ SECOND ANGLE, because a "
        "disjoint-field answer says nothing about rollback: NO reader of this "
        "resource sits inside `GgrsSchedule`. The mechanical-edit chain is the "
        "Q120 propose/admit/project split — propose in `PreUpdate`, and the sim "
        "reads only `ActivePlayerBodyProfile`. Measured by CalculexAmbition, "
        "2026-09-18; the `shared_targets` and per-`[[bin]]` halves re-run here."
    ),
    "DeveloperRuntimeState": (
        "THREE CONTENDED FIELDS OF FOUR, AND EACH IS A KNOWN SHAPE RATHER THAN "
        "A RACE. `census.shared_targets` names `debug`, `preset_flash` and "
        "`slowmo` across nine writer files. ⭐ `debug` and `auto_apply`-shaped "
        "rows are TWO INPUT SURFACES FOR ONE USER TOGGLE — a developer hotkey "
        "(`handle_debug_hotkeys`) and a cube System row (`SystemMenuParams`), "
        "both `= !current`, which is idempotent per press and cannot be made "
        "one writer without deleting one surface. `preset_flash` is an "
        "ARM/DECAY pair: two independent trigger events set it to `1.0` and "
        "`decay_developer_presentation_flash` reduces it once a frame. ⚠ THE "
        "THIRD WRITER OF `debug` IS `toggle_debug_viz` in "
        "`crates/ambition_render/src/rendering/debug_viz.rs`, and it never "
        "coexists with the other two: `DebugVizPlugin` is installed by the four "
        "secondary game apps (twintrack, mary_o, sanic and smash) and by nothing "
        "under `game/ambition_app` — ⚠ the flagship DOES name "
        "the plugin twice under that path, in COMMENTS in `dev/debug_overlay.rs` "
        "and `dev/debug_overlay/gizmos.rs`, so *\"no reference\"* is the wrong "
        "test and *\"no installation\"* is the right one. ⭐ ROLLBACK ANGLE: "
        "`preset_flash`'s only reader is a HUD text gate (`> 0.0`), and "
        "`slowmo`'s reader publishes a `ClockScaleRequest` every frame "
        "unconditionally — deliberately not edge-triggered, per its own doc — "
        "which a `.min()` fold downstream makes order-free. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "WorldSourceHotReload": (
        "A SINGLE-WRITER PATH FIELD AND A FREE-TEXT STATUS LINE NOTHING "
        "BRANCHES ON. ⚠ READ THE `shared_targets` OUTPUT PROPERLY: it reports "
        "`watch_path` as TOUCHED by two files and MUTATION-SHAPED IN ONE, and "
        "the difference is the whole verdict — `hot_reload.rs` sets it once, in "
        "the constructor `WorldSourceHotReload::watching`, and "
        "`dev_runtime.rs` only `.clone()`s it. A census target list is not a "
        "writer list. `last_status` / `pending` / `last_errors` are written by "
        "temporally disjoint events — a filesystem poll, a hotkey or cube row, "
        "and an apply outcome — and the only reader outside production is a "
        "test asserting outcomes; no production branch reads the string. "
        "`auto_apply` is the same two-input-surfaces-one-toggle shape as "
        "`DeveloperRuntimeState`'s `debug`. Measured by CalculexAmbition, "
        "2026-09-18; the touched-versus-mutated split re-run here."
    ),
    "MusicPlaybackState": (
        "FIELD-PRIVATE, AND THE ONLY TWO MUTATORS ARE `begin_track` AND "
        "`silence` — which is why `census.shared_targets` is EMPTY across three "
        "writer files. Each writer is a distinct, non-overlapping trigger: the "
        "adaptive director's tick (`drive_music_director`), an explicit radio "
        "pick from the System menu's Radio row, a frontend-owner-CHANGE latch "
        "(`apply_frontend_music_policy`, gated on "
        "`matches!(owner, Some(Frontend(_)))` and latched through a "
        "`Local<Option<AudioContextOwner>>` so it fires only on change), and a "
        "context reset. ⭐ THE OVERLAP CASE IS DESIGNED RATHER THAN ABSENT: "
        "`title_theme_keeps_playing` lets an identical title track survive a "
        "Frontend→Frontend reset instead of restarting it. ⚠ AND THE RESETS "
        "FUNNEL THROUGH A FREE FUNCTION THE FILE CENSUS CANNOT SEE — "
        "`ambition_audio::music::silence_music_backend` takes "
        "`&mut MusicPlaybackState`, so it is invisible to a `ResMut` scan. It is "
        "NOT a hidden fourth road: its three production callers are "
        "`audio/plugin.rs:274` and `:383` and `music/director/mod.rs:82`, both "
        "files the census already names. Compare `UserSettings`, where the same "
        "`&mut` shape DOES hide a whole crate. Measured by CalculexAmbition, "
        "2026-09-18; the helper's call sites re-derived here."
    ),
    "UserSettings": (
        "NINE WRITER FILES THAT ARE ALL SURFACES, AND THE OWNER IS A CRATE THIS "
        "CENSUS CANNOT SEE. ⛔⛤ `crates/ambition_settings_menu` is the largest "
        "settings writer in the workspace and appears in NO writer list here, "
        "because `apply_settings_option` takes `&mut UserSettings` rather than "
        "`ResMut` — a `ResMut`-keyed census is blind to it by construction. That "
        "function is the single owner of *what a row-step does*, and both app "
        "backends route through it (`menu/dispatch.rs:89` and `:222`, "
        "`kaleidoscope_app.rs:1433`). Beneath it the per-field mutation "
        "primitives are owned once in `crates/ambition_persistence/src/settings` "
        "(`AudioSettings::nudge_master`, `ScreenShaderSettings::nudge_unit`, …), "
        "and the shell's own audio menu calls the same ones with the same "
        "`VOLUME_STEP`. ⇒ Many surfaces, one authority per field. "
        "⚠ THE REMAINING NINE, EACH CHECKED: `load_settings_at_startup` and the "
        "hardware-quality seed fire once (the latter latched on "
        "`video.quality.hardware_seeded`); `crates/ambition_sim_harness/src/runtime.rs` "
        "builds its own headless App and "
        "`game/ambition_app_tools/src/bin/capture_scene.rs` is a standalone "
        "`[[bin]]`, so neither "
        "coexists with the game; `fps_overlay.rs` toggles `video.show_fps` "
        "directly, the same two-input-surfaces-one-toggle shape as "
        "`DeveloperRuntimeState`'s `debug`; and `capture_armed_rebind` "
        "(`kaleidoscope_app.rs:550`) is the ONLY production caller of "
        "`ControlsSettings::set_binding_override` — the other three call sites "
        "are inside `#[cfg(test)]` modules, confirmed by re-running the census's "
        "own `strip_test_modules` over each file rather than by reading the "
        "grep. Measured by CalculexAmbition, 2026-09-18."
    ),
    "AudioLibrary": (
        "THE `&mut` IS A LAZY HANDLE CACHE, NOT AN AUTHORITY — AND THAT IS A "
        "FACT ABOUT THE TYPE RATHER THAN ABOUT ITS FOUR CALLERS. `AudioLibrary` "
        "declares exactly one `&mut self` method of its own, `preload_track`, "
        "and two more reached through it — `resolve_track` and its thin wrapper "
        "`resolve_track_handle` (`crates/ambition_audio/src/library.rs`). All "
        "three do the same thing: *if* `track.source.handle` is `None`, fill it "
        "from `asset_server.load(track.source.asset_path)`. Nothing adds, "
        "removes or reorders a track, and nothing selects one — "
        "`set_selected_track`/`clear` belong to `RadioStationState` and "
        "`begin_track`/`silence` to `MusicPlaybackState`, two other types in "
        "the same file, which is why a file-granular census points here. "
        "⭐ THE MEMOISATION IS ORDER-INDEPENDENT BY CONSTRUCTION: the value "
        "written is a function of the track's own `asset_path`, and "
        "`AssetServer::load` answers the same handle for the same path, so two "
        "callers racing to fill one slot write the same bytes. ⇒ Four writer "
        "files, one authority — the RON catalogue that built the track list — "
        "and four readers that warm it. ⚠ The census cannot see this: `&mut` is "
        "all a `SystemParam` signature carries, and the director threads one "
        "through four call layers to reach `switch_to_music_track`, whose only "
        "library call is the resolve."
    ),
    "MapMenuState": (
        "ONE OWNER — `ambition_menu::map` — AND ONE WRITE FROM OUTSIDE IT THAT "
        "IS A SUPPRESSION, NOT A SECOND AUTHORITY. Inside the owning crate: "
        "`populate_map_rooms` fills `rooms` once and returns early while it is "
        "non-empty, `sync_map_from_save` refreshes the visited set only when "
        "`AmbitionGameSave` is changed, `handle_map_menu_hotkeys` is the input "
        "road (open/minimap/zoom), and `map_menu_pointer_dismiss` closes on a "
        "press and returns early while closed — four disjoint jobs on one "
        "resource. ⭐ THE OUTSIDE WRITE IS `open_kaleidoscope_menu` "
        "(`game/ambition_app/src/menu/kaleidoscope_app.rs`, reached from "
        "`kaleidoscope_menu_open_routing`), and it writes exactly one field with "
        "its reason beside it: *\"Never leave the standalone map panel open "
        "underneath the cube\"* — `map.open = false`. "
        "⚠ THE TWO-BACKENDS TEST IS ON BOTH SIDES, WHICH IS WHAT MAKES THIS A "
        "PARTITION RATHER THAN A RACE: the cube's routing carries "
        "`.run_if(kaleidoscope_backend_active)`, and "
        "`handle_map_menu_hotkeys` reads `InventoryUiBackend::effective()` in "
        "its own body to suppress the `menu.map` open under the cube while "
        "leaving the `M` key working for the Grid backend. Neither side can "
        "open the panel the other is presenting."
    ),
    "MenuControlFrame": (
        "ONE PRODUCER, ONE ADAPTER, AND FOUR CONSUMERS THAT ONLY TAKE — and the "
        "schedule says so by name rather than by convention. "
        "`populate_menu_control_frame_from_actions` "
        "(`schedule/input_systems.rs`) is the sole device->frame writer. ⚠ It is "
        "NOT the only member of `MenuFramePopulate` — that set has two, and the "
        "set's own doc is where the difference is checked: the second, "
        "`populate_seat_menu_frames`, *\"writes `SeatMenuFrames` and NOTHING "
        "else — not `MenuControlFrame`, not `SeatActiveDevices`\"*, and the doc "
        "says that was *\"checked at both signatures rather than assumed from "
        "the names\"*. "
        "`ambition_touch_input::menu_bridge::fold_touch_gestures` ADDS to the "
        "frame and pins itself between the two sets, which is what the sets "
        "exist for (*\"an adapter that must add to the frame after it is "
        "rebuilt had to name `populate_menu_control_frame_from_actions` "
        "directly, from another crate\"*). The remaining four reaches — "
        "`grid_menu_nav`, `grid_menu_open_routing`, `kaleidoscope_focus_nav`, "
        "`kaleidoscope_menu_open_routing` — are CONSUMPTIONS: "
        "`menu_frame.consume_nav_edges()` and `menu.back = false`, each with a "
        "comment at the site saying which edge that system OWNS. "
        "⭐ AND THE ONE AMBIGUITY THIS SHAPE STILL HAD IS CLOSED IN SOURCE: "
        "`MenuNavConsume` orders the writer before its members and says nothing "
        "about their order among themselves, so once they began consuming, "
        "*\"whoever runs first eats the press\"* was the real arbitration and "
        "the scheduler picked it. `game/ambition_app/src/menu/mod.rs` now states "
        "the order — grid first, because it is the backend that survives when "
        "the cube is compiled out — and says both orders are correct because at "
        "most one backend is effective per frame. "
        "⭐⭐ AND THIS ROW IS THE ONE MEMBER OF THE POPULATION THAT ALREADY HAS "
        "A RUNNING ORDER GUARD RATHER THAN A VERDICT: "
        "`menu_frame_readers_are_ordered_against_each_other_in_the_shipped_app` "
        "(`game/ambition_app/tests/update_schedule_census.rs`) asks BEVY's own "
        "`conflicting_systems()` about the shipped graph, with a positive "
        "control (`the_menu_frame_conflict_detector_reports_an_unordered_pair`) "
        "and a negative one (`ordering_the_pair_clears_the_menu_frame_conflict`) "
        "so a zero is a fact about the app and not about a detector that never "
        "reports. ⇒ file-granular multi-writer, single-authority by "
        "construction, and the construction is MEASURED every run rather than "
        "restated here."
    ),
    "ActiveUiCues": (
        "THREE PUBLISHERS OF DISJOINT CUES, EACH FOR A SURFACE THE OTHER TWO DO "
        "NOT HAVE. `publish_shell_ui_cues` "
        "(`ambition_game_shell::basic_presentation`) speaks for the universal "
        "shell chrome, `publish_menu_confirm_prompt` "
        "(`game/ambition_app/src/menu/kaleidoscope_app.rs`) for the cube's "
        "confirm prompt, and `publish_the_select_ui_cue` "
        "(`game/ambition_demo_smash/src/lib.rs`) for the character-select "
        "screen. ⚠ THE "
        "SEPARATION THAT MAKES THIS SAFE IS THE SHELL'S OWN: `pause_menu.rs` "
        "states that the shell menu and the kaleidoscope *\"partition every "
        "live session with no overlap\"* via `ShellPauseMenuSuppressed`, so the "
        "first two cannot both be speaking; the third belongs to a route with "
        "no gameplay session at all. ⇒ Adjudicated on the partition, not on the "
        "absence of a collision — a partition is a fact a reader can re-check."
    ),
    "KaleidoscopeCursor": (
        "CORRECT NOW, AND IT WAS NOT WHEN THIS ROW WAS OPENED — the one member "
        "of the two-backends family whose writers were not all gated. The "
        "cursor is SHARED: `grid_backend.rs` says the *\"shared cursor and drill "
        "state stay on `KaleidoscopeCursor` / `KaleidoscopeSystemNav`\"*, and "
        "both backends move it. Grid writers are "
        "`.run_if(grid_backend_active)`; the grid's hover OBSERVER, "
        "`grid_menu_pointer_hover`, carries the same test in its body because an "
        "observer takes no `run_if`. Cube writers are gated by "
        "`kaleidoscope_menu_visible`, and its press/release observers carry the "
        "body test.\n"
        "    ⛔⛤ `kaleidoscope_pointer_move` CARRIED NEITHER, and that was a "
        "live defect rather than a census artifact: these observers are "
        "installed whenever the cube feature is COMPILED — not when the cube is "
        "SELECTED — and a grid menu control carries the same "
        "`AmbitionMenuControl<MenuPageAction>` its query matches, so with the "
        "flat backend active a mouse move over a GRID row reached the shared "
        "cursor and wrote `owner = Pointer` while the grid's own hover was "
        "writing `owner = Keyboard`. Gated 2026-09-18, below the drag-cancel so "
        "a touch/pen drag still cancels a tap. Held by "
        "`a_pointer_move_with_the_cube_closed_does_not_move_the_shared_cursor` "
        "(`menu/kaleidoscope_app/lunex_kaleidoscope_app_tests.rs`), poison-"
        "verified: remove the gate and the arm fails.\n"
        "    ⭐ THE CENSUS FOUND IT BY LOCALITY, NOT BY SUSPICION. Six of this "
        "backend's seven writers carry the check whether or not they also have a "
        "`run_if` — a double gate that is the house style — so the missing one "
        "stood out only once all seven were listed side by side."
    ),
    "RadioStationState": (
        "CORRECT — ONE SETTER ROAD AND ONE CONTEXT RESET, and the road was "
        "followed to its only caller rather than inferred from the file. The "
        "type is one private field with `selected_track()` / "
        "`set_selected_track()`; the ONLY production caller of the setter is "
        "`set_radio_track` (`ambition_audio/src/library.rs`), whose own only "
        "call site is the system menu's radio row "
        "(`ambition_app/src/menu/kaleidoscope_app.rs`, through the "
        "`SystemMenuParams` bundle). The second writer file is the context "
        "reset: `reset_audio_request_state_on_context_change` "
        "(`actor_monolith/src/audio/plugin.rs`) assigns `Default::default()` "
        "through the `AudioRequestState` bundle when the audio context changes. "
        "⇒ A human picks a station, a context change forgets it. ⚠ `clear()` is "
        "a third mutation API and carries `#[allow(dead_code)]` — it has no "
        "callers today, and a future one would be a second road to the same "
        "field, not another use of this one."
    ),
    "KaleidoscopeOpenState": (
        "CORRECT — TWO FIELDS, TWO OWNERS, AND THE SPLIT IS THE WHOLE TYPE. "
        "`pub amount: f32` and `pub target: f32`. The HOST writes the target: "
        "`gate_kaleidoscope_menu` (`ambition_app/src/menu/kaleidoscope_app.rs`) "
        "assigns `open_state.target = 1.0` while the cube backend is selected and "
        "the overlay is open, `0.0` otherwise, and touches nothing else. The "
        "LIBRARY writes the amount: `animate_cube_ring` "
        "(`ambition_menu_kaleidoscope/src/lib.rs`) eases `amount` toward that "
        "target every frame and reads the target to decide the rate — opening "
        "keeps the gentle ease, closing multiplies by `close_speed_scale`. "
        "⇒ A request and its easing, not two opinions about one number.\n"
        "    ⭐ AND THE HOST COMPARES BEFORE WRITING, for a reason worth "
        "carrying: `gate_kaleidoscope_menu` runs every frame with no run "
        "condition *\"(it has to — it detects the open EDGE)\"*, so an "
        "unconditional `ResMut` write would mark this resource changed on every "
        "frame of the game's life for a value that moves twice per menu visit."
    ),
    "MusicIntent": (
        "CORRECT ABOUT THE WRITERS, AND THE OTHER QUESTION ABOUT THE SAME SYSTEM "
        "IS OWED ELSEWHERE — the two are not the same question and this row "
        "answers only the first. THREE writers, three roles, all inside the actor "
        "monolith. ONE BUILDER: `compute_music_intent` "
        "(`src/music/intent.rs`) folds room, encounter, narrative and radio "
        "requests into the intent every frame, chained before "
        "`drive_music_director` and `.run_if(simulation_authorized)`. ONE "
        "FRONTEND SILENCE: `apply_frontend_music_policy` (`src/audio/plugin.rs`) "
        "clears `simple_track_candidates` and `adaptive` on the frame the "
        "simulation DEAUTHORIZES — deliberately ungated *\"so it can observe the "
        "transition\"* and latched to act once per frontend entry. ⇒ The gate is "
        "the arbitration: the builder runs only while authorized, the silence "
        "acts as that stops. ONE CONTEXT RESET: "
        "`reset_audio_request_state_on_context_change` assigns "
        "`*intent = Default::default()` through the `AudioRequestState` bundle "
        "and is registered `.before` BOTH of the others, which is stated rather "
        "than hoped.\n"
        "    ⛔ AND THE SCHEDULE QUESTION IS SEPARATE AND OPEN: "
        "`compute_music_intent` is banked in "
        "`scripts/check_rollback_mutators_run_in_sim.py` as an `Update` writer of "
        "rollback state, owed to ROLLBACK-MUTATOR-POPULATION. *Who owns the "
        "value* and *does a rewind replay the write* are different questions "
        "about one system; a verdict here must not read as an answer to the "
        "other."
    ),
    "FixedStepsTaken": (
        "NOT A SHARED RESOURCE AT ALL — ONE NAME, TWO TYPES, AND THE CENSUS IS "
        "KEYED ON THE NAME. `struct FixedStepsTaken(u32);` is declared INSIDE a "
        "function twice: `ambition_app/src/app/cli.rs` and "
        "`ambition_app/src/headless.rs` each declare their own, `init_resource` "
        "it into their own app, and increment it from a closure system to count "
        "that harness's fixed steps. ⇒ Two local tick counters that share a "
        "spelling and not one byte of state; a third lives in "
        "`ambition_platformer2d_host/tests/demo_shell_smoke.rs`. ⛔ THE ROW IS A "
        "PHANTOM AND THE POPULATION OF 121 IS OVERSTATED BY IT — recorded rather "
        "than deleted, because a census that quietly drops rows it finds "
        "inconvenient stops being a ratchet. `NAME_COLLISION_NOT_ONE_TYPE` "
        "checks the claim: promote either declaration to module scope and the "
        "guard fails, because then the two writers might really share a type."
    ),
    # ── ONE SELECTOR, TWO PRESENTATIONS ──────────────────────────────────────
    #
    # The menu ships TWO backends — the flat Bevy-UI grid and the 3D cube — and
    # they write the same host-side menu state from two files. That is not two
    # authorities while ONE resource decides which of them is effective:
    # `InventoryUiBackend::effective()` (`ambition_menu/src/backend.rs`) returns
    # exactly one variant, `grid_backend_active` is
    # `BEVY_UI_MENU_BACKEND_ENABLED && effective() == Grid`, and every cube
    # writer is gated by `kaleidoscope_backend_active` or
    # `kaleidoscope_menu_visible`, both of which include
    # `effective() == LunexKaleidoscope`. ⇒ At most one of the two files writes
    # in any frame.
    #
    # ⛔ THE GATE IS PER-SYSTEM, SO THE VERDICT IS TOO. `menu/mod.rs` states the
    # architecture — the two navs are ordered against each other because either
    # order is correct, *"at most one backend is effective in a frame, and each
    # nav is gated on its own backend"* — and an OBSERVER takes no `run_if`, so
    # the cube's and grid's pointer observers carry the same test in their
    # bodies instead. A writer that carries neither is the way this verdict
    # fails, which is why `KaleidoscopeCursor` is a separate row.
    "InventoryUiState": (
        "CORRECT — ONE SELECTOR, TWO PRESENTATIONS. Grid writers: "
        "`grid_menu_action_activated`, `grid_menu_nav`, `grid_menu_open_routing` "
        "(`ambition_app/src/menu/grid_backend.rs`), all registered "
        "`.run_if(grid_backend_active)`. Cube writers: `kaleidoscope_focus_nav`, "
        "`kaleidoscope_menu_action_activated` (`.run_if(kaleidoscope_menu_visible)`) "
        "and `kaleidoscope_menu_open_routing` "
        "(`.run_if(kaleidoscope_backend_active)`), in "
        "`ambition_app/src/menu/kaleidoscope_app.rs`. ⇒ Six writers, two files, "
        "one live at a time. ⚠ `visible` is the field both open-routing systems "
        "raise, and they are ALSO gated on `simulation_authorized` + "
        "`in_base_mode` — *\"the mirror of the Grid backend gate\"* — because "
        "without it the inventory toggle leaked onto the title screen and into a "
        "hosted demo's session. Two gates, same pair, stated on both sides."
    ),
    "KaleidoscopeSystemNav": (
        "CORRECT — the same selector and the same four/three writers as "
        "`InventoryUiState`, plus `grid_menu_tab_activated` on the grid side "
        "(also `.run_if(grid_backend_active)`). ⛔ THE NAME IS A LIE ABOUT "
        "OWNERSHIP AND THE SOURCE SAYS SO: `grid_backend.rs` documents that the "
        "*\"shared cursor and drill state stay on `KaleidoscopeCursor` / "
        "`KaleidoscopeSystemNav`\"* — the `Kaleidoscope` prefix is where the "
        "state was born, not who owns it. A reader who takes the prefix for the "
        "owner will misread every row in this family."
    ),
    "VisualQualityConfirmState": (
        "CORRECT — the same selector. On the cube side the three gated writers "
        "above; on the grid side `grid_menu_tab_activated` plus the "
        "`MenuDispatchParams` bundle, which is how the census names a mutable "
        "reach it cannot attribute to one system. ⚠ THE BUNDLE IS THE REASON "
        "THIS ROW IS NOT INTERESTING AND THE REASON IT COULD HAVE BEEN: a bundle "
        "grants its `ResMut` to every system that takes it, and the mutator "
        "guard has already found five menu systems holding "
        "`ResMut<NewGameResetRequested>` through `SystemMenuParams` that cannot "
        "reach the write at all. ⇒ So this was followed to the expressions, and "
        "the first answer was wrong: the confirm state is NOT written only "
        "through `dispatch_menu_action`. That road owns the ARM and the SPEND — "
        "`step_from`, `take_confirmed`, `cancel` (`menu/dispatch.rs`) — and each "
        "backend ALSO calls `quality_confirm.cancel()` directly at its own "
        "navigation points, five sites in `grid_backend.rs` and one in "
        "`kaleidoscope_app.rs`. ⭐ That is still one authority: `cancel` writes "
        "the ABSENCE of a pending confirmation and is idempotent, so a backend "
        "leaving the screen cannot disagree with the road that armed it — but "
        "the census's two files are two files for a second reason the gate "
        "argument alone does not cover."
    ),
    "KaleidoscopeScroll": (
        "CORRECT — AN OVERRIDE AND ITS RELEASE, one field, one backend. The type "
        "is `system_window_start: Option<usize>` and its doc states the contract: "
        "*\"explicit System scroll-window start, or `None` to follow the "
        "cursor\"*. The pointer road SETS the override — `kaleidoscope_scroll_wheel` "
        "and `kaleidoscope_apply_scroll_drag` "
        "(`ambition_app/src/menu/kaleidoscope_app/scroll.rs`) — and the keyboard "
        "road RELEASES it: `kaleidoscope_focus_nav` "
        "(`ambition_app/src/menu/kaleidoscope_app.rs`) assigns "
        "`scroll.system_window_start = None` when `dx != 0 || dy != 0 || "
        "menu.select`, so *\"the window snaps to follow the cursor again\"*. "
        "⇒ `None` is not a competing value, it is the absence of one; whichever "
        "input last acted owns the window, which is the arbitration this field "
        "exists to express. ⛔ All three are cube-only AND each re-checks "
        "`backend.effective() == LunexKaleidoscope` in its own body on top of its "
        "`run_if` — a double gate that is the house style for this backend's "
        "writers, and the reason a missing one stands out."
    ),
    # ── two SURFACES, one SETTING ────────────────────────────────────────────
    #
    # ⭐ THE ARGUMENT IS ONLY SAFE WHILE NOTHING DERIVES THE VALUE. A control is
    # not an authority because it writes what a human asked for and nothing
    # recomputes it; add one system that DERIVES either of these and the second
    # surface becomes a fight, not a duplicate. The census staying at exactly
    # these two files is what keeps that true, which is why the verdicts name the
    # files rather than only the systems.
    "PortalEffectSelection": (
        "CORRECT — TWO DEVELOPER SURFACES, ONE SETTING, NO DERIVATION. The type "
        "is one field (`active: PortalVisualEffect`) and its own doc says who "
        "moves it: *\"the host's developer menu cycles it\"*. Surface one is the "
        "egui portal inspector's `effects_section` "
        "(`ambition_app/src/dev/portal_inspector.rs`), which hands "
        "`&mut selection.active` to a row widget; surface two is the cube system "
        "menu, which reaches it through the `SystemMenuParams` bundle "
        "(`ambition_app/src/menu/kaleidoscope_app.rs`). ⇒ Both write what a human "
        "asked for, neither derives the value from anything, and a human drives "
        "one surface at a time. ⛔ Both are `#[cfg(feature = \"portal_render\")]` "
        "and the inspector prints `missing_resource` rather than inserting a "
        "default when the resource is absent — a surface that INSERTED one would "
        "be a third opinion about the initial value."
    ),
    "PortalCameraContinuitySelection": (
        "CORRECT — the same pair, the same argument, and the same one-field "
        "shape as `PortalEffectSelection`: `camera_continuity_section` "
        "(`ambition_app/src/dev/portal_inspector.rs`) and the `SystemMenuParams` "
        "bundle (`ambition_app/src/menu/kaleidoscope_app.rs`) each offer "
        "`selection.mode` to a human. Written as its own row rather than folded "
        "into its neighbour because the two resources' writer FILES agreeing "
        "today is a measurement, not a shared definition."
    ),
    "MobileTouchState": (
        "CORRECT — ONE PRODUCTION ROAD, AND THE SECOND FILE IS A TRAIT'S MOCK "
        "SEAM RATHER THAN A SYSTEM. ⛔ THE CENSUS NAMES FUNCTIONS, AND FOUR OF "
        "THESE SIX ARE NOT SYSTEMS AT ALL: `press`, `release`, `set_axis_pair` "
        "and `set_touch_button` (`ambition_touch_input/src/virtual_device.rs`) "
        "are `leafwing_input_manager` trait impls taking `&mut World`, and the "
        "trait requires them — `Buttonlike` and `DualAxislike` each demand a "
        "setter so an input kind can be simulated. Their own docs say which road "
        "they are: *\"Test/mocking seam: press the underlying touch state "
        "directly\"*. `set_touch_button` is a private free function with exactly "
        "two callers, both of them those impls.\n"
        "    ⇒ The production writers are the two systems in "
        "`ambition_touch_input/src/bevy_plugin.rs`: "
        "`update_buttons_from_interactions` (the overlay's `Interaction`s and "
        "raw touches) and `read_joystick_messages` (the stick's messages). And "
        "the module doc states the architecture this resource exists to keep: "
        "touch resolves through the participant's `InputMap` *\"exactly like a "
        "keyboard or gamepad — never as a second system writing gameplay/menu "
        "resources directly\"*. ⚠ A mock seam IS a second writer in any world "
        "that calls it; what makes this a verdict rather than a hope is that its "
        "callers are leafwing's simulation entry points, not this repo's "
        "systems."
    ),
    "FallingSandRoomState": (
        "CORRECT — ONE ROOM-CHANGE BOUNDARY PLUS THREE PER-FIELD OWNERS, read "
        "field by field rather than system by system, because the struct has "
        "five fields and the file-granular count says nothing about which. "
        "`sync_falling_sand_room_state` (`ambition_content/src/falling_sand_sim.rs`) "
        "is the boundary: on a change of active room id it writes "
        "`last_room_id`, `active_room`, clears `seeded_boundaries` and RE-DERIVES "
        "`spouts` — `FallingSandSpoutState::from_save(save.data())` on entry, "
        "`default()` otherwise. The three others each own one field: "
        "`seed_falling_sand_room_boundaries` "
        "(`ambition_content/src/falling_sand.rs`) sets `seeded_boundaries` once "
        "and is guarded by it, `grant_room_swim_controls` owns `swim_snapshot`, "
        "and `capture_falling_sand_switch_interactions` owns `spouts` through "
        "`state.spouts.toggle(..)`. ⇒ The only field two systems write is "
        "`seeded_boundaries`, and they write it as a SET and a CLEAR, which is a "
        "latch and its boundary.\n"
        "    ⛔ AND THE REAL SECOND AUTHORITY IS NOT IN THIS RESOURCE, WHICH IS "
        "WHY THE CENSUS COULD NOT SEE IT. `spouts` mirrors a DURABLE fact: the "
        "toggle writes `save.data_mut().set_switch(&id, on)` on every activation, "
        "and the boundary reads it back with `from_save` on the next entry. The "
        "system's own comment says why the mirror exists — *\"without this write "
        "the save's switch flag stays whatever the encounter pipeline set it to "
        "(which is 'true on first activation' only when the switch's `action` is "
        "`ResetEncounter`)\"* — so the save's switch flag has TWO writers, one of "
        "which exists to compensate for the other's conditionality. That is a "
        "duplicate authority over `AmbitionGameSave`, not over this resource, and "
        "it is what a falling-sand spout fixture would settle."
    ),
    "YarnPresentationCue": (
        "CORRECT ABOUT THE VALUE, AND IT NAMES WHAT IT DOES NOT COVER. Two "
        "writers, one per role: `on_present_line` (`ambition_dialog/src/bridge.rs`) "
        "sets `cue.shout` / `cue.whisper` from the line's markup, and "
        "`clear_yarn_presentation_cue` (`ambition_dialog/src/bindings.rs`) is "
        "three lines that assign `false` to both. The clear is a FRAME BOUNDARY, "
        "not a second opinion — its own doc says *\"reset markup cues before the "
        "bridge writes cues for the current frame\"* — and the repo publishes it "
        "as an ordering seam, `YarnPresentationCueCleared`, which "
        "`refresh_yarn_state_mirror` (`ambition_content/src/plugin.rs`) is "
        "registered `.after(..)`.\n"
        "    ⚠ WHAT THIS VERDICT DOES NOT SETTLE, stated so the next reader does "
        "not assume it did: `on_present_line` is an OBSERVER "
        "(`app.add_observer`), so it carries no set and nothing orders it against "
        "the clear. The seam above orders a CONSUMER after the clear; the "
        "PRODUCER's position in the frame is whatever triggers the line event. "
        "⇒ One owner of the value, and an open question about the moment — which "
        "is a different row from this one."
    ),
    # ── the smash select screen: ONE DRIVER, ONE ARRIVAL RESET ───────────────
    #
    # ⭐ THE FOUR BELOW SHARE ONE ARGUMENT AND ARE WRITTEN OUT FOUR TIMES ON
    # PURPOSE. A shared verdict a reader has to assemble from four rows is how a
    # family stops being re-checked member by member — and three of these four
    # have the same pair of writers while the fourth does NOT, which is the
    # distinction a single row would have flattened.
    "SmashSelect": (
        "CORRECT — ONE DRIVER PLUS ONE ARRIVAL RESET. `drive_the_cursor` "
        "(`ambition_demo_smash/src/select_screen.rs`) is the select screen's "
        "single input driver: it reads the frame, resolves the layout's targets "
        "and writes what the seats decided. The only other writer is "
        "`reset_select_frontend_on_arrival` (`ambition_demo_smash/src/lib.rs`), "
        "which assigns `SmashSelect::default()` and nothing else. ⛔ IT IS NOT A "
        "SECOND OPINION ABOUT THE VALUE: it is keyed on `ShellActivationId` "
        "through a `Local`, returns unless `on_the_select_route`, and exists "
        "because *\"the first visit's root outlives the route change\"* — "
        "measured on the second match, arriving back at the lobby with "
        "`ui_roots=1`, where `start_the_battle_when_asked` then refused while a "
        "roster stood and pressing start did nothing at all. ⇒ Arrival is a "
        "LIFETIME boundary, not an authority."
    ),
    "SelectPage": (
        "CORRECT — the same pair and the same argument as `SmashSelect`: "
        "`drive_the_cursor` (`select_screen.rs`) drives it, "
        "`reset_select_frontend_on_arrival` (`lib.rs`) assigns "
        "`SelectPage::default()` on a NEW arrival only. ⛔ Checked at the SITE "
        "rather than inferred from the shared system: the reset writes four "
        "resources in four consecutive statements and this is one of them, so a "
        "future fifth statement is a change to this verdict's subject."
    ),
    "StartRequested": (
        "CORRECT — the same pair as `SmashSelect`, and this one is WHY the "
        "arrival reset exists. `drive_the_cursor` raises it; "
        "`reset_select_frontend_on_arrival` clears it on a new arrival because a "
        "latch left standing re-fires on the next route — the failure Jon "
        "reported as *\"in the second match I select characters press start, but "
        "it just brings me back to the character screen\"*. ⇒ Two writers, one "
        "of which exists to bound the other's lifetime."
    ),
    "LeaveRequested": (
        "CORRECT — AND IT IS A DIFFERENT SHAPE FROM ITS THREE NEIGHBOURS, WHICH "
        "IS WHY THIS ROW IS SEPARATE. Its two writers are PRODUCER and CONSUMER, "
        "not driver and reset: `drive_the_cursor` (`select_screen.rs`) raises it, "
        "and `leave_the_select_screen_when_asked` (`lib.rs`) spends it — "
        "`asked.0 = false` runs BEFORE both of that system's refusals, so the "
        "latch cannot survive the frame it was read in whatever happens next. "
        "⛔ That is also why it is NOT in `reset_select_frontend_on_arrival`'s "
        "four-statement reset while its three neighbours are: a request that is "
        "always spent needs no arrival to bound it. The system's own comment "
        "names the contrast with `StartRequested`."
    ),
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
    "BaseGravity": (
        "CORRECT FOR FIVE OF THE SIX, AND THE SIXTH IS A SYSTEM NOTHING RUNS — "
        "ROUTED TO Q137. Six files, seven sites. ONE developer road, and it is an "
        "INVERSION rather than a writer: a dev hotkey or the menu's Gravity row "
        "writes `AmbientGravityRequest` and `apply_ambient_gravity_requests` "
        "(`shared_tangle/src/gravity.rs`) applies it inside the sim. That seam "
        "exists because the direct version was caught — its own doc records "
        "*\"a developer control that wrote it from `Update` mutated a rewound "
        "value on a schedule that never rewinds\"*, found by "
        "`check_rollback_mutators_run_in_sim.py` on 2026-09-03. ONE shipped "
        "MECHANIC: `drive_wave_encounters` (`encounter_features/src/systems.rs`) "
        "queues two deferred world commands, off `SwitchAction::FlipGravity` "
        "(negate) and `SetGravity(face)` (absolute), and reaches them only "
        "through the single `drain_switch_activations`, so activation ORDER is "
        "owned by that drain and not contested here. THREE lifecycle resets on "
        "three distinct edges, each stated where it lives: "
        "`reset_gravity_on_room_reset` (`RoomReplayAdmitted` + "
        "`NewGameResetCommitted`), `RoomTransitionCombatReset::clear_carryover` "
        "(a room crossing), and `SessionScopedResources::reset` (session "
        "activation and retirement). All three assign the same "
        "`BaseGravity::default()`, so they cannot disagree about the VALUE — they "
        "differ only in WHEN. ONE driver: `set_base_gravity_dir` "
        "(`sim_harness/src/runtime.rs`), which calls "
        "`rebase_after_direct_setup_mutation` rather than leaving a rollback "
        "baseline behind it.\n"
        "    ⛔ THE SIXTH IS A DUPLICATED MECHANIC THAT CANNOT FIRE. "
        "`gravity_flip_switch_system` (`actor_monolith/src/gravity/lifecycle.rs`) "
        "computes `base.dir = -base.dir` — the SAME expression as the encounter "
        "road's FlipGravity arm. It is registered exactly once in the workspace "
        "and that registration is `app.add_systems(Update, ...)` inside its own "
        "`#[cfg(test)]` module; `gravity/plugin.rs` says so in place — "
        "*\"`gravity_flip_switch_system` is intentionally NOT registered. Nothing "
        "spawns a `GravityFlipSwitch` in-game.\"* ⇒ A DEAD second authority, not a "
        "live one, so nothing can diverge today.\n"
        "    ⚠ BUT IT IS NOT FREE AND IT IS NOT MINE TO DELETE. "
        "`GravityFlipSwitch` is rollback-registered TWICE "
        "(`require_rollback` and `rollback_component_clone`, "
        "`actor_monolith/src/rollback_registration.rs`), so it sits inside the "
        "schema fingerprint two peers compare; `sim_view/src/facts.rs` rebuilds a "
        "view over its always-empty query every tick; and "
        "`rollback_exit_oracle.rs` names it by string. That whole vertical is "
        "`Q137`, which asks for a PRODUCT ruling on retiring a mechanic the "
        "shipped encounter `Switch` already owns — so this row stays and the "
        "verdict routes rather than collapses.\n"
        "    ⚠⛤ AND THE CENSUS COUNTS IT BECAUSE THE FUNCTION IS PRODUCTION CODE. "
        "The file is production, the `ResMut<BaseGravity>` parameter is "
        "production, and only its single CALLER is behind `#[cfg(test)]`. A "
        "writer census that reads parameter lists cannot see that, so "
        "\"multi-writer\" here is a fact about reachable DECLARATIONS and not "
        "about reachable writes — the same distinction that put eight types on "
        "this shortlist for fixtures alone before the test-region cut landed."
    ),
    "MovingPlatformSet": (
        "CORRECT — ONE KINEMATICS ADVANCER, TWO MEMBERSHIP OWNERS OVER DISJOINT "
        "ID NAMESPACES, ONE SESSION CLEAR. Four files, four sites. "
        "`advance_moving_platforms` (`actor_monolith/src/avatar/body_integration.rs`) "
        "is `for platform in platforms.0.iter_mut() { platform.update(sim_dt) }` "
        "and nothing else — it moves each platform along its own motion and never "
        "touches MEMBERSHIP, so it cannot contest the other two. "
        "`apply_world_replacement` (`actor_monolith/src/world/rooms/transaction.rs`) "
        "assigns `platforms.0 = pending.moving_platforms`, the authored set for "
        "the room being published, and the publication FAILS CLOSED if the "
        "composition holds no resource to publish into "
        "(`StagedWorldViolation::NoPlatformStateToPublishInto`, added because "
        "`get_resource_mut` answering `None` produced *\"a room the player falls "
        "through\"*). `hold_the_respawn_platforms` "
        "(`game/ambition_demo_smash/src/lib.rs`) owns exactly the "
        "`respawn_platform_` family: it `retain`s every id outside that prefix "
        "untouched and reconciles the ones inside it against which seats carry "
        "`RespawnGrace`. `SessionScopedResources::reset` is the session edge.\n"
        "    ⇒ The split between the two membership owners is a PREFIX "
        "CONVENTION, and it already has one owner for both halves: "
        "`RESPAWN_PLATFORM_PREFIX`, with `respawn_platform_id` formatting and "
        "`is_respawn_platform_id` parsing, collapsed there after *\"one "
        "convention, two literals\"* (`D-ID-CONVENTION-DRIFT`). MEASURED "
        "2026-09-18: no authored asset under `game/ambition_content/assets` "
        "spells that prefix, so the namespaces are disjoint today. ⚠ CONVENTION, "
        "NOT ENFORCEMENT — a room that authored `respawn_platform_0` would have "
        "it dropped on the first tick no seat wanted one, and nothing refuses "
        "that id.\n"
        "    ⚠ THE WHOLE-VECTOR ASSIGNMENT IS THE ONE INTERACTION, and it "
        "converges rather than fighting: a room publication drops every respawn "
        "platform, and the demo's rule re-derives them from `RespawnGrace` "
        "PRESENCE on its next run rather than from a latch, so the set comes "
        "back. The window is one tick. ⛔ I have NOT measured whether the smash "
        "demo can publish a room mid-match, so this is a convergence argument "
        "about the rule and not a claim that the window is unreachable.\n"
        "    ⭐ AND THE ADVANCER CANNOT DISAGREE ABOUT A RESPAWN PLATFORM'S "
        "POSITION EITHER, which is why the pair needs no ordering edge: the demo "
        "builds them with `from_sweep(.., 0.0, 0.0)`, so `min_x == max_x` and "
        "`speed == 0`, and `MovingPlatformState::update` adds `0.0 * dir * dt` "
        "and reverses at neither bound — position unchanged, `last_delta` zero. "
        "The zero sweep is stated in place as *\"a sweep of zero width at zero "
        "speed\"* rather than a still variant, and it is what makes the advance a "
        "no-op on that family."
    ),
    "VersusMatch": (
        "ROUTED — TWO WHOLE-RESOURCE WRITERS, AND ONE OF THEM IS THE SURVIVOR OF "
        "A REPAIR THAT SAID IT HAD REMOVED THE CLASS. Two files, two sites, and "
        "both assign the whole resource, which the census page calls *\"the least "
        "separable shape there is\"*. `settle_versus_round` "
        "(`game/ambition_app/src/app/versus_rules.rs`) is the ruleset: registered "
        "`.in_set(CombatSet::Settle)` in the SIM schedule, `run_if` the versus "
        "stage is the active route. `track_versus_roster` "
        "(`game/ambition_app/src/app/versus.rs`) writes `*match_state = "
        "VersusMatch::opening()` on exactly one arm — entering "
        "`VERSUS_GAMEPLAY_ROUTE` with no roster published by this experience — so "
        "it is the match-OPEN reset, not a second ruleset. ⇒ They cannot disagree "
        "about a value: one means *a match is beginning* and the other advances "
        "the one that began.\n"
        "    ⛔⛤ WHAT IS WRONG IS THE SCHEDULE, AND THE FILE ALREADY CARRIES THE "
        "ARGUMENT AGAINST ITSELF. The registration beside the rollback "
        "declaration says the ruleset is on the sim schedule *\"ALL of them\"*, "
        "because a KO-card system in `Update` used to mutate this resource — "
        "*\"calling a system 'the presentation half' does not make the resource it "
        "writes presentational.\"* `track_versus_roster` is forty lines below that "
        "comment, registered into `Update`, writing the same resource. The repair "
        "removed one `Update` writer and read as removing the class.\n"
        "    ⚠ AND IT IS NOT A LINE TO MOVE. It stays in `Update` for a stated "
        "reason — the teardown it is chained with must outlive "
        "`GameplaySimulationRoot`, or leaving gameplay disables the cleanup for "
        "that very transition. So the fix is a PROOF, not a relocation: it is "
        "banked as MENU-RESET-MIDSESSION in "
        "`scripts/check_rollback_mutators_run_in_sim.py`, which owes a probe "
        "showing the write precedes the timeline it would otherwise mutate.\n"
        "    ⛔ THE EXPOSURE IS PEER-COMPARED, NOT RESTORE-LOCAL, which is what "
        "separates this from the other `Update` resets: the registration is "
        "`rollback_resource_clone_checksum` with `versus_match_checksum` over the "
        "round, the phase and the per-team wins, so the row FEEDS the peer "
        "checksum. A `SessionScopedResources`-style waiver cannot cover it — the "
        "reset-before-the-timeline arguments in that guard's WAIVERS are about "
        "rows that are restored, not rows two peers compare.\n"
        "    ⇒ ROUTED rather than CORRECT, and the owed work is the probe, not a "
        "verdict: sample `session_world_entity`, `SessionSeatingSource` and "
        "`AmbitionGgrsSession` on the frame the `(true, false)` arm fires. The "
        "waiver next door was written on the false premise that "
        "`maintain_local_session` gates on a live BODY, and it does not — see "
        "that guard's comment for what replaced it."
    ),
    "RoomTransitionCooldown": (
        "CORRECT — ONE TICKER, TWO ARMERS, THREE CLEARS, AND THE SEVENTH WRITER "
        "WAS NOT A WRITER AT ALL. One `f32` countdown (`remaining`, "
        "`shared_tangle/src/safe_position.rs`), and the reason it reads as "
        "contested is that six systems each own one EVENT in its life rather "
        "than one value. The ticker: `tick_room_transition_cooldown` "
        "(`actor_monolith/src/control/input_systems.rs`) is "
        "`remaining = (remaining - wall_dt).max(0.0)` and nothing else. Two "
        "ARMERS, and they arm for different reasons: "
        "`RoomClock::...` in `runtime/src/room_transition/commit.rs` sets it on a "
        "crossing, conditional on `edge_exit`, and "
        "`reload_ldtk_world_from_disk` (`game/ambition_app/src/app/dev_runtime.rs`) "
        "sets `0.10` after a dev hot-reload. Three CLEARS to zero on three "
        "distinct lifecycle edges: `process_new_game_reset_request` "
        "(`actor_monolith/src/session/reset/mod.rs`), "
        "`return_the_replay_subject_to_spawn` "
        "(`runtime/src/sandbox_reset.rs`, which also assigns `default()` — the "
        "same zero), and `SessionScopedResources::reset`. ⇒ An armer and a clear "
        "cannot disagree about a VALUE; the countdown's only invariant is that it "
        "reaches zero, and every writer either starts it, ends it, or walks it "
        "down.\n"
        "    ⛔⛤ AND THE SEVENTH WAS A MUTABLE BORROW FEEDING A PARAMETER NOBODY "
        "READ — REMOVED 2026-09-18, WHICH IS WHY THE BASELINE SAYS SIX. "
        "`apply_player_hit_events` (`ambition_damage/src/lib.rs`) took "
        "`ResMut<RoomTransitionCooldown>` and its only use of the resource is the "
        "read `remaining > 0.0`, which becomes `SafePositionContext { "
        "room_transitioning, .. }`. The `ResMut` existed solely to pass `&mut` "
        "into `handle_player_damage_events`, whose signature spelled that "
        "parameter `_sim_state` — an underscore, so the compiler had already been "
        "told it was unused. The parameter is gone and the system asks for `Res`.\n"
        "    ⚠ TWO COSTS, AND ONLY ONE OF THEM WAS THE CENSUS'S. An exclusive "
        "borrow serialises the damage system against every real writer of the "
        "cooldown in the same schedule, for a read. And a writer census that "
        "reads PARAMETER LISTS counted it as one of seven authorities over a "
        "value it never touches — which is the same lesson as `BaseGravity`'s "
        "sixth writer from the other side: the instrument measures MUTABLE REACH, "
        "and reach is not authorship.\n"
        "    ⭐ THE TELL IS MACHINE-CHECKABLE, SO IT WAS SWEPT RATHER THAN "
        "RECOMMENDED — AND THE ANSWER IS A NEGATIVE WORTH RECORDING. An "
        "`_`-prefixed `&mut` parameter over the 1,294 production files: EIGHT "
        "remain after this repair, and none is this defect. Four are cheap or "
        "dictated (`_context: &mut LoadContext` is an asset-loader trait method; "
        "three `_commands: &mut Commands`, which no system holds exclusively). "
        "Two name COMPONENTS and bundles (`_anim: &mut BodyAnimFacts`, `_writers: "
        "&mut BodyDeathWriters`), which this census is silent about by "
        "construction. One is a render-app hook (`_render_world: &mut World`). And "
        "one is DELIBERATE with its reason in place: `load_character_sprites_in` "
        "keeps `_layouts: &mut Assets<TextureAtlasLayout>` because *\"this is still "
        "where a caller proves it HAS an asset pipeline, and dropping them would "
        "silently make the art-free path look identical\"*. ⇒ Do not re-run this "
        "sweep expecting a list; it was one."
    ),
    "SlotControls": (
        "CORRECT — THREE COMMITTERS, ONE PER HOST AND MUTUALLY EXCLUSIVE; THREE "
        "SHAPERS THROUGH ONE HELPER; ONE DRIVER SEAM. Six files, seven sites, and "
        "the table is documented as *\"WHAT EACH SEAT ACTUALLY RECEIVED — the "
        "committed frame every body reads through its `DrivingParticipant`\"*, so "
        "one wrong publisher is a body that moves on the wrong input rather than "
        "a value nobody reads.\n"
        "    THE THREE COMMITTERS. `publish_seat_controls_when_nobody_else_does` "
        "(`actor_monolith/src/schedule/input_systems.rs`) copies raw -> slots for "
        "a FRAME-STEP composition and stands down whenever anything else "
        "publishes — the predicate is `another_authority_publishes`, "
        "`latches.is_some() || rollback.is_some()`. `publish_latched_slot_controls` "
        "(same file) drains the latch on the TICK clock, installed only when "
        "`install_latched_slot_publication` finds `app.sim_is_fixed_tick()`. "
        "`publish_ggrs_input` (`rollback_ggrs/src/session.rs`) publishes the "
        "session's CONFIRMED inputs, `.before(CoreSimulation)` in `GgrsSchedule`. "
        "Its own doc states the split: *\"THREE hosts publish a seat's frame and "
        "only one of them is this\"* — fixed-tick, rollback, frame-step. ⇒ Not a "
        "race: two of the three are gated on a predicate and a schedule label, "
        "and the third is only registered by the GGRS plugin. See "
        "`SlotControlLatches` for the drain half of the same exclusivity, and for "
        "how nearly the schedule-label half reads the wrong way.\n"
        "    THE THREE SHAPERS ARE ONE ROAD. `derive_slot_direction_gestures` "
        "(`actor_monolith/src/control/input_systems.rs`), "
        "`apply_player_reset_input_system` "
        "(`game/ambition_app/src/app/sim_systems.rs`) and `warp_portal_input` "
        "(`game/ambition_content/src/portal/ability_adapter.rs`) all edit through "
        "`shape_seat_frame` (`actor_monolith/src/control/queries.rs`), which "
        "READS the authoritative table via `seat_frame_this_tick` and WRITES "
        "both: *\"which table holds the tick's input depends on the host; which "
        "table a shaped value must reach does not.\"* Writing the non-"
        "authoritative one is harmless because its owner overwrites it; writing "
        "one only loses a host. ⇒ A shaper cannot contest a committer, because it "
        "starts from whatever the committer published.\n"
        "    ⭐ AND THAT HELPER STATES ITS OWN CEILING — *\"MIGRATION "
        "INFRASTRUCTURE. DO NOT MAKE THIS THE PERMANENT MODEL, AND DO NOT ADD "
        "CLIENTS. Three systems use it\"* — with the staged replacement written "
        "out (`SeatInputProposal -> ConfirmedSeatInput -> EffectiveSlotControls`). "
        "MEASURED 2026-09-18: exactly THREE call sites in the workspace, so that "
        "completeness claim is TRUE, which is worth saying because the one in "
        "`versus.rs` forty lines from its own counterexample was not. ⇒ The thing "
        "to guard here is the ceiling, not the count: a fourth client is the "
        "signal to build the stages, and nothing mechanical refuses one.\n"
        "    THE SIXTH IS THE DRIVER SEAM, `runtime::input_drive::drive_slot_frame`, "
        "and it is a LAST RESORT by construction: it writes `SlotControls` only "
        "when the composition has neither a latch nor (under the rollback "
        "wrapper) a `PendingSeatInputs`, which is the smallest headless fixture. "
        "Collapsed onto one owner 2026-09-18 — the rollback crate carried a "
        "duplicate of this arm."
    ),
    "QuestRegistry": (
        "CORRECT — AN APPEND-ONLY QUEUE WITH FIVE PRODUCERS AND ONE DRAIN, AND "
        "THE SPLIT IS THE COMPILER'S RULE SINCE 2026-09-18. Eight files, nine "
        "sites, and `--shared-targets` separates them cleanly rather than by "
        "argument: FOUR files reach only `push_event` "
        "(`ambition_boss_encounter/src/systems.rs::update_boss_encounters`, "
        "`ambition_encounter_features/src/systems.rs::{drive_wave_encounters, "
        "apply_wave_encounter_effects}`, "
        "`actor_monolith/src/features/ecs/effect_bus.rs::{apply_flag_effects, "
        "apply_quest_effects}`, "
        "`actor_monolith/src/quest/mod.rs::push_room_entered_quest_events`); TWO "
        "reach only `quests` — `apply_quest_advance_events` "
        "(`ambition_persistence/src/quest/registry.rs`), the one reducer, and "
        "`populate_quest_registry` (`game/ambition_content/src/quest.rs`), which "
        "installs the authored DEFINITIONS; and TWO replace the whole resource, "
        "`process_new_game_reset_request` and `SessionScopedResources::reset`. ⇒ "
        "Five appenders cannot disagree about a value, and order is preserved "
        "because the drain is `std::mem::take` of one `Vec` consumed in "
        "insertion order.\n"
        "    ⛤ AND THE ENFORCEMENT WAS MISSING, WHICH IS THE ONLY THING THIS "
        "VERDICT CHANGED. `pending_events` was a `pub` field, so *\"every producer "
        "uses `push_event`\"* was an observation about today's code rather than a "
        "rule — the difference between this and `ClassBRemapLog`, whose private "
        "`Vec` makes its nine appenders a compiler fact. MEASURED FIRST: the field "
        "was touched outside its module in exactly ONE place, a `#[cfg(test)]` "
        "helper, so the change was a read accessor and one call site. Now private, "
        "poison-verified — restoring the direct access fails with `error[E0616]: "
        "field `pending_events` ... is private`.\n"
        "    ⚠ `quests` IS STILL `pub`, AND THAT IS A CHOICE WITH A NUMBER ON IT: "
        "25 sites across 7 files read it, four of them outside this crate (the "
        "save projection, content authoring, the HUD, a quest condition). It is a "
        "read-mostly map rather than an append-only channel, so encapsulating it "
        "buys much less and costs accessors on every reader. ⇒ Recorded rather "
        "than done; the queue was the half where a second writer could reorder or "
        "drop somebody else's event."
    ),
    "OwnedItems": (
        "ROUTED — TEN WRITER FILES COLLAPSE TO A HANDFUL OF IMPLEMENTATIONS, AND "
        "WHAT IS OPEN IS THE SCHEDULE OF TWO OF THEM. The type is already "
        "encapsulated: `counts: [u32; ITEM_COUNT]` is PRIVATE "
        "(`ambition_items/src/lib.rs`) with exactly three `&mut self` methods — "
        "`grant(item, n)`, `take(item, n) -> u32` and `apply_persisted(&[..])` — "
        "so no writer can reach the array. ⇒ Grants are per-item additions and "
        "COMMUTE; `take` is saturating and returns what actually came out, so two "
        "takers of a short bag cannot both succeed and silently double-spend.\n"
        "    ⭐ AND THREE PAIRS OF WRITERS ARE ONE IMPLEMENTATION EACH, which the "
        "file-granular census cannot see — the same shape as "
        "`ProjectileSeqCounter`. `open_ecs_chests` (`features/ecs/chests.rs`) "
        "delegates to `pickups::grant_pickup`, the road `collect_ecs_pickups` "
        "already takes, and says why in place: *\"Teaching the chest a second copy "
        "would be four payload kinds to keep in agreement forever.\"* Both "
        "inventory backends — `grid_menu_action_activated` "
        "(`game/ambition_app/src/menu/grid_backend.rs`) and "
        "`kaleidoscope_menu_action_activated` "
        "(`game/ambition_app/src/menu/kaleidoscope_app.rs`) — reach "
        "`menu::dispatch::dispatch_menu_action`. And the shop\'s two directions "
        "are `ambition_items::shop::{buy, sell}` behind one "
        "`ShopTransactionRequested::apply`. The rest: `apply_item_grants` "
        "(narrative), `grant_pirate_treasure_reward` (quest rewards), "
        "`pickup_portal_gun_system`, `throw_held_item_system` (the one gameplay "
        "take), and three wholesale roads — `reset_inventory_on_new_game` and "
        "`restore_inventory_from_save` through `apply_persisted`, and "
        "`restore_owned_items_to_checkpoint` through "
        "`reduce_owned_items_to_baseline`, which is `*owned = baseline.clone()`.\n"
        "    ⛔ WHAT KEEPS THIS ROUTED IS THE MENU PAIR, AND BOTH ARE ALREADY "
        "BANKED. `grid_menu_action_activated` and "
        "`kaleidoscope_menu_action_activated` are two of the nine acknowledged "
        "offenders in `scripts/check_rollback_mutators_run_in_sim.py`, both owed "
        "to MENU-RESET-MIDSESSION: they write rollback-registered state from a "
        "schedule that does not rewind. ⭐ The useful half for whoever closes that "
        "row is above — TWO banked offenders, ONE road. A fix inside "
        "`dispatch_menu_action` reaches both backends, and there is no second copy "
        "to keep in step.\n"
        "    ⚠ AND THE LIVE BAG IS NOT PEER-COMPARED, which is a separate open "
        "thing filed under Q129: `OwnedItems` is `rollback_resource_clone` — "
        "restored on a rewind, `feeds_peer_checksum() == false` — while "
        "`OwnedItemsBaseline(OwnedItems)` is `rollback_resource_clone_checksum` "
        "projecting `to_persisted()` rows. So a resimulation that diverges in the "
        "LIVE bag is invisible to the peer checksum until a checkpoint capture "
        "folds it into the baseline. That asymmetry is deliberate on the baseline "
        "side and unexamined on this one.\n"
        "    ⚠⛤ METHOD NOTE, because the obvious census is wrong twice: grepping "
        "`\\.(grant|take|apply_persisted)\\(` over these ten files reports FOUR takes "
        "in `kaleidoscope_app.rs` and an `apply_persisted` in "
        "`game/ambition_content/src/quest.rs`, and every one of those five is a "
        "different type — `Option::take` on an armed row and a capture, and "
        "`QuestState::apply_persisted`. A method name is not a receiver. Follow "
        "each writer to the expression that changes `counts` instead."
    ),
    "AuthoredOccurrences": (
        "CORRECT — FIVE WRITER FILES ONTO FOUR `&mut self` METHODS, AND THE ENTRY "
        "RULE IS ENFORCED INSIDE THE TYPE. `rows: BTreeMap<SimId, "
        "OccurrenceWhereabouts>` is PRIVATE "
        "(`shared_tangle/src/lifecycle/continuity.rs`) and there are exactly four "
        "ways to change it: `republish_custody(carried)` — THE ONE ENTRY ROAD, "
        "taken by `project_custody_onto_authored_occurrences` off `InCustodyOf`; "
        "`republish_placements(room, placements)` — the whereabouts updater, "
        "taken by `record_placed_ground_items` (`ambition_held_items/src/lib.rs`); "
        "`adopt_rows(rows)` — wholesale, taken by the durable-load and rewind "
        "roads (`adopt_occurrence_checkpoint_from_save` and `adopt_the_ledger` in "
        "`actor_monolith/src/session/durable_horizon.rs`, "
        "`restore_occurrence_baseline` in the same continuity module); and "
        "`forget_everything()` — the clear, taken by "
        "`process_new_game_reset_request` and `SessionScopedResources::reset`.\n"
        "    ⭐ THE UPDATER CANNOT BECOME AN ENTRY, AND THE TYPE IS WHAT STOPS IT. "
        "`republish_placements` inserts only where the existing row is "
        "`InCustody` or `Placed`, collects every other id into a `BTreeSet` and "
        "is `#[must_use]` — *\"a silent veto here would delete an occurrence from "
        "the durable world and look like nothing happening, so the caller is made "
        "to say what it means by them.\"* The producer agrees from its side: it "
        "skips anything the ledger does not already remember, because *\"an object "
        "cannot change rooms without being carried.\"* ⇒ Q141's claim that this "
        "ledger has *\"exactly ONE entry road\"* HOLDS — CHECKED 2026-09-18 rather "
        "than quoted, which is worth saying on a day two other completeness "
        "claims in this tree turned out one true and one false.\n"
        "    ⛔⛤ AND THE REGISTRATION HISTORY IS THE OPPOSITE OF A FIX TO REACH "
        "FOR. This was `declare_rollback_derived_resource` — in no snapshot — "
        "while `adopt_rows` was already a non-rederived producer, which its own "
        "`rewind_argument` had named as the trigger for becoming registered value "
        "state. The probe was PRESENCE-ONLY, so it reported `count: 1, xor: 0` "
        "whatever the ledger held: the promise was checked for EXISTENCE and never "
        "for TRUTH. ✅ Repaired to `rollback_resource_clone_checksum` over a "
        "domain-separated fold of `(SimId, whereabouts)`, "
        "`GGRS_ROLLBACK_SCHEMA_VERSION` 194 -> 195, poison-verified by reverting "
        "to `Derived` and watching the desync and the tick-7 stall come back. ⇒ "
        "Do NOT read \"make it derived\" as the answer here; that WAS the defect.\n"
        "    ⚠ WHAT IS STILL OPEN IS A PRODUCT QUESTION, NOT AN AUTHORITY ONE. "
        "`Q141` asks whether a runtime-spawned ground item may be durable at all — "
        "an object that arrives already lying on the ground and is never picked up "
        "cannot be remembered, by construction of the single entry road. If the "
        "answer is yes it needs a SECOND entry point stated as deliberately as "
        "the first, and anything that gains one must stop carrying "
        "`SpawnedThisAttempt`, because \"the attempt reclaims it\" and \"the durable "
        "world remembers it\" are contradictory answers about one object."
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
    "AbandonedCheckpointOperation": (
        "A PRODUCER AND A SESSION-BOUNDARY RESET, THE SAME SHAPE ALREADY "
        "ESTABLISHED FOR OTHER SESSION-SCOPED TYPES. "
        "`abandon_failed_checkpoint_restore_system` "
        "(`crates/ambition_platformer2d_runtime/src/room_transition/loading.rs:1433`) "
        "takes the key off a failed load and leaves it here; its own doc "
        "(`:1422`) says the commit executor spends it. "
        "`reset_checkpoint_coordinator_on_activation` "
        "(path: crates/ambition_platformer2d_actor_monolith/src/session/"
        "checkpoint.rs:1733) "
        "does `*abandoned = AbandonedCheckpointOperation::default()` (`:1755`) "
        "only on activation of a NEW session — its own doc: \"the session about "
        "to read these writes them first, so nothing a previous session left "
        "can reach it.\" Not a live-frame race. Measured by CalculexAmbition, "
        "2026-09-18; citations re-derived here before landing."
    ),
    "ActiveGameplaySession": (
        "PRODUCER AND TEARDOWN, NOT TWO ACTIVATION AUTHORITIES. "
        "`adopt_candidate_platformer_session` "
        "(`crates/ambition_platformer2d_provider/src/lifecycle.rs:2009`) calls "
        "`active_session.adopt_world(...)`, which that method's own doc "
        "(`crates/ambition_game_shell/src/session.rs:236`) calls \"the only "
        "road a gameplay session's world reaches this resource by.\" "
        "`translate_shell_session_lifecycle` (`session.rs:566`) calls "
        "`.retire_if_activation(...)` on route deactivation — the teardown "
        "half. Measured by CalculexAmbition, 2026-09-18; the doc citation "
        "corrected here from :234 (the struct) to the method that carries it."
    ),
    "ReservedGameplayScopes": (
        "A THREE-OP LEDGER, EACH OP OWNED BY EXACTLY ONE FILE. "
        "`crates/ambition_game_shell/src/session.rs:155-176` declares "
        "`.reserve()`, `.take()`, `.release()`. `.reserve()` is called only at "
        "`crates/ambition_platformer2d_provider/src/lifecycle.rs:1865` "
        "(candidate prepared); `.release()` only inside `lifecycle.rs`'s "
        "`release_candidate`/`release_candidate_in_world` (abandon/refuse "
        "paths); `.take()` only at `session.rs:672` (route activates, "
        "consuming the reservation). Disjoint operations, not a shared "
        "decision. Measured by CalculexAmbition, 2026-09-18."
    ),
    "ShellActivationGates": (
        "A `BTreeMap<ShellHoldId, SystemId>` "
        "(`crates/ambition_game_shell/src/router.rs:215`), SHARDED BY KEY — AND "
        "THE FILE SAYS SO ITSELF rather than leaving it to be inferred from the "
        "call sites. `router.rs:208-212`: \"KEYED BY HOLD ID, NOT BY ROUTE. A "
        "participant registers ONE evaluator and holds whichever route its "
        "transaction is on — and a transaction-specific hold id "
        "(`content-publication:<request-id>`) is what keeps a stale "
        "transaction's cleanup from freeing its successor's block on the same "
        "route.\" The two registrars key disjointly: "
        "`game/ambition_content/src/reload.rs:1611` keys on "
        "`publication_hold_for(&requested_by)` (`:1602`), and "
        "`crates/ambition_platformer2d_provider/src/lifecycle.rs:1877` keys on "
        "`candidate_session_hold(activation_id)` (`:1875`). "
        "`census.shared_targets` names `register`/`forget` as touched by both "
        "files but MUTATION-SHAPED IN NEITHER — both are method calls, not "
        "field writes. Measured by CalculexAmbition, 2026-09-18; the struct "
        "citation corrected here from :204 to :215, and the stated invariant "
        "cited in place of the inference."
    ),
    "ContentEpochSequence": (
        "THE SOLE MUTATOR IS `allocate()` "
        "(`crates/ambition_platformer2d_runtime/src/content_identity.rs:112`), "
        "A MONOTONIC COUNTER INCREMENT. The type's own doc says comparing two "
        "epochs is a compile error, and the only question an epoch answers is "
        "whether it is the generation something was planned against — any "
        "caller minting a fresh unique id is conflict-free regardless of "
        "interleaving. `census.shared_targets` is empty. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "RoomConstructionPlanPrefetch": (
        "A PRODUCER/CONSUMER CACHE, NOT TWO FILLERS. "
        "`game/ambition_app/src/app/world_flow/room_transition_assets.rs:1407`'s "
        "required `ResMut` builds and fills the plan — its own doc (`:107`) "
        "says \"the PREFETCH builds the same plan the transition will "
        "commit.\" "
        "`crates/ambition_platformer2d_runtime/src/room_transition/loading.rs:642`'s "
        "`Option<ResMut<...>>` only calls `cache.promote(...)` (`:1166`) — a "
        "cache-hit take/evict, never a fill. `census.shared_targets` is "
        "empty. Measured by CalculexAmbition, 2026-09-18."
    ),
    "RoomContentStagingRegistry": (
        "A KEYED REGISTRY, DISJOINT KEYS ON EACH SIDE. "
        "`game/ambition_content/src/plugin.rs:99-103` registers under the "
        "\"duel\" tag via `register_duel_content_staging`; "
        "`crates/ambition_sim_harness/src/runtime.rs:821`'s `stage_actor` "
        "registers under `\"sim-harness\"`/`\"scenario:{id}\"`. The sim-harness "
        "crate additionally composes its own separate App/World per its own "
        "doc (\"a caller-supplied composition closure, so tests, RL agents, "
        "and fuzz drivers can run game simulation without linking the shipped "
        "game binary\"). `census.shared_targets` is empty. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "LocalSeatTopology": (
        "WORLD-INSTANCE SEPARATION, NOT CRATE-LINKAGE ABSENCE. "
        "`crates/ambition_sim_harness` IS an optional dependency of the "
        "shipped game (`game/ambition_app/Cargo.toml:102`, "
        "`rl_sim = [\"dep:ambition_sim_harness\"]`), so \"never linked\" would "
        "be the wrong claim. The harness \"accepts a caller-supplied "
        "composition closure\" and owns its own composed App/World instance "
        "even when linked in, so its write "
        "(`crates/ambition_sim_harness/src/runtime.rs:199`) is never the same "
        "resource INSTANCE as the shipped game's live World that "
        "`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs` "
        "manages. `census.shared_targets` is empty. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "DefaultMusicStarted": (
        "IDEMPOTENT CONVERGENT WRITES: BOTH ONLY EVER MOVE IT false -> true. "
        "`start_default_music_when_ready` "
        "(`crates/ambition_audio/src/library.rs:544`) self-guards "
        "(`if started.0 { return; }`). `apply_frontend_music_policy` "
        "(`crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs:329`) "
        "sets it true behind the same `Local<Option<AudioContextOwner>>` "
        "change-latch already adjudicated CORRECT for the sibling "
        "`MusicPlaybackState`/`RadioStationState`. The sole false-reset is a "
        "third site (`plugin.rs:349`) inside the same shared context-reset "
        "function those siblings already use. Measured by CalculexAmbition, "
        "2026-09-18."
    ),
    "MusicDirectorState": (
        "SAME GATE AS `MusicPlaybackState`, ALREADY ADJUDICATED. Mutated only "
        "inside `drive_music_director` "
        "(`crates/ambition_audio/src/music/director/mod.rs:45`) and "
        "`apply_frontend_music_policy` "
        "(`crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs:329`) "
        "— the same two functions and the same "
        "`.run_if(simulation_authorized)` / owner-change-latch mutual "
        "exclusion. Measured by CalculexAmbition, 2026-09-18."
    ),
    "SfxBankRegistry": (
        "A KEYED COLLECTION WITH A BUILT-IN COLLISION GUARD. "
        "`register(provider_id, ...)` "
        "(`crates/ambition_audio/src/bank_asset.rs:117`) is per-`provider_id`, "
        "and checks fingerprint agreement when a `provider_id` re-registers "
        "(`:123-125`). `publish_resident_sfx_bank_authority` "
        "(`game/ambition_app/src/app/setup_systems.rs:128`) registers one "
        "fixed provider id, gated by a `Local<bool>` that fires once ever. "
        "`promote_loaded_sfx_bank` (`bank_asset.rs:375`) registers whichever "
        "other providers are in its own pending-handles queue as they finish "
        "loading. Disjoint keys plus a same-key consistency check. Measured "
        "by CalculexAmbition, 2026-09-18."
    ),
    "SfxPlaybackState": (
        "A RESET-VERSUS-SET-ON-PLAY PAIR, TEMPORALLY DISJOINT ROLES. "
        "`audio_play_sfx_messages` "
        "(`crates/ambition_audio/src/bank_asset.rs:473`) sets "
        "`playback.last_played = Some(...)` per drained message. The other "
        "writer is the same context-reset function already established for "
        "the sibling audio types "
        "(`crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs:293`, "
        "sets `playback.last_played = None`). Measured by CalculexAmbition, "
        "2026-09-18."
    ),
    "CameraShakeState": (
        "A COMMUTATIVE MAX-COMBINE, ORDER-INDEPENDENT BY CONSTRUCTION. "
        "`apply_camera_shake_requests`/`tick_camera_shake` "
        "(`crates/ambition_platformer2d_shared_tangle/src/camera_ease.rs:229,248`) "
        "are message-kick and per-frame-decay. `sync_player_presentation` "
        "(`game/ambition_app/src/app/player_tick.rs:29`) reaches the same "
        "`kick()` method (`camera_ease.rs:193`) through a helper — "
        "`if target > self.amplitude_px { self.amplitude_px = target }` — so "
        "multiple simultaneous kicks converge on the loudest regardless of "
        "order. The type's own doc (`camera_ease.rs:203`) states the "
        "simulation must not touch it directly — a deliberate "
        "sim/presentation boundary. Measured by CalculexAmbition, "
        "2026-09-18."
    ),
    "PortalCameraContinuityState": (
        "ALREADY REASONED THROUGH BY THE MAINTAINERS. "
        "`crates/ambition_render/src/rendering/camera.rs:203-204`'s own "
        "comment: portal camera continuity is one global for the whole "
        "process, and the writes are last-camera-wins once a composition "
        "really has two. An accepted design, not an oversight. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "PortalViewConeDebugDumpRequest": (
        "TWO INPUT SURFACES FOR ONE USER ACTION, SAME METHOD. Both writers "
        "call the identical `request(reason)` "
        "(`crates/ambition_portal2d_presentation/src/view_cones.rs:167`, sets "
        "`pending = true` and `reason`) — a hotkey "
        "(`crates/ambition_portal2d_presentation/src/view_cones/debug.rs:19`, "
        "reason \"Shift+F8\") and an inspector button "
        "(`game/ambition_app/src/dev/portal_inspector.rs:595`, reason "
        "\"portal inspector\"). Only the display-only `reason` label could "
        "differ under a same-frame double-fire a human cannot produce; the "
        "dump's occurrence never does. Measured by CalculexAmbition, "
        "2026-09-18. ⚠ THREE OF THE FOUR CITATIONS IN THE DRAFT OF THIS ROW "
        "POINTED AT THE WRONG PLACE and were corrected here against the tree: "
        "`request` is in `view_cones.rs`, not its `view_cones/debug.rs` "
        "sibling (the line number happened to match in both); the Shift+F8 "
        "writer is in the presentation crate, not `portal_inspector.rs`; and "
        "the inspector button is `:595`, not `:323`. The conclusion was right "
        "and its addresses were not — re-derive before citing this row."
    ),
    "ScrollbarDragState": (
        "TWO-BACKEND MUTUAL EXCLUSION, SAME SHAPE AS `KaleidoscopeCursor`. "
        "The struct's own doc (`crates/ambition_menu/src/lib.rs:222-225`) "
        "states it: \"Shared by BOTH renderers (only one menu is active at a "
        "time), so it lives in the shared model.\" The grid backend's scroll "
        "install is gated `.run_if(grid_backend_active)` "
        "(`game/ambition_app/src/menu/grid_backend.rs:1193`); the cube's "
        "`scrollbar_press` (`game/ambition_menu_kaleidoscope/src/lib.rs:1209`) "
        "additionally gates on `bars.get(press.entity).is_ok()`, matching "
        "only the cube's own `MenuScrollbar` entities, and "
        "`scrollbar_release` (`:1231`) compare-guards on "
        "`pressed_by == Some(pointer_id)` before clearing. Only one "
        "backend's entity tree exists at a time. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "SeatActiveDevices": (
        "TWO SIGNAL LAYERS ASSERTING THE IDENTICAL FACT, IDEMPOTENT. Both "
        "writers call `mark_primary(ActiveDevice::Touch)` — "
        "`update_seat_active_devices` "
        "(`crates/ambition_input/src/active_input.rs:191`) gated on raw "
        "touch input being live, and `fold_touch_gestures` "
        "(`crates/ambition_touch_input/src/menu_bridge.rs:31`) gated on "
        "folded touch gestures. Both only ever write `Touch` conditioned on "
        "touch actually being live, so they can never disagree. "
        "`census.shared_targets` names `mark_primary` as a method, not a "
        "field. Measured by CalculexAmbition, 2026-09-18."
    ),
    "SeatControlFrameModes": (
        "A DOCUMENTED STAND-IN FOR A SYSTEM THAT DOES NOT RUN IN THIS "
        "COMPOSITION. `set_movement_frame_mode` "
        "(`crates/ambition_sim_harness/src/runtime.rs:706`) operates on the "
        "sim-harness crate's own private, caller-composed headless App (see "
        "`LocalSeatTopology`'s verdict for that crate's doc). Its own "
        "comment (`:722-726`) explains it writes this resource directly "
        "because the headless composition does not run the normal device "
        "stage, `populate_seat_control_frames` — same "
        "`crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs` "
        "file. Never coexists with the shipped game's own world. Measured "
        "by CalculexAmbition, 2026-09-18."
    ),
    "SeatMenuFrames": (
        "A STANDALONE `[[bin]]` SIMULATING THE DEVICE STAGE HEADLESSLY. "
        "`game/ambition_demo_smash_app/src/tools/select_walkthrough.rs` has "
        "its own `fn main` and builds its own headless app, writing this "
        "resource directly (`app.world_mut().resource_mut::<SeatMenuFrames>()`, "
        "around `:132`/`:143`) to simulate what the real device stage, "
        "`populate_seat_menu_frames` — same "
        "`crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs` "
        "file — would have produced. Never runs alongside the shipped game. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "ShellRouter": (
        "`cancel_pending` IS A METHOD, NOT A RAW FIELD — THE CENSUS FIELD-"
        "REGEX MISATTRIBUTES IT. Its only production callers are "
        "`crates/ambition_load_presentation/src/shell_adapter.rs:220` and "
        "`:231`, and `crates/ambition_game_shell/src/plugin.rs:341`, all "
        "calling `router.cancel_pending(&mut loads, &mut prepared)` "
        "(`crates/ambition_game_shell/src/router.rs:652`). The body "
        "(`:657-658`) is `self.pending.take().map(...)` — an idempotent "
        "consume: whichever call happens first empties `pending`; a second "
        "call is a guaranteed no-op since `.take()` on `None` returns "
        "`None`. `census.shared_targets` names `cancel_pending` itself as "
        "the only touched target, mutation-shaped in neither file — "
        "confirming it is a method call, not a field write. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "ShellSequenceCatalog": (
        "TWO DISJOINT App INSTANCES, NEVER BOTH ACTING ON THE SAME ONE. "
        "`game/ambition_app_tools/src/bin/preview_vanity_card.rs` has its "
        "own `fn main` and builds its own App via `configure_preview_shell` "
        "— a standalone preview tool, entirely separate from "
        "`compose_ambition_startup_sequence` "
        "(`game/ambition_app/src/app/shell_host.rs:233`), which configures "
        "the shipped game's own App. `census.shared_targets` is empty. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "BodyClocksView": (
        "AN INTENTIONAL ACCUMULATOR, DOCUMENTED AS SUCH. The struct's own "
        "`BodyClockViewSet` doc "
        "(`crates/ambition_sim_view/src/facts.rs:342-348`) states it "
        "outright: a Reset phase empties the view and a Contribute phase is "
        "where every mechanic pushes its own clocks and nothing else. "
        "`rebuild_body_clocks_view` (`facts.rs:355`) is the Reset-phase "
        "clearer; `publish_mark_clocks` "
        "(`game/ambition_demo_smash/src/mark.rs:304`) is one Contribute-"
        "phase pusher among however many exist, ordered after Reset via "
        "`.chain()`. Not a race. Measured by CalculexAmbition, 2026-09-18."
    ),
    "PresentationPhase": (
        "MUTUALLY EXCLUSIVE BY SIMULATION-BACKEND SELECTION. "
        "`sample_fixed_overstep_phase` "
        "(`crates/ambition_sim_view/src/presented_pose.rs:72`) is registered "
        "only for a non-rollback host (`:344`). "
        "`sample_ggrs_accumulator_phase` "
        "(`crates/ambition_platformer2d_rollback_ggrs/src/lib.rs:59`) is "
        "registered unconditionally inside the GGRS backend plugin's own "
        "`build()` (`:136`), installed only for a rollback host. A host "
        "selects exactly one simulation backend, so exactly one of these two "
        "systems ever exists in a given App — the type's own doc "
        "(`presented_pose.rs:32-42`) enumerates both host cases by name. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "ShrineActivationPulse": (
        "AN ARM/DECAY PAIR, SAME SHAPE AS `WorldSourceHotReload`'S "
        "`auto_apply` AND `DeveloperRuntimeState`'S `preset_flash`. "
        "`heal_save_shrine_system` "
        "(`crates/ambition_platformer2d_actor_monolith/src/shrine.rs:195`) "
        "arms it once per activation trigger (`activation.remaining = "
        "0.78`); `tick_shrine_activation_pulse` "
        "(`crates/ambition_sim_view/src/facts.rs:401-408`) decays it every "
        "frame toward zero, guarded on it being positive. "
        "`census.shared_targets` names `remaining` as mutation-shaped in both "
        "files, which is expected for an arm/decay pair — the roles are "
        "temporally disjoint, not competing. Measured by CalculexAmbition, "
        "2026-09-18."
    ),
    "EditablePortalTuning": (
        "ONE FIELD HAS ONE AUTHOR, PROVED BY A CHANGE-GUARD RATHER THAN A "
        "COMMENT. `sync_portal_reorient_from_settings` "
        "(`game/ambition_content/src/portal/transit_body_adapter.rs:124`) "
        "runs `if editable.reorient_facing != want { editable.reorient_facing "
        "= want; pending.propose(...) }` (`:135-137`) — it does nothing while "
        "the field already agrees with the persisted "
        "`settings.gameplay.portal_reverses_facing`, and republishes that "
        "persisted value the instant it does not, which is exactly what a "
        "developer-panel edit of that one field produces. The panel's own "
        "comment names the same relationship: \"THE MIRROR, NOT THE "
        "AUTHORITY\" (`game/ambition_app/src/dev/portal_inspector.rs:77`). "
        "So for `reorient_facing` specifically the gameplay setting is the "
        "author and the panel edit is deterministically reverted on the next "
        "pass — not a race, because only one side ever wins. The struct's "
        "other fields ARE panel-authored and this system touches none of "
        "them. `publish_editable_portal_tuning` "
        "(`crates/ambition_portal2d/src/tuning.rs:177`) is the only writer of "
        "the downstream `PortalTuning` authority itself. "
        "⛔⛤ AN EARLIER DRAFT OF THIS ROW CLOSED THE QUESTION BY QUOTING THE "
        "DOC BLOCK ABOVE THAT SYSTEM, which then said the field was authored "
        "\"by the settings menu and by the developer panel alike\". That "
        "sentence was FALSE for `reorient_facing` and was retracted in "
        "`38f05f5f9`; it no longer exists in the tree. A comment asserting "
        "resolution is not a measurement of one — the change-guard is. "
        "Measured by CalculexAmbition, 2026-09-18; rewritten against the "
        "current source before landing."
    ),
    "ActiveSessionScope": (
        "THREE DISJOINT OPERATIONS, NEVER CONTENDING ON THE SAME WRITE. The "
        "struct holds `current: Option<SessionScopeId>` and `next_raw: u64` "
        "(path: crates/ambition_platformer2d_shared_tangle/src/lifecycle/"
        "session.rs:37-40). "
        "`reserve()` (called from "
        "`crates/ambition_platformer2d_provider/src/lifecycle.rs:1869`) "
        "mints a fresh id by incrementing `next_raw` WITHOUT touching "
        "`current` — its own doc: \"Mint a fresh scope identity WITHOUT "
        "making it current\" — candidate prep must not activate before its "
        "verdict. The only functions that set `current` are "
        "`begin()`/`publish()`, both called exclusively from "
        "`crates/ambition_game_shell/src/session.rs`'s "
        "`translate_shell_session_lifecycle` (`:677`). `clear_if_current(id)` "
        "(`lifecycle/session.rs:97-101`) is compare-and-clear — only clears "
        "`current` if it still equals `id` — called from two independent "
        "retirement listeners, `session.rs:636` and "
        "`despawn_retired_session_entities` "
        "(`lifecycle/session.rs:781`), safe because whichever runs first "
        "clears it and the second is a guaranteed no-op. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "CaptureProgress": (
        "ONE LIBRARY SYSTEM PLUS FOUR STANDALONE `[[bin]]` TARGETS THAT NEVER "
        "COEXIST. The struct and its only field-writing system, "
        "`save_readback_to_disk` "
        "(`crates/ambition_render/src/capture.rs:159`), live in a shared "
        "library used by whichever capture tool links it; it sets "
        "`progress.completed`/`progress.failed` on the readback event. The "
        "other four writers — "
        "`game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs:245` "
        "(`shoot_when_warm`), the equivalent in `capture_sanic.rs` and "
        "`capture_twintrack.rs`, and "
        "`game/ambition_demo_smash_app/src/tools/match_shots.rs:147` "
        "(`shoot_when_asked`) — only ever set `progress.requested = true` via "
        "a shared `request_capture` helper. Each is its own `[[bin]]` process "
        "(`capture_mary_o.rs:53`, `capture_sanic.rs:46`, "
        "`capture_twintrack.rs:54` each declare `fn main`) or, for "
        "match_shots, a `pub fn run` "
        "(`game/ambition_demo_smash_app/src/tools/match_shots.rs:164`) that "
        "builds its own `crate::build_windowed_demo_app(...)`, called only "
        "from the separate smash_tool binary "
        "(`game/ambition_demo_smash_app/src/bin/smash_tool.rs:98`). No two of "
        "these five writers ever run in the same process. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "GameplayBanner": (
        "A SINGLE-SLOT, LAST-WRITE-WINS COSMETIC NOTIFICATION, DOCUMENTED AS "
        "INTENTIONAL, WITH NO DOWNSTREAM DECISION READER. The type's own doc "
        "(`crates/ambition_combat/src/events.rs:265-266`) states it outright: "
        "\"Gameplay systems either mutate this resource directly or emit "
        "`GameplayBannerRequested` when their parameter list is already "
        "large.\" `show()` (`:274-277`) is a plain overwrite of "
        "`text`/`timer`; ten call sites across boss/combat/interact/pickup/"
        "quest/reset systems call it directly, and "
        "`apply_gameplay_banner_requests` "
        "(`crates/ambition_combat/src/banner.rs:15`) drains the deferred "
        "message form. If two of these fire in the same frame, one banner "
        "text overwrites the other by schedule order — but the ONLY reader "
        "outside tests is a `Res<GameplayBanner>` field on the HUD/feedback "
        "bundle (`game/ambition_app/src/app/feedback.rs:43`), used purely for "
        "display. No system branches on `.text` to make a gameplay decision, "
        "so an arbitrary same-frame overwrite changes which of two transient "
        "flavor messages is shown for ~2 seconds, never game state. "
        "`census.shared_targets` confirms `show` is touched by ten files but "
        "mutation-shaped in NONE of them — a method call, not a field write. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "HudReadouts": (
        "A KEYED COLLECTION, DOUBLY SAFE. The struct "
        "(`crates/ambition_platformer2d_shared_tangle/src/gameplay_presentation/"
        "hud.rs:347`) is `by_slot: BTreeMap<HudSlotId, HudReadout>`; `set()` "
        "(`:351-353`) and `clear_slot()` (`:372-374`) each take the caller's "
        "own `HudSlotId` key. `publish_versus_hud` "
        "(`game/ambition_app/src/app/versus_rules.rs:578`) writes "
        "`ROUNDS_HUD_SLOT`/`ANNOUNCE_HUD_SLOT`; the other four writers each "
        "belong to a distinct demo — ambition_demo_mary_o, ambition_demo_"
        "sanic, ambition_demo_smash, ambition_demo_twintrack — and each demo "
        "is its own `[[bin]]` target under `game/ambition_demo_*_app`, so "
        "besides using disjoint slot keys, no two of these five writers ever "
        "run in the same process either. `census.shared_targets` names "
        "`set`/`clear_slot` as methods, not fields, touched but "
        "mutation-shaped in neither of the two files it lists. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "CausalRecording": (
        "AN APPEND-ONLY DIAGNOSTIC LOG WHOSE OWN DOC FORBIDS A GAMEPLAY "
        "READER. The wrapped log's doc "
        "(`crates/ambition_causal/src/log.rs:44-49`) states it outright: "
        "\"write-only from the simulation, read-only from a tool ... a "
        "decision that consulted it would desync.\" `census.shared_targets` "
        "returns the EMPTY SET over its nine writer files: every real "
        "write is a call to `record()` "
        "(`crates/ambition_causal/src/log.rs:157`), which appends and "
        "auto-assigns an id — commutative with any other domain's append, "
        "since nothing downstream branches on order. `stamp_causal_frame` "
        "is the SOLE writer of the tick/frame stamp `record()` applies, "
        "registered alone into `bevy::app::First` "
        "(`crates/ambition_platformer2d_runtime/src/causal.rs:26-27`), so "
        "no second file contends that half either — the "
        "`assert_no_offthread_loss` write site the census also lists "
        "(`:150-157`) is the string `\"ResMut<CausalRecording>\"` inside a "
        "panic message, not a real access. THREE of the nine writer files "
        "are separate tool targets that never coexist with the shipped "
        "game: rl_random_walker and moveset_takes are `[[bin]]` in "
        "`game/ambition_app_tools/Cargo.toml:114-116,147-149`, and "
        "`ladder_probe::trace_seam` "
        "(`game/ambition_demo_smash_app/src/tools/ladder_probe.rs:150-252`) "
        "builds its OWN `App` via `build_demo_app()` (`:396`), reachable "
        "only from smash_tool's `main` "
        "(`game/ambition_demo_smash_app/src/bin/smash_tool.rs:82,85`). "
        "Measured by CalculexAmbition, 2026-09-18; two independent "
        "investigations converged on this verdict, both catching the same "
        "panic-message false positive."
    ),
    "DialogState": (
        "A DECISION THAT IS SNAPSHOTTED BEFORE THE CONTENDED FIELD CAN "
        "MOVE AGAIN. `census.shared_targets` names `selected_option` and "
        "`pending_advance` as mutation-shaped in both "
        "`crates/ambition_dialog/src/systems.rs` and "
        "`crates/ambition_dialog/src/bridge.rs`; `close` and `current_line` "
        "are touched by both but mutation-shaped in NEITHER — `close` is a "
        "method call (`crates/ambition_dialog/src/runtime.rs:259-262`), not "
        "a field, and `current_line` is only ever `.clear()`-ed, which the "
        "assignment/`_mut` heuristic cannot see. The only place "
        "`selected_option` becomes a real DECISION is `confirm_or_advance`'s "
        "`self.pending_select = Some(self.selected_option.min(...))` "
        "(`crates/ambition_dialog/src/runtime.rs:490-493`): this COPIES the "
        "index into a separate `Option<usize>` field at the instant of the "
        "confirm press. From then on, whichever domain next writes "
        "`selected_option` — bridge.rs resetting it to 0 for a new line or "
        "option list (`bridge.rs:221,293,342,361`), or systems.rs continuing "
        "to navigate — cannot retroactively change the value already "
        "captured in `pending_select`. `dispatch_pending_dialog_requests` "
        "drains that snapshot exactly once with `.take()` "
        "(`bridge.rs:204`), inside one system's uninterrupted execution, so "
        "no other system can observe it mid-consumption. `pending_advance` "
        "is a plain sticky flag consumed the same way (`std::mem::take` at "
        "`bridge.rs:228`), idempotent under repeated `true` writes. This is "
        "a REBUILT design, not an accidental one: the type's own doc "
        "(`crates/ambition_dialog/src/runtime.rs:222-238`) records that "
        "three earlier close paths each reset a different field set with no "
        "two agreeing, unified into the single "
        "`clear_conversation_presentation` (`:239-254`) that every close, "
        "select and advance road now calls (`bridge.rs:242`, "
        "`runtime.rs:485`). Measured by CalculexAmbition, 2026-09-18."
    ),
    "GameAssets": (
        "A KEYED ASSET TABLE WHOSE ONE CONTENDED FIELD HAS EXPLICIT "
        "SCHEDULE ORDERING. `census.shared_targets` names exactly one field, "
        "`parallax_layers` "
        "(`crates/ambition_sprite_sheet/src/game_assets/mod.rs:455`), shared "
        "by `game/ambition_app/src/app/world_flow/parallax_residency.rs` and "
        "`game/ambition_app/src/app/world_flow/room_transition_assets.rs`, "
        "touched but mutation-shaped in NEITHER — both go through `&mut "
        "self` methods, not a field assignment. `ensure_theme_loaded` "
        "(`crates/ambition_sprite_sheet/src/game_assets/mod.rs:340-372`) "
        "only inserts a theme's layers when absent — idempotent and "
        "additive; `retain_themes` (`:396-400`) is the ONLY eviction path, "
        "and `retire_departed_parallax_themes` is registered "
        "`.after(prefetch_neighbor_room_preparation_system)` "
        "(`game/ambition_app/src/app/world_flow/room_transition_presentation.rs:274-275`), "
        "with the registration comment naming why: \"the prefetch may load a "
        "neighbour's theme this frame, and retiring before it would drop "
        "what it just decided to keep.\" Every other writer touches a "
        "DISJOINT named/keyed slot under `characters` or `entities`, guarded "
        "by an if-already-present early return — "
        "`game_assets.characters.sheet(AI_SLOP_DISPLAY_NAME).is_some()` "
        "(`game/ambition_demo_mary_o/src/ai_slop.rs:92`) versus "
        "`characters.props.contains_key(RING_SPRITE_KIND)` "
        "(`game/ambition_demo_sanic/src/lib.rs:847`) — the same "
        "check-then-insert-once shape as the already-adjudicated "
        "`HudReadouts`, keyed by sheet/prop name instead of a HUD slot. "
        "`game/ambition_app/Cargo.toml:157-158` shows the mary_o/sanic "
        "registration systems DO link into the shipped app, so their "
        "disjoint keys — not process separation — are what keeps them safe. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "PendingMechanicalEdits": (
        "A PER-DOMAIN KEYED LEDGER, REBUILT FROM A SINGLE-BOOL DEFECT ON "
        "RECORD. The type "
        "(`crates/ambition_platformer2d_core/src/movement/tuning.rs:317`) "
        "wraps a `BTreeSet<MechanicalDomain>` keyed by `TypeId`; its own doc "
        "(`:285-303`) narrates the exact defect this census flags — a shared "
        "bool where one domain's publish silently ate another's proposal — "
        "and states the fix: \"CENTRALIZE THE ADMISSION, NOT THE PROPOSAL "
        "IDENTITY ... a publisher drains ONLY its own.\" `propose` "
        "(`:387-389`) is a set insert, idempotent for repeat calls; `take` "
        "(`:408-410`) is `self.0.remove(&domain)`, provably scoped to the "
        "caller's own key by the `Eq`/`Ord`/`Hash` impls over `TypeId` "
        "(`:362-381`), which the type's doc (`:324-340`) explains was itself "
        "a fix for a string-label collision hazard the first draft had. "
        "RE-MEASURED 2026-09-18 by grepping the DEFINITION side "
        "(`MechanicalDomain::of::<`) rather than the writer list: SEVEN "
        "production domains, seven distinct markers — "
        "`FeelTuningDomain`, `DeveloperBodyProfileDomain`, "
        "`EditablePlayerStatsDomain`, `MovementTuningDomain`, "
        "`EditableAbilitySetDomain`, `PortalTuningDomain`, and "
        "`SpawnPlayerCloneDomain` "
        "(`game/ambition_app/src/app/player_clone.rs:55`, added with Q136's "
        "first landed ingress road). An eighth `of::<>` site, `FixtureDomain` "
        "(`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs:658`), "
        "sits under the `#[cfg(test)]` at `:610` and is not in this "
        "population. ⚠ The clone domain's first draft keyed on the live "
        "`SpawnPlayerCloneRequest` resource instead of a marker; unique by "
        "`TypeId`, but it would have stood as a precedent for keying on any "
        "type in scope, against the contract at `:333`. Changed to a marker "
        "before landing. The remaining writer, "
        "`game/ambition_content/src/portal/transit_body_adapter.rs:137`, "
        "deliberately reuses `PortalTuningDomain` alongside "
        "`crates/ambition_portal2d/src/tuning.rs:168` — inserting the same "
        "set element twice is still idempotent. The sole publisher, "
        "`publish_editable_portal_tuning` "
        "(`crates/ambition_portal2d/src/tuning.rs:176-195`), is the only "
        "writer of the actual authority (`*active = editable.0`) and the "
        "only caller of `.take()` for that domain. ⚠ The draft of this row "
        "also quoted `transit_body_adapter.rs:103-105` as calling the shape "
        "\"two authors of one authored value\"; that paragraph is correct "
        "for the OTHER fields but was retracted for `reorient_facing` in "
        "`38f05f5f9` — see the `EditablePortalTuning` row. The quote is "
        "dropped here; the idempotent-insert argument does not need it. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "BossEncounterRegistry": (
        "THREE WRITERS, TWO OF WHICH CONVERGE TO THE SAME DEFAULT AND ONE OF "
        "WHICH IS A RUN-ONCE LATCH. `populate_boss_encounter_registry` "
        "(`crates/ambition_boss_encounter/src/systems.rs:23`) only populates "
        "when `:27` finds `specs_loaded` false, and sets it true at `:35` "
        "and `:54` — a change-latch that re-arms only after a wipe. `reset` "
        "(`crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs:444`) "
        "is the SINGLE shared function both `:353` (session activation) and "
        "`:429` (session retire) call to set `*boss_registry = "
        "BossEncounterRegistry::default()` (`:493`) — one function with two "
        "temporally disjoint call sites, not two authorities. "
        "`process_new_game_reset_request` "
        "(`crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs:288`) "
        "does the same whole-resource reset at `:522`, only after "
        "`publication_succeeded`. Resetting to Default twice is idempotent, "
        "and the populate latch re-runs regardless of which wipe fired last, "
        "so no reader can observe an ambiguous order. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "CharacterLoadStates": (
        "THREE WRITER FILES, ONE OWNING FUNCTION. "
        "`materialize_character_demand` "
        "(`crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs:436`) "
        "is documented as \"the engine's ONE decode path\" and is the only "
        "place `record` (the by_token/cast write) is called. "
        "`demand_room_character_sheets` "
        "(`game/ambition_app/src/app/world_flow/room_transition_assets.rs:399`) "
        "calls it at `:468`, and that one helper is itself called from "
        "`game/ambition_app/src/app/startup_loading.rs:456` (initial room) "
        "and `room_transition_assets.rs:729` (room transition/prefetch) — "
        "many surfaces, one authority, the same shape as `UserSettings`' "
        "`apply_settings_option`. `retire_previous_session_cast` "
        "(`character_runtime/mod.rs:834`) is the only other writer and it "
        "touches only the disjoint `cast` field via `cast_mut()`/"
        "`enter_scope`, never `by_token`. `census.shared_targets` returns "
        "the empty set for this type, consistent with a field-private "
        "resource whose only cross-file contact is through the one owning "
        "function. Measured by CalculexAmbition, 2026-09-18."
    ),
    "CharacterLoadDemand": (
        "EVERY PRODUCER IS AN IDEMPOTENT SET-INSERT AND EVERY CONSUMER SITE "
        "IS ONE FUNCTION. `crates/ambition_characters/src/load_demand.rs:17` "
        "stores `pending: BTreeSet<String>`; `request`/`request_all` (`:27`) "
        "only ever inserts. `project_demand` "
        "(`crates/ambition_match/src/staging.rs:35`) — the trait default "
        "both `reconcile_roster_with_frozen_topology` "
        "(`game/ambition_app/src/app/versus.rs:448`) and "
        "`track_versus_roster` (`:455`) call — is `demand.request_all(...)` "
        "and nothing else. `forward_into` "
        "(`game/ambition_app/src/app/world_flow/room_transition_assets.rs:504`) "
        "is likewise one `request_all` call, invoked from "
        "`game/ambition_app/src/app/startup_loading.rs:476` and "
        "`room_transition_assets.rs:1079`. The only two "
        "`.take()`/`.take_within_budget()` sites in the whole tree are "
        "`crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs:493` "
        "and `:896`, both inside `materialize_character_demand`. Every "
        "producer is commutative; the sole consumer is a single function. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "ConstructionSchemaCatalog": (
        "A KEYED LEDGER WHERE EVERY WRITER OWNS A DISTINCT DOMAIN STRING. "
        "`crates/ambition_platformer2d_shared_tangle/src/construction/schema_catalog.rs:16` "
        "stores `domains: BTreeMap<String, String>`; `try_contribute` "
        "(`:52`) is idempotent for a repeated identical dump under one key "
        "and REFUSES (rather than clobbers) a differing one. "
        "`crates/ambition_platformer2d_actor_monolith/src/gravity/plugin.rs:56` "
        "contributes under `GRAVITY_ZONE_CONSTRUCTION_DOMAIN`, "
        "`crates/ambition_portal2d/src/plugin.rs:64` under "
        "`PORTAL_GUN_CONSTRUCTION_DOMAIN`, and "
        "`crates/ambition_platformer2d_runtime/src/sim_core_resources.rs:246` "
        "under `ACTOR_CONSTRUCTION_DOMAIN` — three distinct constants, so no "
        "two writers can ever reach the same map entry. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "GameplayTraceBuffer": (
        "ONE FLAGGED TARGET IS A GETTER AND THE OTHER IS AN APPEND-ONLY "
        "RING. `crates/ambition_gameplay_trace/src/buffer.rs:19` documents "
        "the buffer as \"a rolling ring buffer ... that the game's recorder "
        "systems push into\" — plural by design. `census.shared_targets` "
        "names `current_tick` as touched by two files but mutation-shaped in "
        "neither: `current_tick` (`buffer.rs:105`) takes `&self` and only "
        "reads `self.tick`, so "
        "`crates/ambition_encounter_features/src/systems.rs:483` and "
        "`crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs:302` "
        "are both READERS the crude regex flagged as contended writers — a "
        "census instrument artifact. `push_event` (`buffer.rs:229`) is a "
        "genuine mutator, but it only pops the oldest entry when full and "
        "pushes the new one to the back; two production callers "
        "(`encounter_features/src/systems.rs:486` and "
        "`projectile/systems.rs:304`) appending diagnostic trace events can "
        "never overwrite each other's entry, so there is no "
        "last-writer-wins ambiguity to adjudicate. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "LocalSessionOwnership": (
        "ONE ACQUIRER, AND EVERY OTHER WRITE IS THE SAME IDEMPOTENT CLEAR. "
        "`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs:479`, "
        "inside `maintain_local_session`, is the ONLY production site that "
        "sets `started = Some(policy)`; every other assignment in the file is "
        "`started = None` (`:298`) or a call to `release()` (`:66`), which is "
        "the same `self.started = None`. `decide_mechanical_edit_admission` "
        "(`:190`) calls `release()` at `:219`; "
        "`restart_local_ggrs_after_hot_reload` "
        "(`game/ambition_app/src/app/dev_runtime.rs:300`) calls it at `:314` "
        "after checking `session_is_active`; "
        "`rebase_local_timeline_onto_the_new_generation` "
        "(`game/ambition_content/src/reload.rs:740`) calls it at `:747` "
        "after the same check plus `rebasable_local_timeline`. Three "
        "disjoint triggers (mechanical-edit refusal, LDtk hot reload, "
        "content-generation publish) all reduce to the same idempotent "
        "null-out; only one function ever re-arms it. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "PreparedSessionRegistry": (
        "A KEYED LEDGER (BY `LoadId`) WHOSE METHODS REFUSE RATHER THAN "
        "CLOBBER. `crates/ambition_game_shell/src/preparation.rs:221` stores "
        "`records: BTreeMap<LoadId, PreparationRecord>`; `request` (`:227`) "
        "asserts the load id is fresh, and `publish` (`:244`) returns `None` "
        "rather than overwrite if a record is already prepared or consumed. "
        "`apply` (`crates/ambition_game_shell/src/router.rs:468`), "
        "`cancel_pending` (`:652`) and `advance_pending` (`:697`) are the "
        "only call sites of request/cancel/consume. `process_shell_commands` "
        "(`crates/ambition_game_shell/src/plugin.rs:245`) calls `apply`; "
        "`advance_pending_route` (`:280`) calls `advance_pending` and, at "
        "`:341`, the SAME `cancel_pending` that "
        "`process_shell_presentation_events` "
        "(`crates/ambition_load_presentation/src/shell_adapter.rs:168`) also "
        "calls (at `:220`) — one function, two call sites. "
        "`crates/ambition_platformer2d_provider/src/lifecycle.rs:1559` calls "
        "`publish` and `:1624` the read-only `retain_requested` sweep. Every "
        "write is keyed to one transaction's own load id. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "NarrativeMusicRequest": (
        "ONE SETTER, TWO IDEMPOTENT CLEARERS ON DISJOINT TRIGGERS. "
        "`crates/ambition_conversation/src/music.rs:13` is a single `track: "
        "Option<String>` field; `request()` (`:23`) is the only way to set it "
        "and `clear()` (`:27`) the only way to null it. `cmd_music` "
        "(`game/ambition_content/src/yarn_vocabulary.rs:363`) — the "
        "`<<music>>` Yarn command handler — is the sole production caller of "
        "`request()` (`:365`). "
        "`release_narrative_music_on_room_change` "
        "(`crates/ambition_platformer2d_actor_monolith/src/music/intent.rs:35`) "
        "calls `clear()` only when `active_room.is_changed()`; "
        "`reset_audio_request_state_on_context_change` "
        "(`crates/ambition_platformer2d_actor_monolith/src/audio/plugin.rs:222`) "
        "calls `clear()` at `:263` only on a genuine `AudioContextChanged` "
        "owner edge (`previous != current`). One arm, two decay triggers "
        "that can never disagree because both converge on the same `None`. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "SelectCursors": (
        "ONE LIVE DRIVER BETWEEN TWO IDEMPOTENT LIFECYCLE RESETS, PLUS A "
        "STANDALONE TOOL BINARY. `drive_the_cursor` "
        "(`game/ambition_demo_smash/src/select_screen.rs:928`) is the "
        "per-frame mover while the select route is active. "
        "`reset_select_frontend_on_arrival` "
        "(`game/ambition_demo_smash/src/lib.rs:3591`) resets to Default on "
        "ROUTE ARRIVAL, latched at `:3608` against a "
        "`Local<Option<ShellActivationId>>` declared at `:3600` so it fires "
        "once per arrival; `:4099` registers the same Default reset on "
        "experience departure through `resetting::<R>()` "
        "(`crates/ambition_game_shell/src/scope.rs:244`), a generic "
        "exit-scope helper. Entry and exit are disjoint edges either side of "
        "the live-driver window. "
        "`game/ambition_demo_smash_app/src/bin/smash_tool.rs:31` documents "
        "itself as \"instruments, not the demo: smash_demo is the playable "
        "shell\"; its select_walkthrough tool "
        "(`game/ambition_demo_smash_app/src/tools/select_walkthrough.rs:29`) "
        "builds its own app via `build_demo_app()` and writes the resource "
        "at `:128` inside that separate process, never the shipped "
        "smash_demo binary. Measured by CalculexAmbition, 2026-09-18."
    ),
    "ActiveAudioSelection": (
        "ONE OWNER FOR `current`, AND EVERY OTHER WRITE IS A PROVIDER-KEYED "
        "IDEMPOTENT MERGE. `select_shell_audio_context` "
        "(`crates/ambition_game_shell/src/session.rs:442`) is the only "
        "production system that calls `select_gameplay` (`:472`), "
        "`clear_if_owner` (`:488`, `:546`) or `clear` (`:533`) — the only "
        "setters/clearers of `current`. `refresh_provider_sfx_ids` "
        "(`crates/ambition_audio/src/selection.rs:677`) filters "
        "`current.sfx_sources` by the exact `provider_id` argument, so a "
        "late bank can never expand another provider's authority; "
        "`promote_loaded_sfx_bank` "
        "(`crates/ambition_audio/src/bank_asset.rs:375`) and "
        "`publish_resident_sfx_bank_authority` "
        "(`game/ambition_app/src/app/setup_systems.rs:128`, a `Local<bool>` "
        "run-once latch) both call it keyed to their own provider id. "
        "`authorize_staged_character_presentation_sources` "
        "(`crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs:58`) "
        "documents itself as \"Idempotent by construction: ... merges by "
        "union\" and no-ops when `selection.current()` is `None`. Measured "
        "by CalculexAmbition, 2026-09-18."
    ),
    "SessionSeatingSource": (
        "ROUTE-GATED MUTUAL EXCLUSION FOR THE TWO IN-PROCESS PRODUCERS, AND "
        "A THIRD APP THAT NEVER LINKS IN AT ALL. `track_versus_roster` "
        "(`game/ambition_app/src/app/versus.rs:448`) only writes "
        "`SessionSeatingSource::decided` (`:498`) when `:459` finds "
        "`route_id == VERSUS_GAMEPLAY_ROUTE`. `game/ambition_demo_smash/src/lib.rs` "
        "is installed as a ROUTE inside the SAME process "
        "(`game/ambition_app/src/app/shell_host.rs:103` installs "
        "`SmashExperiencePlugin`), but its own writes are gated the same "
        "way: `reset_select_frontend_on_arrival` (`lib.rs:3591`) checks "
        "`on_the_select_route` at `:3604` before writing `::pending` (`:3622`), "
        "and `start_the_battle_when_asked` (`:3758`) makes the same check at "
        "`:3792` before `::decided` (`:3863`) — "
        "`ShellRouter.active` names exactly one route, so "
        "`VERSUS_GAMEPLAY_ROUTE` and `SMASH_SELECT_ROUTE` can never both be "
        "active. `declare_the_couch` "
        "(`game/ambition_demo_twintrack/src/participants.rs:165`) belongs to "
        "the twintrack crate, whose `game/ambition_demo_twintrack/Cargo.toml` "
        "names no dependency on the smash or flagship crates, and whose "
        "reverse is true in `game/ambition_app/Cargo.toml` and "
        "`game/ambition_demo_smash/Cargo.toml` — so it never shares a "
        "process with either. "
        "`crates/ambition_platformer2d_rollback_ggrs/src/local_session.rs:135` "
        "only stamps back the disjoint `frozen_topology` field onto "
        "whichever app's own Decided value is live. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "ShellHostConfiguration": (
        "EVERY WRITE HAPPENS ONCE, SYNCHRONOUSLY, DURING APP ASSEMBLY, NEVER "
        "AS A RUNNING SYSTEM. `ShellComposition::install` "
        "(`crates/ambition_platformer2d_provider/src/composition.rs:97`) is "
        "the shared entrypoint every composition calls to set `spec` once. "
        "`crates/ambition_platformer2d/src/app.rs:1105` calls `install(...)` "
        "and then, only under `StartAt::Launcher`, immediately overrides "
        "`spec` at `:1133` in the same synchronous function. "
        "`compose_ambition_shell_host_inner` "
        "(`game/ambition_app/src/app/shell_host.rs:56`) sets `spec` at "
        "`:121`; `compose_ambition_startup_sequence` (`:233`) is documented "
        "as an OPTIONAL step (\"`--direct` ... simply don't compose it\") "
        "called afterward from `game/ambition_app/src/app/cli.rs:273` to "
        "deliberately override `spec` at `shell_host.rs:254` toward the "
        "startup-vanity route instead of the launcher. "
        "`game/ambition_app_tools/src/bin/preview_vanity_card.rs:79` is a "
        "standalone `[[bin]]` tool building its own `MinimalShellPlugins` "
        "App. None of these runs as a Bevy system; each is a plain `&mut "
        "App` builder function invoked once, in a fixed order chosen by "
        "whichever binary composes that App, before `.run()` and before any "
        "reader exists. Measured by CalculexAmbition, 2026-09-18."
    ),
    "ShellRouteCatalog": (
        "A KEYED REGISTRY WHERE EVERY CALLER REGISTERS A DIFFERENT ROUTE ID, "
        "WRITTEN ONLY DURING APP ASSEMBLY. "
        "`crates/ambition_game_shell/src/router.rs:64` stores `routes: "
        "BTreeMap<ShellRouteId, ShellRouteSpec>`; `register` (`:69`) is a "
        "plain keyed insert. `ShellComposition::install` "
        "(`crates/ambition_platformer2d_provider/src/composition.rs:97`) "
        "registers the experience's own launcher/gameplay routes once per "
        "App; `crates/ambition_platformer2d/src/app.rs:1127` additionally "
        "registers the `StartAt::Launcher` route under its OWN distinct id "
        "right after calling `install(...)`, in the same synchronous builder "
        "function. `game/ambition_app/src/app/shell_host.rs:115` registers "
        "`AMBITION_LAUNCHER_ROUTE` and `:239` "
        "(`compose_ambition_startup_sequence`, an OPTIONAL later step per "
        "`:220`) registers the distinct `AMBITION_STARTUP_ROUTE` id. "
        "`game/ambition_app_tools/src/bin/preview_vanity_card.rs:61` is a "
        "standalone `[[bin]]` tool with its own catalog instance. No two of "
        "these calls ever target the same route id, and every call happens "
        "once during single-threaded App construction before any system "
        "runs. Measured by CalculexAmbition, 2026-09-18."
    ),
    "ShellRouteHolds": (
        "A KEYED LEDGER (route, hold id) WHERE EVERY WRITER OWNS A DISTINCT "
        "HOLD-ID NAMESPACE. `crates/ambition_game_shell/src/router.rs:175` "
        "stores `holds: BTreeMap<ShellRouteId, BTreeSet<ShellHoldId>>`; the "
        "doc at `:209` states it is \"KEYED BY HOLD ID, NOT BY ROUTE\" so a "
        "transaction-specific id \"keeps a stale transaction's cleanup from "
        "freeing its successor's block\". "
        "`crates/ambition_load_presentation/src/shell_adapter.rs:18` fixes a "
        "single constant hold id (`ambition.load-presentation.ready-hold`) "
        "that only that file ever holds or releases (`:113`). "
        "`crates/ambition_platformer2d_provider/src/lifecycle.rs:1773` mints "
        "a hold id keyed per activation (`session-publication:{activation}`); "
        "`game/ambition_content/src/reload.rs:1385` mints one keyed per "
        "reload transaction. `crates/ambition_game_shell/src/plugin.rs:264` "
        "documents that a gate is evaluated and its OWN hold released in one "
        "exclusive World operation (\"a gate never releases itself\"); "
        "`:320` and `:347` release only the specific hold that verdict just "
        "resolved. No two writers ever reach the same (route, hold id) pair. "
        "Measured by CalculexAmbition, 2026-09-18."
    ),
    "YarnContentBindings": (
        "A PLAIN `Vec` PUSHED AT PLUGIN-BUILD TIME, EACH CALLER "
        "CONTRIBUTING A DISJOINT INSTALLER. "
        "`crates/ambition_dialog/src/bindings.rs:84` stores `installers: "
        "Vec<YarnBindingInstaller>`. "
        "`crates/ambition_conversation/src/dialog.rs:78` pushes "
        "`authored_conditions::install_condition_binding` (plus its command "
        "twin); `game/ambition_content/src/bosses/mod.rs:408` pushes "
        "`yarn::install_cut_rope_yarn_bindings`; "
        "`game/ambition_content/src/plugin.rs:111` pushes "
        "`duel_arena::install_duel_yarn_binding` and `:117` pushes "
        "`yarn_vocabulary::install_game_bindings`. Every site is a "
        "`.installers.push(...)` inside a `Plugin::build`, executed once "
        "during App construction, and each names a different function — Vec "
        "append never overwrites a sibling's entry regardless of order. "
        "`app_running` "
        "(`crates/ambition_conversation/src/dialog/yarn_harness.rs:25`) "
        "builds its OWN `App::new()` for tests and extends the list at "
        "`:42` inside that separate, throwaway App instance. Measured by "
        "CalculexAmbition, 2026-09-18."
    ),
    "LocalSeatOffer": (
        "TWO WRITER FILES, TWO ENTIRELY SEPARATE GAMES WITH NO DEPENDENCY "
        "EDGE BETWEEN THEM. `crates/ambition_input/src/seating.rs:130` "
        "declares the resource. `maintain_smash_local_seat_offer` "
        "(`game/ambition_demo_smash/src/lib.rs:3541`) is the smash demo's "
        "own writer; `declare_the_couch` "
        "(`game/ambition_demo_twintrack/src/participants.rs:165`) is "
        "TwinTrack's. `game/ambition_demo_smash/Cargo.toml` names no "
        "dependency on the twintrack crate and "
        "`game/ambition_demo_twintrack/Cargo.toml` names none on the smash "
        "crate — unlike `SessionSeatingSource`, where the smash demo is "
        "installed as a route inside the flagship, these two products never "
        "link into one binary at all, so the two writer files can never run "
        "in the same process. Measured by CalculexAmbition, 2026-09-18."
    ),
    "Warmup": (
        "THREE UNRELATED, FILE-PRIVATE TYPES THAT MERELY SHARE A NAME. "
        "`game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs:29` "
        "declares a private `struct Warmup { remaining, walk_right, settle, "
        "... }`; `game/ambition_demo_sanic_app/src/bin/capture_sanic.rs:30` "
        "declares a DIFFERENTLY SHAPED private `struct Warmup { remaining, "
        "walk_right, settle }` with no positioning field; "
        "`game/ambition_demo_twintrack_app/src/bin/capture_twintrack.rs:37` "
        "declares a THIRD private `struct Warmup { remaining, run_right, "
        "settle }`. None is `pub`, each lives in its own `[[bin]]` "
        "capture-tool crate, confirmed by the matching `[[bin]]` entry in "
        "each of `game/ambition_demo_mary_o_app/Cargo.toml`, "
        "`game/ambition_demo_sanic_app/Cargo.toml` and "
        "`game/ambition_demo_twintrack_app/Cargo.toml`, and none is "
        "reachable from the other two. `multi_writer_resource_census.py` "
        "collapses a type on its LAST PATH SEGMENT, so three unrelated "
        "private structs sharing the identifier `Warmup` are exactly the "
        "shape its own docstring warns is a namespacing artifact, not a "
        "duplicated authority. Measured by CalculexAmbition, 2026-09-18."
    ),
    "RoomTransitionLoadState": (
        "⛔⛤ NOT A CLEAN BILL. THIS ROW IS AN OPEN FINDING WITH A RUNNING "
        "MEASUREMENT, RECORDED HERE SO IT STOPS BEING RE-DERIVED. "
        "`active` is a single `Option<ActiveRoomTransitionLoad>` and the "
        "type's own doc "
        "(`crates/ambition_platformer2d_runtime/src/room_transition/loading.rs:293-296`) "
        "says \"there is exactly one active transition\". Four production "
        "files write it, and `census.shared_targets` names `active` as "
        "MUTATION-SHAPED in three of them. "
        "⭐ WHAT IS ORDERED: `loading.rs`'s four writers "
        "(`begin_room_transition_load_system`, "
        "`authorize_ready_room_transition_system`, "
        "`abandon_failed_checkpoint_restore_system`, "
        "`finalize_unpresented_room_transition_failure_system`) are installed "
        "as ONE `.chain()` in `Update` "
        "(`crates/ambition_platformer2d_runtime/src/room_transition/mod.rs:70-86`), "
        "whose membership is asserted by "
        "`the_readiness_chain_still_carries_the_checkpoint_terminalization` "
        "(`loading.rs`); `commit.rs`'s pair sits in the sim schedule's "
        "`RoomTransitionSet::Apply` chain (`mod.rs:88-105`); and the "
        "presentation trio is chained in `Update` "
        "(`game/ambition_app/src/app/world_flow/room_transition_presentation.rs:239-257`). "
        "⛔ WHAT IS NOT: the relationship BETWEEN those groups. MEASURED "
        "2026-09-18 by asking Bevy's own `conflicting_systems()` about the "
        "shipped app — 16 unordered pairs write this resource, held as a "
        "ratchet by "
        "`room_transition_load_state_writers_are_ordered_against_each_other_in_the_shipped_app` "
        "(`game/ambition_app/tests/update_schedule_census.rs`) with a "
        "positive control "
        "(`the_room_transition_conflict_detector_reports_an_unordered_pair`) "
        "and a negative one "
        "(`ordering_the_pair_clears_the_room_transition_conflict`), so the 16 "
        "is a fact about the app and not about a detector that never fires. "
        "⚠ THE DETECTOR IS BLIND TO THE ROLLBACK HALF BY CONSTRUCTION, which "
        "is why the 16 is a lower bound: `retire_committed_room_transition` "
        "and `retire_cancelled_room_transition` "
        "(`crates/ambition_platformer2d_rollback_ggrs/src/lifecycle_commit.rs:305`, "
        "`:366`) are plain `fn(&mut World, ..)` called directly by the commit "
        "executor, not registered systems, so the schedule graph has no node "
        "for them. They carry their own guards instead, and ONLY ONE OF THE "
        "FOUR CLEARERS GUARDS THIS WAY: `retire_cancelled_room_transition` "
        "compare-matches `active.intent` before clearing and its doc names "
        "the hazard exactly — \"retiring whatever happens to be active would "
        "take out a newer, unrelated transaction opened by somebody else in "
        "the same frame\". `retire_committed_room_transition` does not clear "
        "at all when `cover_required`, handing retirement to the "
        "presentation settle barrier (`:314-330`, whose own comment reads "
        "\"presentation restores its own mode when it retires the cover\"), "
        "which is a clean "
        "complementary split rather than a race. `loading.rs:710,807`, "
        "`commit.rs:761,1209` and `room_transition_presentation.rs:517,594,632` "
        "clear whatever is active. ⇒ The adjudication is: the single-slot "
        "invariant is upheld by ORDERING in three groups and by a compare-"
        "guard in one place, and the 16 cross-group pairs are unproven "
        "either way. ⭐⭐ AND THE SIXTEEN ARE ONE MISSING EDGE RATHER THAN "
        "SIXTEEN JUDGEMENTS, ATTRIBUTED 2026-09-18 by "
        "`name_the_room_transition_conflicts`: EVERY pair is "
        "`ReadinessSet(Update)` × `NO ROOM-TRANSITION SET`, and the arithmetic "
        "closes exactly — the readiness chain holds 4 systems and the app-side "
        "`Update` chain holds 4 writers of this resource (contribute/poll in "
        "`room_transition_assets.rs`, drive/handle in "
        "`room_transition_presentation.rs`), so 4 × 4 = 16. Both chains are "
        "internally ordered; neither is ordered against the other, and neither "
        "app-side writer declares a room-transition set, so no set-level pin "
        "reaches them. ⇒ ONE ordering decision between two `Update` chains "
        "closes this row, and it is still a judgement — the readiness chain "
        "OPENS transactions and the presentation chain drives and retires "
        "them, so the two orders differ on what a transaction opened this "
        "frame may observe — but it is a single one, and the ratchet should "
        "fall 16 → 0 in one edit. Measured by "
        "YardratAmbition, 2026-09-18; row found by CalculexAmbition, who "
        "correctly declined to write it against files another session held."
    ),
    "LoadCoordinator": (
        "A KEYED LEDGER WHOSE EVERY MUTATION IS EITHER KEYED, IDEMPOTENT, OR "
        "GATED ON THE PLAN STILL BEING ACTIVE — SO A LATE WRITE IS A NO-OP "
        "RATHER THAN A RESURRECTION. "
        "`crates/ambition_load/src/coordinator.rs:35` is `plans: "
        "BTreeMap<LoadId, PlanRecord>`, and all nine writer files reach it "
        "through exactly two methods. `census.shared_targets` names `retire` "
        "(four files) and `apply` (two) as touched but MUTATION-SHAPED IN "
        "NEITHER, which is the census reporting method calls rather than "
        "field writes. "
        "⭐ THE FOUR ROADS, each closed on its own: `apply(LoadCommand::"
        "Begin)` (`:82`) INSERTS under the new plan's own id and marks only "
        "the predecessor it explicitly `supersedes`; every other mutating "
        "arm goes through `mutate_plan` (`:65`), whose first act is "
        "`active_plan_mut` (`:386`) — `plans.get_mut(load_id).filter(|plan| "
        "plan.state == LoadPlanState::Active)` — so a command for a plan that "
        "has been cancelled, superseded or retired finds nothing and returns "
        "without touching the map; `apply(LoadCommand::Cancel)` (`:187`) is "
        "the one arm that deliberately skips the Active filter, because "
        "cancelling an already-cancelled plan is the same assignment twice "
        "(`plan.state = LoadPlanState::Cancelled`); and `retire` (`:374`) is "
        "`self.plans.remove(load_id).is_some()`, idempotent by construction — "
        "whichever caller retires a barrier first removes it, and the second "
        "gets `false` and changes nothing. "
        "⇒ Two files retiring the same room-transition barrier in one frame "
        "is not a race, and a stale command arriving after a supersession "
        "cannot revive a dead plan. "
        "⚠ WHAT THIS ROW DOES NOT SETTLE, said plainly rather than left for "
        "the next reader to discover: five of the nine writers are the "
        "room-transition lifecycle, and they share ONE `LoadId` across files "
        "by design. The question of whether those systems are ORDERED "
        "against each other is real, is measured, and belongs to the sibling "
        "row — see `RoomTransitionLoadState`, which holds 16 unordered pairs "
        "as a ratchet. The difference is that `LoadCoordinator`'s own "
        "structure makes an out-of-order arrival inert, which is exactly what "
        "the single `Option` slot in that sibling does NOT do. Measured by "
        "YardratAmbition, 2026-09-18."
    ),
    "SeatRawFrames": (
        "A SLOT-KEYED STAGING TABLE WHOSE SHIPPED PIPELINE IS MEASURED "
        "ORDERED — ZERO UNORDERED PAIRS, WITH A CONTROL. The type "
        "(`crates/ambition_characters/src/control.rs:80`) is `slots: "
        "[ControlFrame; SlotControls::MAX_SLOTS]`, and its own doc "
        "(`:74-77`) states the staging contract: \"Device- or "
        "wall-clock-derived stages use this table before "
        "`PrimarySlotInputCommit`\", with `SlotControls` as the distinct "
        "post-latch simulation input. The shaping API is `shape()` (`:100`), "
        "which edits ONE seat's frame in place rather than replacing the "
        "table, so two stages adjusting different seats — or different fields "
        "of one frame — are not competing writes in the first place. "
        "⭐ MEASURED 2026-09-18, not argued: Bevy's own `conflicting_systems()` "
        "reports ZERO unordered pairs on this resource across every schedule "
        "of the shipped app, against 23,123 conflicting pairs in total, held "
        "by "
        "`seat_raw_frame_writers_are_ordered_against_each_other_in_the_shipped_app` "
        "(`game/ambition_app/tests/update_schedule_census.rs`). "
        "⛔ THE ZERO CARRIES ITS OWN CONTROL BECAUSE IT HAS A SECOND CAUSE. An "
        "EXCLUSIVE system reports its conflict with an EMPTY id list, so a "
        "type written only by exclusive systems scores zero while being wholly "
        "unordered — and that run counts 8,724 such unattributable pairs. "
        "`the_seat_raw_frame_conflict_detector_reports_an_unordered_pair` "
        "plants two unordered `ResMut<SeatRawFrames>` systems and requires the "
        "detector to report exactly 1, so the zero is a fact about the app's "
        "ordering rather than about an instrument that cannot see this type. "
        "⚠ IT IS STILL A LOWER BOUND ON THE WRITERS, and the reason is the "
        "sixth writer. `drive_slot_frame` "
        "(`crates/ambition_platformer2d_runtime/src/input_drive.rs:40`) is a "
        "plain `fn(&mut World, ..)` that a driver CALLS — the authored-input "
        "SDK seam for scripted input, test drivers and the rollback backend's "
        "pending inputs — so it has no node in any schedule graph and no "
        "ordering question about it can reach the detector. It is nonetheless "
        "single-owner as of 2026-09-18: the rollback crate used to carry a "
        "duplicate of both its arms and now adds only its "
        "`PendingSeatInputs` arm and delegates here "
        "(`crates/ambition_platformer2d_rollback_ggrs/src/session.rs:959`), "
        "with the facade picking one by `#[cfg(feature = \"rollback\")]` "
        "(`crates/ambition_platformer2d/src/lib.rs:562-565`). Its own doc "
        "records that the two arms are ORDER-DEPENDENT and that the rollback "
        "wrapper relies on the latch arm coming first. "
        "⇒ Five scheduled writers, provably ordered against each other; one "
        "called seam with a single owner and a stated internal order. ⚠ A "
        "test pinning the frame-stepped host's order "
        "(`input_set_populate_runs_before_primary_slot_publication`, "
        "`game/ambition_content/src/portal/plugin.rs:341`) says nothing about "
        "the rollback host, the same asymmetry `PresentationPhase` records — "
        "which is why this row rests on the shipped-app measurement rather "
        "than on that test. Measured by YardratAmbition, 2026-09-18; row "
        "flagged by CalculexAmbition, who spotted that it reaches a file this "
        "session holds and was re-merged the same day."
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
#: ⛔ SAME SHAPE, SAME GAP, SAME FIX as [`BASELINE`] above: this was
#: `dict[str, int]` and a same-cardinality writer swap on
#: `EncounterMusicRequest` left the guard green. The file set is the owner.
SESSION_WORLD_BASELINE: dict[str, tuple[str, ...]] = {
    "EncounterMusicRequest": (
        "crates/ambition_boss_encounter/src/encounter_script.rs",
        "crates/ambition_boss_encounter/src/systems.rs",
        "crates/ambition_encounter_features/src/systems.rs",
        "crates/ambition_platformer2d_actor_monolith/src/music/intent.rs",
        "crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs",
        "game/ambition_content/src/bosses/cut_rope/mod.rs",
        "game/ambition_demo_mary_o/src/death.rs",
        "game/ambition_demo_mary_o/src/flag.rs",
        "game/ambition_demo_mary_o/src/star.rs",
    ),
    "LdtkRuntimeIndex": (
        "crates/ambition_platformer2d_ldtk/src/bevy_runtime/asset.rs",
        "game/ambition_app/src/app/dev_runtime.rs",
    ),
    "RoomGeometry": (
        "crates/ambition_platformer2d_actor_monolith/src/world/rooms/transaction.rs",
        "crates/ambition_sim_harness/src/runtime.rs",
    ),
}

#: ⛤ **IT WAS FOUR TYPES AND EIGHT WRITERS ON 2026-09-17; IT IS TWO AND NINE.**
#: `RoomGeometry` and `RoomSet` left because their second writer was never a
#: writer: `process_new_game_reset_request` bound both without `mut` — one an
#: explicit *"A GUARD, NOT A WRITE TARGET"*, the other reading `.start` and
#: passing `&room_set` on — and `handle_ldtk_hot_reload` did the same for
#: geometry. All three now ask for `SessionWorldRef`, which is the identical
#: `Single<_, With<SessionRoot>>` refusal without the exclusive borrow.
#: `EncounterMusicRequest` went 8 -> 9 the same day, in the opposite direction:
#: the census learned `session_world_component_mut::<T>(world)` and found the
#: session reset writing it that way.

#: ⭐⛤ **VERDICTS FOR THE OTHER POPULATION, which had a ratchet and no way to
#: record an answer until 2026-09-18.** Same rule as [`ADJUDICATED`] and a
#: different lifetime: a session boundary reclaims these, so "two writers" is a
#: question about one session's state rather than about the App's.
SESSION_WORLD_ADJUDICATED: dict[str, str] = {
    "EncounterMusicRequest": (
        "CORRECT — NINE WRITERS, A TWO-TIER PROTOCOL, AND THE PRIORITY TIER IS "
        "OWNER-CHECKED BY THE COMPILER SINCE 2026-09-18. The component is built "
        "for many writers on purpose: `priority_track` is a focused fight's "
        "claim, `base_track` is the wave/arena tier rewritten EVERY FRAME "
        "including `None`, `priority_owner` names who holds the claim, and "
        "`last_applied` is the intent adapter's mirror. `desired_track()` ranks "
        "priority above base, so the per-frame `None` cannot silence a boss.\n"
        "    MEASURED with comments stripped, which matters because the naive "
        "grep reads a comment MENTIONING `priority_track` as a write: SIX of the "
        "nine files call `claim_priority`/`release_priority` and touch no field "
        "at all (`ambition_boss_encounter/src/{encounter_script,systems}.rs`, "
        "`game/ambition_content/src/bosses/cut_rope/mod.rs`, and "
        "`game/ambition_demo_mary_o/src/{death,flag,star}.rs`); ONE writes only "
        "`base_track` (`ambition_encounter_features/src/systems.rs`); ONE writes "
        "only `last_applied` (`actor_monolith/src/music/intent.rs`); and ONE is "
        "the session reset clearing it at a boundary "
        "(`actor_monolith/src/session/reset/mod.rs`). ⇒ Nobody wrote "
        "`priority_track` or `priority_owner` directly. A perfect separation, "
        "held entirely by convention over `pub` fields.\n"
        "    ⛤ SO THE FIELDS ARE PRIVATE NOW. The tier can only be reached "
        "through `claim_priority` (a later claim wins outright — *\"two focused "
        "fights at once is not a state worth arbitrating\"*) and "
        "`release_priority` (which no-ops unless the caller still owns it — *\"a "
        "source with nothing to say says nothing, rather than silencing whoever "
        "does\"*). `set_base_track` and `mark_applied` carry the other two roads. "
        "Poison-verified: assigning `priority_track` from the base-tier writer "
        "fails with `error[E0616]`. The module doc had already recorded shipping "
        "the un-owned clear once; the discipline was universal and nothing kept "
        "it that way."
    ),
    "RoomGeometry": (
        "CORRECT — THE ROOM PUBLICATION AND A TEST HARNESS'S SETUP ROAD, AND THE "
        "PAIR ONLY BECAME VISIBLE ON 2026-09-18. The authority is "
        "`apply_world_replacement` "
        "(`actor_monolith/src/world/rooms/transaction.rs`), which writes the "
        "published room's geometry onto the root the transaction's own scope "
        "resolved, on the publication's verdict. The second file is "
        "`SimHarness::add_block` (`ambition_sim_harness/src/runtime.rs`), whose "
        "doc says what it is for: *\"used by symmetry tests to place a known "
        "target without authoring a room\"*. It pushes one block and immediately "
        "calls `rebase_after_direct_setup_mutation`, which is a SETUP road "
        "re-establishing the rollback baseline — not a per-frame producer "
        "competing for the value.\n"
        "    ⚠ WHY IT IS IN THE POPULATION AT ALL, stated rather than waived: "
        "`ambition_sim_harness` is an ordinary crate, not a `#[cfg(test)]` item "
        "and not a whole test file, so neither of this census's two exclusions "
        "reaches it. That is the right answer — a harness that mutates session "
        "state IS a second road, and the question is whether it can race the "
        "first. It cannot: nothing schedules it, a test calls it between frames.\n"
        "    ⛔⛤ AND THE PAIR WAS INVISIBLE UNTIL THE INSTRUMENT LEARNED THE "
        "PUBLICATION'S SPELLING. `apply_world_replacement` wrote through a bare "
        "`world.get_mut::<RoomGeometry>(root)` — three tokens like any other "
        "component write — so the authoritative writer was simply absent from "
        "this census and the harness read as the only one. It spells the road "
        "`session_world_component_mut_at` now."
    ),
    "LdtkRuntimeIndex": (
        "CORRECT — ONE PER-FRAME SYNC AND ONE VERDICT-GATED REBUILD, AND THEY ARE "
        "NOT THE SAME FACT. `sync_ldtk_level_set` "
        "(`ambition_platformer2d_ldtk/src/bevy_runtime/asset.rs`) sets the ACTIVE "
        "AREA, early-returning unless `needs_level_set_sync(&active_area)`, and "
        "then hands the same `LevelSet` to both LDtk bundles. The dev hot reload "
        "(`game/ambition_app/src/app/dev_runtime.rs`) REPLACES the whole index "
        "with a candidate built from the reloaded project, and it does that "
        "inside the staged closure — `session_world_component_mut::<LdtkRuntimeIndex>` "
        "under exclusive world access, on the room publication's own verdict. ⇒ "
        "A replacement and an active-area sync cannot interleave: the closure "
        "runs at a command flush, and the sync re-derives from whatever index it "
        "then finds.\n"
        "    ⚠ `sync_ldtk_level_set` IS ONE OF THE NINE ACKNOWLEDGED OFFENDERS in "
        "`scripts/check_rollback_mutators_run_in_sim.py`, owed to "
        "ROLLBACK-MUTATOR-POPULATION — it mutates rollback-registered state from "
        "a schedule that does not rewind. That is a SCHEDULE question and is "
        "banked there; it is not an authority dispute between these two, which is "
        "what this verdict answers.\n"
        "    ⛔⛤ AND THE SECOND WRITER WAS ONLY VISIBLE AFTER THE CENSUS LEARNED A "
        "SECOND SPELLING. Until 2026-09-18 it read `SessionWorldMut<T>` alone, so "
        "the dev reload counted through its SYSTEM signature — where the "
        "parameter is now a shared borrow — while the write it actually performs, "
        "through `session_world_component_mut`, was invisible. The count was "
        "right for the wrong reason, which is the worst kind of right."
    ),
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
#: MEASURED 2026-09-17 — and thirteen of those are written from exactly one OTHER
#: FILE. ⛔ That is not "would be single-writer without it", which is what this
#: comment said until 2026-09-18 and is false for two of the thirteen:
#: `ActiveCutscene` has two writing systems in that other file and
#: `ProjectileSeqCounter` three. The green line was corrected and this paragraph
#: kept the wrong phrasing, twenty lines above the note explaining it. The
#: thirteen are
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


#: ⛔⛤ ONE NAME, TWO TYPES — a row this census CANNOT read correctly by
#: construction, because it is keyed on the type's NAME. Two `struct`/`enum`
#: declarations sharing a spelling are two types unless one is reachable from
#: the other's writer — a `struct` declared INSIDE a function is local to it, a
#: private `struct` declared at module scope inside its OWN `[[bin]]` crate is
#: unreachable for the same reason, and both are the same claim: EACH writer
#: file declares its OWN copy rather than importing one shared item. ⇒ The
#: verdict says so, and this table makes the claim checkable per writer file.
NAME_COLLISION_NOT_ONE_TYPE: dict[str, tuple[str, ...]] = {
    "FixedStepsTaken": (
        "game/ambition_app/src/app/cli.rs",
        "game/ambition_app/src/headless.rs",
    ),
    "Warmup": (
        "game/ambition_demo_mary_o_app/src/bin/capture_mary_o.rs",
        "game/ambition_demo_sanic_app/src/bin/capture_sanic.rs",
        "game/ambition_demo_twintrack_app/src/bin/capture_twintrack.rs",
    ),
}

#: ⛔ ANTI-VACUITY for [`name_collision_shortfalls`]'s tree-wide scan. MEASURED
#: 2026-09-18: 3,754 `struct`/`enum` declarations across the same
#: `census.production_files()` population `MIN_FILES` already floors. A scan
#: that silently stopped matching declarations would report a clean tree, the
#: same failure `MIN_FILES`/`MIN_TYPES` exist to catch on the file/type side.
MIN_DECLARATIONS = 1500

#: `struct Name` / `enum Name`, ANY indentation. Indentation is reported as
#: DETAIL below (fn-local vs module-level), never as the filter — a rule that
#: only looked for one shape would have missed whichever case it did not name.
_TYPE_DECL = re.compile(
    r"^([ \t]*)(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+(\w+)\b", re.MULTILINE
)


def name_collision_shortfalls(
    multi: dict[str, list[str]], files: list[str]
) -> list[str]:
    """Every way a [`NAME_COLLISION_NOT_ONE_TYPE`] claim can have stopped being
    true, PLUS every multi-writer type not in that table whose name is not
    proof of one type either.

    `files` is the caller's own population (`main()` passes
    `census.production_files()`, already floored by `MIN_FILES`) — this
    function does not re-derive it, so it has no `git` dependency of its own
    and runs the same against a caller's synthetic corpus.

    ⚠ WHAT THIS DELIBERATELY DOES NOT DO: see a macro-generated `struct`, or a
    declaration outside the given `files` — the same population the rest of
    this census already scans, not the wider tree. A consumer needing either
    of those is asking a different question.

    ⛔ A COLLISION FOUND FOR A TYPE WITH NO VERDICT IS AN ERROR, not a report:
    the name is not proof of one type, so every count and verdict keyed on it
    is suspect until a human reads which declaration each writer actually
    means — the same reason `phantom` in `main()` is fatal rather than printed
    and continued past.
    """
    problems: list[str] = []

    # ⭐ RE-VERIFY EACH TABLED ROW FIRST, exactly as before: per-file resolution
    # is anchored to THIS SCRIPT's own location, not to `files` or the caller's
    # cwd, so a claim about the repository stays a claim about the repository
    # no matter what corpus the unlisted-collision scan below is given.
    for ty, expected in sorted(NAME_COLLISION_NOT_ONE_TYPE.items()):
        if ty not in ADJUDICATED:
            problems.append(
                f"{ty} is recorded as a name collision but carries no verdict; "
                "the table and ADJUDICATED must agree."
            )
            continue
        writers = multi.get(ty)
        if writers is None:
            problems.append(
                f"{ty} is no longer written from more than one file, so the "
                "collision has no subject. Remove it here and in ADJUDICATED."
            )
            continue
        if sorted(writers) != sorted(expected):
            problems.append(
                f"{ty} is now written from {', '.join(sorted(writers))}, and the "
                f"claim names {', '.join(sorted(expected))}. A new writer file is "
                "a new question, not another copy of the same name."
            )
            continue
        for relative in expected:
            source = pathlib.Path(__file__).resolve().parents[1] / relative
            text = source.read_text(errors="replace") if source.exists() else ""
            # ANY indentation — fn-local (FixedStepsTaken) and module-level
            # private-per-crate (Warmup) are the same claim: THIS file declares
            # its own copy rather than importing one shared item.
            own = re.search(
                rf"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+{ty}\b",
                text,
                re.MULTILINE,
            )
            if not own:
                problems.append(
                    f"{ty} has no declaration of its own in {relative}, so the "
                    "writers may name ONE type after all — which would make "
                    "this row a real multi-writer question."
                )

    # ⭐ THE OTHER DIRECTION: a multi-writer type NOT in the table whose name
    # nonetheless matches more than one declaration somewhere in `files`. The
    # loop above re-verifies the rows already tabled; this is what finds a new
    # one. Gated on its own anti-vacuity floor so a broken regex reports a
    # broken regex rather than a clean tree.
    declared_in: dict[str, list[tuple[str, bool]]] = {}
    total_declarations = 0
    for f in files:
        text = census.strip_comments(
            pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        )
        for m in _TYPE_DECL.finditer(text):
            indent, ty = m.group(1), m.group(2)
            declared_in.setdefault(ty, []).append((f, bool(indent)))
            total_declarations += 1

    if total_declarations < MIN_DECLARATIONS:
        problems.append(
            f"only {total_declarations} struct/enum declaration(s) found across "
            f"{len(files)} file(s) (expected {MIN_DECLARATIONS}+); the "
            "unlisted-collision scan's regex is broken, not the tree, so it was "
            "skipped rather than reporting a false clean."
        )
        return problems

    for ty in sorted(multi):
        if ty in NAME_COLLISION_NOT_ONE_TYPE:
            continue
        hits = declared_in.get(ty, [])
        if len(hits) > 1:
            where = ", ".join(
                f"{f} ({'fn-local' if indented else 'module-level'})"
                for f, indented in hits
            )
            problems.append(
                f"{ty} matches {len(hits)} struct/enum declarations in the "
                f"production tree ({where}) and is not recorded in "
                "NAME_COLLISION_NOT_ONE_TYPE — it may be two unrelated types "
                "sharing a name, the way FixedStepsTaken and Warmup were."
            )
    return problems


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

#: Turbofish rollback registrations — `rollback_resource_clone::<T>`,
#: `declare_rollback_derived_resource::<T>` and their siblings.
_ROLLBACK_TURBOFISH = re.compile(
    r"\b(?:rollback_[a-z_]+|declare_rollback_derived_[a-z_]+)"
    r"::<\s*(?:[A-Za-z0-9_]+::)*([A-Za-z_][A-Za-z0-9_]*)"
)


def rollback_registered_shortlist(multi: dict[str, list[str]]) -> tuple[int, list[str]]:
    """The multi-writer types that are also rollback-registered, and which lack a verdict.

    ⛔⛤ **THIS WAS A SENTENCE IN THE DOCSTRING AND IT WENT STALE FIVE TIMES ON
    2026-09-18 ALONE** — "21 of the 102", then 31, eight, seven, six, five, four,
    all while the intersection itself never moved. Every restatement was correct
    when written and wrong within the hour, because the number counts VERDICTS
    and verdicts are what this campaign spends. ⇒ A number that moves every time
    somebody does the work cannot live in prose. It is printed now.

    ⚠ A LOWER BOUND, and the bound is the parse: only the turbofish spelling is
    visible, which finds ~396 names where the registry holds 491 rows. A row
    registered through any non-turbofish form is invisible here.
    """
    names: set[str] = set()
    for path in census.rust_files(("crates", "game")):
        for match in _ROLLBACK_TURBOFISH.finditer(
            pathlib.Path(path).read_text(errors="replace")
        ):
            names.add(match.group(1))
    intersecting = set(multi) & names
    return len(intersecting), sorted(intersecting - set(ADJUDICATED))



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

    # ⭐ AND A VERDICT THAT SAYS "THESE ARE NOT THE SAME TYPE" IS CHECKED TOO.
    collisions = name_collision_shortfalls(multi, files)
    if collisions:
        print("a name-collision verdict no longer describes the tree:\n")
        for problem in collisions:
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
        recorded = SESSION_WORLD_BASELINE[ty]
        if len(world[ty]) != len(recorded):
            world_moves.append(
                f"  {ty} moved: {len(recorded)} -> {len(world[ty])} writer file(s)."
            )
        elif set(world[ty]) != set(recorded):
            gone = sorted(set(recorded) - set(world[ty]))
            new_files = sorted(set(world[ty]) - set(recorded))
            world_moves.append(
                f"  {ty} kept {len(world[ty])} writer file(s) and CHANGED WHICH ONES:\n"
                + "\n".join(f"      LEFT    {f}" for f in gone)
                + ("\n" if gone and new_files else "")
                + "\n".join(f"      ARRIVED {f}" for f in new_files)
            )
    # ⛔ THE SAME PHANTOM RULE AS THE RESOURCE SIDE, for the same reason: the debt
    # line below is a SUBTRACTION, and a verdict on a type that has since become
    # single-writer would understate the unread half while looking settled.
    for ty in sorted(set(SESSION_WORLD_ADJUDICATED) - set(world)):
        world_moves.append(
            f"  {ty} carries a SESSION_WORLD_ADJUDICATED verdict and is no longer "
            "a multi-writer session-world component. Remove the verdict — a "
            "repair is not an amnesty, and the debt count below depends on it."
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
    grew = sorted(t for t in multi if t in BASELINE and len(multi[t]) > len(BASELINE[t]))
    shrank = sorted(t for t in multi if t in BASELINE and len(multi[t]) < len(BASELINE[t]))
    # ⛔⛤ THE SAME-CARDINALITY SWAP, which `grew`/`shrank` cannot see by
    # construction: one writer file replaced by another leaves the count exactly
    # where it was. Every verdict argues from the files it NAMES, so this is the
    # axis that rots citations silently.
    swapped = sorted(
        t
        for t in multi
        if t in BASELINE
        and len(multi[t]) == len(BASELINE[t])
        and set(multi[t]) != set(BASELINE[t])
    )

    if arrived or left or grew or shrank or swapped:
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
        for ty in swapped:
            gone = sorted(set(BASELINE[ty]) - set(multi[ty]))
            new_files = sorted(set(multi[ty]) - set(BASELINE[ty]))
            print(
                f"  {ty} kept {len(multi[ty])} writer file(s) and CHANGED WHICH ONES. "
                "Its verdict argues from the files it names, so re-read it before "
                "moving the baseline:"
            )
            for f in gone:
                print(f"      LEFT    {f}")
            for f in new_files:
                print(f"      ARRIVED {f}")
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
    world_owed = sorted(set(world) - set(SESSION_WORLD_ADJUDICATED))
    print(
        f"  + {len(world)} session-world component(s) written from more than one "
        f"file, of {len(world_all)} reached mutably at all — a population the "
        "resource census is silent about by construction. "
        + (
            f"{len(SESSION_WORLD_ADJUDICATED)} adjudicated, UNADJUDICATED: "
            f"{', '.join(world_owed)}."
            if world_owed
            else f"All {len(SESSION_WORLD_ADJUDICATED)} carry a verdict."
        )
    )
    # ⭐ WHERE TO SPEND THE NEXT VERDICT, MEASURED ON EVERY RUN rather than
    # quoted. See `rollback_registered_shortlist` for why it stopped being prose.
    registered, owed = rollback_registered_shortlist(multi)
    print(
        f"  ⭐ {registered} of them are rollback-registered (turbofish parse, a "
        "LOWER BOUND), where a second writer is a DIVERGENCE rather than a "
        "design smell — which is why this is the queue and the count above is "
        "not. "
        + (
            f"{len(owed)} of those "
            + ("carries" if len(owed) == 1 else "carry")
            + f" no verdict: {', '.join(owed)}."
            if owed
            else "Every one of them carries a verdict, so this queue is SPENT. "
            "⚠ Not the same as clean: the parse is a lower bound, and a row "
            "registered through any non-turbofish form was never in it."
        )
    )
    print(
        "  ⚠ multi-writer is NOT a defect by count. This ratchets the population "
        "so a new one cannot land unnoticed."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
