#!/usr/bin/env python3
"""Classify the rollback-cleared message rows by whether each carries a room.

ISL-1b (`docs/planning/engine/island-rollback-and-compact-peer-input.md`):
before a room rewinds alone, every message a room's act writes has to say
which room it came from, or be routed by something that does. This reads the
`message-clear` rows of `game/ambition_app/tests/rollback_schema_baseline.txt`,
finds each type's declaration in the tracked Rust sources (tests excluded),
and sorts it by what its fields carry:

- its room (a `LiveRoomInstance`, a `RoomWrite`, or a `room` field);
- an entity, a peer id or a participant, from which the room is derivable;
- neither.

A field-text classifier: a carrier spelled another way is missed, so read
the "neither" list before acting on it. A measurement, not a guard.

    python3 scripts/measure/message_rows_by_room_carrier.py
"""
import re, subprocess, pathlib, collections, sys
repo = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(__file__).resolve().parents[2]
rows = [l.split("\t") for l in (repo/"game/ambition_app/tests/rollback_schema_baseline.txt").read_text().splitlines()[1:]]
msgs = [(r[0], r[2]) for r in rows if r[1] == "message-clear"]
files = subprocess.run(["git","ls-files","*.rs"], cwd=repo, capture_output=True, text=True).stdout.split()
src = {f: (repo/f).read_text(errors="replace") for f in files}
def body_of(name):
    leaf = name.split("<")[0].split("::")[-1]
    pat = re.compile(r"pub\s+(struct|enum)\s+" + re.escape(leaf) + r"\b[^{;(]*([{(;])")
    for f, text in src.items():
        if "/tests/" in f or f.endswith("tests.rs"):
            continue
        m = pat.search(text)
        if not m: continue
        start = m.end()-1
        if m.group(2) == ";": return f, ""
        open_c, close_c = ("{","}") if m.group(2)=="{" else ("(",")")
        depth = 0
        for i in range(start, len(text)):
            if text[i] == open_c: depth += 1
            elif text[i] == close_c:
                depth -= 1
                if depth == 0: return f, text[start:i+1]
    return None, None
out = collections.defaultdict(list)
for row, name in msgs:
    f, body = body_of(name)
    generic = "<" in name
    if body is None:
        kind = "not found"
    elif re.search(r"LiveRoomInstance|RoomWrite|\broom\s*:", body):
        kind = "carries its room"
    elif re.search(r"\bEntity\b|SimId|LiveBodyId|PlayerSlot|participant", body):
        kind = "carries an entity or id (room derivable)"
    else:
        kind = "carries neither"
    out[kind].append((row, name, f))
for kind, items in sorted(out.items(), key=lambda kv: -len(kv[1])):
    print(f"== {kind}: {len(items)}")
    for row, name, f in items:
        print(f"   {row}\t{name}\t{f}")
