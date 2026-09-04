#!/usr/bin/env python3
"""Verify architectural absence and dependency contracts.

Source contracts search production code after stripping comments, so prose about a
removed symbol cannot violate the guard. Dependency contracts inspect `cargo
metadata` transitively. Each contract has a narrow predicate and reason; remove or
invert the contract when the architecture intentionally changes. Tests red-probe
the checker so a broken measurement cannot report success."""

from __future__ import annotations

import argparse
import functools
import json
import re
import subprocess
import sys
from bisect import bisect_right
from pathlib import Path

# Every entry is one architectural absence. `paths` are git pathspecs limited to
# production source; `patterns` are Python regexes matched against comment-
# stripped lines. Keep patterns narrow — see the module docstring.
#
# CONFINEMENT, NOT SINGULARITY — and the names now say so.
#
# A contract that excludes the one allowed file proves "no reference exists
# OUTSIDE this file". It does not prove that exactly one call exists inside it,
# that the intended call still exists, or that a second resolver was not added
# beside the first. `provider_of_character` already has TWO calls inside
# `presentation.rs` and satisfies its guard.
#
# These were originally named `one-caller-of-*` / `one-reader-of-*`, and never were. It is just
# not the stronger claim, so the ids no longer imply it.
#
# If exact singularity ever matters for one of these, add a positive count
# assertion INSIDE the allowed file rather than renaming it back.
ABSENCE_CONTRACTS: list[dict] = [
    {
        "id": "a-production-condition-states-why-it-said-no",
        "paths": ["crates/*/src", "game/*/src", "examples/*/src"],
        "patterns": [
            {
                "grep": r"from_bool_unexplained",
                # The definition site declares it; every other mention CALLS it.
                "match": r"(?<!fn )from_bool_unexplained",
            }
        ],
        "reason": (
            "A CONDITION THAT ANSWERS 'NO' MUST SAY WHY, and that is a product "
            "requirement rather than polish -- M5 of "
            "`engine/authored-gameplay-logic-and-orchestration.md`, restated in "
            "`engine/inspection-diagnostics-and-workbench.md` as structured "
            "'why not' explanation. `ConditionOutcome::NotSatisfied(WhyNot { term, "
            "subject, observed })` is the vocabulary; `from_bool_unexplained` is "
            "the fixture arm, and its own doc says so: 'FIXTURES ONLY: a "
            "production evaluator that reaches for this has a why-not it is not "
            "stating, and the grep for this name is the list of them.' "
            "That sentence described a grep somebody had to remember to run. It "
            "is a contract now. "
            "The failure it prevents is the hardest thing in a level to diagnose "
            "from outside: a gated wall that will not open, with nothing said "
            "about which term blocked it or what the domain saw. Seven published "
            "conditions across four domains state one today; the eighth is where "
            "a convention kept by hand stops being kept."
        ),
    },
    {
        "id": "the-two-move-drivers-do-not-author-their-own-presses",
        "paths": [
            "game/ambition_app_tools/src/bin/moveset_takes.rs",
            "game/ambition_app_tools/src/bin/moveset_render.rs",
        ],
        "patterns": [
            r"(jump|attack|special|grab|taunt)_pressed",
            r"attack_strong_hint",
            r"attack_held",
        ],
        "reason": (
            "ONE DEFINITION OF 'PERFORM THIS MOVE'. `moveset_takes` records what the "
            "engine does with a press and `moveset_render` photographs it, and for a "
            "while each had its own: its own take-off loop, its own aim settle, its own "
            "retry count, and its own hold schedule -- the recorder held while "
            "`tick < TAKE_TICKS / 4` and the renderer while `shot < frames / 4`. Those "
            "happened to agree at 37 ticks, which is why nothing caught it: two tools "
            "photographing the same move by coincidence. Worse, the renderer's spelling "
            "made CAPTURE PARAMETERS change the move -- asking for 12 pictures instead "
            "of 24 charged the smash half as long. "
            "`support/move_exercise.rs` owns the press table, `action_frame`, "
            "`HOLD_TICKS` and `prepare`; a driver that constructs a button field is "
            "growing a second answer to a question that has one. If a driver needs a "
            "posture or a schedule the shared exercise cannot express, add it THERE."
        ),
    },
    {
        "id": "the-recorders-do-not-resolve-their-own-combat-geometry",
        "paths": [
            "game/ambition_app_tools/src/bin/moveset_takes.rs",
            "game/ambition_app_tools/src/bin/moveset_render.rs",
            # The shared serializer is held to the same rule it exists to
            # enforce: it reads the view and joins identity, and resolves
            # nothing.
            "crates/ambition_sim_harness/src/combat_observation.rs",
        ],
        "patterns": [
            r"world_volume|world_aabb",
            r"DamageableVolumes",
            r"ResolvedHurtboxes",
        ],
        "reason": (
            "ONE RESOLVER FOR HIT AND HURT GEOMETRY, AND IT IS THE ENGINE'S. "
            "`moveset_takes::sample` queried `Hitbox`, built its own owner-position "
            "map and called `world_volume` itself -- a second implementation of a "
            "rule `CombatGeometryView` already owns, which had already been wrong "
            "once: it reached for `world_aabb`, so a rotated box, a disc and a "
            "sweeping arc were all recorded as the axis-aligned rectangle CONTAINING "
            "them. It also had no damageable geometry at all, so a recording could "
            "show a strike passing through a fighter and could not say whether that "
            "fighter was hittable there. "
            "`CombatGeometryView` resolves strike volumes with the same call the "
            "combat resolver uses and applies the runtime's three-way damageable "
            "rule (published / published-empty is intangible / unpublished falls "
            "back to the coarse box); `combat_observation` serializes it. A tool "
            "that reads `Hitbox` or `DamageableVolumes` directly is growing a second "
            "answer to a question that has one. If the view is missing a fact the "
            "observatory needs, EXTEND THE VIEW -- that is how `damage` and "
            "`hurtbox_source` got there."
        ),
    },
    {
        "id": "ending-a-move-goes-through-the-one-teardown-path",
        "paths": [
            "crates/",
            "game/",
            # `cancel_move_playback` IS the path, and it is the removal.
            ":(exclude)crates/ambition_combat/src/moveset/mod.rs",
        ],
        # POSIX ERE -- `git grep` does not take non-capturing groups. Matching
        # `remove::<` as a substring covers `try_remove::<` too, which is the
        # spelling two of the three offenders used.
        "patterns": [r"remove::<[ A-Za-z0-9_:]*MovePlayback"],
        "reason": (
            "ENDING A MOVE MEANS ENDING ITS STRIKE BOXES. A move's hit volumes are "
            "spawned entities the playback owns by id; stripping `MovePlayback` on "
            "its own orphans them, and they stand until the next tick's "
            "`retire_orphaned_strike_volumes` sweep. `cancel_move_playback` is the "
            "one teardown path and despawns both. "
            "This has been rediscovered twice. The helper's own doc records FOUR "
            "hand-copies, one of them carrying the comment 'Tear down exactly as "
            "natural completion does (the ONE teardown path)' -- a claim the code "
            "made true by duplication. Three more survived that consolidation: the "
            "smash respawn, the versus round reset, and an interrupted boss windup. "
            "A comment cannot hold this invariant, because the wrong version is one "
            "line shorter and reads correct. "
            "If a site genuinely has no boxes to despawn (a windup, say), call the "
            "helper anyway -- it is a no-op there, and being a no-op is not a reason "
            "to keep a second meaning of 'cancel this move' alive."
        ),
    },
    {
        "id": "the-generic-brain-does-not-grow-new-platform-fighter-edges",
        "paths": [
            "crates/ambition_characters/src/brain/",
            # the codec that rewinds the brain, outside `brain/` and named the
            # fighter's fields by hand — see the pattern note below.
            "crates/ambition_characters/src/snapshot_impls.rs",
            # THE PLATFORM-FIGHTER BRAIN ITSELF. `brain/fighter/` is the
            # capability; it is allowed to know what it is.
            ":(exclude)crates/ambition_characters/src/brain/fighter",
            # 1. ✔ CLOSED 2026-08-28, so the CARVE-OUT GOES WITH IT. The generic
            #    `BrainSnapshot` carried `attack_kit:
            #    Vec<fighter::options::AttackCandidate>` — every brain paying for
            #    a field only one of them reads, in a type only one of them owns.
            #    The kit was never fighter vocabulary (`AttackBinding`'s own doc:
            #    *"the ordinary gesture vocabulary, not a fighter-only bypass"*)
            #    and lives in `brain/attack_kit.rs` now. ⛔ an exclusion kept past
            #    the edge it excused is a hole, not a record.
            # 2. the generic `StateMachineCfg` has a `Fighter` VARIANT holding
            #    `FighterCfg`/`FighterState`, so the shared state-machine brain
            #    cannot compile without the fighter one. This is the big edge:
            #    removing it needs a registration seam so a capability supplies
            #    its own brain variant.
            ":(exclude)crates/ambition_characters/src/brain/state_machine/mod.rs",
            # 4. `Brain`'s rollback cursor codec encodes the fighter's own state
            #    fields. The capability owes its own rollback row before this can
            #    go — the pattern `SmashHoldState` proved. ⇒ the file is excluded
            #    HERE and re-covered by its own contract below, which watches
            #    `fighter::` on it and lets only the enum variant through. A
            #    whole-file exemption for one known edge is a hole: a returning
            #    `fighter::` type would have walked in beside it.
            ":(exclude)crates/ambition_characters/src/snapshot_impls.rs",
            # 5. `brain/mod.rs` maps the variant to the string `"fighter"` for
            #    diagnostics. The SMALLEST edge and the one a registration seam
            #    answers for free: a registered brain carries its own name, so
            #    the generic side stops enumerating names it does not own.
            ":(exclude)crates/ambition_characters/src/brain/mod.rs",
        ],
        # Watch both direct module references and the enum variant used by the
        # rollback codec outside `brain/`.
        "patterns": [r"\bfighter::", r"StateMachineCfg::Fighter"],
        "reason": (
            "The 2026-08-19 GPT review withdrew the standing 'do not carve yet' hold on "
            "D166: 'I no longer think that should be treated as indefinitely binding... "
            "the carve has now been earned.' It also said to make the SEMANTIC boundary "
            "load-bearing first and let a dedicated platform-fighter capability crate "
            "follow only if the dependency result comes out clean. This contract is that "
            "boundary, stated as a checkable claim: the generic brain names the "
            "platform-fighter brain in exactly TWO places now (2026-08-28, down from "
            "three when this was written and five when it was first costed), and it "
            "may not grow a third. Without it the carve gets costed from a photograph months later, "
            "which is how the previous estimate went stale."
        ),
    },
    {
        "id": "the-brain-codec-names-the-fighter-only-through-the-enum-variant",
        "paths": ["crates/ambition_characters/src/snapshot_impls.rs"],
        # ⛔ ONE PATTERN, DELIBERATELY. The sibling contract above watches BOTH
        # `fighter::` and `StateMachineCfg::Fighter`; this file legitimately
        # matches the second (the rollback cursor codec has to encode the variant
        # it is rewinding), so it is excluded there and covered here by the half
        # that is still forbidden.
        "patterns": [r"\bfighter::"],
        "reason": (
            "The rollback cursor codec matches `StateMachineCfg::Fighter` because it "
            "encodes the brain it rewinds, and that edge goes when the registration "
            "seam does. It must not acquire any OTHER platform-fighter dependency in "
            "the meantime. The sibling contract excluded this whole file for the one "
            "known match, which meant a new `fighter::` type could be added here and "
            "the ratchet would stay green -- the exemption was broader than the edge "
            "it excused, during the migration it exists to ratchet. Zero `fighter::` "
            "matches here as of 2026-08-28, when `AttackVerb` moved to "
            "`brain::attack_kit`; this keeps it that way."
        ),
    },
    {
        "id": "player-input-frame-mirror-does-not-return",
        "paths": ["crates/", "game/"],
        "patterns": [r"\bPlayerInputFrame\b"],
        "reason": (
            "Per-tick participant input is owned by SlotControls and selected by "
            "Brain::Player(slot); body mechanics consume ActorControl. Reintroducing "
            "an entity-local PlayerInputFrame would create a second input authority "
            "whose lifetime can diverge under possession, couch play, and rollback."
        ),
    },
    {
        "id": "a-second-writer-of-a-match-global-must-answer-ownership",
        "paths": [
            "crates/",
            "game/",
            # ActiveMatch is PUBLISHED by activation - the one place a match
            # becomes active - and RETIRED by the versus stage's ownership-gated
            # teardown.
            #
            # HOW TO TELL THEM APART, when this next goes red: grep the path this list already
            # names for the write.
            #
            # Activation stayed the one writer. 2026-09-03: activation's file is
            # `match_activation.rs` (preparation left for `ambition_match`); the
            # exemption moved with the writer, it did not widen.
            ":(exclude)crates/ambition_platformer2d_actor_monolith/src/character_runtime/match_activation.rs",
            ":(exclude)game/ambition_app/src/app/versus.rs",
        ],
        "patterns": [
            r"commands *\. *(insert_resource *\([^;]*|remove_resource::< *[A-Za-z0-9_: ]*)(ActiveMatch)",
        ],
        "reason": (
            "`ActiveMatch` is a GLOBAL resource shared by "
            "every experience in the host, and unlike `MatchParticipantRoster` it "
            "carries no `published_by` - so there is no ownership question to ask and "
            "no way to ask it. It is safe TODAY because exactly one writer "
            "touches it. The roster was safe the same way until Smash's character "
            "select published one from a different route, and Versus deleted "
            "another game's match every frame; the fix was an owner field, learned "
            "three separate times, the third one only after a stage opened with one "
            "fighter instead of two. This contract does not add an owner - it makes "
            "the SECOND writer visible at the moment it appears, which is when the "
            "ownership question actually has to be answered rather than months "
            "later from a photograph. If this list has to grow, growing it IS the "
            "review. "
            "GRADUATED 2026-08-10: `DeclaredCombatRules` was watched here for "
            "exactly this reason and the second writer arrived - the smash demo "
            "declares a DI budget, so the versus stage is no longer alone. The "
            "review the contract exists to force HAPPENED and its answer is an "
            "owner field: the resource carries `declared_by`, both stages name "
            "themselves, and both give it back with `releasing_owned`. A type "
            "that can answer the ownership question does not need a contract "
            "asking whether anyone will, so it left this pattern rather than "
            "being waived into it. `ActiveMatch` still cannot answer and stays. "
            "NOTE this contract sees `commands` writes only. A shell experience "
            "scope can also DELETE a match global by declaring "
            "`releasing::<ActiveMatch>()`, which is invisible here and is how "
            "two experiences came to claim sole ownership of both match globals "
            "at once (fixed 2026-08-07). That class is checked by "
            "`app_it::experience_scope_ownership`, which asks the composed scope "
            "registry rather than the source text - the two are complementary "
            "and neither subsumes the other."
        ),
    },
    {
        "id": "the-seat-topology-has-one-engine-side-creator",
        "paths": [
            "crates/",
            "game/",
            # The engine system that freezes it from the roster. This is the one
            # place a shipped build gets a topology.
            ":(exclude)crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs",
            # The rollback observatory legitimately creates one for a proof
            # session that has no roster (Ambition's own rooms). It is behind
            # `dev_tools`, which is exactly the problem below — it may HAVE one,
            # it may not be the only one that does.
            ":(exclude)game/ambition_app/src/dev/rollback_observatory.rs",
        ],
        # `commands.` is the SYSTEM spelling. The checker excludes test PATHS
        # and cannot see an inline `#[cfg(test)]` module, and `versus.rs` has one
        # that inserts a topology by hand to drive the reconciler. Keying on
        # `commands.` separates a system from a fixture's `app.insert_resource`
        # without excluding a production file wholesale — the same discriminator
        # the roster contract uses, and for the same reason.
        "patterns": [
            r"commands *\. *(insert_resource *\([^;]*LocalSeatTopology|remove_resource::< *[A-Za-z0-9_: ]*LocalSeatTopology)",
        ],
        "reason": (
            "`LocalSeatTopology` is what makes a session stop re-sampling devices "
            "- it freezes the roster's seat count and the handle->device mapping, "
            "and every consumer takes `Option<Res<..>>` and returns early without "
            "it. For months the ONLY thing that created one was the rollback "
            "observatory, behind `#[cfg(feature = \"dev_tools\")]`, which the "
            "android persona omits: so `reconcile_roster_with_frozen_topology` "
            "returned on its first line every frame in every build a player runs, "
            "and `assign_local_seat_devices` always used live discovery - the "
            "behaviour its own doc calls the bug. Every test passed because tests "
            "construct the resource by hand. A second creator makes 'is it frozen' "
            "depend on which one ran, which is the same question with two answers "
            "that this resource exists to prevent."
        ),
    },
    {
        "id": "the-global-roster-is-retired-only-by-its-owner",
        "paths": [
            "crates/",
            "game/",
            # The two experiences that publish a roster are the two that may
            # retire one, and BOTH ask `is_published_by` first. They are named
            # here rather than trusted: the point is that a THIRD site cannot
            # appear without this list changing.
            ":(exclude)game/ambition_app/src/app/versus.rs",
            ":(exclude)game/ambition_demo_smash/src/lib.rs",
        ],
        # `commands.remove_resource` is the SYSTEM spelling; the checker excludes test PATHS but
        # cannot see an inline `#[cfg(test)]` module, and `input_systems.rs` has one that
        # retires a roster to prove a seat is retired with it. Keying on `commands.` separates a
        # system from a test's `app.world_mut()` without excluding a production file wholesale.
        # an exclusive system using `world.remove_resource` would slip; if one ever appears,
        # this pattern grows rather than the exclusion list. ERE, not PCRE — `git grep -E`. A
        # `(?:…)` group makes git exit 2 with "Invalid preceding regular expression", which the
        # harness turns into a crash whose exit code 1 looks EXACTLY like the contract firing.
        #
        # the module path is OPTIONAL. The first draft matched only the bare
        # type name, and a probe using
        # `remove_resource::<crate::character_runtime::MatchParticipantRoster>`
        # sailed straight through — a contract that only sees one spelling of the
        # thing it forbids is a contract the next person writes around by accident.
        "patterns": [
            r"commands *\. *remove_resource::< *([A-Za-z0-9_]+ *:: *)*MatchParticipantRoster"
        ],
        "reason": (
            "`MatchParticipantRoster` is a GLOBAL resource shared by every "
            "experience in the host, and clearing 'the roster' is how one game "
            "deletes another's match. The rule has been learned three times: "
            "Versus's teardown got an `is_published_by` guard after it deleted "
            "Smash's match every frame, Smash's select-arrival reset got one, and "
            "on 2026-08-01 the reconciler - which had none - rebuilt Smash's "
            "roster with a builder that stamps VERSUS ownership, so Versus then "
            "deleted it as its own and Smash's match opened with one fighter "
            "instead of two. Nothing named the rule in one place, so each site "
            "learned it separately and the third had not. A new removal site "
            "belongs beside an ownership question; if this list has to grow, the "
            "growth is the review."
        ),
    },
    {
        "id": "central-rollback-does-not-enumerate-domains",
        "paths": ["crates/ambition_platformer2d_runtime/src/rollback/mod.rs"],
        "patterns": [
            # The host may compose a domain's ONE public rollback offer. It may
            # not reach through that seam to name any concrete gameplay type.
            {
                "grep": r"ambition_platformer2d_actor_monolith::",
                "match": r"ambition_platformer2d_actor_monolith::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_boss_encounter::",
                "match": r"ambition_boss_encounter::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_characters::",
                "match": r"ambition_characters::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_combat::",
                "match": r"ambition_combat::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_conversation::",
                "match": r"ambition_conversation::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_cutscene::",
                "match": r"ambition_cutscene::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_encounter::",
                "match": r"ambition_encounter::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_items::",
                "match": r"ambition_items::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_platformer2d_shared_tangle::",
                "match": r"ambition_platformer2d_shared_tangle::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_portal2d::",
                "match": r"ambition_portal2d::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_projectiles::",
                "match": r"ambition_projectiles::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_sprite_sheet::",
                "match": r"ambition_sprite_sheet::(?!register_rollback_state\b)",
            },
            {
                "grep": r"ambition_vfx::",
                "match": r"ambition_vfx::(?!register_rollback_state\b)",
            },
        ],
        "reason": (
            "Domain-owned rollback migration. The runtime may compose a domain's "
            "single `register_rollback_state` offer, but the concrete types and "
            "their projections belong in the crate that owns them. A new "
            "gameplay registration therefore changes the owning crate, not a "
            "runtime `domains/` census. Runtime-adjacent state — engine_core, "
            "persistence, sfx, sim_view, time, and host/session machinery — may "
            "still be registered directly here."
        ),
    },
    {
        "id": "registration-does-not-demand-art",
        # BOTH halves of registration, since the P1.7 split. A guard scoped to the file that
        # kept the seam would have watched the wrong half of its own subject, which is the
        # "instrument that measures nothing" shape this file's header names.
        "paths": [
            "crates/ambition_platformer2d_actor_monolith/src/character_runtime/definition.rs",
            "crates/ambition_characters/src/prepared.rs",
        ],
        "patterns": [r"CharacterLoadDemand::request", r"\bdemand\.request\("],
        "reason": (
            "Registering a character DECLARES it; it does not ask for its art. "
            "Demanding at registration defeats the room/match/worn projection "
            "model — every registered character would decode whether or not "
            "anything staged it. Staging is the only demand source (queue A1). "
            "`CharacterLoadDemand` is legitimately named in this file's prose, "
            "which is why this matches code only."
        ),
    },
    {
        "id": "the-worlds-path-is-confined-to-ldtk-paths",
        # The subject includes TEST modules: five of them spelled the path too,
        # and a contract that guards only the package would have watched half a
        # migration.
        "include_tests": True,
        "paths": [
            "tools/ambition_ldtk_tools/",
            # The one legitimate home, and the test that pins what it returns.
            ":(exclude)tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py",
            ":(exclude)tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py",
        ],
        "patterns": [r'/\s*"worlds"'],
        "reason": (
            "The LDtk worlds directory is built in ONE place, `ldtk/paths.py`, "
            "whose own docstring says it exists 'so individual commands do not "
            "recreate stale repository-layout assumptions'. Fifteen commands and "
            "five test modules then recreated them anyway, in three different "
            "spellings, and when the worlds moved out of "
            "`crates/ambition_platformer2d_actor_monolith/assets` every one of them broke: 11 of 149 "
            "tests red on a clean tree, and a bare `ldtk edit measure` dying with "
            "FileNotFoundError because its `--ldtk` default pointed at a "
            "directory that had not existed for weeks (2026-07-28). A second "
            "copy of a correct path is a copy that has not gone stale YET."
        ),
    },
    {
        "id": "no-string-keyed-sheet-row-lookup",
        "paths": ["crates/", "game/", "fixtures/", "tools/"],
        "patterns": [r"\.row_index_of\(", r"\bfn row_index_of\b"],
        "reason": (
            "A sheet row addressed by an arbitrary &str returned None when the "
            "sprite called the animation `death` and the policy asked for "
            "`dead`, and NOTHING DREW — an absence rendered as an absence. The "
            "lookup was deleted in favour of resolved bindings, and the story "
            "survives only as prose in two `binding.rs` module docs "
            "(binding-resolution boundary, 2026-07-25/26)."
        ),
    },
    {
        # "migrated" is a claim about what no longer exists, which is exactly the
        # kind of claim this file is for.
        "id": "outlander-does-not-hand-order-its-own-composition",
        # This contract is scoped to the external consumer; in-repo composition
        # is measured separately.
        "paths": ["fixtures/external_consumer/"],
        # The fixture's tests ARE the consumer — a third party exercising the
        # public API — so a test that rebuilds the composition by hand is exactly
        # the second path this forbids, not an exemption from it.
        "include_tests": True,
        "patterns": [
            # The engine ordering rules `ambition_platformer2d::app` now owns.
            r"\badd_headless_foundation\b",
            r"\binit_engine_states\b",
            r"\bPlatformerEnginePlugins\b",
            r"\bPlatformerHostPlugins\b",
            r"\bPlatformerAssetsPlugin\b",
            r"\bPlatformerPresentationPlugin\b",
            r"\bMinimalShellPlugins\b",
            # Bevy's own group: `PlatformerApp` decides windowed-vs-headless and
            # which five plugins a GPU-less window disables. A consumer adding
            # `DefaultPlugins` itself has taken that decision back.
            r"\bDefaultPlugins\b",
        ],
        "reason": (
            "The fixture had three hand-ordered builders totalling ~110 lines "
            "(`build_outlander_app`, `build_outlander_rollback_app`, "
            "`build_windowed_app`) plus two shared helpers, and between them they "
            "restated eight engine ordering rules — four of which failed "
            "silently: an asset source registered after `AssetPlugin` sealed its "
            "sources, a GPU-less window missing one of five disables, engine "
            "groups before `init_engine_states`, a host naming no initial route "
            "and therefore preparing nothing. All three now go through "
            "`ambition_platformer2d::app::PlatformerApp`, and `src/bin/dump.rs` — the last "
            "hand-ordered path, which also installed the WINDOWED host in a "
            "headless dump — was retired with it. Reintroducing any of these "
            "names in the consumer means a second composition exists, which is "
            "the state slice A4 is defined as ending. Retiring them is also what "
            "closed `ambition_platformer2d::engine` and `ambition_platformer2d::windowed_host` on the A1 "
            "ratchet, so a regression here shows up in two places."
        ),
    },
    {
        "id": "rollback-exit-oracle-is-not-quarantined",
        # The one contract whose SUBJECT is a test file. Everything else is about
        # production code, so tests are excluded by default — see `violations`.
        "include_tests": True,
        "paths": ["game/ambition_app/tests/rollback_exit_oracle.rs"],
        "patterns": [
            # An `#[ignore]` here is either opt-in TOOLING (a bisection you run
            # when the oracle is red) or a DISABLED GUARD. Only the second is
            # the absence this contract is about, and the reason string is what
            # tells them apart — so the contract requires one, which also makes
            # "why is this off?" answerable without reading the test body.
            {
                "grep": r"#\[ignore",
                "match": r"#\[ignore(?!\s*=\s*\"(?:diagnostic|audit|measurement)\b)",
            },
            "known GGRS divergence",
        ],
        "reason": (
            "This oracle guards a KNOWN determinism failure, so an ordinary "
            "green run has to include it. It was `#[ignore]`d and red for a "
            "long time; the file now explains that history in prose, and the "
            "prose must not be mistaken for the attribute coming back "
            "(queue A6). Release blocker for rollback multiplayer. A new "
            "`#[ignore]` is allowed only as `diagnostic`/`audit`/`measurement` "
            "tooling — anything else is a guard being switched off, which is "
            "queue E1's rule applied where it was learned."
        ),
    },
    {
        "id": "the-character-domain-is-not-named-after-a-character",
        "paths": ["crates/ambition_characters/"],
        "patterns": [
            # An ITEM named after one creature. Comment-stripped, so the doc
            # citations in `entry.rs` ("48.0 is not invented: it is the
            # protagonist, `player_robot_v3`") and the `npc_puppy_slug` fixtures
            # in `character_catalog/binding.rs` are not what this is about — a
            # crate may EXPLAIN itself with a concrete example and may TEST with
            # one. What it may not do is own one's policy.
            {
                # `git grep` prefilters with ERE, then the Python `match`
                # refines — the declaration keyword is what makes this about
                # OWNERSHIP rather than about the string appearing at all.
                "grep": r"player_robot|PLAYER_ROBOT",
                "match": r"\b(?:fn|const|static|struct|enum|trait)\s+\w*(?:player_robot|PLAYER_ROBOT)\w*",
            },
        ],
        "reason": (
            "`ambition_characters` is the reusable authored-template layer — "
            "the crate whose whole claim is that a character is DATA. On "
            "2026-08-12 it acquired `apply_player_robot_slash_sfx` and three "
            "`PLAYER_ROBOT_*` cue constants: canonical protagonist presentation "
            "policy, in the domain crate, because the bulk move that lowered "
            "`moveset/prefabs.rs` took everything that was ADJACENT in the file "
            "rather than everything preparation CALLS. Measured, the overlay had "
            "exactly one production caller and it was the protagonist road; "
            "`prepare_character` never reached it (GPT 5.6 review of 1579ab3). "
            "The generic builder vocabulary it reads — SWING_SFX_CUE, "
            "SLASH_ARC_VFX, SLASH_POKE_VFX — is genuinely the builders' and "
            "stays. The test is whether preparation needs it, and an item NAMED "
            "after one creature has already answered no."
        ),
    },
    {
        "id": "the-character-fold-is-not-a-public-capability",
        "paths": ["crates/ambition_characters/src/prepared.rs"],
        "patterns": [
            # A PUBLIC mint or a PUBLIC consumer of the staged partial. Either
            # one alone is enough: the fold is spellable the moment both ends of
            # the pipe are reachable, and they were both `pub` for one day.
            {
                "grep": r"pub (fn|struct) (finalize_cast|prepare_for_registration|StagedCharacter|StagedRegistration|StagedCharacterOverrides)",
                "match": r"^\s*pub (?:fn|struct) (?:finalize_cast|prepare_for_registration|StagedCharacter|StagedRegistration|StagedCharacterOverrides)\b",
            },
        ],
        "reason": (
            "`CharacterPreparationPlugin` exists to stop a cast being folded "
            "before the catalog it inherits from is installed — a provider that "
            "folded early would bake an empty row in permanently, and which "
            "provider goes first is a composition detail no provider can see. "
            "That guarantee was module privacy until campaign P1.7 moved the "
            "model down and left the lifecycle up, which forced `finalize_cast` "
            "and `prepare_for_registration` to be `pub` so the App layer could "
            "reach them. The staged value stayed opaque and the docs called "
            "early folding 'unspellable' — but opacity of a FIELD prevents "
            "nothing when both ends of the pipe are public: "
            "`finalize_cast([prepare(..).staged], whatever_catalog_exists_now, ..)` "
            "is ordinary safe code (GPT 5.6 review, priority 2). The lifecycle "
            "moved down beside the fold, and what crosses the crate boundary is "
            "a CONTRIBUTION (`stage_authored_character`) and a finished READ "
            "(`PreparedCharacterRegistry`) — never the fold. `test-support` "
            "keeps `prepare_and_finalize_for_test` for tests of pure folding; "
            "that is a separate, feature-gated road and this contract does not "
            "name it."
        ),
    },
    {
        "id": "fight-tests-do-not-hand-roll-damage",
        "paths": ["crates/", "game/", "fixtures/"],
        "patterns": [r"\b\w*_hp\s*-=", r"\bhp\s*-=\s*\d"],
        "reason": (
            "The two-provider fight test used to compute an AABB overlap by "
            "hand and subtract integers from local variables: no entities, no "
            "`MovePlayback`, no hit events, no `BodyHealth`. It proved the test "
            "could do arithmetic. Damage is asserted through the production "
            "path or not at all (queue A2)."
        ),
    },
    {
        "id": "the-catalog-default-action-set-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            # The exemption is the same ONE file it always was; only its address changed, and this
            # guard catching the move is the guard working.
            ":!crates/ambition_characters/src/prepared.rs",
            ":!crates/ambition_combat/src/worn_kit.rs",
            ":!crates/ambition_characters/src/actor/character_catalog/mod.rs",
        ],
        "patterns": [r"\bbuild_default_action_set\b"],
        "reason": (
            "`apply_worn_character_kit` is the ONE place the catalog's default "
            "action set is read and weighed against the definition's authored "
            "one (queue C3, 2026-07-28). A second reader forks the precedence, "
            "and the failure is silent: the two answers agree for every "
            "character that authored nothing, which is most of them, so it "
            "presents only on the one character somebody bothered to author — "
            "exactly how the seated-fighter gap hid behind two duelists whose "
            "authored set happened to equal the default. "
            "⚠ 2026-07-29: the fold MOVED to `definition.rs`, where "
            "`finalize_character` performs it once per character at the "
            "preparation barrier. `starting_character.rs` keeps a caller for "
            "the ids nothing REGISTERED — most of the legacy cast — which have "
            "no prepared value to weigh against. Two files, still one decision "
            "per character; the day a registered character is resolved in both "
            "is the day this contract has stopped meaning anything, so read "
            "them together before adding a third. "
            "2026-09-03: the unregistered-id caller moved out of the actor kernel "
            "with the worn-kit compiler (`ambition_combat::worn_kit::WornKit::resolve`); "
            "the kernel no longer reads the catalog's default set at all."
        ),
    },
    {
        "id": "the-provider-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            ":!crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs",
        ],
        "patterns": [r"\bprovider_of_character\("],
        "reason": (
            "Which provider owns a character — registry first, catalog owners "
            "second — is decided in `presentation.rs` and consumed everywhere "
            "else. A second caller is a second answer, and the failure is a "
            "body emitting in the wrong provider's voice: audible, "
            "attributable to nothing, and only on a crossover stage where two "
            "providers are live at once."
        ),
    },
    {
        "id": "the-catalog-axis-tuning-is-confined-to-one-file",
        # ENGINE crates only, and that is the claim rather than a concession.
        # What must stay singular is the place the ENGINE weighs a catalog
        # tuning against a definition's. A game reading its own catalog is a
        # game reading its own content — Mary-O's inline tests assert her
        # classic-physics numbers that way, and forbidding it would be
        # forbidding a provider to know what it authored.
        "paths": [
            "crates/",
            # The exemption is the same ONE file it always was; only its address changed, and this
            # guard catching the move is the guard working.
            ":!crates/ambition_characters/src/prepared.rs",
            ":!crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs",
            ":!crates/ambition_characters/src/actor/character_catalog/mod.rs",
        ],
        "patterns": [r"\baxis_tuning\("],
        "reason": (
            "The catalog's movement feel is read in one place and weighed "
            "against the definition's there. This one nearly broke on the "
            "commit that introduced it: the seated projection first read the "
            "prepared value DIRECTLY, so for a character with catalog tuning "
            "and no authored tuning the worn path inserted the marker and the "
            "projection removed it on the same tick. Two paths answering one "
            "question, reintroduced by the commit closing that exact failure. "
            "⚠ 2026-07-29: `definition.rs` joined the allowed set because the "
            "fold moved to the barrier — and the projection's half of that old "
            "bug is GONE with it, because there is no longer a prepared value "
            "the projection could read `None` from while the worn path read a "
            "row."
        ),
    },
    {
        "id": "the-movement-tuning-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            # The ONE production caller, and the re-export that lets it be one.
            ":!crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs",
            ":!crates/ambition_platformer2d_actor_monolith/src/avatar/mod.rs",
        ],
        "patterns": [r"\bmovement_tuning_for_character\("],
        "reason": (
            "`movement_tuning_for_character` resolves prepared-vs-catalog for a "
            "body's movement tuning, and it is one of the four surviving "
            "character resolvers Campaign 1 decided to KEEP rather than collapse "
            "(X12: they answer different fields on different cadences, and one "
            "universal resolver is the premature abstraction the review warns "
            "against). Keeping four is only safe while each has exactly one "
            "caller — the campaign's content is a guard that no FIFTH appears. "
            "Found 2026-07-28 by verifying the claim that every resolver was "
            "pinned: three were, this one was not, and an unguarded resolver is "
            "the one a second caller grows on."
        ),
    },
    {
        "id": "the-motion-model-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            ":!crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs",
        ],
        "patterns": [r"\bmotion_model_spec_for_character\("],
        "reason": (
            "The definition-first movement policy resolver (R-a, 2026-07-28). "
            "The catalog-only `motion_model_spec_for_character_id` is "
            "deliberately NOT covered — it is the fallback this one calls, and "
            "two tests plus a from-scratch bundle legitimately have no registry "
            "to consult. What must stay singular is the place that WEIGHS them."
        ),
    },
    {
        "id": "the-actor-mirrors-stay-deleted",
        "paths": ["crates/", "game/", "fixtures/"],
        "patterns": [r"\bActorIntent\b", r"\bActorCooldowns\b"],
        "reason": (
            "AC1 of the authority-convergence campaign (2026-08-13). These two "
            "components MIRRORED state the body already owned: an intent the "
            "brain had already published and cooldowns the move playback already "
            "tracked, kept in a second place and reconciled by a sweep. The "
            "campaign deleted them (29 and 25 references at arming, 0 at close) "
            "and the ratchet is the point — a mirror does not come back as a "
            "mirror, it comes back as one innocent helper that needs 'just the "
            "intent' somewhere the body is not in scope. "
            "MIGRATED HERE 2026-08-15 from a goal-guard check that was a bare "
            "`grep -rn --include=*.rs`. That form reddens on a doc comment "
            "EXPLAINING the removal, which is the recurrence this file's header "
            "documents three times; contracts strip comments before matching."
        ),
    },
    {
        "id": "no-build-legacy-body-then-patch-it",
        "paths": ["crates/", "game/", "fixtures/"],
        "patterns": [r"\badopt_character_intrinsics\b"],
        "reason": (
            "AC5 of the same campaign. This was the seam of the two-step "
            "construction the campaign existed to remove: build a body from the "
            "legacy archetype, THEN patch the authored character's intrinsics "
            "over it. While that call existed, the character definition was a "
            "correction applied to a legacy default rather than the authority, "
            "so every intrinsic had two possible origins and 'which one won' was "
            "a function of ordering. Construction reads the definition directly "
            "now; a reappearance of this symbol means the legacy default is back "
            "underneath, which is the failure mode 'DELETION IS THE PROOF' names. "
            "MIGRATED HERE 2026-08-15 from a goal-guard check, same reason as "
            "`the-actor-mirrors-stay-deleted`."
        ),
    },
]

# Files whose content is ABOUT the contracts rather than governed by them.
SELF_REFERENTIAL = {"scripts/check_absence_contracts.py"}

# Dependency-edge contracts, read from `cargo metadata` rather than from text.
#
# A grep cannot express "crate A must not depend on crate B". It can find the `use` that proves
# it, and miss the one added through a re-export tomorrow; it cannot see a dependency introduced
# through an intermediary at all.
#
# `forbidden` is checked TRANSITIVELY. The claim being guarded is never "no
# direct dependency line" — it is that a foundation crate cannot REACH gameplay,
# and reaching it through one intermediary is the same architectural failure with
# an extra hop. A layering inversion almost never arrives as a direct edge.
DEPENDENCY_CONTRACTS: list[dict] = [
    {
        "id": "geometry-is-the-floor",
        "crate": "ambition_geometry",
        "forbidden": "*",
        "reason": (
            "Shapes, boxes and reference frames — the vocabulary every other "
            "crate is written in terms of, and the one layer with no workspace "
            "dependency at all. This is what `ambition_platformer2d_core`'s "
            "contract used to say; the floor MOVED DOWN when the kernel was "
            "carved out, and the guarantee has to move with it or it is not a "
            "floor, it is a habit."
        ),
    },
    {
        "id": "projectile-spec-is-a-floor",
        "crate": "ambition_projectile_spec",
        "forbidden": "*",
        "reason": (
            "Content-free spawn data. It exists so a consumer can take the "
            "vocabulary without taking a 16,927-line platformer crate, and that "
            "is worth exactly nothing if it grows a workspace dependency of its "
            "own — the closure it was carved to shrink would grow straight back. "
            "⚠ this is the SECOND crate to carry `forbidden: \"*\"`, which is "
            "the rule an `allowed` entry has to satisfy: a named allowance is "
            "only safe if the crate it names is itself a floor."
        ),
    },
    {
        "id": "engine-core-is-the-floor",
        "crate": "ambition_platformer2d_core",
        "forbidden": "*",
        # the ONE edge, named. Core sits on the geometry kernel and on
        # nothing else; `ambition_geometry` carries `forbidden: "*"` above, so
        # the chain still bottoms out with no outward edge. Widening this list
        # is how "the floor" becomes "roughly the floor".
        "allowed": ["ambition_geometry"],
        "reason": (
            "The movement and body vocabulary every other crate is written in "
            "terms of. It depends on NO workspace crate except the geometry "
            "kernel it was carved from, and that is what makes it the layer "
            "everything else can agree on rather than one more participant in "
            "a cycle. A second edge out of here makes the whole graph a "
            "suggestion."
        ),
    },
    {
        "id": "platformer-primitives-stays-a-foundation",
        "crate": "ambition_platformer2d_shared_tangle",
        "forbidden": [
            "ambition_platformer2d_actor_monolith",
            "ambition_characters",
            "ambition_combat",
            "ambition_platformer2d_runtime",
            "ambition_content",
            "ambition_platformer2d",
        ],
        "reason": (
            "Session scope, binding resolution and stable ids — the vocabulary "
            "gameplay crates USE. It sits directly above `engine_core` and "
            "below everything that has opinions about actors, so a dependency "
            "on one of those inverts the layering the refactor timeline is "
            "built on (foundations < gameplay_core < content)."
        ),
    },
    {
        "id": "characters-do-not-depend-on-the-actor-integration-layer",
        "crate": "ambition_characters",
        "forbidden": ["ambition_platformer2d_actor_monolith", "ambition_platformer2d_runtime", "ambition_platformer2d"],
        "reason": (
            "`ambition_platformer2d_actor_monolith` depends on `ambition_characters`, which makes "
            "the reverse edge a cycle waiting to be discovered by the compiler "
            "at the worst moment. It also matters for the deferred "
            "`ambition_platformer2d_actor_monolith` decomposition: if a coherent actor kernel exists "
            "at all, `ambition_characters` is below it."
        ),
    },
    {
        "id": "engine-crates-do-not-consume-the-umbrella-facade",
        "crate": "ambition_platformer2d_actor_monolith",
        "forbidden": ["ambition_platformer2d"],
        "reason": (
            "`ambition_platformer2d` is the facade a CONSUMER builds a game against; it "
            "re-exports `ambition_platformer2d_actor_monolith` among thirty-odd others. An engine "
            "crate reaching back through it is circular by construction, and it "
            "is how a headless consumer ends up compiling the render stack. "
            "⚠ deliberately scoped to engine crates: `ambition_content` DOES "
            "depend on the facade today, and whether that should stop is a "
            "MEASUREMENT question the campaign defers rather than a rule."
        ),
    },
]

# Consumer-module ratchets freeze the exact named set rather than a count:
#   1. `named ⊆ allowed ∪ baseline` forbids new internal-module dependencies.
#   2. `baseline ⊆ named` removes baseline entries as consumers migrate away.
# An empty allowlist therefore converges monotonically toward the public SDK.
MODULE_ALLOWLISTS: list[dict] = [
    {
        "id": "outlander-names-only-the-public-sdk",
        # This contract measures only the external-consumer fixture.
        "paths": ["fixtures/external_consumer/"],
        # Fixture tests exercise the same public API and are part of this consumer surface.
        "include_tests": True,
        "facade": "ambition_platformer2d",
        # Reviewed public SDK surface. Add a module only when the SDK contract names it;
        # an allowlist entry is a compatibility commitment, not a ratchet escape hatch.
        "allowed": {
            "actor",
            "app",
            "bevy",
            "character",
            "rollback",
            "sim",
            "view",
            "world",
        },
        # Empty baseline: this consumer names no implementation-shaped modules.
        "baseline": set(),
        "reason": (
            "A game depends on `ambition_platformer2d`, and `ambition_platformer2d` is currently the list "
            "of crates the engine happens to be built from — so a consumer's "
            "imports encode our implementation topology and we cannot move an "
            "implementation without breaking them (ADR 0031). Outlander reaches "
            "through the facade for `ambition_platformer2d::runtime::rollback::put_f32`: a "
            "third party building a game is naming an internal serialisation "
            "helper. Each name in `baseline` is one leak still open. The set may "
            "not GAIN a member, and it may not KEEP one the consumer has stopped "
            "naming — see the two invariants above. Zero means consumers name "
            "only the SDK."
        ),
    },
    {
        "id": "minimal-game-names-only-the-public-sdk",
        # The minimal-game fixture has an independent public-SDK ratchet.
        "paths": ["fixtures/minimal_game/"],
        "include_tests": True,
        "facade": "ambition_platformer2d",
        # `bevy` is a documented facade re-export and therefore part of the public surface.
        "allowed": {"actor", "app", "bevy", "character", "sim", "view", "world"},
        # Empty baseline: the minimal consumer now names only reviewed SDK
        # surface. New implementation-module imports must fail this ratchet.
        "baseline": set(),
        "reason": (
            "The movement-only minimal game is the consumer-matrix row Outlander "
            "structurally cannot fill: it asks for almost nothing, so whatever it "
            "still has to name is a floor on what EVERY game must know. Four "
            "modules, all of them the room/experience declaration path — since "
            "reduced to ZERO by slices B and C. The set "
            "may not GAIN a member, and it may not KEEP one this game stops "
            "naming."
        ),
    },
    {
        "id": "sim-harness-names-only-the-public-sdk",
        # The reusable sim harness is a real engine customer: it owns a Bevy App,
        # queries bodies, mutates test setup, drives participants, and can start
        # rollback. If it needs crate-shaped facade paths, those are SDK gaps.
        "paths": ["crates/ambition_sim_harness/"],
        "include_tests": True,
        "facade": "ambition_platformer2d",
        "allowed": {
            "actor",
            # ⭐ OFFSCREEN CAPTURE IS AN SDK CONCEPT, added 2026-08-29 when the
            # harness took the deterministic capture session in. It is NOT a
            # crate mirror — the crate is `ambition_render` — and it is behind a
            # default-off feature, so a sim that takes no pictures links none of
            # it. ⛔ the rule this widens is "no implementation topology", and a
            # game-concept module is exactly what the rule asks for.
            "capture",
            "character",
            "engine",
            "item",
            # ⭐ WHAT AN OBSERVER READS IS AN SDK CONCEPT, added 2026-08-30 when
            # the harness took the combat observation in. It is NOT a crate
            # mirror — the crate is `ambition_sim_view` — and it is the same
            # widening `capture` got: the rule is "no implementation topology",
            # and naming the CAPABILITY is exactly what the rule asks for. The
            # gap it closes was five crate-shaped paths (`engine_core`,
            # `sim_view`, `mount`, `projectiles`, `platformer::sim_id`) reached
            # for one question the engine already answers.
            "observation",
            "participant",
            "rollback",
            "session",
            "settings",
            "sim",
            "world",
        },
        "baseline": set(),
        "reason": (
            "The programmatic simulation harness is part of the supported engine "
            "surface, not an implementation crate. It used to depend only on the "
            "facade while naming `actors`, `engine_core`, `platformer`, `runtime`, "
            "`characters`, `entity_catalog`, `boss_encounter`, `input`, and "
            "`persistence` through it. Those names encoded implementation topology. "
            "Its baseline is now zero; new crate-mirror imports are SDK regressions."
        ),
    },
    {
        "id": "capability-demo-names-only-the-public-sdk",
        # The capability itself deliberately stays below the facade. Its tests are
        # the outside author/host proving that the capability composes through the SDK.
        "paths": ["examples/capability_demo/tests/"],
        "include_tests": True,
        "facade": "ambition_platformer2d",
        "allowed": {
            "app",
            "content",
            "participant",
            "rollback",
            "session",
            "sim",
            "world",
        },
        "baseline": set(),
        "reason": (
            "The capability example is the worked external-author path. Its tests "
            "must compose and inspect through semantic SDK modules, without knowing "
            "that session storage lives in shared_tangle, action registration in "
            "ambition_input, or rollback presentation timing in ambition_sim_view."
        ),
    },
]

# Central rollback ownership ratchet.
# New domain state must not enter central registration, and entries that leave
# must be pruned from the frozen baseline.

# Frozen as a SET, never as a count.
#
# §2e's subject, made non-increasing.
#
# ONE invariant, not two, and the asymmetry is deliberate. The set may not GROW. The other two
# ratchets need both halves; copying that here would add a rule with no failure behind it, which is
# how a guard becomes ceremony.
CAPABILITY_FOOTPRINT_BASELINE = (
    "scripts/baselines/capability-footprint-baseline.json"
)

CAPABILITY_FOOTPRINT_SENTINEL = "fixtures/minimal_game"


@functools.cache
def sentinel_linked_closure(root: Path) -> set[str]:
    """The `ambition_*` crates the sentinel actually LINKS, from cargo's resolver.

    Process-local cache: one checker/test invocation observes one repository tree.
    Re-running Cargo for the same root cannot add correctness while the process is
    alive; it only repeats dependency resolution. A later invocation gets a fresh
    observation.

    ⚠ Until slice H this walked the workspace manifest graph from `ambition_platformer2d`,
    which was correct while every facade edge was unconditional and became the
    wrong subject the moment they were features: a static walk counts an
    optional edge the sentinel never enabled, so the counter could never move.
    The baseline's own subject line has always been "what a consumer links by
    depending on the facade" — so ask cargo what the sentinel resolves, in the
    sentinel's own workspace, with the sentinel's own feature choices.

    `--locked` on purpose: a dependency change that alters the sentinel's
    lockfile must arrive WITH that lockfile, or this check fails loudly instead
    of silently rewriting it.
    """
    raw = subprocess.run(
        [
            cargo_binary(),
            "tree",
            "--locked",
            "--prefix",
            "none",
            "--edges",
            "normal",
        ],
        cwd=root / CAPABILITY_FOOTPRINT_SENTINEL,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {
        line.split(" ", 1)[0]
        for line in raw.splitlines()
        if line.startswith("ambition")
    }


def capability_footprint_violations(root: Path) -> list[str]:
    """Crates the sentinel links that were not in the frozen closure."""
    baseline = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
    return sorted(sentinel_linked_closure(root) - set(baseline["ambition_closure"]))


def capability_footprint_departures(root: Path) -> list[str]:
    """Crates the frozen closure still names that the sentinel no longer links.

    ⛔ THE RATCHET WAS ONE-DIRECTIONAL FOR SIX WEEKS AND THAT IS HOW THE BASELINE
    ROTTED. Growth is the violation, so growth is all it looked for -- and a
    DEPARTURE left the closure and the counts pruned by whoever noticed, while
    every sub-list that named the crate kept naming it. On 2026-09-03 five such
    names were found across two lists, one of them twelve days old, and they were
    not cosmetic: `reachable_only_through_the_facade` is the "closable by a
    manifest edit?" list a carve decision is taken off, and three of its four
    entries had already left.

    ⭐ A DEPARTURE IS GOOD NEWS AND STILL FAILS, on purpose. The footprint
    shrinking is the outcome the whole campaign wants; what must not happen is
    the shrink landing while the record of it does not. Red here means "re-freeze
    the baseline IN THIS COMMIT", which is the same discipline the checklist
    already applies to a carve that adds a crate.
    """
    baseline = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
    return sorted(set(baseline["ambition_closure"]) - sentinel_linked_closure(root))


ROLLBACK_SCHEMA_BASELINE = (
    "scripts/baselines/rollback-schema-baseline.json"
)


@functools.cache
def rollback_schema_usage(root: Path) -> dict[str, list[str]]:
    """Return stable schema names and all types in the rollback wire format.

    Cache the read-only census because several invariants query the same tree.
    The wire format is distributed across central and domain-owned impls, so the
    guard scans both. Type paths are qualified by crate to disambiguate local
    `crate::...` spellings.
    """
    # The central runtime-adjacent registrations plus domain-owned offers.
    #
    # The wire format was never a synonym for either central path.
    #
    # Deliberately NOT a glob over the whole `rollback/` directory: `codec.rs`,
    # `session.rs` and friends contain dotted string literals that are not
    # registration names, and a ratchet that swallows them measures noise.
    registration_paths = [root / "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"]
    # AND IT HAPPENED AGAIN, ONE LEVEL FURTHER OUT. The two
    # paragraphs above describe registrations leaving one FILE. They have now
    # left the runtime CRATE: `RollbackRegistrar` lets a domain register its own
    # state, so `resource.gate_portal_phases` reported as having left the schema
    # while it was still very much in the wire format.
    #
    # so stop hand-listing places and follow the MARKER instead, the same
    # lesson `encoded_types` learned by following the types. A federated
    # registration site is exactly a file whose function takes the registrar:
    # `&mut impl RollbackRegistrar`. Nothing else spells that, the trait's own
    # definition does not, and a domain that starts registering is picked up
    # without editing this script — which is the whole point, since a guard that
    # needs an edit per domain is the census it exists to prevent.
    # the marker is the TRAIT NAME, not one spelling of the bound: the first
    # federated domain writes `<R>(registrar: &mut R) where R: RollbackRegistrar`,
    # and a scanner keyed to `&mut impl RollbackRegistrar` matched none of it.
    #
    # and the DEFINING crate is excluded on purpose — the trait's own doc
    # examples spell registration names that were never in the wire format, which
    # is precisely the "swallows noise" failure the paragraph above warns about.
    registration_paths.extend(
        sorted(
            path
            for path in root.glob("crates/*/src/**/*.rs")
            if not is_test_path(str(path))
            # the BOUND, not a mention. `RollbackRegistrar` also appears in
            # imports, prose and a test-only `impl`, and matching those swallowed
            # `ability.cooldown` and the trait's own doc examples — names that
            # were never registrations. A federated registration site is a
            # function generic over the registrar, and nothing else is.
            and "R: RollbackRegistrar" in path.read_text(errors="replace")
        )
    )
    def _registration_text(path: Path) -> str:
        """Production registrations only.

        ⛔ `is_test_path` reads the PATH, and a federated domain keeps its
        registration in an ordinary source file with a `#[cfg(test)] mod tests`
        inside it — whose fake registrar names (`gate.a`, `gate.alpha`) are not
        the wire format. Cut the file at its test module.
        """
        text = path.read_text(errors="replace")
        # A declaration's file is excluded by `is_test_path` already.
        match = re.search(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{", text)
        return text if match is None else text[: match.start()]

    registration = "\n".join(
        _registration_text(path)
        for path in registration_paths
        if path.exists() and not is_test_path(str(path))
    )

    encoded: set[str] = set()
    for source in sorted(root.glob("crates/*/src/**/*.rs")):
        if is_test_path(str(source)):
            continue
        text = source.read_text(errors="replace")
        # Both `snapshot_pod!` and `snapshot_unit_enum!` GENERATE that impl inside the macro body,
        # and the macro's own definition spells it `impl $crate::snapshot::SnapshotState for $ty` —
        # so every type encoded through either macro was invisible.
        #
        # Exactly the failure this function's own header describes twice already — an instrument
        # that measures less than it says and reports the success condition.
        crate = source.relative_to(root).parts[1]
        for match in re.findall(
            r"impl SnapshotState for ([A-Za-z0-9_:<>]+)", text
        ):
            # `crate::Foo` inside `ambition_platformer2d_actor_monolith` IS `ambition_platformer2d_actor_monolith::Foo`;
            # collapse the doubled prefix so the frozen name reads like a path.
            encoded.add(f"{crate}::{match}".replace("::crate::", "::"))
        # the macro takes a `$ty:path` first, so the type is everything up to
        # the opening brace. Both macros are matched by ONE pattern on purpose: a
        # second pattern is a second thing to forget when a third macro lands.
        for match in re.findall(
            r"snapshot_(?:pod|unit_enum)!\(\s*([A-Za-z0-9_:<>]+)\s*\{", text
        ):
            encoded.add(f"{crate}::{match}".replace("::crate::", "::"))

    return {
        "stable_schema_names": sorted(
            set(re.findall(r'"([a-z_]+\.[a-z_.]+)"', registration))
        ),
        "encoded_types": sorted(encoded),
    }


def rollback_schema_violations(root: Path) -> tuple[list[str], list[str]]:
    """`new, stale` — invariant 1's breaches and invariant 2's."""
    baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
    current = rollback_schema_usage(root)
    new: list[str] = []
    stale: list[str] = []
    for key in ("stable_schema_names", "encoded_types"):
        frozen = set(baseline[key])
        live = set(current[key])
        new.extend(f"{key}: {item}" for item in sorted(live - frozen))
        stale.extend(f"{key}: {item}" for item in sorted(frozen - live))
    return new, stale


_LINE_COMMENT = re.compile(r"//.*$")
_HASH_COMMENT = re.compile(r"#(?!\[).*$")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


def is_test_path(path: str) -> bool:
    """Whether `path` names a separate test file.

    Contracts exclude test paths unless `include_tests` is set. Inline
    `#[cfg(test)]` modules are not detectable by this path-only check, so
    contracts that would match inline tests must narrow their paths.
    """
    return (
        "/tests/" in path
        or path.endswith("/tests.rs")
        or path.endswith("_tests.rs")
        or "/tests/" in path
        or path.endswith("_test.rs")
    )


def strip_comments_for(path: str, line: str) -> str:
    """Return `line` with comment text removed before pattern matching.

    Keep the stripping conservative so code is never hidden as prose. Hash
    comments are stripped only for languages where `#` is comment syntax; Rust
    attributes such as `#[ignore]` must remain visible.
    """
    stripped = _BLOCK_COMMENT.sub(" ", line)
    stripped = _LINE_COMMENT.sub("", stripped)
    if not path.endswith((".rs", ".toml")):
        stripped = _HASH_COMMENT.sub("", stripped)
    return stripped


def git_grep(pattern: str, paths: list[str], root: Path) -> list[tuple[str, int, str]]:
    """Every `path, line number, text` match for `pattern` under `paths`."""
    command = ["git", "grep", "-n", "-I", "-E", pattern, "--", *paths]
    result = subprocess.run(
        command, cwd=root, capture_output=True, text=True, check=False
    )
    # git grep exits 1 for "no matches", which is the outcome this file wants.
    if result.returncode not in (0, 1):
        raise RuntimeError(f"git grep failed: {result.stderr.strip()}")
    hits = []
    for raw in result.stdout.splitlines():
        parts = raw.split(":", 2)
        if len(parts) != 3:
            continue
        path, number, text = parts
        try:
            hits.append((path, int(number), text))
        except ValueError:
            continue
    return hits


def violations(contract: dict, root: Path) -> list[tuple[str, int, str]]:
    """Every production line that violates `contract`, comments excluded.

    A pattern is either a string (git grep and the confirming match are the same
    expression) or a `{"grep": ..., "match": ...}` pair. The pair exists because
    `git grep -E` is POSIX ERE and has no lookaround: the coarse expression finds
    candidate lines cheaply and the precise Python one decides. Splitting them is
    what lets a contract say "an ignore WITHOUT a diagnostic reason" instead of
    settling for "an ignore", which is the difference between a contract that
    survives and one that gets waived.
    """
    found: list[tuple[str, int, str]] = []
    include_tests = contract.get("include_tests", False)
    for pattern in contract["patterns"]:
        if isinstance(pattern, str):
            grep, confirm = pattern, pattern
        else:
            grep, confirm = pattern["grep"], pattern["match"]
        compiled = re.compile(confirm)
        for path, number, text in git_grep(grep, contract["paths"], root):
            if path in SELF_REFERENTIAL:
                continue
            if not include_tests and is_test_path(path):
                continue
            if compiled.search(strip_comments_for(path, text)):
                found.append((path, number, text.strip()))
    found.sort()
    return found


def _leading_identifier(text: str) -> list[str]:
    """The identifier a use-tree item starts with, or nothing."""
    match = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)", text)
    # `self` re-exports the facade root, which names no module.
    if not match or match.group(1) == "self":
        return []
    return [match.group(1)]


def use_tree_heads(text: str, index: int) -> list[str]:
    """The top-level module names a use tree introduces at `index`.

    ⚠ **This exists because a line regex is EVADABLE, and silently.**
    `ambition_platformer2d::time` is easy to match. `use ambition_platformer2d::{time::Foo, audio::Bar};`
    is the same two leaks written in idiomatic Rust, and a
    `\\bambition_platformer2d::([a-z_]+)` pattern matches neither of them — it sees `{` and
    stops. The fixture happens to contain no brace-grouped facade import today,
    so a line regex would have been green, correct, and wrong the first time
    somebody wrote ordinary Rust.

    Forbidding the braced form was the cheaper fix and it was rejected: a
    contract that outlaws standard syntax to keep its own parser simple is a
    contract that gets waived. So the tree is parsed. Only the depth-1 heads
    matter — `{a::{b, c}, d}` names `a` and `d` — which is why this needs no
    recursion into nested groups.
    """
    while index < len(text) and text[index].isspace():
        index += 1
    if index >= len(text):
        return []
    if text[index] != "{":
        return _leading_identifier(text[index:])

    heads: list[str] = []
    depth = 0
    item: list[str] = []
    for position in range(index, len(text)):
        character = text[position]
        if character == "{":
            depth += 1
            if depth == 1:
                item = []
                continue
        elif character == "}":
            depth -= 1
            if depth == 0:
                heads.extend(_leading_identifier("".join(item)))
                return heads
        if depth == 1 and character == ",":
            heads.extend(_leading_identifier("".join(item)))
            item = []
            continue
        item.append(character)
    # An unterminated group is malformed Rust; take what was readable rather
    # than reporting the file as clean.
    heads.extend(_leading_identifier("".join(item)))
    return heads


def facade_modules(text: str, facade: str) -> list[tuple[int, str]]:
    """Every `offset, top-level module` named through `facade::` in `text`."""
    found: list[tuple[int, str]] = []
    for match in re.finditer(rf"\b{re.escape(facade)}::", text):
        for head in use_tree_heads(text, match.end()):
            found.append((match.start(), head))
    return found


def allowlist_usage(contract: dict, root: Path) -> dict[str, list[tuple[str, int]]]:
    """Every facade module the consumer names, mapped to where it names it.

    Whole-file rather than line-by-line, because a use tree spans lines and the
    heads have to be attributed to the `ambition_platformer2d::` that introduced them.
    Comments are stripped with the same line-local helper the other tables use,
    so the prose recurrence this module exists to survive is survived here too:
    a doc comment naming `ambition_platformer2d::runtime` is not a consumer naming it.
    """
    listed = subprocess.run(
        ["git", "ls-files", "--", *contract["paths"]],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()

    usage: dict[str, list[tuple[str, int]]] = {}
    for relative in listed:
        if not relative.endswith(".rs"):
            continue
        if not contract.get("include_tests", False) and is_test_path(relative):
            continue
        try:
            raw = (root / relative).read_text(errors="replace")
        except OSError:
            continue
        stripped = "\n".join(
            strip_comments_for(relative, line) for line in raw.splitlines()
        )
        starts = [0]
        for line in stripped.splitlines():
            starts.append(starts[-1] + len(line) + 1)
        for offset, module in facade_modules(stripped, contract["facade"]):
            number = max(1, bisect_right(starts, offset))
            usage.setdefault(module, []).append((relative, number))
    return {module: sorted(where) for module, where in sorted(usage.items())}


def allowlist_violations(
    contract: dict, usage: dict[str, list[tuple[str, int]]]
) -> tuple[list[str], list[str]]:
    """`new, stale` — invariant 1's breaches and invariant 2's."""
    named = set(usage)
    allowed = set(contract["allowed"])
    baseline = set(contract["baseline"])
    return sorted(named - allowed - baseline), sorted(baseline - named)


#: ⭐ MOVED to `scripts/lib/cargo_bin.py` 2026-09-02 and re-exported here, so the
#: name every call site in this file already uses keeps working. The lesson this
#: docstring recorded — "a check that can only ever report `command not found`
#: can never pass" — was true and was only applied in four of six places;
#: `check_capability_ships.py` and the sub-workspace lockfile test called bare
#: `cargo` and crashed on a machine where rustup's cargo is not on PATH.
sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from cargo_bin import cargo_binary  # noqa: E402,F401


@functools.cache
def workspace_graph(root: Path) -> dict[str, set[str]]:
    """Every workspace crate's DIRECT workspace dependencies, from the manifests.

    Process-local cache: dependency contracts are parameterized over the same
    workspace. Resolve Cargo metadata once per root instead of once per contract.

    `--no-deps` keeps this to the workspace: registry crates are somebody else's
    layering problem. Dev-dependencies are included deliberately — a test that
    reaches upward compiles the upward edge, and "only in tests" is exactly the
    excuse under which a layering inversion first arrives.
    """
    raw = subprocess.run(
        [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    metadata = json.loads(raw)
    members = {package["name"] for package in metadata["packages"]}
    return {
        package["name"]: {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in members
        }
        for package in metadata["packages"]
    }


def reachable(graph: dict[str, set[str]], start: str) -> dict[str, list[str]]:
    """Every crate `start` can reach, mapped to the path that gets there.

    The path is the whole value of reporting this: "A depends on B" is arguable,
    "A -> C -> B" is a fix.
    """
    found: dict[str, list[str]] = {}
    queue = [(start, [start])]
    while queue:
        crate, path = queue.pop(0)
        for dependency in sorted(graph.get(crate, ())):
            if dependency in found or dependency == start:
                continue
            found[dependency] = path + [dependency]
            queue.append((dependency, path + [dependency]))
    return found


def dependency_violations(contract: dict, graph: dict[str, set[str]]) -> list[str]:
    crate = contract["crate"]
    if crate not in graph:
        return [f"contract names `{crate}`, which is not a workspace member"]
    reached = reachable(graph, crate)
    forbidden = contract["forbidden"]
    targets = sorted(reached) if forbidden == "*" else forbidden
    # an `allowed` entry is a NAMED edge, never a category. A floor crate that
    # sits on a smaller floor is still a floor; a floor with an open-ended
    # exception list is not. Each name here has to be a crate that itself
    # carries a `forbidden: "*"` contract, or the invariant has just moved
    # somewhere nobody is checking.
    allowed = set(contract.get("allowed", ()))
    return [
        " -> ".join(reached[target])
        for target in targets
        if target in reached and target not in allowed
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="exit 1 when a contract is violated"
    )
    parser.add_argument(
        "--allowlist-open-count",
        action="store_true",
        help=(
            "print the number of baseline modules a consumer still names, and "
            "nothing else — the campaign's progress metric, for a goal check"
        ),
    )
    args = parser.parse_args()

    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    )

    if args.allowlist_open_count:
        # Deliberately the count of BASELINE modules still named, not of every
        # module named: a new module is invariant 1's problem and this number
        # must not be able to rise when one appears. It only ever falls.
        #
        # Empty baselines are still active guards: a new private-module import
        # makes the corresponding contract fail.
        still_open = sum(
            len(set(allowlist_usage(contract, root)) & set(contract["baseline"]))
            for contract in MODULE_ALLOWLISTS
        )
        print(still_open)
        return 0

    broken = 0
    for contract in ABSENCE_CONTRACTS:
        found = violations(contract, root)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path, number, text in found:
            print(f"       {path}:{number}: {text}")

    graph = workspace_graph(root)
    for contract in DEPENDENCY_CONTRACTS:
        found = dependency_violations(contract, graph)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path in found:
            print(f"       {path}")

    for contract in MODULE_ALLOWLISTS:
        usage = allowlist_usage(contract, root)
        new, stale = allowlist_violations(contract, usage)
        still_open = sorted(set(usage) & set(contract["baseline"]))
        facade = contract["facade"]
        if not new and not stale:
            print(
                f"  ok   {contract['id']}"
                f"  ({len(still_open)} of {len(contract['baseline'])} baseline "
                f"modules still named)"
            )
        else:
            broken += 1
            print(f"  RED  {contract['id']}")
            print(f"       {contract['reason']}")
            for module in new:
                sites = usage[module]
                print(
                    f"       NEW  {facade}::{module} "
                    f"({len(sites)} uses) is in neither the reviewed public "
                    f"surface nor the frozen baseline"
                )
                for path, number in sites:
                    print(f"            {path}:{number}")
            for module in stale:
                print(
                    f"       STALE  {facade}::{module} is in the baseline but "
                    f"the consumer no longer names it — PRUNE it in this "
                    f"commit, or the baseline is a budget and the slot it "
                    f"leaves behind can be filled silently"
                )
        # §2a of the growth method: which paths does the consumer still name,
        # and how many times. Frequency is the crude cost proxy and it is
        # usually right, so the instrument prints it rather than making the
        # next slice go and count by hand.
        if still_open:
            ranked = sorted(
                ((len(usage[module]), module) for module in still_open),
                reverse=True,
            )
            summary = "  ".join(f"{module}:{count}" for count, module in ranked)
            print(f"       still named — {summary}")

    grown = capability_footprint_violations(root)
    left = capability_footprint_departures(root)
    if left:
        broken += 1
        print("  RED  capability-footprint-baseline-is-stale")
        print(
            "       ⭐ THE FOOTPRINT SHRANK, which is the outcome the campaign "
            "wants — but the baseline still names crates the sentinel no longer "
            "links, and every sub-list that names them goes stale silently. "
            "Re-freeze it in THIS commit."
        )
        for crate in left:
            print(f"       LEFT   {crate} is no longer in the consumer's closure")
    if not grown:
        footprint = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
        print(
            f"  ok   capability-footprint-may-not-grow  "
            f"({footprint['closure_size']} crates linked, "
            f"{footprint['never_asked_for_count']} a movement-only game never asked for)"
        )
    else:
        broken += 1
        print("  RED  capability-footprint-may-not-grow")
        print(
            "       Depending on `ambition_platformer2d` links these too. §2e: a perfectly "
            "semantic API can still force a movement-only game to compile and "
            "link every unrelated gameplay domain — no forbidden path is named "
            "and the footprint is still wrong."
        )
        for crate in grown:
            print(f"       NEW    {crate} entered the consumer's closure")

    new, stale = rollback_schema_violations(root)
    if not new and not stale:
        baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
        print(
            f"  ok   rollback-wire-format-changes-are-declared  "
            f"({len(baseline['stable_schema_names'])} stable names, "
            f"{len(baseline['encoded_types'])} encoded types across "
            f"{len({t.split('::')[0] for t in baseline['encoded_types']})} crates)"
        )
    else:
        broken += 1
        print("  RED  rollback-wire-format-changes-are-declared")
        print(
            "       Every type in the rollback wire format, wherever it is "
            "encoded. THE SET MAY GROW — what it may not do is drift "
            "unacknowledged. A new entry is a wire-format change: peers whose "
            "schemas differ cannot agree about a snapshot, so adding one means "
            "updating this baseline AND bumping "
            "GGRS_ROLLBACK_SCHEMA_VERSION in the SAME commit, and saying why."
        )
        print(
            "       ⚠ this rule was 'the set may only shrink' until 2026-08-23, "
            "inherited from `central-rollback-ownership-may-not-grow` — a "
            "MIGRATION constraint from when the orphan rule forced every impl "
            "into one file. Slice F federated that, and the shrink-only reading "
            "outlived its condition: it forbade genuinely new canonical "
            "gameplay state. `resource.impact_hitstop` is the case that "
            "settled it — a freeze that decides the sim clock IS simulation "
            "truth, and burying it in an already-registered type to appease a "
            "ratchet would make the architecture worse to satisfy a guard."
        )
        for item in new:
            print(
                f"       NEW    {item} entered the rollback wire format — "
                "declare it (baseline + schema version) or drop it"
            )
        for item in stale:
            print(
                f"       STALE  {item} left the rollback registration but is "
                f"still in the baseline — PRUNE it in this commit"
            )

    total = (
        len(ABSENCE_CONTRACTS)
        + len(DEPENDENCY_CONTRACTS)
        + len(MODULE_ALLOWLISTS)
        + 2
    )
    if broken:
        print(f"\n{broken} of {total} absence contracts are violated.")
        print(
            "Either the reintroduction is a mistake, or the architecture changed "
            "on purpose — in which case DELETE or INVERT the contract in the same "
            "commit rather than waiving it."
        )
        return 1 if args.check else 0
    print(f"\n{total} of {total} absence contracts hold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
