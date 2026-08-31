"""The report must never turn geometry into a claim the runtime did not make.

⛔⛔ THE FAILURE THIS GUARDS. A strike volume and a hurtbox occupying the same
rectangle is not a hit: the victim may be intangible, on the same team, shielded,
or already struck by that same strike. A report that folded the two into one
"hits" number would be confidently wrong in exactly the cases somebody opens it
to investigate.
"""

from __future__ import annotations

import importlib.util
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
    assert hit["overlap_ticks"] == missed["overlap_ticks"] > 0
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
    assert m["overlap_ticks"] == 0, "a body with no damageable volume cannot be overlapped"
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
    assert m["max_reach_px"] == 32.0
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
    assert m["max_reach_px"] == 32.0
