#!/usr/bin/env python3
"""Cross-crate schedule ordering that names a foreign SYSTEM instead of a phase.

⭐⭐ THE PREREQUISITE THIS MEASURES, in the architecture program's own words:
*"Every load-bearing cross-capability ordering relationship must be expressible
using public phase/set vocabulary rather than foreign system identities."* A
capability that says `my_system.before(other_crate::their_system)` has taken
private ordering authority over a domain it does not own, and moving the two into
separately composable crates does not make them separately composable — the edge
is in the code, not in the packaging.

⛔ ORDERING AGAINST A **SET** IS THE GOOD FORM AND IS NOT COUNTED.
`.before(CombatSet::Resolve)` is a capability installing itself into published
phase vocabulary, which is exactly the shape this prerequisite wants. The
discriminator is the final path segment: a set or variant is `CamelCase`, a
system function is `snake_case`.

⛔⛔ AND BEING THE RUNTIME IS NOT AN EXEMPTION — my first version made it one and
the architecture note's OWN example is the case it excused. The runtime owns
phases and the order of phases; it does not own the pairwise order of two
capabilities' private systems. The named poison is
`ambition_mount::enforce_mount_rider_link` chained with
`actor_monolith::rebuild_dismounted_rider_brains`, and the crate writing that
chain is `ambition_platformer2d_runtime` — a composition layer. ⇒ Writer is
reported as INFORMATION, never as a reason to drop a row.

⚠ AND `.before(...)` / `.after(...)` IS NOT THE ONLY SPELLING. That same example
is a CHAINED TUPLE, so a regex over `.before(` misses it entirely — the first
version of this script scored the architecture note's own example as absent. The
population is therefore every foreign system NAMED ANYWHERE INSIDE an
`add_systems(...)` call: installing another crate's system is the same authority
question as ordering it (*"who installs it? its own plugin/composition unit"*).

    python3 scripts/measure_foreign_system_ordering.py
    python3 scripts/measure_foreign_system_ordering.py --all   # include intra-crate
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ROOTS = ("crates", "game", "tools")

ADD_SYSTEMS = re.compile(r"\badd_systems\s*\(")
PATH = re.compile(r"\b([a-z_][A-Za-z_0-9]*(?:::[A-Za-z_0-9]+)+)")


def add_systems_blocks(text: str) -> list[str]:
    """Every `add_systems( ... )` call body, by paren matching."""
    blocks = []
    for match in ADD_SYSTEMS.finditer(text):
        i = match.end() - 1
        depth = 0
        while i < len(text):
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        blocks.append(text[match.end() : i])
    return blocks

#  a composition layer is named by its ROLE, not by a list of crates that
# happen to exist today: anything whose crate name ends in one of these is the
# layer the architecture model puts global ordering in.
COMPOSITION_SUFFIXES = ("_runtime", "_host", "_provider", "_app")


def workspace_crates() -> set[str]:
    """Every crate name in the workspace, read from the tree.

    ⛔⛔ WITHOUT THIS THE FIRST RUN REPORTED 28 CAPABILITY VIOLATIONS AND THE
    TRUE NUMBER WAS A THIRD OF THAT. A bare module path — `actors::sync_visuals`,
    `far_side::composite_far_side_bodies` — is a crate ordering its OWN systems
    through a sibling module, which is not a boundary at all; only a path whose
    head is another CRATE crosses one. Testing `head in ("crate", "super",
    "self")` catches the qualified spellings and misses the bare one, which is
    the commonest.
    """
    names = set()
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for manifest in base.rglob("Cargo.toml"):
            for line in manifest.read_text(encoding="utf-8", errors="ignore").split("\n"):
                if line.startswith("name") and "=" in line:
                    names.add(line.split("=", 1)[1].strip().strip('"'))
                    break
    return names


# ⛔⛔ THE CHAIN BETWEEN THEM IS THE POINT, AND A REGEX IS THE WRONG TOOL FOR IT.
# A system is rarely written `name.in_set(S)`; it is written
# `name.after(X).in_set(S)`, across lines, so a matcher demanding adjacency
# reports "in no set" for the commonest spelling — which is how this hint told me
# `commit_seat_raw_frames` belonged to nothing while `ambition_platformer2d_host`
# installs it into `PrimarySlotInputCommit` three lines down.
#
# ⚠ AND MY FIRST FIX FOR THAT WAS A NESTED-QUANTIFIER REGEX THAT BACKTRACKED THE
# SCRIPT PAST TWO MINUTES. A tool nobody can run is a tool nobody runs. This
# scans backwards from each `.in_set(` over the chained calls instead — linear,
# bounded, and it says in code what the regex said in punctuation.
IN_SET_CALL = re.compile(r"\.in_set\(\s*([A-Za-z_][A-Za-z_0-9:]*)")
IDENT_TAIL = re.compile(r"([A-Za-z_][A-Za-z_0-9]*)\s*$")


def _system_before(text: str, at: int) -> str | None:
    """The identifier a chain of `.method(...)` calls hangs off, scanning back."""
    i = at
    for _ in range(12):  # bounded: a real chain is a handful of calls
        head = text[:i]
        match = IDENT_TAIL.search(head)
        if not match:
            return None
        name = match.group(1)
        before = head[: match.start(1)].rstrip()
        if not before.endswith("."):
            return name
        # `name` was a method in the chain — step over its argument list
        close = before[:-1].rstrip()
        if not close.endswith(")"):
            return name
        depth, j = 0, len(close) - 1
        while j >= 0:
            if close[j] == ")":
                depth += 1
            elif close[j] == "(":
                depth -= 1
                if depth == 0:
                    break
            j -= 1
        if j < 0:
            return None
        i = j
    return None


def sets_by_system() -> dict[str, set[str]]:
    """Which published set, if any, each system is already installed into.

    ⭐⭐ A PEER'S SUGGESTION AND IT SORTS THE CHEAP FIXES OUT FOR FREE. The rows
    in this report read identically and are at least THREE different jobs:

      ABSENCE            nothing to name, so the consumer named a system.
                         Fix: publish a marker set, install into it, order
                         against it — three crates.
      WRONG MEMBERSHIP   the right phase exists and the system is not IN it.
                         Fix: one `.in_set(...)` where the system is installed.
      UNUSED SET         the system is ALREADY in a published set and the
                         consumer did not use it. Fix: ONE LINE, in the
                         consumer's own crate, touching nobody else's lane.

    ⇒ Printing the set a named system already belongs to turns the third case
    into a visible one-liner instead of something each reader rediscovers. ⚠ It
    does NOT mean the fix is automatic: a set is usually WIDER than the system,
    so ordering against it is strictly stronger, and a consumer that must
    interleave *within* the set cannot take it.

    ⛔⛔ AND THE QUESTION THIS ANSWERS IS NARROWER THAN IT READS. It says "this
    system is installed into a set SOMEWHERE IN THE SOURCE" — not "in the
    composition you are building". For a capability that is compiled out, or one
    whose install lives behind a feature this build does not enable, the hint
    will name a set for an installation that does not happen. ⇒ A consumer edge
    written on that basis orders against a set with no members and is a silent
    no-op, which is the failure the `scripted_input` comment already names for
    systems: *"ordering against a system nobody composed is a silent no-op"*.

    ⚠ The reason it is safe as written is that it reads SOURCE and not a runtime
    registry — a peer's `EncounterRegistry` finding is the same trap one layer
    over: an index of what is alive right now looks exactly like a roster of what
    exists, same key type and opposite membership rule, and only the doc
    distinguishes them. Reachability is not enough; the thing reached has to be
    the population the question is about.
    """
    found: dict[str, set[str]] = {}
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in base.rglob("*.rs"):
            if "tests" in path.name or "/tests/" in str(path):
                continue
            text = raw_text(path)
            for match in IN_SET_CALL.finditer(text):
                system = _system_before(text, match.start())
                if system:
                    found.setdefault(system, set()).add(match.group(1).split("::")[-1])
    return found


def raw_text(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8", errors="ignore")


def crate_of(path: pathlib.Path) -> str:
    rel = path.relative_to(REPO)
    return rel.parts[1] if len(rel.parts) > 1 else "?"


def is_composition_layer(crate: str) -> bool:
    return crate.endswith(COMPOSITION_SUFFIXES)


def strip_comments_and_tests(text: str) -> str:
    """Line comments out; inline `#[cfg(test)] mod` blocks out.

    ⛔ THE SECOND HALF IS NOT OPTIONAL, and a sibling instrument shipped without
    it: `measure_kernel_module_graph.py` promised in prose to exclude tests and
    excluded only test FILES, which put a whole module inside a reported
    dependency cycle on the strength of one fixture line. A test ordering two
    systems is a fixture, not an architecture.
    """
    out: list[str] = []
    i = 0
    pattern = re.compile(r"#\[cfg\(test\)\]\s*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{")
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
        i = j + 1
    joined = "".join(out)
    return "\n".join(line.split("//")[0] for line in joined.split("\n"))


def findings(include_local: bool) -> list[tuple[str, str, str, str, str]]:
    crates = workspace_crates()
    rows: list[tuple[str, str, str, str, str]] = []
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            name = path.name
            if "tests" in name or "/tests/" in str(path):
                continue
            # ⛔⛔ A WHOLE FILE CAN BE TEST-ONLY. `#![cfg(test)]` is an INNER
            # attribute gating the entire module, and it is invisible both to a
            # filename heuristic and to the inline-`mod` stripper below — the two
            # exclusions that look like they cover tests. Measured 2026-09-06:
            # `features/ecs/fighter_harness.rs` carries one, and counting it put
            # TWO false capability violations in this report. There is exactly one
            # such file in the tree, which is why it hid: a rule with a single
            # instance is one nobody trips over until it matters.
            if re.search(r"^\s*#!\[\s*cfg\s*\(\s*test\s*\)\s*\]", raw_text(path), re.MULTILINE):
                continue
            crate = crate_of(path)
            body = strip_comments_and_tests(
                path.read_text(encoding="utf-8", errors="ignore")
            )
            for block in add_systems_blocks(body):
                ordered = {
                    m.group(1)
                    for m in re.finditer(
                        r"\.(?:before|after)\(\s*([a-z_][A-Za-z_0-9]*(?:::[A-Za-z_0-9]+)+)",
                        block,
                    )
                }
                # ⛔⛔ A CHAINED TUPLE IS ORDERING, AND MISSING IT SCORED THE
                # ARCHITECTURE NOTE'S OWN EXAMPLE AS MERE INSTALLATION.
                # `(ambition_mount::enforce_mount_rider_link,
                #   actor_monolith::features::rebuild_dismounted_rider_brains).chain()`
                # asserts relative order over two crates' private systems from a
                # third — which is the named poison — and contains no `.before`.
                # ⇒ Every path inside a `( ... ).chain()` span counts as ordered.
                for chain in re.finditer(r"\)\s*\.chain\(\)", block):
                    end = chain.start()
                    depth, k = 0, end
                    while k >= 0:
                        if block[k] == ")":
                            depth += 1
                        elif block[k] == "(":
                            depth -= 1
                            if depth == 0:
                                break
                        k -= 1
                    span = block[k : end + 1]
                    inside = [t for t in PATH.findall(span) if t.split("::")[-1][:1].islower()]
                    owners = {t.split("::")[0] for t in inside if t.split("::")[0] in crates}
                    owners.discard(crate)
                    # ⭐ THE DISCRIMINATOR IS TWO DIFFERENT FOREIGN OWNERS, not
                    # "a chain exists". A composition layer sequencing systems it
                    # installs is what a runtime is FOR; the defect the note
                    # names is a THIRD crate fixing the relative order of two
                    # OTHER crates' private systems. Without this the measure
                    # reported 174 and most of it was a runtime doing its job.
                    if len(owners) >= 2:
                        ordered |= set(inside)
                for target in PATH.findall(block):
                    last = target.split("::")[-1]
                    # A set or a variant is CamelCase; a system is a snake_case fn.
                    if not last[:1].islower():
                        continue
                    head = target.split("::")[0]
                    # FOREIGN means "another crate", not "not `crate::`".
                    local = head not in crates or head == crate
                    if local and not include_local:
                        continue
                    kind = "ordering" if target in ordered else "install"
                    rows.append(
                        (crate, target, str(path.relative_to(REPO)),
                         ("local" if local else "foreign"), kind)
                    )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="include intra-crate ordering")
    args = parser.parse_args()

    rows = findings(include_local=args.all)
    foreign = [r for r in rows if r[3] == "foreign"]
    # ⭐ TWO QUESTIONS, NOT ONE, AND CONFLATING THEM TURNS A WORK LIST INTO A
    # PROGRAM. ORDERING is a crate asserting relative order over systems it does
    # not own — the sharp defect, and small. INSTALL is a crate adding another
    # crate's system to a schedule at all — the broader "who installs it"
    # question, and it is most of what a composition layer does today.
    ordering = [r for r in foreign if r[4] == "ordering"]
    install = [r for r in foreign if r[4] == "install"]
    capability = [r for r in ordering if not is_composition_layer(r[0])]
    composition = [r for r in ordering if is_composition_layer(r[0])]

    print("⛔ FOREIGN SYSTEMS NAMED IN A SCHEDULE — every one is the prerequisite's subject.")
    print("   (writer role is information, not an exemption: the runtime owns PHASES,")
    print("    not the pairwise order of two capabilities' private systems.)\n")
    memberships = sets_by_system()
    print(f"-- written by a capability / ruleset: {len(capability)}")
    for crate, target, where, _, _k in sorted(set(capability)):
        system = target.split("::")[-1]
        already = memberships.get(system, set())
        hint = (
            f"\n          ⭐ ALREADY IN A PUBLISHED SET: {', '.join(sorted(already))}"
            "  — one-line fix in the consumer"
            if already
            else "\n          ⛔ in no set — needs one published before it can be named"
        )
        print(f"   {crate}\n       -> {target}\n          {where}{hint}")
    print(f"\n-- written by a composition layer: {len(composition)}")
    for crate, target, where, _, _k in sorted(set(composition)):
        print(f"   {crate}\n       -> {target}\n          {where}")

    print(f"\n   ORDERING a foreign system (the sharp defect): {len(ordering)}"
          f"   — capability {len(capability)}, composition {len(composition)}")
    print(f"   INSTALLING a foreign system (the broader question): {len(install)}")
    print(f"   TOTAL foreign systems named inside add_systems: {len(foreign)}")
    if args.all:
        print(f"   intra-crate (not a boundary): {len([r for r in rows if r[3] == 'local'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
