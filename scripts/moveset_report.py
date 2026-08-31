#!/usr/bin/env python3
"""Turn a recorded take into numbers an agent can act on — and diff two of them.

⭐⭐ THE QUESTION THIS ANSWERS. A take is 150 ticks of geometry; nobody, human or
model, reads that to learn how far a move reaches or when it first connects. This
derives the measurements from what the RUNTIME published and writes them beside a
short causal summary, so "how many pixels past the body does this attack reach"
is a field rather than an afternoon in the Rust source.

⛔⛔ TWO KINDS OF CLAIM, NEVER MERGED. `contact_ticks` comes from the runtime's
own hit-once memory: it is what the game says CONNECTED. `overlap_ticks` is this
script measuring rectangles: it is where a strike and a hurtbox were in the same
place. They differ whenever the victim was intangible, shielded, on the same
team, or already struck by that strike — and a report that reported one number
would be wrong in exactly the cases somebody is investigating.

    python3 scripts/moveset_report.py --takes data/takes/takes.json \\
        --character npc_pirate_admiral --verb special_up --out /tmp/inspection

    python3 scripts/moveset_report.py --takes new.json --against old.json \\
        --character npc_pirate_admiral --verb special_up
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

# ⛔ THE TICK RATE IS THE BUNDLE'S, NOT A CONSTANT HERE. A report that hardcoded
# 60 would silently mis-state every duration the day the sim rate changed.
DEFAULT_SIM_HZ = 60.0


def _role(row: dict, take: dict) -> str:
    """What this row IS, with the fallback an older take needs."""
    role = row.get("role")
    if role:
        return role
    owned = row.get("subject_owned")
    if owned is True:
        return "subject_owned"
    if owned is False:
        return "other"
    seat = row.get("seat")
    if seat is not None:
        return "subject" if seat == take.get("seat", 0) else "target"
    return "other"


def _body(frame: dict, take: dict, role: str) -> dict | None:
    for body in frame.get("bodies") or []:
        if _role(body, take) == role:
            return body
    return None


def _bounds(volume: dict) -> tuple[float, float, float, float]:
    """The volume's axis-aligned bounds — the broad phase the engine itself uses.

    ⛔ IT IS THE BOX, NOT THE SHAPE. Reach measured off the box overstates a
    sweeping arc; every field derived from this says `_aabb` or is documented as
    a broad-phase answer.
    """
    (x, y), (hx, hy) = volume["pos"], volume["half"]
    return (x - hx, y - hy, x + hx, y + hy)


def _overlaps(a: dict, b: dict) -> bool:
    ax0, ay0, ax1, ay1 = _bounds(a)
    bx0, by0, bx1, by1 = _bounds(b)
    return ax0 <= bx1 and bx0 <= ax1 and ay0 <= by1 and by0 <= ay1


def measure(take: dict, sim_hz: float = DEFAULT_SIM_HZ) -> dict:
    """Every metric this take supports, from what the runtime published."""
    frames = take.get("frames") or []
    tick_s = 1.0 / sim_hz if sim_hz else 0.0

    phases: dict[str, list[int]] = {}
    active_ticks: list[int] = []
    overlap_ticks: list[int] = []
    contact_ticks: list[int] = []
    contacts: list[dict] = []
    spawns: list[dict] = []
    reach = 0.0
    reach_tick = None
    extents = {"x_min": None, "x_max": None, "y_min": None, "y_max": None}
    subject_path: list[tuple[int, float, float]] = []
    target_path: list[tuple[int, float, float]] = []
    seen_spawns: set[str] = set()

    for tick, frame in enumerate(frames):
        subject = _body(frame, take, "subject")
        target = _body(frame, take, "target")
        if subject:
            subject_path.append((tick, subject["pos"][0], subject["pos"][1]))
            state = subject.get("move_state") or {}
            phase = state.get("phase")
            if phase:
                phases.setdefault(phase, []).append(tick)
        if target:
            target_path.append((tick, target["pos"][0], target["pos"][1]))

        mine = [h for h in frame.get("hitboxes") or [] if _role(h, take) == "subject_owned"]
        if mine:
            active_ticks.append(tick)
        origin = subject["pos"] if subject else None
        for volume in mine:
            x0, y0, x1, y1 = _bounds(volume)
            extents["x_min"] = x0 if extents["x_min"] is None else min(extents["x_min"], x0)
            extents["x_max"] = x1 if extents["x_max"] is None else max(extents["x_max"], x1)
            extents["y_min"] = y0 if extents["y_min"] is None else min(extents["y_min"], y0)
            extents["y_max"] = y1 if extents["y_max"] is None else max(extents["y_max"], y1)
            if origin:
                # ⭐ REACH IS FROM THE BODY ORIGIN, and it is signed by FACING so
                # a back air and a forward air do not read the same. The far edge
                # in the attack's own direction is the number a designer means by
                # "how far does this hit".
                far = max(abs(x0 - origin[0]), abs(x1 - origin[0]))
                if far > reach:
                    reach, reach_tick = far, tick

        # GEOMETRY: where a strike and a hurtbox were in the same place.
        hurt = (target or {}).get("hurtboxes") or []
        if mine and hurt and any(_overlaps(v, h) for v in mine for h in hurt):
            overlap_ticks.append(tick)

        # THE RUNTIME'S OWN ANSWER: what actually connected.
        for row in frame.get("contacts") or []:
            if row.get("owner_role") != "subject_owned":
                continue
            key = (row.get("strike"), row.get("victim"))
            if key in {(c["strike"], c["victim"]) for c in contacts}:
                continue
            contacts.append(
                {
                    "tick": tick,
                    "time_s": round(tick * tick_s, 4),
                    "strike": row.get("strike"),
                    "victim": row.get("victim"),
                    "victim_role": row.get("victim_role"),
                }
            )
            contact_ticks.append(tick)

        for shot in frame.get("projectiles") or []:
            if _role(shot, take) != "subject_owned":
                continue
            key = shot.get("id") or f"anon@{tick}"
            if key in seen_spawns:
                continue
            seen_spawns.add(key)
            spawns.append(
                {"tick": tick, "id": shot.get("id"), "pos": shot.get("pos"), "kind": "projectile"}
            )

        # ⭐⭐ A SUMMON IS A BODY, NOT A SHOT, and it is the thing people open
        # this view to watch. The pirate's shark carries no seat and no worn
        # character; what makes it the subject's is the ownership the recording
        # already resolved.
        for body in frame.get("bodies") or []:
            if _role(body, take) != "subject_owned":
                continue
            key = body.get("id") or f"anon-body@{tick}"
            if key in seen_spawns:
                continue
            seen_spawns.add(key)
            spawns.append(
                {"tick": tick, "id": body.get("id"), "pos": body.get("pos"), "kind": "summon"}
            )

    def _window(name: str) -> dict | None:
        ticks = phases.get(name)
        if not ticks:
            return None
        return {"first_tick": ticks[0], "last_tick": ticks[-1], "ticks": len(ticks)}

    def _travel(path: list[tuple[int, float, float]], lo: int | None, hi: int | None) -> float | None:
        if lo is None or hi is None:
            return None
        span = [(x, y) for tick, x, y in path if lo <= tick <= hi]
        if len(span) < 2:
            return 0.0
        return round(sum(
            ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5
            for a, b in zip(span, span[1:])
        ), 3)

    first_active = active_ticks[0] if active_ticks else None
    first_contact = contact_ticks[0] if contact_ticks else None

    # ⭐ WHAT THE CONTACT DID TO THE TARGET, from the target's own published
    # velocity. Not a knockback formula re-run here — the number the runtime
    # actually gave the body.
    launch = None
    if first_contact is not None:
        after = [f for f in frames[first_contact : first_contact + 12]]
        speeds = []
        for frame in after:
            target = _body(frame, take, "target")
            if target and target.get("velocity"):
                vx, vy = target["velocity"]
                speeds.append((vx * vx + vy * vy) ** 0.5)
        if speeds:
            launch = round(max(speeds), 3)

    return {
        "frames": len(frames),
        "sim_hz": sim_hz,
        "consequence_chain": _chain(take, frames, contacts, tick_s),
        "move_chain": _move_chain(take, frames, contacts, tick_s),
        "startup": _window("Startup"),
        "active": _window("Active"),
        "recovery": _window("Recovery"),
        "invuln": _window("Invuln"),
        "armor": _window("Armor"),
        # A volume was live, which is not the same as the authored Active window
        # — a move may author several, and a window may author none.
        "first_active_tick": first_active,
        "live_volume_ticks": len(active_ticks),
        "max_live_volumes": max(
            (len([h for h in (f.get("hitboxes") or []) if _role(h, take) == "subject_owned"])
             for f in frames),
            default=0,
        ),
        "max_reach_px": round(reach, 3) if active_ticks else None,
        "max_reach_tick": reach_tick,
        "attack_extents": {k: (round(v, 3) if v is not None else None) for k, v in extents.items()},
        # GEOMETRY, said so in the name.
        "overlap_ticks": len(overlap_ticks),
        "first_overlap_tick": overlap_ticks[0] if overlap_ticks else None,
        # THE RUNTIME'S ANSWER.
        "contacts": contacts,
        "first_contact_tick": first_contact,
        "last_contact_tick": contact_ticks[-1] if contact_ticks else None,
        "target_launch_speed": launch,
        "subject_travel_before_active": _travel(subject_path, 0, first_active),
        "subject_travel_during_active": _travel(
            subject_path, first_active, active_ticks[-1] if active_ticks else None
        ),
        "target_displacement_after_contact": _travel(
            target_path, first_contact, len(frames) - 1
        ),
        "spawns": spawns,
    }


def provenance(take: dict, source: Path | None, bundle: dict | None) -> dict:
    """Enough to tell a current answer from a stale one.

    ⛔⛔ AN ARTIFACT THAT CANNOT BE DATED IS ONE THAT LOOKS CURRENT FOREVER. A
    report derived from a recording made before a tuning change reads exactly
    like one made after it, and the numbers are what somebody balances a fighter
    with. Every field here is a fact about WHERE THIS CAME FROM, not about the
    move.
    """
    import datetime

    recorded = None
    if source is not None and source.exists():
        recorded = (
            datetime.datetime.fromtimestamp(source.stat().st_mtime)
            .replace(microsecond=0)
            .isoformat()
        )
    return {
        "source_take": str(source) if source else None,
        "source_take_recorded": recorded,
        # The schemas of everything this passed through. A reader that finds an
        # unknown one must say so rather than guess.
        "take_schema": (bundle or {}).get("schema"),
        "observation_schema": (bundle or {}).get("observation_schema"),
        "report_schema": "ambition.moveset_report.v1",
        # What the take itself says it recorded, so a resolved move that changed
        # id is visible rather than silently compared against another move.
        "resolved_move": take.get("intended_move"),
        "verb": take.get("verb"),
    }


def _move_chain(take: dict, frames: list, contacts: list, tick_s: float) -> dict | None:
    """A → B, as HARD FACTS rather than a verdict.

    ⭐⭐ THE PLAN'S OWN RULE: "prefer reporting hard facts rather than prematurely
    classifying every sequence as a true combo." Whether B is guaranteed depends
    on a ruleset (DI, teching, escape options) this observatory does not model,
    so it reports when B was REQUESTED, when the engine ACCEPTED it, when its
    geometry went live, when the target's hitstun ended, and whether B reached —
    and lets the reader draw the conclusion.

    `None` when the take played fewer than two moves, which is every ordinary
    single-move recording.
    """
    order: list[tuple[str, int]] = []
    for tick, frame in enumerate(frames):
        subject = _body(frame, take, "subject")
        move = ((subject or {}).get("move_state") or {}).get("id")
        if move and (not order or order[-1][0] != move):
            order.append((move, tick))
    # A move re-entered after another one is a THIRD entry, not the first again;
    # the first two in time are the pair a chain probe is about.
    if len(order) < 2:
        return None
    (first, first_tick), (second, second_tick) = order[0], order[1]

    def ticks_of(move: str) -> list[int]:
        return [
            tick
            for tick, frame in enumerate(frames)
            if ((_body(frame, take, "subject") or {}).get("move_state") or {}).get("id") == move
        ]

    def live_ticks(move: str) -> list[int]:
        window = set(ticks_of(move))
        return [
            tick
            for tick in window
            if any(
                _role(h, take) == "subject_owned" for h in (frames[tick].get("hitboxes") or [])
            )
        ]

    def contact_ticks(move: str) -> list[int]:
        window = set(ticks_of(move))
        return [c["tick"] for c in contacts if c["tick"] in window]

    def reaches(move: str) -> bool:
        for tick in live_ticks(move):
            target = _body(frames[tick], take, "target")
            hurt = (target or {}).get("hurtboxes") or []
            mine = [
                h for h in (frames[tick].get("hitboxes") or [])
                if _role(h, take) == "subject_owned"
            ]
            if any(_overlaps(v, h) for v in mine for h in hurt):
                return True
        return False

    a_contact = contact_ticks(first)
    # When the target's reduced-authority window ran out. A B that lands after
    # this is a fresh engagement, not a follow-up — the distinction the reader
    # is here for.
    hitstun_ends = None
    if a_contact:
        for tick in range(a_contact[0], len(frames)):
            target = _body(frames[tick], take, "target")
            if target and not (target.get("hitstun_s") or 0.0) > 0.0 and tick > a_contact[0]:
                hitstun_ends = tick
                break

    b_live = live_ticks(second)
    b_contact = contact_ticks(second)
    return {
        "first": {
            "move": first,
            "first_tick": first_tick,
            "first_contact_tick": a_contact[0] if a_contact else None,
        },
        "second": {
            "move": second,
            # What the SCENARIO asked for, when it asked deliberately.
            "requested_tick": (take.get("chain") or {}).get("at"),
            # When the engine let it start, which is the fact a cancel window
            # decides and the one nobody can read off an authored table.
            "accepted_tick": second_tick,
            "first_live_tick": b_live[0] if b_live else None,
            "first_contact_tick": b_contact[0] if b_contact else None,
            # ⛔ REACH IS GEOMETRY; the contact beside it is the runtime's.
            "geometry_reached_target": reaches(second),
        },
        "target_hitstun_ended_tick": hitstun_ends,
        "gap_ticks": (second_tick - a_contact[0]) if a_contact else None,
        "gap_s": round((second_tick - a_contact[0]) * tick_s, 4) if a_contact else None,
        # ⛔ NOT A COMBO VERDICT. Whether the target could have escaped depends on
        # a ruleset this observatory does not model.
        "second_started_within_hitstun": (
            hitstun_ends is not None and second_tick <= hitstun_ends
        ),
    }


def _chain(take: dict, frames: list, contacts: list, tick_s: float) -> list:
    """What each contact DID, tick by tick, from the victim's own published state.

    ⭐⭐ THE QUESTION IS CAUSAL, NOT POSITIONAL. "The target ended up over there"
    is not an answer; "on tick 3 it took 4 damage, gained 0.088s of hitstun and
    left at (44.6, -32.2)" is. Every number here is one the runtime published
    about the victim, differenced across the contact rather than recomputed from
    a knockback formula — a second implementation of the launch rule would be
    exactly the kind of thing this whole program exists to remove.

    ⛔ IT REPORTS WHAT CHANGED, NOT WHY. The resolution vocabulary — ignored,
    blocked, armored, wallet-shielded, damaged — lives in
    `ambition_damage::BodyHitResolved` behind the `causal` feature, and a report
    that guessed at it from a damage delta would be inventing the one fact the
    engine already announces.
    """
    if not contacts:
        return []

    def victim_at(tick: int, victim_id: str | None, victim_role: str | None) -> dict | None:
        """The victim's row on a tick, by identity — and by ROLE when the
        recording carries no identity for it.

        ⛔ THE FALLBACK IS THE CONTACT'S OWN ROLE, not "the target". A contact
        names whom it struck; assuming the target would attribute a hit on a
        summon to the fighter standing behind it.
        """
        if tick < 0 or tick >= len(frames):
            return None
        bodies = frames[tick].get("bodies") or []
        if victim_id:
            for body in bodies:
                if body.get("id") == victim_id:
                    return body
        if victim_role:
            for body in bodies:
                if _role(body, take) == victim_role:
                    return body
        return None

    chain = []
    for contact in contacts:
        tick = contact["tick"]
        before = victim_at(tick - 1, contact.get("victim"), contact.get("victim_role"))
        at = victim_at(tick, contact.get("victim"), contact.get("victim_role"))
        if at is None:
            continue
        steps = []

        def delta(name: str, key: str, digits: int = 3):
            was = (before or {}).get(key)
            now = at.get(key)
            if now is None or was is None or was == now:
                return
            steps.append(
                {
                    "tick": tick,
                    "what": name,
                    "before": round(was, digits) if isinstance(was, float) else was,
                    "after": round(now, digits) if isinstance(now, float) else now,
                }
            )

        delta("damage taken", "damage_taken")
        delta("hitstun", "hitstun_s")
        delta("hitlag", "hitlag_s")
        if before and before.get("velocity") != at.get("velocity"):
            steps.append(
                {
                    "tick": tick,
                    "what": "velocity",
                    "before": [round(v, 1) for v in (before.get("velocity") or [])],
                    "after": [round(v, 1) for v in (at.get("velocity") or [])],
                }
            )
        # Where the victim ended up, once the freeze it bought has run out.
        settled = victim_at(
            min(tick + 12, len(frames) - 1), contact.get("victim"), contact.get("victim_role")
        )
        if settled and at.get("pos") and settled.get("pos"):
            moved = (
                (settled["pos"][0] - at["pos"][0]) ** 2 + (settled["pos"][1] - at["pos"][1]) ** 2
            ) ** 0.5
            steps.append(
                {
                    "tick": min(tick + 12, len(frames) - 1),
                    "what": "displacement over the next 12 ticks",
                    "before": 0.0,
                    "after": round(moved, 3),
                }
            )
        chain.append(
            {
                "tick": tick,
                "time_s": round(tick * tick_s, 4),
                "strike": contact.get("strike"),
                "victim": contact.get("victim"),
                "victim_role": contact.get("victim_role"),
                # ⭐ THE ENGINE'S OWN WORD FOR WHAT IT DECIDED, when the
                # recording was made by a build carrying the inspector.
                "resolution": _resolution(take, frames, tick, at),
                "steps": steps,
            }
        )
    return chain


def _resolution(take: dict, frames: list, tick: int, victim: dict | None) -> dict | None:
    """WHY the hit resolved as it did, from the causal log — never inferred.

    ⛔⛔ A DAMAGE DELTA CANNOT TELL `Blocked` FROM `Ignored` FROM A ZERO-DAMAGE
    WINDBOX. The resolver announces its own decision on `BodyHitResolved`, the
    monolith turns it into a `damage` fact, and this reads that. `None` means the
    recording was made by a build without the `causal` feature — which is an
    ABSENCE OF EVIDENCE, and the summary says so rather than filling it in.

    ⛔⛔ A SEATED FIGHTER'S SUBJECT IS ITS SEAT, NOT ITS `SimId`. `body_subject`
    prefers `SubjectKey::Seat` for any body a participant drives and falls back
    to `Sim` only for the rest — so matching on the id alone finds nothing for
    exactly the two bodies an inspection scenario is about, and every resolution
    reads as "this build has no inspector".
    """
    facts = take.get("causal")
    if not facts or victim is None:
        return None
    if tick < 0 or tick >= len(frames):
        return None
    sim_tick = frames[tick].get("sim_tick")
    if sim_tick is None:
        return None
    seat = victim.get("seat")
    victim_id = victim.get("id")
    # ⛔⛔ AND A `SimId` CARRIES A KIND PREFIX THE CAUSAL SUBJECT DOES NOT.
    # `SimId::placement(id)` prints `placement:npc_pirate_admiral#seat1`;
    # `body_subject` keys on `ActorIdentity::id`, which is the bare
    # `npc_pirate_admiral#seat1`. Comparing the two whole strings never matches,
    # and the report reads as "this build has no inspector" for a recording full
    # of facts.
    bare = victim_id.split(":", 1)[-1] if victim_id else None
    for fact in facts:
        if fact.get("sim_tick") != sim_tick or fact.get("domain") != "damage":
            continue
        subject = fact.get("subject") or ""
        named = subject.removeprefix("sim:")
        names_this_body = (
            (seat is not None and (fact.get("participant") == seat or subject == f"seat:{seat}"))
            or (victim_id is not None and named in (victim_id, bare))
        )
        # A recording whose facts name nobody at all is still evidence about the
        # tick; one that names SOMEBODY ELSE is not.
        if subject and not names_this_body:
            continue
        return {
            "kind": fact.get("kind"),
            "summary": fact.get("summary"),
            "fields": fact.get("fields") or {},
        }
    return None


def report(
    take: dict,
    sim_hz: float = DEFAULT_SIM_HZ,
    source: Path | None = None,
    bundle: dict | None = None,
) -> dict:
    """One scenario's machine-readable authority."""
    return {
        "schema": "ambition.moveset_report.v1",
        "provenance": provenance(take, source, bundle),
        "scenario": {
            "subject": take.get("subject") or take.get("character"),
            "subject_id": take.get("subject_id"),
            "target": take.get("target"),
            "target_id": take.get("target_id"),
            # ⛔ THE PREMISE. The same move against a live opponent and against a
            # passive one are two measurements, and every number below depends on
            # which this was.
            "target_behavior": take.get("target_behavior"),
            "verb": take.get("verb"),
            "label": take.get("label"),
        },
        "activation": {
            "intended_move": take.get("intended_move"),
            "observed_moves": take.get("moves_seen"),
            "reached_intended_move": take.get("reached_intended_move"),
            "outcome": take.get("outcome"),
            "prepared": take.get("prepared"),
        },
        "measurements": measure(take, sim_hz),
    }


def _fmt(value) -> str:
    if value is None:
        return "—"
    if isinstance(value, float):
        return f"{value:g}"
    return str(value)


def summary(doc: dict) -> str:
    """The same facts, as a short causal read for a person or a model."""
    scenario, activation, m = doc["scenario"], doc["activation"], doc["measurements"]
    lines = [
        f"# {scenario['subject']} · {scenario.get('label') or scenario['verb']}",
        "",
        f"- subject: `{scenario['subject']}` ({_fmt(scenario.get('subject_id'))})",
        f"- target: `{_fmt(scenario.get('target'))}` "
        f"({_fmt(scenario.get('target_id'))}), behaving `{_fmt(scenario.get('target_behavior'))}`",
        f"- drove `{scenario['verb']}` → intended `{_fmt(activation['intended_move'])}`, "
        f"engine played {activation.get('observed_moves')} "
        f"({'reached' if activation.get('reached_intended_move') else 'DID NOT REACH'})",
        "",
        "## Timing",
        "",
    ]
    for name in ("startup", "active", "recovery"):
        window = m.get(name)
        lines.append(
            f"- {name}: {_fmt(window and window['ticks'])} ticks"
            + (f" (first at {window['first_tick']})" if window else "")
        )
    lines += [
        f"- first live volume: tick {_fmt(m['first_active_tick'])}"
        f" · live for {m['live_volume_ticks']} tick(s), up to {m['max_live_volumes']} at once",
        "",
        "## Reach",
        "",
        f"- max reach from body origin: {_fmt(m['max_reach_px'])} px"
        f" (tick {_fmt(m['max_reach_tick'])})",
        f"- attack extents: x {_fmt(m['attack_extents']['x_min'])}…"
        f"{_fmt(m['attack_extents']['x_max'])}, y {_fmt(m['attack_extents']['y_min'])}…"
        f"{_fmt(m['attack_extents']['y_max'])}",
        f"- subject travel before the first live volume: {_fmt(m['subject_travel_before_active'])} px",
        f"- subject travel while live: {_fmt(m['subject_travel_during_active'])} px",
        "",
        "## Contact",
        "",
        # ⛔⛔ THE TWO CLAIMS, SIDE BY SIDE AND NAMED. Geometry says where things
        # were; the runtime says what connected. Reporting one as the other is
        # how a tool starts lying confidently.
        f"- geometric overlap with the target: {m['overlap_ticks']} tick(s)"
        f" (first at {_fmt(m['first_overlap_tick'])}) — MEASURED FROM BOXES",
        f"- runtime-resolved contacts: {len(m['contacts'])}"
        f" (first at {_fmt(m['first_contact_tick'])}) — THE ENGINE'S OWN ANSWER",
    ]
    if m["overlap_ticks"] and not m["contacts"]:
        lines.append(
            "- ⚠ the strike and the target's hurtbox overlapped and the runtime "
            "resolved NO hit: intangibility, team, shield, or a strike that had "
            "already spent its hit on this body."
        )
    lines += [
        f"- target launch speed after first contact: {_fmt(m['target_launch_speed'])}",
        f"- target displacement after contact: {_fmt(m['target_displacement_after_contact'])} px",
    ]
    chain = m.get("move_chain")
    if chain:
        lines += ["", "## Move chain", ""]
        lines += [
            f"- A `{chain['first']['move']}` first contact: tick "
            f"{_fmt(chain['first']['first_contact_tick'])}",
            f"- B `{chain['second']['move']}` requested at "
            f"{_fmt(chain['second']['requested_tick'])}, ACCEPTED at "
            f"{_fmt(chain['second']['accepted_tick'])}"
            f" ({_fmt(chain['gap_ticks'])} ticks / {_fmt(chain['gap_s'])}s after A connected)",
            f"- B first live volume: tick {_fmt(chain['second']['first_live_tick'])}"
            f" · first contact: tick {_fmt(chain['second']['first_contact_tick'])}",
            f"- B geometry reached the target: "
            f"{'yes' if chain['second']['geometry_reached_target'] else 'no'}",
            f"- target hitstun ended: tick {_fmt(chain['target_hitstun_ended_tick'])}"
            f" — B started {'INSIDE' if chain['second_started_within_hitstun'] else 'AFTER'} it",
            "- ⚠ these are FACTS, not a combo verdict. Whether the target could "
            "have escaped depends on a ruleset this report does not model.",
        ]
    if m.get("consequence_chain"):
        lines += ["", "## What each contact did", ""]
        for link in m["consequence_chain"]:
            lines.append(
                f"- tick {link['tick']} ({link['time_s']}s): `{_fmt(link['strike'])}` "
                f"→ `{_fmt(link['victim'])}` ({_fmt(link['victim_role'])})"
            )
            if link.get("resolution"):
                resolution = link["resolution"]
                lines.append(
                    f"    - the engine RESOLVED it as: {resolution['summary']}"
                    + (f" ({resolution['fields']})" if resolution.get("fields") else "")
                )
            for step in link["steps"]:
                lines.append(
                    f"    - {step['what']}: {_fmt(step['before'])} → {_fmt(step['after'])}"
                )
        if not any(link.get("resolution") for link in m["consequence_chain"]):
            lines.append(
                "- ⚠ WHAT changed, not WHY. ignored / blocked / armored / "
                "wallet-shielded / damaged is the runtime's own vocabulary; it "
                "travels on `BodyHitResolved` and this recording was made by a "
                "build without the `causal` feature. Re-record with "
                "`--features causal` to get it."
            )
    if m["spawns"]:
        lines += ["", "## Spawns", ""]
        for spawn in m["spawns"]:
            lines.append(f"- tick {spawn['tick']}: {spawn['kind']} `{_fmt(spawn['id'])}` at {spawn['pos']}")
    return "\n".join(lines) + "\n"


# ── before/after ────────────────────────────────────────────────────────────

# The fields a change is usually ABOUT. A diff over every key would bury the
# three numbers somebody changed a move to move.
COMPARED = [
    ("startup", lambda m: (m.get("startup") or {}).get("ticks")),
    ("active", lambda m: (m.get("active") or {}).get("ticks")),
    ("recovery", lambda m: (m.get("recovery") or {}).get("ticks")),
    ("first live volume", lambda m: m.get("first_active_tick")),
    ("live ticks", lambda m: m.get("live_volume_ticks")),
    ("max reach px", lambda m: m.get("max_reach_px")),
    ("first contact", lambda m: m.get("first_contact_tick")),
    ("contacts", lambda m: len(m.get("contacts") or [])),
    ("overlap ticks", lambda m: m.get("overlap_ticks")),
    ("launch speed", lambda m: m.get("target_launch_speed")),
    ("travel before active", lambda m: m.get("subject_travel_before_active")),
]


def compare(before: dict, after: dict) -> dict:
    """What changed between two reports of the same scenario."""
    rows = []
    for name, get in COMPARED:
        was, now = get(before["measurements"]), get(after["measurements"])
        if was == now:
            continue
        delta = None
        if isinstance(was, (int, float)) and isinstance(now, (int, float)):
            delta = round(now - was, 3)
        rows.append({"metric": name, "before": was, "after": now, "delta": delta})
    return {
        "schema": "ambition.moveset_report_diff.v1",
        # ⛔ COMPARING TWO DIFFERENT SCENARIOS IS NOT A DIFF OF A CHANGE. Naming
        # both makes a mismatched pair visible instead of persuasive.
        "before_scenario": before["scenario"],
        "after_scenario": after["scenario"],
        "comparable": before["scenario"] == after["scenario"],
        "changes": rows,
    }


def compare_text(diff: dict) -> str:
    if not diff["comparable"]:
        head = (
            "⚠ these two reports are NOT the same scenario — the comparison below "
            "mixes a change in the move with a change in what it was measured against.\n\n"
        )
    else:
        head = ""
    if not diff["changes"]:
        return head + "no measured difference\n"
    width = max(len(row["metric"]) for row in diff["changes"])
    lines = [
        f"{row['metric']:<{width}}  {_fmt(row['before'])} → {_fmt(row['after'])}"
        + (f"  ({row['delta']:+g})" if isinstance(row["delta"], (int, float)) else "")
        for row in diff["changes"]
    ]
    return head + "\n".join(lines) + "\n"


def _load(path: Path, character: str | None, verb: str | None) -> tuple[dict, float, dict]:
    doc = json.loads(path.read_text(encoding="utf8"))
    takes = doc.get("takes", doc) if isinstance(doc, dict) else doc
    sim_hz = doc.get("sim_hz", DEFAULT_SIM_HZ) if isinstance(doc, dict) else DEFAULT_SIM_HZ
    matches = [
        t
        for t in takes
        if (character is None or t.get("character") == character)
        and (verb is None or t.get("verb") == verb)
    ]
    if not matches:
        raise SystemExit(
            f"no take in {path} matches character={character!r} verb={verb!r}; "
            f"it holds {sorted({t.get('character') for t in takes})}"
        )
    return matches[0], sim_hz, doc if isinstance(doc, dict) else {}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--takes", required=True, type=Path, help="a moveset_takes takes.json")
    parser.add_argument("--character", help="subject catalog id")
    parser.add_argument("--verb", help="repertoire verb")
    parser.add_argument("--out", type=Path, help="directory for report.json + summary.md")
    parser.add_argument(
        "--against", type=Path, help="a second takes.json to compare this one against"
    )
    args = parser.parse_args()

    take, sim_hz, bundle = _load(args.takes, args.character, args.verb)
    doc = report(take, sim_hz, args.takes, bundle)

    if args.against:
        other, other_hz, other_bundle = _load(args.against, args.character, args.verb)
        diff = compare(report(other, other_hz, args.against, other_bundle), doc)
        print(compare_text(diff))
        if args.out:
            args.out.mkdir(parents=True, exist_ok=True)
            (args.out / "diff.json").write_text(json.dumps(diff, indent=2), encoding="utf8")
            print(f"file://{(args.out / 'diff.json').resolve()}")
        return 0

    text = summary(doc)
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "report.json").write_text(json.dumps(doc, indent=2), encoding="utf8")
        (args.out / "summary.md").write_text(text, encoding="utf8")
        # ⭐ THE PER-TICK TRACE, one JSON object per line. `report.json` is the
        # authority and `summary.md` is the read; this is what a question the
        # report did not anticipate is answered from, without loading a 5MB
        # recording of the whole grid.
        with (args.out / "trace.jsonl").open("w", encoding="utf8") as trace:
            for tick, frame in enumerate(take.get("frames") or []):
                trace.write(json.dumps({"tick": tick, **frame}) + "\n")
        # The filmstrip, from the one tool that draws one.
        try:
            sheet = _filmstrip(take)
        except Exception as error:  # noqa: BLE001 - a missing picture is not a failed report
            print(f"(no filmstrip: {error})")
        else:
            (args.out / "filmstrip.svg").write_text(sheet, encoding="utf8")
        print(text)
        print(f"file://{(args.out / 'report.json').resolve()}")
        print(f"file://{args.out.resolve()}")
    else:
        print(text)
    return 0


def _filmstrip(take: dict) -> str:
    """The key-frame sheet, from `render_take_diagnostic`.

    ⛔ NOT A SECOND DRAWING. One tool draws a diagnostic sheet; this bundles the
    one it draws.
    """
    import importlib.util
    import sys

    if "render_take_diagnostic" not in sys.modules:
        spec = importlib.util.spec_from_file_location(
            "render_take_diagnostic", Path(__file__).resolve().parent / "render_take_diagnostic.py"
        )
        module = importlib.util.module_from_spec(spec)
        sys.modules["render_take_diagnostic"] = module
        spec.loader.exec_module(module)
    return sys.modules["render_take_diagnostic"].sheet(take)


if __name__ == "__main__":
    raise SystemExit(main())
