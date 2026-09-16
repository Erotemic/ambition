#!/usr/bin/env python3
"""Do the consolidation ledger's SEMANTIC items still point at things that exist?

The ledger's 114 items were read against one commit and carry no mechanism for
noticing that the tree moved underneath them. This checks the two facts that CAN
be checked mechanically:

1. every `source_paths` / evidence `sources` entry is a file that exists;
2. every CamelCase name in a `current_truth` sentence resolves to a definition
   somewhere in the tracked Rust sources.

⛔⛤ **AND (2) IS THE WEAK ONE — IT CANNOT SEE OUTSIDE THE WORKSPACE.** MEASURED
2026-09-16: it reported four unresolved names and ALL FOUR were false positives.
`ResMut` and `TypeId` come from Bevy and std; `LoadId` is `ambition_load`'s and
`RunGgrsSystems` is `bevy_ggrs`'s, both live and both used in dozens of places.
A name this check cannot resolve is a name it cannot SEE the definition of, which
is a claim about the scan. ⇒ Triage every finding by hand before believing one.

⛔⛔ **AND NEITHER CHECK SAYS THE CLAIMS ARE STILL TRUE.** A `current_truth`
sentence can go completely stale while every path exists and every type still
compiles — the owner changes, a second writer appears, a road is deleted and the
sentence describing it survives. That needs re-reading the source behind the
item, which is what the census README asks for and what this cannot substitute
for. ⇒ A green run here bounds the CHEAP failure and says nothing about the
expensive one. Do not cite it as a ledger refresh.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LEDGER = REPO / "docs/planning/consolidation/consolidation-ledger.json"

#: Names owned by Bevy, std or another crate outside this workspace. A definition
#: for these will never be found here, and their absence is not a finding.
EXTERNAL = frozenset({"ResMut", "TypeId", "LoadId", "RunGgrsSystems"})

IDENT = re.compile(r"\b([A-Z][a-z0-9]+(?:[A-Z][a-z0-9]+)+)\b")

#: ⛔ ANTI-VACUITY. A corpus this small means the scan broke, and a clean verdict
#: over it would be a claim about the scan rather than about the ledger.
MIN_CORPUS_KIB = 20_000


#: An item whose `representation` is exactly this claims a Bevy Resource.
RESOURCE_REPRESENTATION = "Resource"
#: The subject type is the first CamelCase token of `current_truth`.
SUBJECT = re.compile(r"\s*`?([A-Z][A-Za-z0-9]+)")


def declaration_kinds(source_by_file: dict[str, str], ty: str) -> list[str]:
    """`Resource`/`Component` as DECLARED on each `pub struct|enum <ty>`."""
    # ⚠ THE ATTRIBUTE BLOCK IS OPTIONAL ON PURPOSE. Requiring one made a type
    # with NO derives invisible — and a type with no derives cannot be a
    # Resource, so it is exactly a case the rule must be able to flag. Found by
    # the control arm, which asked for `neither` and got nothing at all.
    pat = re.compile(rf"((?:#\[[^\]]*\]\s*)*)pub (?:struct|enum) {re.escape(ty)}\b")
    kinds = []
    for text in source_by_file.values():
        for match in pat.finditer(text):
            attrs = match.group(1)
            kind = []
            if re.search(r"\bResource\b", attrs):
                kind.append("Resource")
            if re.search(r"\bComponent\b", attrs):
                kind.append("Component")
            kinds.append("+".join(kind) or "neither")
    return kinds


def workspace_packages() -> set[str]:
    """Ask cargo which crates the workspace HAS. Do not model it from globs.

    ⛔⛤ **THE LEDGER KEEPS A SECOND COPY OF THE WORKSPACE AND NOTHING COMPARED
    IT.** `workspace_crates` holds 80 rows with per-crate LOC, dependency and
    public-surface measurements, read at `source_commit` and carrying no
    mechanism for noticing that a crate was added, removed or renamed. MEASURED
    2026-09-16: it is EXACT today — 80 and 80, no difference in either direction,
    every recorded path still present. That is precisely when to install the
    comparison, because a duplicate authority is invisible until the day it is
    wrong, and on that day it is a confident wrong answer.

    ⚠ `cargo metadata --no-deps` needs no build and takes ~45 ms here.
    """
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    )
    return {pkg["name"] for pkg in json.loads(out.stdout)["packages"]}


def main() -> int:
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    items = ledger["items"]
    if not items:
        print("⛔⛔ the ledger holds no items; refusing to report on an empty corpus")
        return 1

    missing: list[tuple[str, str]] = []
    paths_seen = 0
    for item in items:
        cited = list(item.get("source_paths", []))
        cited += [s for e in item.get("evidence", []) for s in e.get("sources", [])]
        for rel in cited:
            paths_seen += 1
            if not (REPO / rel).exists():
                missing.append((item["id"], rel))

    tracked = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    blob = []
    for rel in tracked:
        try:
            blob.append((REPO / rel).read_text(encoding="utf-8"))
        except OSError:
            # ⛔ A swallowed read error reports its own finding as a clean scan.
            print(f"⛔⛔ could not read {rel}; the corpus is incomplete")
            return 1
    source = "\n".join(blob)
    if len(source) // 1024 < MIN_CORPUS_KIB:
        print(
            f"⛔⛔ scanned only {len(source)//1024} KiB of Rust across {len(tracked)} "
            f"file(s), floor is {MIN_CORPUS_KIB} KiB"
        )
        return 1

    cited_names: dict[str, set[str]] = {}
    for item in items:
        for match in IDENT.finditer(item.get("current_truth", "")):
            if match.group(1) not in EXTERNAL:
                cited_names.setdefault(match.group(1), set()).add(item["id"])
    unresolved = [
        (name, sorted(ids))
        for name, ids in cited_names.items()
        if not re.search(rf"\b(struct|enum|trait|type|fn)\s+{name}\b", source)
    ]

    if missing or unresolved:
        if missing:
            print(f"⛔ {len(missing)} cited source path(s) no longer exist:")
            for item_id, rel in sorted(set(missing)):
                print(f"    {item_id}: {rel}")
        if unresolved:
            print(f"⛔ {len(unresolved)} name(s) in `current_truth` resolve to no definition:")
            for name, ids in sorted(unresolved):
                print(f"    {name}  cited by {', '.join(ids)}")
            print(
                "⚠ TRIAGE EACH BY HAND. A name defined OUTSIDE this workspace is "
                "invisible here and is not a defect — add it to EXTERNAL. A name "
                "that genuinely went is a ledger item describing a road that is gone."
            )
        return 1

    # ⛔⛤ RULE 4: AN ITEM THAT SAYS "Resource" MUST NAME A `#[derive(Resource)]`.
    # MEASURED 2026-09-16: of nine items whose `representation` is exactly
    # "Resource", EIGHT were right and one was not. `AUTH-CONTENT-BINDING` --
    # marked SOURCE_CONFIRMED -- recorded `ActiveContentBinding` as a Resource; it
    # is a Component, one declaration, `world/rooms/transaction.rs`. ⇒ The
    # distinction is load-bearing: a Resource is PROCESS-GLOBAL and a Component is
    # carried by an ENTITY, which for C05 is the difference between "another
    # App-global write at the generation boundary" and "a value the admitted
    # candidate already owns". This is the "expensive failure" the docstring above
    # says this check cannot see — one narrow slice of it now IS checkable.
    by_file = dict(zip(tracked, blob))
    wrong = []
    claimed = 0
    for item in items:
        if str(item.get("representation")) != RESOURCE_REPRESENTATION:
            continue
        claimed += 1
        match = SUBJECT.match(item.get("current_truth", ""))
        if not match:
            wrong.append(f"    {item['id']}: no subject type in `current_truth`")
            continue
        ty = match.group(1)
        kinds = declaration_kinds(by_file, ty)
        if not kinds:
            # ⚠ A type this scan cannot FIND is a claim about the scan, not a
            # finding — the name-resolution rule above already reports those.
            continue
        if not any("Resource" in kind for kind in kinds):
            wrong.append(
                f"    {item['id']}: says `representation: Resource`, but {ty} is "
                f"declared {kinds}"
            )
    # ⛔ ANTI-VACUITY: if no item claims "Resource" any more, this rule checked
    # nothing and its silence would mean nothing.
    # ⛔ ANTI-VACUITY, SCALED TO THE CORPUS. On the real ledger a rule that
    # checked fewer than three items has effectively checked nothing and its
    # silence would mean nothing. ⚠ But the arms beside this guard plant
    # three-line ledgers to test ONE rule each, and a flat floor made them fail
    # on this rule instead of on their own subject — the same ordering mistake
    # the workspace rule made an hour earlier. The floor applies where the
    # population exists.
    if len(items) >= 50 and claimed < 3:
        print(
            f"⛔⛔ only {claimed} item(s) claim `representation: Resource` across "
            f"{len(items)} ledger items; this rule checked almost nothing"
        )
        return 1
    if wrong:
        print(
            f"⛔ {len(wrong)} ledger item(s) record a representation source "
            "disagrees with:\n"
        )
        print("\n".join(wrong))
        print(
            "\n⇒ A Resource is PROCESS-GLOBAL and a Component is carried by an\n"
            "  ENTITY. An item that gets this wrong describes a different\n"
            "  consolidation problem from the one the tree has."
        )
        return 1

    # RULE 3: the ledger's copy of the workspace must still BE the workspace.
    try:
        real = workspace_packages()
    except (OSError, subprocess.CalledProcessError) as exc:
        # ⛔ A check that cannot reach its authority REFUSES. Skipping here would
        # make every future run green for a reason that has nothing to do with
        # the ledger.
        print(f"⛔⛔ could not ask cargo for the workspace membership: {exc}")
        return 1
    recorded = {crate["package"] for crate in ledger.get("workspace_crates", [])}
    if not recorded:
        print("⛔⛔ the ledger records no workspace crates; refusing to compare "
              "against an empty set")
        return 1
    added = sorted(real - recorded)
    gone = sorted(recorded - real)
    if added or gone:
        print(
            f"⛔ the ledger's workspace copy no longer matches cargo "
            f"({len(recorded)} recorded, {len(real)} real):"
        )
        for name in added:
            print(f"    IN THE WORKSPACE, NOT IN THE LEDGER: {name}")
        for name in gone:
            print(f"    IN THE LEDGER, NOT IN THE WORKSPACE: {name}")
        print(
            "⇒ Every per-crate measurement beside these rows was taken at the "
            "ledger's `source_commit`. A crate the ledger has never seen has no "
            "row at all, and a crate that left takes a row of numbers with it "
            "that still reads as current."
        )
        return 1

    print(
        f"ok: {len(items)} ledger items, {paths_seen} cited source path(s) all exist, "
        f"{len(cited_names)} name(s) in `current_truth` all resolve, "
        f"{len(recorded)} workspace crate(s) match cargo exactly "
        f"({len(tracked)} tracked .rs files, {len(source)//1024} KiB)"
    )
    print(
        "⚠ this says nothing about whether the CLAIMS are still true — see the "
        "module docstring."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
