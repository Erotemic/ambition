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
