"""The report must never turn geometry into a claim the runtime did not make.

⛔⛔ THE FAILURE THIS GUARDS. A strike volume and a hurtbox occupying the same
rectangle is not a hit: the victim may be intangible, on the same team, shielded,
or already struck by that same strike. A report that folded the two into one
"hits" number would be confidently wrong in exactly the cases somebody opens it
to investigate.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "moveset_report", REPO / "scripts/moveset_report.py"
)
tool = importlib.util.module_from_spec(_spec)
sys.modules["moveset_report"] = tool
_spec.loader.exec_module(tool)


def _body(role: str, x: float, phase: str | None = None, hurt: bool = True, vel=(0.0, 0.0)) -> dict:
    return {
        "role": role,
        # A `SimId`, because `overlaps` names its victims by one and a nameless
        # body cannot be told from another nameless body.
        "id": f"sim:{role}",
        "pos": [x, 100.0],
        "half": [10.0, 20.0],
        "velocity": list(vel),
        "hurtboxes": [{"pos": [x, 100.0], "half": [8.0, 18.0], "shape": {"kind": "aabb"}}]
        if hurt
        else [],
        "hurtbox_source": "published" if hurt else "intangible",
        "move_state": {
            "id": "jab",
            "phase": phase,
            "elapsed_s": 0.0,
            "duration_s": 0.4,
            "attack_facing": 1.0,
            "landed_hit": False,
        }
        if role == "subject"
        else None,
    }


def _strike(x: float, role: str = "subject_owned") -> dict:
    return {"pos": [x, 100.0], "half": [12.0, 6.0], "role": role, "shape": {"kind": "aabb"}}


def _take(*, contact_at: int | None, target_hurt: bool = True, subject_x=None) -> dict:
    """A move: 3 ticks of startup, 4 active, 3 recovery, target standing at 130."""
    frames = []
    for tick in range(10):
        phase = "Startup" if tick < 3 else "Active" if tick < 7 else "Recovery"
        x = subject_x(tick) if subject_x else 100.0
        frame = {
            "bodies": [
                _body("subject", x, phase),
                _body("target", 130.0, hurt=target_hurt),
            ],
            # The strike is live during the authored Active window and reaches
            # the target, which stands at 130.
            "hitboxes": [_strike(x + 20.0)] if 3 <= tick < 7 else [],
            "projectiles": [],
            "contacts": [],
        }
        if contact_at is not None and tick >= contact_at:
            frame["contacts"] = [
                {
                    "strike": "s1",
                    "owner_id": "subject",
                    "owner_role": "subject_owned",
                    "victim": "t1",
                    "victim_role": "target",
                }
            ]
        frames.append(frame)
    return {
        "character": "npc_pirate_admiral",
        "subject": "npc_pirate_admiral",
        "target": "npc_sandbag",
        "target_behavior": "passive",
        "verb": "attack",
        "intended_move": "jab",
        "moves_seen": ["jab"],
        "reached_intended_move": True,
        "frames": frames,
    }


def test_overlap_is_reported_separately_from_a_resolved_hit() -> None:
    """⛔⛔ THE CENTRAL HONESTY PROPERTY. Same geometry, two different answers."""
    hit = tool.report(_take(contact_at=4))["measurements"]
    missed = tool.report(_take(contact_at=None))["measurements"]

    # The GEOMETRY is identical in both takes...
    assert hit["target_overlap_ticks"] == missed["target_overlap_ticks"] > 0
    # ...and only one of them says the runtime resolved a hit.
    assert len(hit["contacts"]) == 1 and hit["first_contact_tick"] == 4
    assert missed["contacts"] == [] and missed["first_contact_tick"] is None


def test_the_summary_flags_an_overlap_that_resolved_nothing() -> None:
    text = tool.summary(tool.report(_take(contact_at=None)))
    assert "resolved NO hit" in text
    assert "MEASURED FROM BOXES" in text
    assert "THE ENGINE'S OWN ANSWER" in text


def test_an_intangible_target_can_be_overlapped_and_never_hit() -> None:
    m = tool.report(_take(contact_at=None, target_hurt=False))["measurements"]
    assert m["target_overlap_ticks"] == 0, "a body with no damageable volume cannot be overlapped"
    assert m["contacts"] == []


def test_the_authored_windows_are_counted_from_the_runtime_move_clock() -> None:
    m = tool.report(_take(contact_at=4))["measurements"]
    assert m["startup"]["ticks"] == 3
    assert m["active"]["ticks"] == 4 and m["active"]["first_tick"] == 3
    assert m["recovery"]["ticks"] == 3
    # A volume being LIVE is a different fact from the authored window.
    assert m["first_active_tick"] == 3
    assert m["live_volume_ticks"] == 4


def test_reach_is_measured_from_the_body_origin() -> None:
    m = tool.report(_take(contact_at=4))["measurements"]
    # subject at 100, strike centre at 120 with half-width 12 → far edge 132.
    assert m["aabb_reach_bound_px"] == 32.0
    assert m["attack_extents"]["x_max"] == 132.0


def test_travel_before_and_during_the_active_window_are_separate_numbers() -> None:
    """A lunge that covers ground BEFORE it becomes live is a different move from
    one that covers it while it can hit you."""
    m = tool.report(_take(contact_at=4, subject_x=lambda t: 100.0 + t * 5.0))["measurements"]
    assert m["subject_travel_before_active"] == 15.0, "ticks 0..3 at 5px"
    assert m["subject_travel_during_active"] == 15.0, "ticks 3..6 at 5px"


def test_a_diff_names_what_changed_and_nothing_else() -> None:
    before = tool.report(_take(contact_at=6))
    after = tool.report(_take(contact_at=4))
    diff = tool.compare(before, after)
    assert diff["comparable"] is True
    changed = {row["metric"]: (row["before"], row["after"]) for row in diff["changes"]}
    assert changed["first contact"] == (6, 4)
    assert "startup" not in changed, "an unchanged metric is not a change"
    assert "6 → 4" in tool.compare_text(diff)


def test_two_different_scenarios_are_not_silently_comparable() -> None:
    """⛔ A DIFF ACROSS SCENARIOS MIXES A CHANGE IN THE MOVE WITH A CHANGE IN
    WHAT IT WAS MEASURED AGAINST."""
    before = tool.report(_take(contact_at=4))
    after = tool.report(_take(contact_at=4))
    after["scenario"]["target_behavior"] = "cpu"
    diff = tool.compare(before, after)
    assert diff["comparable"] is False
    assert "NOT the same scenario" in tool.compare_text(diff)


def test_the_report_records_the_premise_it_was_measured_under() -> None:
    scenario = tool.report(_take(contact_at=4))["scenario"]
    assert scenario["target"] == "npc_sandbag"
    assert scenario["target_behavior"] == "passive"


def test_the_chain_says_what_the_contact_did_and_never_why() -> None:
    """⛔ WHAT CHANGED IS DERIVABLE; WHY IS NOT.

    Damage, hitstun, hitlag and velocity are facts the runtime published about
    the victim, differenced across the contact. The RESOLUTION — ignored,
    blocked, armored, wallet-shielded, damaged — is the engine's own vocabulary
    and travels on a feature-gated channel; a report that guessed it from a
    damage delta would be inventing the one fact the engine already announces.
    """
    take = _take(contact_at=4)
    # The victim reacts on the contact tick.
    for tick, frame in enumerate(take["frames"]):
        target = frame["bodies"][1]
        target["damage_taken"] = 12 if tick >= 4 else 0
        target["hitstun_s"] = 0.3 if tick >= 4 else 0.0
        target["velocity"] = [120.0, -40.0] if tick >= 4 else [0.0, 0.0]

    chain = tool.report(take)["measurements"]["consequence_chain"]
    assert len(chain) == 1 and chain[0]["tick"] == 4
    changed = {step["what"]: (step["before"], step["after"]) for step in chain[0]["steps"]}
    assert changed["damage taken"] == (0, 12)
    assert changed["hitstun"] == (0.0, 0.3)
    assert changed["velocity"] == ([0.0, 0.0], [120.0, -40.0])

    text = tool.summary(tool.report(take))
    assert "What each contact did" in text
    assert "WHAT changed, not WHY" in text, (
        "the report must say what it cannot answer, or a reader assumes it did"
    )


def test_the_engines_own_resolution_is_read_never_inferred() -> None:
    """⛔⛔ A DAMAGE DELTA CANNOT TELL `Blocked` FROM `Ignored`.

    Both leave HP unchanged, and so does a windbox that authored no damage. The
    resolver announces its decision; a report that guessed would be inventing
    the one fact the engine already publishes.
    """
    take = _take(contact_at=4)
    for tick, frame in enumerate(take["frames"]):
        frame["sim_tick"] = 1000 + tick
        frame["bodies"][1]["seat"] = 1
    take["causal"] = [
        {
            "sim_tick": 1004,
            "domain": "damage",
            "kind": "body_hit_resolved",
            "summary": "blocked a 12-damage hit",
            # ⛔ A SEATED FIGHTER'S SUBJECT IS ITS SEAT. `body_subject` prefers
            # `SubjectKey::Seat` for any body a participant drives, so matching
            # on the SimId alone finds nothing for exactly the two bodies an
            # inspection scenario is about.
            "subject": "seat:1",
            "participant": 1,
            "fields": {"resolution": "Blocked", "raw_damage": "12"},
        }
    ]
    chain = tool.report(take)["measurements"]["consequence_chain"]
    assert chain[0]["resolution"]["fields"]["resolution"] == "Blocked"
    text = tool.summary(tool.report(take))
    assert "the engine RESOLVED it as: blocked a 12-damage hit" in text
    assert "WHAT changed, not WHY" not in text, (
        "the caveat is for a recording that HAS no resolution, not one that does"
    )


def test_an_empty_causal_array_says_which_kind_of_nothing_it_is() -> None:
    """⛔⛔ THREE ANSWERS, AND `[]` USED TO GIVE ONE. A take always carried
    `causal: []`, so "this build has no recorder" and "the recorder ran and
    matched nothing" produced the same artifact and the same advice — and the
    two want OPPOSITE next actions: re-record with the feature, or go find why
    the join missed. `capabilities.causal_resolution` is what tells them apart,
    and a take written before that field existed must say it cannot tell.

    ⭐ ALL THREE ARMS, because the interesting one is the middle: a report that
    says "re-record with `--features causal`" to somebody who ALREADY has the
    feature sends them to do nothing for an afternoon."""
    unavailable = _take(contact_at=4)
    unavailable["capabilities"] = {"causal_resolution": False}
    assert tool.report(unavailable)["measurements"]["consequence_chain"][0][
        "resolution"
    ] is None
    text = tool.summary(tool.report(unavailable))
    assert "--features causal" in text
    assert "WITHOUT the `causal` feature" in text

    available = _take(contact_at=4)
    available["capabilities"] = {"causal_resolution": True}
    text = tool.summary(tool.report(available))
    assert "the recorder WAS present" in text
    assert "Re-recording will not help" in text
    assert "--features causal" not in text, (
        "the report told somebody who already has the feature to go and enable it"
    )

    # A take from before the field existed. It must not guess either way.
    legacy = _take(contact_at=4)
    legacy.pop("capabilities", None)
    text = tool.summary(tool.report(legacy))
    assert "does not say" in text and "which kind of nothing" in text


def test_a_sim_id_prefix_does_not_hide_the_resolution() -> None:
    """⛔⛔ MEASURED ON A REAL RECORDING. `SimId::placement(id)` prints
    `placement:npc_pirate_admiral#seat1`; the causal subject keys on
    `ActorIdentity::id`, which is the bare `npc_pirate_admiral#seat1`. Comparing
    the two whole strings never matches, and a recording FULL of facts reads as
    "this build has no inspector"."""
    take = _take(contact_at=4)
    for tick, frame in enumerate(take["frames"]):
        frame["sim_tick"] = 1000 + tick
        frame["bodies"][1]["id"] = "placement:npc_pirate_admiral#seat1"
    for frame in take["frames"]:
        for row in frame["contacts"]:
            row["victim"] = "placement:npc_pirate_admiral#seat1"
    take["causal"] = [
        {
            "sim_tick": 1004,
            "domain": "damage",
            "kind": "hit_resolved",
            "summary": "took 4",
            "subject": "sim:npc_pirate_admiral#seat1",
            "fields": {"outcome": "damaged", "damage": "4"},
        }
    ]
    chain = tool.report(take)["measurements"]["consequence_chain"]
    assert chain[0]["resolution"]["fields"]["outcome"] == "damaged"


def test_a_resolution_for_another_body_is_not_this_contacts() -> None:
    take = _take(contact_at=4)
    for tick, frame in enumerate(take["frames"]):
        frame["sim_tick"] = 1000 + tick
        frame["bodies"][1]["seat"] = 1
    take["causal"] = [
        {
            "sim_tick": 1004,
            "domain": "damage",
            "kind": "body_hit_resolved",
            "summary": "somebody else took one",
            "subject": "seat:0",
            "participant": 0,
            "fields": {},
        }
    ]
    chain = tool.report(take)["measurements"]["consequence_chain"]
    assert chain[0]["resolution"] is None, "a fact about another body is not this contact's"


def test_a_contact_that_changed_nothing_reports_no_steps() -> None:
    """A hit the victim absorbed with no published change is a real observation,
    and inventing a step for it would be a lie about the tick."""
    chain = tool.report(_take(contact_at=4))["measurements"]["consequence_chain"]
    assert len(chain) == 1
    # The fixture's target never changes damage/hitstun/velocity, so the only
    # step is the displacement window — and it is zero.
    assert all(step["after"] == 0.0 for step in chain[0]["steps"]), chain[0]["steps"]


def _chained(*, second_at: int | None, requested: int) -> dict:
    """A: startup 3 / active 4 / recovery 3, then optionally a SECOND instance of
    the same move — which is what a jab combo is."""
    take = _take(contact_at=4)
    take["chain"] = {"verb": "attack", "label": "Jab", "at": requested}
    frames = take["frames"]
    for _ in range(12):
        frames.append(json.loads(json.dumps(frames[-1])))
    for tick, frame in enumerate(frames):
        subject = frame["bodies"][0]
        if tick < 10:
            continue
        # After A finishes the subject plays nothing...
        subject["move_state"] = None
        frame["hitboxes"] = []
        frame["contacts"] = []
        # ...until the engine accepts B.
        if second_at is not None and tick >= second_at:
            subject["move_state"] = {
                "id": "jab",
                "phase": "Startup" if tick < second_at + 3 else "Active",
                "elapsed_s": 0.0,
                "duration_s": 0.4,
            }
            if tick >= second_at + 3:
                frame["hitboxes"] = [_strike(120.0)]
    return take


def test_a_repeated_move_is_a_second_instance_not_the_same_one() -> None:
    """⛔⛔ `jab → jab` SHARES AN ID. Comparing against the last RECORDED id
    misses the second press entirely — the subject plays nothing between them,
    so the previous recorded id is still `jab` — and that is exactly the chain a
    jab combo is made of."""
    chain = tool.report(_chained(second_at=14, requested=12))["measurements"]["move_chain"]
    assert chain is not None
    assert chain["first"]["move"] == "jab" and chain["first"]["first_contact_tick"] == 4
    assert chain["second"]["accepted"] is True
    assert chain["second"]["accepted_tick"] == 14
    # ⛔ AND THE SECOND INSTANCE'S WINDOW IS ITS OWN. Scoping by id alone would
    # credit the FIRST jab's contact to the second.
    assert chain["second"]["first_contact_tick"] is None
    assert chain["second"]["first_live_tick"] == 17


def test_a_press_the_engine_never_played_is_an_answer() -> None:
    """⛔⛔ Measured on the admiral: a jab requested on tick 8, with the first jab
    still playing until 17, produced no second jab at all. A report that omitted
    the section would leave a reader thinking the probe had not run."""
    chain = tool.report(_chained(second_at=None, requested=8))["measurements"]["move_chain"]
    assert chain is not None
    assert chain["second"]["accepted"] is False
    assert chain["second"]["accepted_tick"] is None
    assert chain["second"]["requested_tick"] == 8
    text = tool.summary(tool.report(_chained(second_at=None, requested=8)))
    assert "NEVER PLAYED IT" in text
    assert "Sweep `--chain-at`" in text


def test_the_buffered_gap_between_request_and_acceptance_is_named() -> None:
    """⭐ MEASURED: a press on tick 14 was accepted on 18. The request is not the
    acceptance, and the difference is the action buffer doing its job."""
    text = tool.summary(tool.report(_chained(second_at=18, requested=14)))
    assert "BUFFERED for 4 tick(s)" in text


def test_a_single_move_take_has_no_chain_section() -> None:
    assert tool.report(_take(contact_at=4))["measurements"]["move_chain"] is None


def test_a_summon_is_a_spawn_even_though_it_is_a_body() -> None:
    """⛔⛔ THE SHARK IS THE THING PEOPLE OPEN THIS VIEW TO WATCH.

    A summon carries no seat and no worn character; it is the subject's because
    the recording resolved the ownership. Counting only projectiles reported the
    admiral's up-B as a move that spawns nothing.
    """
    take = _take(contact_at=None)
    for tick, frame in enumerate(take["frames"]):
        if tick >= 5:
            frame["bodies"].append(
                {
                    "role": "subject_owned",
                    "id": "shark#1",
                    "pos": [120.0, 90.0],
                    "half": [16.0, 8.0],
                    "hurtboxes": [],
                }
            )
    spawns = tool.report(take)["measurements"]["spawns"]
    assert [(s["kind"], s["tick"]) for s in spawns] == [("summon", 5)]
    assert "summon `shark#1`" in tool.summary(tool.report(take))


def test_a_v1_take_without_roles_still_measures() -> None:
    """An artifact recorded before roles existed falls back to seat and the
    subject-owned boolean."""
    take = _take(contact_at=None)
    for frame in take["frames"]:
        for body in frame["bodies"]:
            body["seat"] = 0 if body.pop("role") == "subject" else 1
        for box in frame["hitboxes"]:
            box["subject_owned"] = box.pop("role") == "subject_owned"
    take["seat"] = 0
    m = tool.report(take)["measurements"]
    assert m["first_active_tick"] == 3
    assert m["aabb_reach_bound_px"] == 32.0


def test_touching_boxes_are_not_an_overlap_because_the_runtime_says_so() -> None:
    """⛔⛔ THE REPORT DISAGREED WITH THE GAME, SILENTLY AND IN ONE DIRECTION.

    `CombatVolume::intersects` documents the contract — *"box-vs-box preserves
    the strict platformer contract (edge-touching is NOT an overlap)"* — and its
    broad phase is `bounds().strict_intersects(..)`. This script used `<=`, so
    two volumes whose edges met exactly counted as overlapping here and as a miss
    in the runtime. Reported through a field called `overlap_ticks`, that reads
    as "the strike was on the target and the engine ignored it", which is the
    one conclusion a reader must never draw from an instrument's rounding.

    ⭐ AND THE FIELDS SAY WHAT THEY MEASURE NOW. `_bounds` already carried the
    rule — *"every field derived from this says `_aabb`"* — and none of them did:
    `overlap_ticks`, `max_reach_px` and `geometry_reached_target` all claimed the
    engine's authority for a broad phase. Schema is `v2` for the rename.
    """
    touching_a = {"pos": (0.0, 0.0), "half": (10.0, 10.0)}
    touching_b = {"pos": (20.0, 0.0), "half": (10.0, 10.0)}  # right edge meets left edge
    assert not tool._overlaps(touching_a, touching_b), (
        "edge-touching boxes counted as an overlap; the runtime calls that a MISS"
    )

    # ⛔ THE PREMISE: a real overlap still reads as one, or the arm above passes
    # because the function stopped answering yes to anything.
    overlapping_b = {"pos": (19.0, 0.0), "half": (10.0, 10.0)}
    assert tool._overlaps(touching_a, overlapping_b)

    # …and the derived names carry their own provenance.
    take = _take(contact_at=4)
    m = tool.report(take)["measurements"]
    for name in ("target_overlap_ticks", "first_target_overlap_tick", "aabb_reach_bound_px"):
        assert name in m, f"{name} is missing — a broad-phase field must say so"
    for name in ("overlap_ticks", "max_reach_px", "geometry_reached_target"):
        assert name not in m, (
            f"`{name}` is back: a bounds measurement under a name that claims the "
            "engine's geometry is how this instrument started lying"
        )


def test_two_spacings_are_two_scenarios() -> None:
    """⛔⛔ REPRODUCED BY THE 2026-08-31 REVIEW: reports from takes at 40px and
    80px requested spacing returned `comparable = True`.

    `compare()` decides comparability by whole-dict equality on the report's
    `scenario`, and that dict omitted the two inputs a reader is most likely to
    have changed between recordings. A diff of two DIFFERENT experiments is not a
    diff of a change — it reads "this move's reach moved" out of "I stood
    somewhere else", which is exactly the persuasive-but-wrong evidence a tool
    like this exists to prevent.

    ⭐ THE CHAIN ARM MATTERS AS MUCH. A chained A→B take and a single-move take of
    A differ in what the body was DOING, and every timing number below depends on
    it.
    """
    near = _take(contact_at=4)
    near["requested_spacing"] = 40.0
    far = _take(contact_at=4)
    far["requested_spacing"] = 80.0
    diff = tool.compare(tool.report(near), tool.report(far))
    assert not diff["comparable"], (
        "40px and 80px compared as the same scenario; a reach delta from this "
        "pair would be read as a change in the MOVE"
    )

    chained = _take(contact_at=4)
    chained["requested_spacing"] = 40.0
    chained["chain"] = {"verb": "attack_side", "label": "ftilt", "at": 6}
    assert not tool.compare(tool.report(near), tool.report(chained))["comparable"]

    # ⛔ THE PREMISE. Identical takes must still compare, or the assertions above
    # pass because nothing is ever comparable.
    same = _take(contact_at=4)
    same["requested_spacing"] = 40.0
    assert tool.compare(tool.report(near), tool.report(same))["comparable"]

    # ⚠ …and what the bodies REACHED is a measurement, not the scenario: a rig
    # that settled a pixel differently must not declare the pair incomparable.
    drifted = _take(contact_at=4)
    drifted["requested_spacing"] = 40.0
    drifted["spacing_at_press"] = 41.7
    assert tool.compare(tool.report(near), tool.report(drifted))["comparable"]


def _gapless_self_cancel() -> dict:
    """A jab cancelled into a FRESH jab with no idle tick between them.

    ⛔⛔ THE CASE THE ENGINE SUPPORTS AND THE REPORT COULD NOT SEE. A self-cancel
    replaces `MovePlayback("jab")` with a new one in the SAME update — the
    runtime pins the clock reset — so the recording shows `jab` on every tick and
    an id comparison finds ONE move. The instance is what separates them.
    """
    take = _take(contact_at=4)
    take["chain"] = {"verb": "attack", "label": "Jab", "at": 5}
    frames = take["frames"]
    for _ in range(12):
        frames.append(json.loads(json.dumps(frames[-1])))
    for tick, frame in enumerate(frames):
        subject = frame["bodies"][0]
        # Instance 0 until tick 8, instance 1 from tick 8 — no gap, same id.
        second = tick >= 8
        subject["move_state"] = {
            "id": "jab",
            "phase": "Active" if second else "Recovery",
            "elapsed_s": 0.05 if second else 0.20,
            "duration_s": 0.30,
            "attack_facing": 1.0,
            "landed_hit": False,
            "instance": 1 if second else 0,
        }
    return take


def test_a_gapless_self_cancel_is_two_move_instances() -> None:
    """⛔⛔ REPRODUCED BY THE 2026-08-31 REVIEW: the second jab reported
    `accepted: false`, because instances were discovered with
    `move and move != previous` and both uses share the id.

    ⭐ AND THE SECOND HALF THE REVIEW DID NOT REACH: `ticks_of` scoped an
    instance by walking until the ID changed, so the two uses were ONE window
    and the first's contact was credited to the second. Its own comment claimed
    to handle exactly this. Both are fixed by the same runtime fact.
    """
    doc = tool.report(_gapless_self_cancel())["measurements"]["move_chain"]
    assert doc is not None, "the chain probe saw no second move at all"
    assert doc["second"]["accepted"], (
        "a gapless `jab → jab` reported the second use as never accepted"
    )
    assert doc["second"]["accepted_tick"] == 8

    # ⛔⛔ AND THE SCOPING, WHICH ONLY BITES WHEN THE FIRST USE MISSES.
    # `ticks_of` walks FORWARD, so the second instance is already protected from
    # the first's contact by its own start tick. The direction that goes wrong is
    # the FIRST instance's window running ON past the boundary — and that is
    # invisible while the first use also has a contact of its own, because the
    # earlier one wins. So the fixture gives the first use NOTHING: a jab that
    # whiffed, cancelled into a jab that hit.
    #
    # ⚠ THREE VERSIONS OF THIS ASSERTION PASSED WITHOUT THE FIX before this one.
    # An arm that cannot fail is worse than no arm.
    take = _gapless_self_cancel()
    for tick, frame in enumerate(take["frames"]):
        frame["contacts"] = (
            [
                {
                    "strike": "s2",
                    "owner_id": "subject",
                    "owner_role": "subject_owned",
                    "victim": "t1",
                    "victim_role": "target",
                }
            ]
            if tick == 10
            else []
        )
    doc = tool.report(take)["measurements"]["move_chain"]
    assert doc["second"]["first_contact_tick"] == 10
    assert doc["first"]["first_contact_tick"] is None, (
        "the first use WHIFFED and was credited with the second use's contact — "
        "its window ran on past the instance boundary"
    )


def test_a_take_without_instances_still_measures() -> None:
    """⚠ A RECORDING FROM BEFORE THE ENGINE PUBLISHED THE INSTANCE still reads,
    with the old hole and no pretence otherwise: without the fact there is
    nothing to separate two adjacent uses of one move, and inventing a boundary
    would be worse than missing one."""
    take = _gapless_self_cancel()
    for frame in take["frames"]:
        frame["bodies"][0]["move_state"].pop("instance", None)
    doc = tool.report(take)["measurements"]["move_chain"]
    # One continuous move: the probe reports no second, which is all the data
    # supports.
    assert doc is None or not doc["second"]["accepted"]


def test_the_engines_exact_overlap_outranks_the_bounds_approximation() -> None:
    """⭐⭐ THE SHAPE, NOT THE BOX, WHEN THE RECORDING CARRIES ONE.

    A circle or an OBB whose BOUNDS overlap while the shapes miss is the case
    Python cannot answer — `_bounds` flattens every volume, and the review found
    the report claiming contact the engine denied. `CombatVolume::intersects`
    answers it in Rust, and a strike row's `overlaps` carries that answer.

    ⛔ THE FIXTURE IS THE DISAGREEING CASE ON PURPOSE: bounds that overlap, and
    an engine answer of "no". If the report preferred its own arithmetic the two
    would differ, and the whole point of publishing the exact fact is that it
    wins.
    """
    take = _take(contact_at=None)
    for frame in take["frames"]:
        for box in frame.get("hitboxes") or []:
            # Bounds that DO overlap the target's hurtbox…
            box["pos"] = list(frame["bodies"][1]["hurtboxes"][0]["pos"])
            # …and an engine answer that says they do not.
            box["overlaps"] = []
    m = tool.report(take)["measurements"]
    assert m["target_overlap_ticks"] == 0, (
        "the report preferred its own bounds arithmetic over the engine's exact "
        "answer — which is the disagreement this field exists to end"
    )
    # AND IT MUST SAY WHICH ROAD IT TOOK. This assertion — zero overlap
    # ticks for a fixture whose BOXES overlap — used to sit under a field named
    # `aabb_overlap_ticks`, so the test itself proved the field was not measuring
    # what its name claimed. A number and its provenance travel together now.
    assert m["target_overlap_source"] == "runtime_exact"

    # ⛔ THE PREMISE, both halves. With the engine saying YES the count is
    # non-zero, and with the field ABSENT the bounds fallback still measures —
    # a take recorded before `overlaps` existed must not silently read as a miss.
    yes = _take(contact_at=None)
    for frame in yes["frames"]:
        for box in frame.get("hitboxes") or []:
            box["pos"] = list(frame["bodies"][1]["hurtboxes"][0]["pos"])
            box["overlaps"] = [frame["bodies"][1]["id"]]
    yes_m = tool.report(yes)["measurements"]
    assert yes_m["target_overlap_ticks"] > 0
    assert yes_m["target_overlap_source"] == "runtime_exact"

    legacy = _take(contact_at=None)
    for frame in legacy["frames"]:
        for box in frame.get("hitboxes") or []:
            box["pos"] = list(frame["bodies"][1]["hurtboxes"][0]["pos"])
            box.pop("overlaps", None)
    legacy_m = tool.report(legacy)["measurements"]
    assert legacy_m["target_overlap_ticks"] > 0, (
        "a take from before the engine published `overlaps` stopped measuring at "
        "all, rather than falling back to bounds"
    )
    # ⭐ AND THE FALLBACK SAYS SO, which is the half that makes the exact number
    # trustworthy: a reader can tell an engine answer from an approximation
    # without knowing when the take was recorded.
    assert legacy_m["target_overlap_source"] == "aabb_fallback"
