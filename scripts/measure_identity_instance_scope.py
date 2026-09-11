#!/usr/bin/env python3
"""Can a simulated identity say WHICH INSTANCE of a room it belongs to?

⭐⭐ **A8 ASKS EXACTLY THIS AND HAS BEEN HELD FOR THE ANSWER.** Its step 2:
*"Identify the exact values that lose instance scope."* Step 1 says to run two
instances of one prepared room with identical local placement IDs — but before
any of that is worth building, the cheaper half is readable: an identity that
cannot EXPRESS an instance loses it by construction, no matter what the runtime
does.

⛔⛔ **THE SUBJECT IS THE CONSTRUCTOR, NOT THE CALL SITE.** `SimId` is a newtype
over `String` and its only road in is `impl SimId`'s constructors; every live
identity in the game is one of their outputs. So the question "can two instances
of one room be told apart" is answered by their PARAMETERS, exhaustively, in one
file — and a call-site survey would answer a different and much vaguer question.

⇒ A constructor is INSTANCE-SCOPED if some parameter can differ between two
instances of one prepared room. `placement(id)` takes the map's own id, which is
identical in both, so it cannot. `spawned(spawner, n)` derives from its spawner
and inherits whatever scope that had — INHERITED is a real third answer, not a
hedge.

⚠ IT RULES ON NOTHING. Whether an instance-scoped identity is WANTED is A8's
decision; this says which values could not carry one today.

    python3 scripts/measure_identity_instance_scope.py
"""
from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
SIM_ID = REPO / "crates/ambition_platformer2d_shared_tangle/src/sim_id.rs"
GEO_ID = REPO / "crates/ambition_platformer2d_core/src/geo_id.rs"

SIGNATURE = re.compile(r"^    pub fn ([a-z_0-9]+)\(([^)]*)\)\s*->\s*Self\s*\{", re.M)

# ⛔ A PARAMETER NAME IS THE EVIDENCE, and these are the words that would carry
# an instance. Derived from the two identity files' own vocabulary rather than
# guessed: grep either file for any of them and you get the whole set.
INSTANCE_WORDS = ("room", "instance", "world", "shard", "realm", "scope")
# Identities built FROM another identity inherit its scope. The parameter type
# is the evidence, not the name.
INHERITS = ("&SimId", "&GeoId", "&ambition_platformer2d_core::GeoId")


def constructors(text: str) -> list[tuple[str, list[str]]]:
    out = []
    for match in SIGNATURE.finditer(text):
        name = match.group(1)
        raw = match.group(2)
        params = [p.strip() for p in raw.split(",") if p.strip() and p.strip() != "&self"]
        out.append((name, params))
    return out


def classify(params: list[str]) -> tuple[str, str]:
    joined = " ".join(params)
    for word in INSTANCE_WORDS:
        if re.search(rf"\b{word}\b", joined):
            return "INSTANCE-SCOPED", f"names `{word}`"
    for kind in INHERITS:
        if kind in joined:
            return "INHERITED", f"derives from a {kind.lstrip('&')}"
    if not params:
        return "GLOBAL", "takes nothing — one per process by construction"
    return "NOT INSTANCE-SCOPED", "no parameter can differ between two instances"


def main() -> int:
    if not SIM_ID.exists():
        raise SystemExit(f"{SIM_ID} is gone — this census has no subject")
    rows = constructors(SIM_ID.read_text())
    # ⛔ THE FLOOR. An empty or near-empty parse makes every claim below vacuous,
    # and a signature-shape change is exactly how this script would silently stop
    # seeing its own subject.
    if len(rows) < 6:
        raise SystemExit(
            f"parsed only {len(rows)} SimId constructor(s) from {SIM_ID.name}: the "
            "signature pattern no longer matches, so this census is about nothing"
        )

    print(f"{len(rows)} `SimId` constructor(s) — can each tell two instances of one room apart?\n")
    print(f"  {'constructor':<16} {'verdict':<20} why")
    print(f"  {'-' * 16} {'-' * 20} {'-' * 46}")
    buckets: dict[str, list[str]] = {}
    for name, params in rows:
        verdict, why = classify(params)
        buckets.setdefault(verdict, []).append(name)
        print(f"  {name:<16} {verdict:<20} {why}")

    print()
    for verdict in ("INSTANCE-SCOPED", "INHERITED", "GLOBAL", "NOT INSTANCE-SCOPED"):
        names = buckets.get(verdict, [])
        print(f"  {len(names):>2} {verdict:<20} {', '.join(names) or '—'}")

    scoped = buckets.get("INSTANCE-SCOPED", [])
    if not scoped:
        print(
            "\n⛔⛔ NO IDENTITY CONSTRUCTOR TAKES A ROOM OR AN INSTANCE.\n"
            "   Two instances of one prepared room mint the SAME `SimId` for every\n"
            "   authored placement, because the identity is the map's own id and\n"
            "   nothing else. That is A8 step 2's answer for the identity axis, and\n"
            "   it is readable without running anything."
        )

    # ── the geometry identity, which SimId::geometry is a function of ──────
    if GEO_ID.exists():
        geo = GEO_ID.read_text()
        # ⛔⛤ **THE REGION FIRST, THEN THE PATTERN — AND MY FIRST VERSION HAD IT
        # BACKWARDS.** Scanning the whole file for variant-shaped lines reported
        # `Clone` (from a `#[derive(..)]`) and every `GeoFace` variant as if they
        # were `GeoSource`'s. Recognising a thing by its shape across a file is
        # how a census gets members it never meant to have.
        block = re.search(r"pub enum GeoSource \{(.*?)\n\}", geo, re.S)
        if block is None:
            raise SystemExit(
                "`pub enum GeoSource` is gone from geo_id.rs — the variant census "
                "below would silently be about some other enum"
            )
        variants = re.findall(
            r"^    ([A-Z][A-Za-z]*)(?:\s*\{([^}]*)\}|\(([^)]*)\))?,",
            block.group(1),
            re.M,
        )
        if variants:
            print("\n`GeoSource` variants — the same question, one layer down\n")
            for name, fields, tuple_fields in variants:
                payload = (fields or tuple_fields or "").strip() or "(no payload)"
                payload = " ".join(payload.split())
                carries = any(
                    re.search(rf"\b{w}\b", payload) for w in INSTANCE_WORDS
                )
                mark = "INSTANCE-SCOPED" if carries else "NOT INSTANCE-SCOPED"
                print(f"  {name:<14} {mark:<20} {payload[:60]}")
            print(
                "\n⛔⛤ `TileLayer { layer }` LOOKS UNSCOPED AND IS NOT — I CALLED IT A\n"
                "   COLLISION AND READING THE MINTING SITE CORRECTED ME. The producer\n"
                "   (`ldtk/intgrid.rs::emit_collision_blocks_from_intgrid`) passes a\n"
                "   LEVEL-SCOPED key, `\"{level}/{layer}\"` — 'because an active area can\n"
                "   span multiple levels that each carry this layer'.\n"
                "   ⇒ THE SCOPE IS A STRING CONVENTION INSIDE A FIELD CALLED `layer`,\n"
                "   and the type cannot hold anyone to it: `GeoId::tile_layer(\"Collision\",\n"
                "   0)` compiles, and this repository\'s own unit tests construct exactly\n"
                "   that. A second producer that spells the key bare is a silent\n"
                "   collision no signature refuses — which is the shape A8 should fix by\n"
                "   making the level a FIELD, not by adding a rule about the string."
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
