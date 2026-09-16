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
⚠ **DELIBERATELY NOT A `--maintenance` JOB, AND THIS SAYS SO SO NOBODY "FIXES"
IT.** MEASURED 2026-09-16: it takes **35 s** — it reads every tracked `.rs` file
(1,915 of them, ~34 MB) to resolve `current_truth` names. The maintenance lane is
65 s in total, so adding this would grow it by more than half for a check that
already runs in `pytest scripts/tests` beside its own arms. ⇒ Reachable by one
local command was the actual requirement; running in the FASTEST lane was not.
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


def stale_mood(items: list[dict]) -> list[str]:
    """Ledger prose stating an imperative about a campaign `queue.md` calls done.

    ⛔⛤ **THE LEDGER IS JSON, AND THE GUARD THAT CATCHES THIS SCANS `.md` ONLY.**
    MEASURED 2026-09-16: `PUB-ROOM-REPLACEMENT`'s CENSUS row already said *"a
    refusal drops the candidate roots AND the staged world; N is untouched"*
    while its LEDGER item still said *"failure can therefore occur after partial
    live replacement"* and asked *"A10 should publish all candidate room/session
    state with one authority switch"*. The two copies of one claim drifted APART
    and only the markdown half was ever swept.

    ⇒ The pattern is IMPORTED from `check_discharged_holds_are_rewritten` rather
    than restated, so widening it there widens it here. One authority for the
    grammar; two artifacts scanned.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import check_discharged_holds_are_rewritten as holds

    queue = REPO / "docs/planning/queue.md"
    if not queue.exists():
        raise SystemExit("⛔⛔ queue.md is not there; this rule has no authority")
    done = holds.rows_marked_done(queue)
    if not done:
        raise SystemExit(
            "⛔⛔ `queue.md` marks NO row done, so this rule can never fire. That "
            "is a claim about the parser, not about the ledger."
        )
    out = []
    for item in items:
        for field in ("current_truth", "consolidation_hypothesis"):
            text = str(item.get(field) or "")
            for _, name, line in holds.stale_gates(text + "\n", done):
                out.append(
                    f"    {item['id']}.{field}: names {name}, which `queue.md` "
                    f"marks finished\n      {line.strip()[:96]}"
                )
    return out


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
    claimed = checked = skipped = 0
    for item in items:
        rep = str(item.get("representation"))
        # ⭐ WIDENED 2026-09-16 from an exact "Resource" to a LEADING word, which
        # takes the population from 9 to 23: "Resource plus body-component
        # projection", "Component on SessionRoot", "Component string key" all
        # state a storage kind first and qualify it after. MEASURED: the 14 extra
        # items all AGREE with source, so this widening buys coverage rather than
        # findings — and it was poisoned against a real mismatch before being
        # believed, because a wider population is not a wider REACH.
        lead = (
            "Resource" if rep.startswith("Resource")
            else "Component" if rep.startswith("Component")
            else None
        )
        if lead is None:
            continue
        claimed += 1
        match = SUBJECT.match(item.get("current_truth", ""))
        kinds = declaration_kinds(by_file, match.group(1)) if match else []
        if not kinds:
            # ⚠ A subject this scan cannot resolve is a claim about the SCAN, not
            # a finding — `current_truth` often opens with an ordinary word
            # ("The", "Developer", "Player"), and the first CamelCase token is
            # then not a type at all. Counted and REPORTED rather than silently
            # dropped: "it ran and found nothing" and "it never ran" must be
            # different strings.
            #
            # ⛔⛤ **DO NOT "FIX" THE SKIP COUNT TO ZERO BY ADDING A
            # `subject_type` FIELD — I OPENED ALL EIGHT AND THEY HAVE NO SINGLE
            # SUBJECT.** Seven are the `EDIT-*` family plus `AUTH-CHECKPOINT-
            # RESTORE`, whose representation is literally *"Resources and, where
            # needed, component projection"* or *"Resources and operation keys"*:
            # they describe a PROTOCOL spanning an editable mirror, a proposal, an
            # admission and a live target. `AUTH-PLAYER-STATS` is *"Components
            # with editable mirror and sync snapshot"*. Naming one type for any of
            # them would invent a shape the item does not have, and the check
            # would then verify a fiction. ⇒ These are unresolvable BY NATURE, not
            # by parser weakness, and the printed count is the honest report.
            skipped += 1
            continue
        checked += 1
        if not any(lead in kind for kind in kinds):
            wrong.append(
                f"    {item['id']}: says `representation: {rep}`, but "
                f"{match.group(1)} is declared {kinds}"
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

    # RULE 5: the ledger's own prose, in the mood the markdown sweep reads.
    moody = stale_mood(items)
    if moody:
        print(
            f"⛔ {len(moody)} ledger field(s) state an imperative about a campaign "
            "this repository has finished:\n"
        )
        print("\n".join(moody))
        print(
            "\n⇒ REWRITE THE CLAIM AS CURRENT STATE. A `consolidation_hypothesis`\n"
            "  asking for work that is done reads as open work to the next census\n"
            "  update, which is the one reader this file has."
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
        f"{len(recorded)} workspace crate(s) match cargo exactly, "
        f"{checked} of {claimed} storage-kind claim(s) verified against source "
        f"({skipped} skipped: subject not a resolvable type), "
        f"({len(tracked)} tracked .rs files, {len(source)//1024} KiB)"
    )
    print(
        "⚠ this says nothing about whether the CLAIMS are still true — see the "
        "module docstring."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
