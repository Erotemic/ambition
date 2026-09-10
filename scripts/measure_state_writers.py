#!/usr/bin/env python3
"""Who WRITES the state a decomposition packet is about, by family and by crate.

⭐⭐ ONE INSTRUMENT, SEVERAL DOMAINS, because "what counts as a write" is one
question and two copies of it would drift. A7 asks for item occurrence / holder /
inventory / checkpoint writers; A4 asks for accepted-control writers. Same
method, different type lists -- so the domains are DATA at the top of this file
and the scanner below is the single authority.

⭐ A7'S HOLD, in its own words: *"after A1, enumerate item
occurrence, holder, inventory and checkpoint baseline writers"*
(`docs/planning/engine/actor-monolith-work-frontier.md`, A7). The packet says the
census comes BEFORE any type moves, so this is the deliverable, not a step
toward one.

⛔⛔ **THIS IS A LOWER BOUND AND THE SEAL IS THE UPPER ONE.** A text scan cannot
see every write to a Bevy component: a rollback codec, a `Reflect` path, a
`serde` restore or a helper that takes `&mut T` and is called from elsewhere all
write without naming the type at the write site. The reliable enumeration is to
SEAL the type — make its fields private — and let `rustc` list the callers that
stop compiling. Do that for the number you are going to act on; use this to know
where to look and to see the SHAPE across crates.

⇒ So every count here is "sites that NAME the type in a write-capable position",
and the report says which mechanism each one is, because the mechanisms have
different costs to move:

    mut_query   `Query<&mut T>` / a bare `&mut T` in a query tuple -- a
                SCHEDULED road into the state, and the row that counts
    mut_param   `name: &mut T` -- a delegation seam, somebody else's road
                borrowed. A chain of these is ONE authority, not N writers
    res_mut     `ResMut<T>` / `resource_mut::<T>` -- a resource writer
    construct   `T { .. }` / `T::new(..)` -- a value being minted
    insert      named inside `insert(..)` / `spawn((..))` -- attached to a body
    remove      `remove::<T>()` -- custody ended
    register    rollback / reflect / save registration -- writes you cannot grep

⛔⛔ **TWO CORRECTIONS THE FIRST RUN NEEDED, AND BOTH INFLATED THE CRATE THE
PACKET IS ABOUT.** `actor_monolith` came out at 17 occurrence writers; 8 of them
were `ActorConstructionParams::GroundItem { .. }` — a variant of a
construction-params enum that shares the component's name, so every `match` arm
reading one counted as a write — and one more was `map.insert("GroundItem", ..)`,
a STRING KEY in the LDtk converter registry. A scan that cannot tell a variant
from a type, or code from text, reports its own method as a finding about the
tree. The discriminator now used is Rust's own convention: a CamelCase qualifier
before the name is an enum, a snake_case one is a module path.

⛔ AND TWO MORE OF THE SAME CLASS, found by reading every foreign site rather
than trusting the total: a `matches!(released, Some(ReleasedAs(..)))` PATTERN
reads and was counted as construction (2 sites in `ambition_abilities`), and
`-> &ambition_items::OwnedItems` is a RETURN TYPE that hands the value out
(1 site). ⇒ Read the sites, not the number: three separate defects in this scan
were only visible from the rows.

⚠ TESTS ARE EXCLUDED, deliberately and by three rules: a filename, an inline
`#[cfg(test)] mod`, and a file-level `#![cfg(test)]` inner attribute. The third
is invisible to the first two and there is exactly one such file in this tree,
which is why it hides -- the sibling ordering census records two false rows from
missing it.

    python3 scripts/measure_state_writers.py --domain item
    python3 scripts/measure_state_writers.py --domain control --sites
    python3 scripts/measure_state_writers.py --domain item --family custody
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ROOTS = ("crates", "game", "tools")

# The four concepts A7 names, and the types that hold each one today. Keeping
# them as SEPARATE families is the point of the packet: the goal is that a body
# can hold an item without that becoming a durable occurrence fact, and a census
# that merged them could not show whether the code already agrees.
DOMAINS: dict[str, dict[str, list[str]]] = {
    # A7 -- item custody and accounting.
    "item": {
        "occurrence": ["GroundItem", "WorldItem", "SettledItem", "ItemMotion", "ItemEmerge"],
        "custody": ["ItemCustody", "ReleasedAs", "ItemStruckBody"],
        "inventory": ["OwnedItems"],
        "checkpoint": [
            "MintedItemBaseline",
            "OwnedItemsBaseline",
            "MintedItemDescription",
            "ItemCheckpointRestoreInputs",
        ],
    },
    # A4 -- accepted control and body execution. The packet's four
    # responsibilities, in its own order: "accepted driver relation, input
    # projection, live body execution and custody reconciliation".
    #
    # ⚠ THE TYPE LIST IS THE MEASUREMENT'S SCOPE, AND A WIDE ONE IS A WORSE
    # ANSWER, NOT A FULLER ONE. The first list here carried `PlayerSlot`,
    # `ActorControlFrame`, `ActionSet` and `BodyBaseSize`. `PlayerSlot(0)` is a
    # VALUE that appears wherever a seat is named, `ActorControlFrame` is the
    # inner value `ActorControl` wraps, and `ActionSet` is an ability roster
    # rather than input projection -- so the report described "code that mentions
    # control vocabulary", which is not a writer map. ⇒ One component per
    # responsibility: the thing whose mutation IS the responsibility.
    "control": {
        "driver_relation": ["DrivingParticipant"],
        "input_projection": ["ActorControl"],
        "body_execution": ["BodyKinematics"],
        "custody_reconciliation": ["InCustodyOf", "BodyCustodySettled"],
    },
}

OWNERS: dict[str, dict[str, set[str]]] = {
    "item": {
        "occurrence": {"ambition_held_items", "ambition_world_items"},
        "custody": {"ambition_held_items"},
        "inventory": {"ambition_items"},
        "checkpoint": {"ambition_platformer2d_actor_monolith"},
    },
    # ⚠ FOR A4 THE "OWNER" IS WHERE THE TYPE IS DEFINED TODAY, WHICH IS THE
    # QUESTION, NOT THE ANSWER. The packet's destination is "coherent logical
    # actor/control modules in the SAME package first", so a foreign row here
    # means a crate outside the type's home writes it -- useful -- while the
    # monolith writing its own is exactly the population A4 wants to re-group
    # INTERNALLY. Read the per-crate rows, not the foreign total.
    "control": {
        "driver_relation": {"ambition_characters"},
        "input_projection": {"ambition_characters"},
        "body_execution": {"ambition_platformer2d_core"},
        "custody_reconciliation": {"ambition_platformer2d_shared_tangle"},
    },
}

MECHANISMS: list[tuple[str, str]] = [
    # ⭐⭐ **A SCHEDULED WRITER AND A DELEGATION SEAM ARE NOT THE SAME ROW, AND
    # COUNTING THEM TOGETHER INFLATES A CHAIN INTO A CROWD.** `dispatch.rs` takes
    # `owned: &mut OwnedItems` and hands it to `dispatch_item_confirm`, which
    # hands it to `apply_menu_action` — three sites, ONE authority threaded
    # through three frames of stack. A `Query<&mut T>` or a `ResMut<T>` in a
    # system signature is a genuinely separate road into the state; a `&mut T`
    # parameter is somebody else's road, borrowed.
    ("mut_query", r"(?:Query|Single|Populated)\s*<[^;]{{0,200}}&\s*mut\s+{t}\b|^\s*&\s*mut\s+{t}\b"),
    ("res_mut", r"(?:ResMut\s*<\s*{t}\b|resource_mut::<\s*{t}\b|get_resource_mut::<\s*{t}\b)"),
    ("remove", r"remove(?:_resource)?::<\s*{t}\b"),
    ("register", r"(?:register[_a-z]*\w*|reflect|from_save|to_save|deserialize)\s*[(<:].{{0,80}}\b{t}\b"),
    ("mut_param", r"\w+\s*:\s*(?:Option\s*<\s*)?&\s*mut\s+{t}\b"),
    ("construct", r"\b{t}\s*(?:\{{|::new\b|::default\b|::from\b|\()"),
    ("insert", r"(?:insert|insert_if_new|spawn|spawn_batch|try_insert|insert_resource|init_resource)\s*[(<].{{0,200}}\b{t}\b"),
]


def is_test_file(path: pathlib.Path, text: str) -> bool:
    if "tests" in path.name or "/tests/" in str(path):
        return True
    return bool(re.search(r"^\s*#!\[\s*cfg\s*\(\s*test\s*\)\s*\]", text, re.MULTILINE))


def blank(span: str) -> str:
    """The same span, emptied but the SAME HEIGHT.

    ⛔⛔ **DELETING A BLOCK MOVES EVERY LINE NUMBER BELOW IT, AND THIS SCANNER
    REPORTS LINE NUMBERS.** The first version spliced test modules and type
    bodies OUT of the text and then counted `\n`s in what was left, so every
    site under a stripped block was cited at the wrong line — `sentry.rs:411`
    was reported as production when line 411 of the ORIGINAL file is inside
    `mod tests`. A citation that points at the wrong line is worse than none:
    it looks checkable and reads as checked.

    ⛔ AND IT ALSO JOINED THE SEAM. `text[:start] + text[end:]` concatenates the
    line before a stripped block with the line after it, which can spell a match
    that exists in neither.

    ⇒ Replace the span with its own newlines: same height, no content, no seam.
    """
    return "\n" * span.count("\n")


def strip_test_mods(text: str) -> str:
    out, i = [], 0
    pattern = re.compile(r"#\[cfg\(test\)\]\s*(?:(?:///?[^\n]*|//![^\n]*|#\[[^\]]*\])\s*)*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{")
    while True:
        match = pattern.search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i : match.start()])
        j, depth = match.end() - 1, 0
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        out.append(blank(text[match.start() : j + 1]))
        i = j + 1
    joined = "".join(out)
    return "\n".join(line.split("//")[0] for line in joined.split("\n"))


def strip_type_bodies(text: str) -> str:
    """Drop `struct X { .. }` / `enum X { .. }` BODIES.

    ⛔⛔ **A VARIANT DECLARATION IS NOT A WRITE, AND SKIPPING THE `pub enum` LINE
    DOES NOT SKIP ITS BODY.** `ActorConstructionParams` declares a `GroundItem {
    spec, held }` variant, and the variant's own line inside the enum matched
    the construction pattern — so `actor_monolith` was credited with writing an
    occurrence at the place the params type is DEFINED. Same for a struct field
    whose type is one of these.
    """
    out, i = [], 0
    pattern = re.compile(
        r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+\w+[^{;]*\{", re.MULTILINE
    )
    while True:
        match = pattern.search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i : match.start()])
        j, depth = match.end() - 1, 0
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        out.append(blank(text[match.start() : j + 1]))
        i = j + 1
    return "".join(out)


def crate_of(path: pathlib.Path) -> str:
    rel = path.relative_to(REPO)
    return rel.parts[1] if len(rel.parts) > 1 else "?"


def owning_crate(domain: str, family: str) -> set[str]:
    """Which crate the family's types live in — a write from HERE is the domain
    doing its job; a write from anywhere else crosses a boundary."""
    return OWNERS[domain][family]


def scan(families: dict[str, list[str]]):
    rows = []
    compiled = {
        name: {
            mech: re.compile(pattern.format(t=re.escape(t)))
            for mech, pattern in MECHANISMS
        }
        for fam in families.values()
        for name in fam
        for t in [name]
    }
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            raw = path.read_text(encoding="utf-8", errors="ignore")
            if is_test_file(path, raw):
                continue
            body = strip_type_bodies(strip_test_mods(raw))
            if not any(t in body for fam in families.values() for t in fam):
                continue
            lines = body.split("\n")
            for family, types in families.items():
                for t in types:
                    for n, line in enumerate(lines, start=1):
                        if t not in line:
                            continue
                        # ⚠ A DECLARATION IS NOT A WRITE. `pub struct GroundItem`
                        # and `pub enum ItemCustody` are the type existing, and
                        # counting them would put the owner at the top of its own
                        # report for defining what it owns.
                        if re.match(r"\s*(?:pub\s+)?(?:struct|enum|impl|type|use)\b", line):
                            continue
                        # ⛔⛔ **A CamelCase QUALIFIER IS AN ENUM VARIANT, NOT THIS
                        # TYPE — AND IT WAS 8 OF THE 17 THE FIRST RUN ATTRIBUTED
                        # TO `actor_monolith`.** `ActorConstructionParams::
                        # GroundItem { .. }` is a variant of a construction-params
                        # enum that happens to share the component's name, and a
                        # `match` arm reading one is not a write to anything.
                        # `ambition_held_items::GroundItem { .. }` IS the
                        # component, and the discriminator is Rust's own
                        # convention: a module path is snake_case, a type is not.
                        if re.search(rf"[A-Z]\w*::{re.escape(t)}\b", line):
                            continue
                        # ⛔ A PATTERN IS A READ. `matches!(released,
                        # Some(ReleasedAs(Release::Throw)))` destructures; it
                        # writes nothing, and the tuple-struct syntax is
                        # indistinguishable from construction without knowing
                        # which side of a `match` you are on.
                        # ⛔ A TUPLE STRUCT'S PATTERN IS SPELLED LIKE ITS
                        # CONSTRUCTOR. `DrivingParticipant(slot)` reads on the
                        # left of a `let` and writes on the right, so an
                        # `if let Some(DrivingParticipant(slot)) = ..` and a
                        # `match` arm both looked like minting a driver relation.
                        # 20 of the monolith's 24 driver rows were that.
                        if (
                            "matches!(" in line
                            or "if let" in line
                            or "while let" in line
                            or "=>" in line
                        ):
                            continue
                        # ⛔ AND A RETURN TYPE IS A READ. `pub fn remembered(&self)
                        # -> &ambition_items::OwnedItems` hands one out.
                        arrow = line.find("->")
                        if arrow != -1 and arrow < line.find(t):
                            continue
                        # ⚠ AND A STRING LITERAL IS A KEY, NOT A TYPE.
                        # `map.insert("GroundItem", convert_ground_item)` in the
                        # LDtk converter registry was reported as an `insert`
                        # write by a scan that could not tell code from text.
                        if re.search(rf'"[^"]*\b{re.escape(t)}\b[^"]*"', line):
                            continue
                        # ⛔ AND A MULTI-LINE STRING HAS NO CLOSING QUOTE ON THIS
                        # LINE. `"control invariant: {} entities hold
                        # DrivingParticipant({slot:?}); \` is a panic message
                        # continued on the next line, and the paired-quote rule
                        # above cannot see it. An ODD number of quotes before the
                        # name means the name is inside one.
                        if line[: line.find(t)].count('"') % 2 == 1:
                            continue
                        for mech, _ in MECHANISMS:
                            if compiled[t][mech].search(line):
                                rows.append(
                                    (family, t, mech, crate_of(path),
                                     str(path.relative_to(REPO)), n, line.strip()[:110])
                                )
                                break
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--domain", choices=sorted(DOMAINS), default="item")
    parser.add_argument("--family", help="one family of the chosen domain only")
    parser.add_argument("--sites", action="store_true", help="print every site")
    args = parser.parse_args()

    all_families = DOMAINS[args.domain]
    if args.family and args.family not in all_families:
        parser.error(f"--family must be one of {sorted(all_families)} for --domain {args.domain}")
    families = {args.family: all_families[args.family]} if args.family else all_families
    rows = scan(families)

    print(f"⛔ {args.domain.upper()} STATE WRITERS — a LOWER BOUND. Seal the type "
          "to get the upper one.\n")
    for family in families:
        fam_rows = [r for r in rows if r[0] == family]
        owners = owning_crate(args.domain, family)
        foreign = [r for r in fam_rows if r[3] not in owners]
        print(f"== {family.upper()}  ({', '.join(sorted(owners))} owns it)")
        print(f"   {len(fam_rows)} write-capable sites, {len(foreign)} of them OUTSIDE the owning crate")
        by_crate = collections.Counter(r[3] for r in fam_rows)
        for crate, n in sorted(by_crate.items(), key=lambda kv: (-kv[1], kv[0])):
            mark = "   " if crate in owners else " ⛔"
            mechs = collections.Counter(r[2] for r in fam_rows if r[3] == crate)
            detail = ", ".join(f"{m}:{c}" for m, c in sorted(mechs.items()))
            print(f"  {mark} {crate:44s} {n:3d}   {detail}")
        if args.sites:
            for _, t, mech, crate, where, line_no, text in sorted(fam_rows, key=lambda r: (r[3], r[4], r[5])):
                print(f"        {where}:{line_no}  [{mech}] {t}  {text}")
        print()

    print(f"   TOTAL write-capable sites across the {len(families)} families:", len(rows))
    print()
    print("⛔ WHAT THIS METHOD CANNOT SEE — the number travels with these or not at all.")
    print("   * A ROLLBACK CODEC / `Reflect` / `serde` RESTORE writes without naming")
    print("     the type at the write site. `register_checkpoint_rollback_state` and the")
    print("     save codecs are roads into all four families that no text scan reaches.")
    print("   * A HELPER TAKING `&mut T` is counted where it is DECLARED, not at each")
    print("     call. `mut_param` rows are seams; the caller graph behind them is not")
    print("     enumerated here.")
    print("   * WHETHER A `ResMut<T>` EVER MUTATES is not decided by this scan. Every")
    print("     foreign row in this report was read by hand and does write; the")
    print("     owning-crate rows were not exhaustively read.")
    print("   * A MACRO-GENERATED write is invisible.")
    print("   ⇒ SEAL THE TYPE for a number you are going to act on: make its fields")
    print("     private and let `rustc` enumerate the callers that stop compiling.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
