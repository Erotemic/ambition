#!/usr/bin/env python3
"""Rollback rows that are SNAPSHOTTED but contribute nothing to the session checksum.

⛔⛔ **THESE REWIND CORRECTLY AND ARE INVISIBLE TO PEER COMPARISON.**
`rollback_component_clone_probed` installs `rollback_component_with_clone` — the
value is saved and restored — plus a `ChecksumProbe`, which is a desync
LOCALIZATION aid a test runs around a restore. It contributes nothing to the
frame checksum two peers compare.

⛔⛤ **THE POPULATION IS THE KIND, NOT THE SENTENCE — AND IT WAS THE SENTENCE FOR
SIX DAYS, WHICH HID 116 ROWS.** This census used to select rows whose `detail`
contained *"not in the session checksum"*: 59 rows. But the mechanical property
in this script's own title is `RollbackEntryKind::feeds_peer_checksum() == false`
for a VALUE-BEARING kind, which is `component-clone` and `resource-clone` —
**175 rows**. The other 116 have the same mechanical standing and simply carry a
different sentence, most of them
*"state checksum supplied by another authoritative projection"*.

⚠ AND THAT SENTENCE WAS WHY NOBODY MEASURED THEM. It used to read *"state
checksum supplied by another authoritative projection"* — emitted by
`rollback_component_clone` / `rollback_resource_clone`, whose only bound is
`T: Clone`, so neither method could establish it. The honest 59 said "not in the
session checksum" and got an instrument; the unverifiable 99 said "covered
elsewhere" and got a reassurance. Same mechanics, opposite attention, decided by a
string a method chose. ⇒ The claim came out at schema v194 and those rows now say
"not in the session checksum" too. See `Q122` for why removing a false sentence
cost a schema version.

⭐ RAISED BY A GUARD THAT FOUND NOTHING. The canonical-finiteness observer
(`game/ambition_app/tests/canonical_state_is_finite.rs`) is exhaustive BY
CONSTRUCTION over the canonical-checksum half, because passing through
`canonical_f32_bits` is what makes a value canonical. It is exhaustively BLIND to
these rows, which never call `encode` at all. A green run there is not "the
canonical state is finite", and this script is how you size the gap.

⚠ **THIS CLASSIFIES, IT DOES NOT RULE.** Promoting a row to canonical changes
what two peers agree about (netcode.md, N3). The float count and the
self-description below are measured; anything about WHEN a divergence would be
read is a reader question this script does not answer.

    python3 scripts/measure_unchecksummed_rollback_rows.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[1]
BASELINE = REPO / "game/ambition_app/tests/rollback_schema_baseline.txt"
# ⛔ SELECT ON THE KIND. These are the two VALUE-BEARING kinds for which
# `RollbackEntryKind::feeds_peer_checksum()` is false — see the module docstring
# for the 59-vs-175 history. The other `false` kinds (message-clear,
# entity-mapping, required-rollback, derived, dynamic-anchor) carry no value of
# their own, so they are not rollback state hiding from a peer.
UNHASHED_KINDS = ("component-clone", "resource-clone")
# The sentence this census USED to select on, kept only so the report can show
# which rows describe themselves honestly and which claim coverage.
PROBED_DETAIL = "not in the session checksum"
# `Vec2`/`Vec3`/`Quat`/`Rect` are floats wearing a name; `Timer`/`Duration` hold one.
FLOAT_BEARING = ("f32", "f64", "Vec2", "Vec3", "Quat", "Rect", "Timer", "Duration")


def rows() -> list[tuple[str, str, str]]:
    """`(schema name, type, detail sentence)` for every unhashed value-bearing row."""
    out = []
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        if "\t" not in line:
            continue
        parts = line.split("\t")
        if len(parts) >= 4 and parts[1] in UNHASHED_KINDS:
            out.append((parts[0], parts[2], parts[3]))
    return out


_DEFINITION_INDEX: dict[str, list[str]] | None = None


def _definition_index() -> dict[str, list[str]]:
    """`type name -> ["path:line", ..]`, built with ONE `git grep`.

    ⚠ NOT AN OPTIMISATION FOR ITS OWN SAKE. This ran one `git grep` per type
    across 59 types, twice over (definition and readers), and the arm that walks
    the population took 20 s of a lane other people wait on. The lane's cost is
    shared; a census that is correct and slow gets run less often, which is its
    own way of not being run.
    """
    global _DEFINITION_INDEX
    if _DEFINITION_INDEX is None:
        index: dict[str, list[str]] = {}
        out = subprocess.run(
            ["git", "grep", "-n", "-E", r"(struct|enum) [A-Z]\w*", "--", "crates/", "game/"],
            capture_output=True,
            text=True,
            cwd=REPO,
        ).stdout.splitlines()
        pattern = re.compile(r"\b(?:struct|enum)\s+([A-Z]\w*)")
        for row in out:
            head, _, body = row.partition(":")
            line, _, rest = body.partition(":")
            match = pattern.search(rest)
            if match:
                index.setdefault(match.group(1), []).append(f"{head}:{line}")
        _DEFINITION_INDEX = index
    return _DEFINITION_INDEX


# ⛔⛤ TYPES THIS REPOSITORY DOES NOT DEFINE, WITH THEIR FLOATS STATED.
# Widening the population from 59 to 175 pulled in two bevy engine types, and the
# `git grep '(struct|enum) Name'` index cannot see them. An unlocated type is
# reported as float-free, which is the REASSURING direction — and `Transform` is
# three Vec3/Quat. So they are named here rather than left to a scan that is
# structurally blind to them.
EXTERNAL_TYPES: dict[str, tuple[str, str, str]] = {
    "Transform": ("struct", "Quat,Vec3", "bevy engine type: translation, rotation, scale"),
    "Name": ("struct", "-", "bevy engine type: a String label on an entity"),
}


def definition(ty: str) -> tuple[str, str, str]:
    """`(kind, float-bearing field types, the type's own first doc line)`."""
    if ty in EXTERNAL_TYPES:
        return EXTERNAL_TYPES[ty]
    found = _definition_index().get(ty, [])
    # ⚠ A TEST FIXTURE MAY DECLARE A TYPE OF THE SAME NAME.
    found = [f for f in found if "/tests" not in f and "tests.rs" not in f]
    if not found:
        return ("?", "?", "")
    path, lineno = found[0].split(":")[0], int(found[0].split(":")[1])
    src = (REPO / path).read_text(encoding="utf-8", errors="replace").split("\n")
    body, depth, started = [], 0, False
    for line in src[lineno - 1 : lineno + 60]:
        body.append(line)
        depth += line.count("{") + line.count("(") - line.count("}") - line.count(")")
        if "{" in line or "(" in line:
            started = True
        if started and depth <= 0:
            break
    text = "\n".join(body)
    kind = "enum" if re.search(rf"enum {ty}\b", text) else "struct"
    floats = sorted({f for f in FLOAT_BEARING if re.search(rf"\b{f}\b", text)})
    doc = ""
    for line in reversed(src[max(0, lineno - 14) : lineno - 1]):
        stripped = line.strip()
        if stripped.startswith("///"):
            content = stripped[3:].strip()
            if content:
                doc = content
        elif stripped.startswith("#[") or stripped == "":
            continue
        else:
            break
    return (kind, ",".join(floats) or "-", doc)


# ⛔⛤ THE INTERSECTION, WHICH IS THE SHARPEST LIST THIS SCRIPT CAN PRODUCE.
#
# ⚠ THE FLOOR IS ON THE INTERSECTION, NOT ON ITS OPERANDS. Both halves being
# large says nothing about the overlap, and an `ok: 0 compared` over two
# non-empty sets is exactly the failure this rule exists for. So the assertion
# lives here, beside the join, and not beside either input.
def sharp_rows(
    unprobed: list[tuple[str, str]], per_tick_read: list[str]
) -> list[tuple[str, str, str]]:
    """Rows that are outside the checksum AND read every tick AND carry a float.

    A float drifts by rounding rather than by a logic error, the drift compounds,
    the reader sees it on the next frame, and no checksum two peers compare can
    see any of it. That is the whole desync shape in one row.

    ⚠ AN UNRESOLVED TYPE (`?`) IS NOT COUNTED AS FLOAT-BEARING. It is not known
    to be float-free either — `definition()` simply could not find it, and
    silently promoting "unknown" to "risky" would inflate this list with the
    instrument's own blind spots. `every_type_resolves` is the arm that keeps
    `?` from happening at all; if it ever fires, this list is short, not long.
    """
    per_tick = set(per_tick_read)
    rows = [
        (name, ty, definition(ty)[1])
        for name, ty in unprobed
        if name in per_tick and definition(ty)[1] not in ("-", "?")
    ]
    if not rows:
        raise AssertionError(
            "the no-projection x per-tick x float-bearing intersection is EMPTY "
            f"over {len(unprobed)} no-projection rows and {len(per_tick_read)} "
            "per-tick readers. Both operands are non-empty, so an empty overlap "
            "is a claim about the JOIN and not a clean bill of health — check "
            "that `definition()` still resolves these types and that the row "
            "names in `per_tick_read` are still spelled like `unprobed`'s."
        )
    return rows


# ---------------------------------------------------------------------------
# THE READER CHECK
#
# ⛔⛔ **THE LATENCY CLASS COMES FROM THE READERS, NOT FROM THE TYPE'S OWN DOC
# COMMENT.** Read by hand on 2026-09-10, the 14 float-free rows were picked as
# likely event-latched state on the reasoning that a row with nothing continuous
# to drift is read on an event. **Ten of them are read every tick.** The prose
# and the readers disagreed for ten of fourteen, so the prose is not evidence.
#
# ⚠ THIS IS A TRIAGE, NOT A VERDICT. A `Query<..&T..>` with no change-detection
# filter runs every tick, so its divergence propagates on the next frame; a
# `Changed<..>`/`Added<..>` filter or a point `get::<T>()` inside a branch may
# not. The second class is what a human must read. This prints the split and
# names which rows need reading rather than deciding for them.

READ_IN_QUERY = re.compile(r"Query\s*<[^;{]{0,400}?&\s*(?:mut\s+)?[\w:]*\b{t}\b", re.S)
GATED = re.compile(r"\b(?:Changed|Added|Or)\s*<")
POINT_READ = re.compile(r"\.get(?:_mut)?::<\s*[\w:]*\b{t}\b")


# ⚠ A SCAN ROOT IS A CITATION A FILE MOVE BREAKS IN SILENCE. This census read
# `crates/` and `game/` only, and the workspace has 51 more `.rs` files under
# `tests/`, `fixtures/` and `examples/`. Measured 2026-09-16: widening to the two
# non-test roots moved no row in this population, which is the evidence that the
# blind spot was empty HERE — not that it cannot matter. `tests/` stays out
# because it is the workspace-policy crate, and the triage drops test paths anyway.
READER_ROOTS = ("crates/", "game/", "examples/", "fixtures/")

_MENTION_INDEX: dict[str, list[str]] = {}


def prime_mentions(types: list[str]) -> None:
    """One `git grep` for every subject at once, attributed per type.

    ⚠ SAME REASON `_definition_index` IS BATCHED, and the reason now bites
    harder: the population went from 59 to 175, and a grep per type is ~0.13 s
    each. The alternation is one subprocess; attribution is one regex pass over
    the matched LINES, which is a far smaller corpus than the tree.
    """
    wanted = sorted({t for t in types if t not in _MENTION_INDEX})
    if not wanted:
        return
    alternation = re.compile(r"\b(" + "|".join(re.escape(t) for t in wanted) + r")\b")
    for t in wanted:
        _MENTION_INDEX[t] = []
    out = subprocess.run(
        ["git", "grep", "-n", "-E", r"\b(" + "|".join(wanted) + r")\b", "--", *READER_ROOTS],
        capture_output=True,
        text=True,
        cwd=REPO,
    ).stdout.splitlines()
    for row in out:
        for name in set(alternation.findall(row)):
            _MENTION_INDEX[name].append(row)


def _mentions(ty: str) -> list[str]:
    if ty not in _MENTION_INDEX:
        prime_mentions([ty])
    return _MENTION_INDEX[ty]


# ⛔⛤ A MARKER IS NEVER BORROWED, ONLY FILTERED ON — AND `reader_sites` LOOKS FOR
# A BORROW. Its first run over the widened population reported `FeatureSimEntity`
# with NO PRODUCTION READER while 81 sites spell `With`/`Without`/`Has` of it.
# That is the reassuring direction, and it is the third way this triage has been
# wrong about a zero (after `&'static T` and the checksum probe).
#
# ⚠ A PRESENCE FILTER IS A DIFFERENT KIND OF READ, not a weaker one. `queue.md`'s
# rule: a component whose PRESENCE is read by a query filter is AUTHORITATIVE even
# when its value is derived — rollback cares whether anything reads it before its
# writer runs again, not how it was computed.
PRESENCE_FILTER = r"(?:With|Without|Has|Added|Changed)\s*<\s*(?:&\s*)?(?:'\w+\s+)?(?:[\w:]*::)?{t}\b"

# ⛔⛤ AND A RESOURCE IS READ THROUGH `Res<T>`, NEVER `&T`. The fourth blind spot,
# and it arrived WITH the widened population: `resource-clone` rows were outside
# this census until the selector moved to the kind, so nothing here had ever been
# a resource. `FriendlyFire`, `PortalFrameHistory` and `SaveRestored` all reported
# NO PRODUCTION READER while each is a `Res`/`ResMut` parameter in a live system.
RESOURCE_READ = r"Res(?:Mut)?\s*<\s*(?:'\w+\s+)?(?:[\w:]*::)?{t}\b"


def _matching_sites(ty: str, template: str) -> list[str]:
    pattern = re.compile(template.format(t=re.escape(ty)))
    out = []
    for row in _mentions(ty):
        path = row.split(":")[0]
        # ⚠ A TEST, A REGISTRATION AND A RE-EXPORT ARE NOT READERS.
        if "test" in path or "rollback_registration" in path:
            continue
        if pattern.search(row.split(":", 2)[-1]):
            out.append(row[:150])
    return out


def presence_filter_sites(ty: str) -> list[str]:
    """Sites that read the component's PRESENCE through a query filter."""
    return _matching_sites(ty, PRESENCE_FILTER)


def resource_read_sites(ty: str) -> list[str]:
    """Sites that take the type as a `Res`/`ResMut` system parameter."""
    return _matching_sites(ty, RESOURCE_READ)


def reader_sites(ty: str) -> tuple[list[str], list[str]]:
    """`(per-tick query sites, sites a human must read)`."""
    per_tick, needs_reading = [], []
    found = _mentions(ty)
    for row in found:
        path = row.split(":")[0]
        # ⚠ A TEST, A REGISTRATION AND A RE-EXPORT ARE NOT READERS, and counting
        # them is how a row with no reader at all looks well used.
        if "test" in path or "rollback_registration" in path:
            continue
        body = row.split(":", 2)[-1]
        if re.search(POINT_READ.pattern.format(t=re.escape(ty)), body):
            needs_reading.append(row[:150])
            continue
        # ⛔⛔ TWO DEFECTS THE FIRST RUN OF THIS TRIAGE HAD, both found by
        # checking it against a hand reading rather than believing it:
        #   * `&'static T` -- a lifetime between the `&` and the type. Bevy
        #     `SystemParam` type aliases spell every borrow that way, so
        #     `OwnedPortalGunPair` came back with NO READER when its reader is a
        #     `Query<.. Option<&'static ..OwnedPortalGunPair>>` in a menu. A zero
        #     is a claim about the scan.
        #   * A CHECKSUM PROBE IS NOT A READER. `fn seat_credit_probe(credit:
        #     &SeatCredit)` takes `&T` and reads it, and counting it made a
        #     component that NOTHING in production consults look well used --
        #     hiding the exact case this census exists to surface.
        if "_probe(" in body or "probes.rs" in path:
            continue
        if re.search(rf"&\s*(?:'\w+\s+)?(?:mut\s+)?(?:[\w:]*::)?{re.escape(ty)}\b", body):
            window = "\n".join(
                (REPO / path).read_text(encoding="utf-8", errors="replace").split("\n")[
                    max(0, int(row.split(":")[1]) - 12) : int(row.split(":")[1]) + 4
                ]
            )
            (needs_reading if GATED.search(window) else per_tick).append(row[:150])
    return per_tick, needs_reading


def main() -> int:
    subjects = rows()
    # ⛔ ANTI-VACUITY, AND THE FLOOR CARRIES ITS OWN POPULATION. 175 rows on
    # 2026-09-16, selected by KIND. The floor is deliberately above the 59 the
    # old sentence-keyed selector returned, so a revert to selecting on `detail`
    # trips it instead of quietly reporting a third of the population.
    assert len(subjects) >= 120, (
        f"only {len(subjects)} rows carry an unhashed value-bearing kind "
        f"{UNHASHED_KINDS}; measured at 175 on 2026-09-16, of which 59 carry the "
        f"'{PROBED_DETAIL}' sentence. A number near 59 means the selector went back "
        "to matching prose; a number near 0 means this scan lost the baseline."
    )
    floaty = 0
    by_sentence: dict[str, int] = {}
    print(f"⛔ ROLLBACK ROWS OUTSIDE THE SESSION CHECKSUM: {len(subjects)}\n")
    for name, ty, detail in subjects:
        kind, floats, doc = definition(ty)
        if floats != "-":
            floaty += 1
        by_sentence[detail] = by_sentence.get(detail, 0) + 1
        print(f"   {kind:6} {floats:22} {name:44} {doc[:70]}")
    print(f"\n   {floaty} of {len(subjects)} carry a float-bearing field type.")
    print("\n⚠ Snapshotted and restored; NOT compared between peers. A probe on a")
    print("  row is a localization aid, not a checksum contribution.")
    print("\n⛔ HOW THESE ROWS DESCRIBE THEMSELVES — the sentence comes from the")
    print("   registrar METHOD, so it is a fact about the ROAD, not about the type:")
    for detail, count in sorted(by_sentence.items(), key=lambda kv: -kv[1]):
        named = "names a value probe" if "probed" in detail else "names none"
        print(f"   {count:4}  [{named:18}]  {detail}")
    print("\n⛔⛤ AND THAT COLUMN IS NOT THE PROBE STRENGTH. THIS SCRIPT CANNOT")
    print("  ANSWER THAT QUESTION AND MUST NOT LOOK LIKE IT DOES. It reads a text")
    print("  baseline; whether a row carries a probe is decided in Rust, at")
    print("  registration. `rollback_component_clone` installs a PRESENCE probe")
    print("  (`ChecksumProbe::presence_for`) even though its detail names no probe")
    print("  at all — an earlier version of this report called those rows")
    print("  \"NO PROBE\" and was wrong, by reading prose for a fact the code owns.")
    print("\n⇒ THE AUTHORITY IS `RollbackChecksumProbes`, at runtime:")
    print("  `strength_tally()` splits Value / Complete (zero-sized, presence IS")
    print("  the value) / Presence, and `presence_only_type_names()` enumerates the")
    print("  weak half. `every_presence_only_probe_is_named_with_its_reason` in")
    print("  `game/ambition_app/tests/rollback_exit_oracle.rs` asserts each one is")
    print("  listed WITH A REASON. Measured 2026-09-16: 364 probes — 220 value,")
    print("  28 complete, 116 presence-only, 45 of those derived.")
    print("\n⚠ A PROBE IS A LOCALIZATION AID, NOT COVERAGE EITHER WAY. It tells a")
    print("  desync hunt WHERE; it contributes nothing to the checksum two peers")
    print("  compare. Every row below is outside that checksum.")

    # ⛔⛤ THE TRIAGE OVER THE NO-PROBE ROWS, which is the classification S7 asks
    # for. These have no checksum contribution AND no localization probe, so a
    # divergence in one is both invisible and unlocatable. What decides whether
    # that matters is who READS the value and how soon.
    #
    # ⚠ `reader_sites` was BUILT AND NEVER CALLED by this script — only its test
    # exercised it. A capability with one customer in a test file looks exactly
    # like an absent one from the report.
    # ⚠ "NAMES NO VALUE PROBE" IS WHAT THIS SELECTS, NOT "HAS NO PROBE" — see the
    # note printed above. These are the rows whose detail claims no value
    # projection; the code gives them a presence probe, which counts carriers and
    # is blind to the value.
    unprobed = [(n, t) for n, t, d in subjects if "probed" not in d]
    prime_mentions([t for _, t in unprobed])
    unread, gated_only, per_tick_read, presence_only = [], [], [], []
    for name, ty in unprobed:
        tick, gated = reader_sites(ty)
        tick = tick or resource_read_sites(ty)
        presence = presence_filter_sites(ty)
        if tick:
            per_tick_read.append(name)
        elif presence:
            presence_only.append(name)
        elif gated:
            gated_only.append(name)
        else:
            unread.append((name, ty))
    print(f"\n⛔ OF THE {len(unprobed)} ROWS WITH NO VALUE PROJECTION — outside the")
    print("   checksum, and localizable only by a carrier COUNT:")
    print(f"   {len(per_tick_read):4}  read by an unfiltered per-tick query — a divergence")
    print("         propagates on the next frame")
    print(f"   {len(presence_only):4}  read only as PRESENCE, through a query filter — the component's")
    print("         existence is authoritative even where its value is derived")
    print(f"   {len(gated_only):4}  only gated or point reads — a human must decide when it is read")
    print(f"   {len(unread):4}  NO PRODUCTION READER THE TRIAGE CAN SEE")
    for name, ty in unread:
        print(f"         {name:44} {ty}")
    # The selector, its floor and the reason all live on `sharp_rows`.
    sharp = sharp_rows(unprobed, per_tick_read)
    print(f"\n⛔⛤ NO PROJECTION x READ EVERY TICK x FLOAT-BEARING: {len(sharp)}")
    print("   A float drifts by rounding, the drift compounds, the reader sees it")
    print("   next frame, and no peer checksum can see any of it.")
    for name, ty, floats in sharp:
        print(f"   {name:44} {ty:34} {floats}")

    print("\n⚠ A ZERO-READER ROW IS A CLAIM ABOUT THE SCAN FIRST. `&'static T` in a")
    print("  Bevy `SystemParam` alias and a checksum probe taking `&T` both fooled")
    print("  this triage once; both are handled and pinned by its test.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
