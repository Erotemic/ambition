#!/usr/bin/env python3
"""The consolidation ledger's STATUS fields must agree with each other.

⛔⛤ **THE DEFECT THIS EXISTS FOR, MEASURED 2026-09-17.** `architecture_census.py`
printed `suspect_duplicate_authority: 4` -- the number the campaign quotes for
"how many duplicate-authority families are still open" -- while
`architecture-census.md` declared THREE of those four closed in the same table
that listed them, and one of the three had been closed for a day. The ledger's
`metric_tags` was the machine-readable copy and the stale one, so a reader who
ran the instrument the README tells them to run got the wrong answer, and a
reader who read the page got the right one.

⇒ ONE FACT, ONE OWNER, ONE ROAD: the state of a duplicate-truth family is
`duplicate_authority_state` on its ledger item. The metric tag is carried by
exactly the `OPEN_PRESSURE` ones, the census page's State column must equal the
field, and the page's headline split is three numbers this check derives.

⛔⛤ **AND THE SECOND RULE IS THE SAME SHAPE ONE FIELD OVER.** `blocked_by` is a
HOLD in structured form, and `check_discharged_holds_are_rewritten` -- which
exists precisely to find a hold that outlived what it waited for -- scans bold
markers in markdown and cannot see a JSON list. MEASURED 2026-09-17: **15 of the
19 `blocked_by` entries named A10**, which `queue.md` marked ✅ DONE on
2026-09-16, and three more named a "peer-stable identity" campaign no page
defines by that name. Fourteen ledger items described themselves as held by a
door that was already open.

⇒ `blocked_by` means a LIVE gate and must name something resolvable: a `queue.md`
row id, or a `Q<number>` in `awaiting-maintainer-decision.md`. A discharged gate
moves to `was_blocked_by` as a receipt carrying its discharge and its date.

⛔⛤ **AND THE THIRD RULE IS THE SAME SHAPE AGAIN, ONE COLUMN OVER. MEASURED
2026-09-18: FOURTEEN OF 114 ROWS.** Every ledger item carries a `status` -- its
evidence class -- and so does the census page, in the last column of its
tables. The presence rule below had landed a day earlier and deliberately
declined to compare the two, because the page's columns differ between
sections. Three disagreements then turned up by hand, one at a time, while
doing other work; comparing all of them found eleven more.

⇒ All fourteen ran the same way: the page read `SOURCE_CONFIRMED` off a dated
measurement and the ledger still read `SOURCE_INFERRED`. Nobody downgrades a
row by accident -- the page is where a measurement gets WRITTEN, and the ledger
is the copy somebody has to remember separately. **THE PAGE IS THE OWNER**,
because the cell that says `SOURCE_CONFIRMED` sits beside the paragraph that
earned it, and the ledger's copy carries no evidence at all.

⚠ **WHAT THIS DOES NOT SAY.** That a gate lifted is not that the work behind it
happened, and a `RESOLVED` state is a claim a human made by reading source. This
check compares the ledger's copies of its own status to each other and to the
page that publishes them; it cannot re-read the tree. See
`check_consolidation_ledger_still_resolves.py`, which says the same about itself.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
LEDGER = REPO / "docs/planning/consolidation/consolidation-ledger.json"
CENSUS = REPO / "docs/planning/consolidation/architecture-census.md"
QUEUE = REPO / "docs/planning/queue.md"
QUESTIONS = REPO / "docs/planning/awaiting-maintainer-decision.md"

FAMILY = "duplicate_truth_family"
TAG = "suspect_duplicate_authority"
OPEN = "OPEN_PRESSURE"

#: The page's headline split, derived rather than counted by hand. Exactly one
#: line may state it, because two lines stating it is the defect above.
SPLIT = re.compile(r"(\d+) open, (\d+) resolved, (\d+) legitimate\s+separation")
QUESTION_ID = re.compile(r"^## (Q\d+)\b")
#: A receipt has to say it is spent, and when.
DISCHARGED = re.compile(r"DISCHARGED \d{4}-\d{2}-\d{2}")


def census_state_cells(text: str) -> dict[str, str]:
    """The State column of `architecture-census.md`'s duplicate-family table."""
    cells = {}
    for line in text.split("\n"):
        if not line.startswith("| DUP-"):
            continue
        columns = line.split("|")
        cells[columns[1].strip()] = columns[3].strip()
    return cells


#: A row's evidence class, as the census page's last column spells it.
STATUSES = (
    "SOURCE_CONFIRMED",
    "SOURCE_INFERRED",
    "DOC_CLAIM",
    "NEEDS_COMPILED_VERIFICATION",
    "NEEDS_RUNTIME_VERIFICATION",
)
#: A cell may carry its token and then explain itself, exactly as the State
#: column may: `SOURCE_CONFIRMED (the population and the axis; the per-site
#: classification is not re-derived)` is legal, a bare second spelling is not.
LEADING_STATUS = re.compile(r"(" + "|".join(STATUSES) + r")\b")
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)


def census_status_cells(text: str) -> dict[str, set[str]]:
    """Each id's evidence class as the census page's own tables publish it.

    \u26d4 ANCHORED ON THE FIRST CELL, WHICH IS THE WHOLE DIFFICULTY. The obvious
    rule -- a line naming the id, with a status token somewhere on it -- was
    measured against the shipped page and is wrong three ways: 16 lines are
    PROSE that mention an id, 10 table rows name a second id inside their
    evidence (`ORDER-MECHANICAL-EDIT` appears in three rows that are not its
    own), and an evidence cell may narrate an old token (*"this row was
    SOURCE_INFERRED because..."*). A row's subject is its first cell, nothing
    else.

    \u26a0 A ROW WITH NO STATUS COLUMN IS NOT A ROW THAT DISAGREES. Two whole
    tables -- the `AUTH-*` authority map and the `EDIT-*` editor surfaces --
    end on a different column. They are skipped rather than guessed at, which
    is why the caller floors the comparison count: skipping everything is
    indistinguishable from a healthy page whose parse has rotted.
    """
    cells: dict[str, set[str]] = {}
    for line in text.split("\n"):
        if not line.lstrip().startswith("|"):
            continue
        columns = COMMENT.sub("", line).split("|")
        if len(columns) < 3:
            continue
        ident = columns[1].strip().strip("*").strip("`").strip()
        tail = [c for c in columns[2:] if c.strip()]
        if not tail:
            continue
        found = LEADING_STATUS.match(tail[-1].strip())
        if found:
            cells.setdefault(ident, set()).add(found.group(1))
    return cells


def check(ledger: dict, census: str, done: set[str], rows: set[str], questions: set[str]) -> list[str]:
    """Every disagreement between the ledger's status copies. Empty is green."""
    bad: list[str] = []
    states = ledger.get("duplicate_authority_states") or {}
    if len(states) < 2:
        bad.append(
            "⛔⛔ the ledger declares fewer than two `duplicate_authority_states`, so "
            "the vocabulary this check reads is not there"
        )
        return bad
    families = {}
    for item in ledger["items"]:
        ident = item["id"]
        tagged = TAG in item.get("metric_tags", [])
        if item.get("category") != FAMILY:
            if tagged:
                bad.append(f"{ident}: carries `{TAG}` but is not a {FAMILY}")
            continue
        state = item.get("duplicate_authority_state")
        families[ident] = state
        if state not in states:
            bad.append(f"{ident}: `duplicate_authority_state` {state!r} is not in the vocabulary")
            continue
        if tagged != (state == OPEN):
            bad.append(
                f"{ident}: state is {state} and the `{TAG}` tag is "
                f"{'present' if tagged else 'absent'} — the tag is carried by exactly "
                f"the {OPEN} families, so the campaign's count disagrees with its own row"
            )

    # ⛔⛤ **EVERY LEDGER ROW MUST BE READABLE ON THE PAGE.** The state check
    # below covers only the duplicate-authority family rows — 8 of 114 — so a
    # row of any other category could go stale, contradict the page, or lose
    # its page row entirely without anything noticing. Found 2026-09-18 by
    # `CAP-OPTIONAL-RES-CENSUS`, where the page cell said *"CLASSIFIED — THE
    # AXIS WAS WRONG"* while the ledger still said `SOURCE_INFERRED` /
    # `NEEDS_SEMANTIC_REVIEW`, and by `TEST-TWO-APP`, whose `DOC_CLAIM` said a
    # witness had not been run while four arms of it sat passing in the tree.
    #
    # ⚠ PRESENCE IS THE HALF THAT NEEDS NO PARSE: a ledger row the page never
    # names is a row no reader can reach, whatever it says. The STATUS
    # comparison below is the other half, and was left undone for a day
    # because the page's columns differ between sections -- see
    # `census_status_cells` for what that cost and how it is anchored.
    # ⛔⛤ WORD-BOUNDED, AND THE FIRST DRAFT WAS `item["id"] not in census` —
    # a substring test, whose poison PASSED. Renaming the page's row to
    # `CRATE-BODY-SEED-POISONED` leaves `CRATE-BODY-SEED` inside it, so the
    # check saw the id it was looking for in a row that no longer names it. A
    # containment test between two identifiers is almost never the test you
    # want; these ids share prefixes by construction (`CRATE-*`, `ORDER-*`,
    # `DUP-*`), so the failure is not hypothetical.
    missing_on_page = sorted(
        item["id"]
        for item in ledger["items"]
        if isinstance(item.get("id"), str)
        and not re.search(rf"(?<![A-Za-z0-9-]){re.escape(item['id'])}(?![A-Za-z0-9-])", census)
    )
    if missing_on_page:
        bad.append(
            f"{len(missing_on_page)} ledger row(s) are named nowhere on the census "
            f"page, so nothing a reader can reach carries them: {missing_on_page}"
        )

    # ── the evidence class, which the page and the ledger both publish
    # ⛔⛤ **THE DEFECT THIS RULE EXISTS FOR, MEASURED 2026-09-18: FOURTEEN OF
    # 114 ROWS.** The rule above had just caught three ids whose page row and
    # ledger row disagreed (`CAP-OPTIONAL-RES-CENSUS`, `TEST-TWO-APP`,
    # `ROAD-DYNAMIC-SPAWN`), each found by hand while doing something else.
    # Comparing all of them found eleven more, every one the same way round:
    # the page read `SOURCE_CONFIRMED` off a dated measurement and the ledger
    # still read `SOURCE_INFERRED`.
    #
    # ⇒ That direction is the tell. Nobody downgrades a row by accident; the
    # page is where a measurement gets WRITTEN and the ledger is the copy that
    # has to be remembered separately. Two owners of one fact, and the fact
    # moves on only one of them.
    #
    # ⚠ THE PAGE WINS, AND THAT IS A CHOICE THIS CHECK CANNOT JUSTIFY BY
    # ITSELF. It is where the evidence sits -- the cell that says
    # `SOURCE_CONFIRMED` sits beside the paragraph that measured it, so a
    # reviewer can see whether the token is earned. The ledger's copy carries
    # no evidence at all, which is exactly why it is the one that drifts.
    page_status = census_status_cells(census)
    for item in ledger["items"]:
        ident, status = item.get("id"), item.get("status")
        if not isinstance(ident, str) or status not in STATUSES:
            continue
        published = page_status.get(ident)
        if published and status not in published:
            bad.append(
                f"`{ident}` is {status} in the ledger and "
                f"{'/'.join(sorted(published))} on the census page, which is where "
                "its evidence is written"
            )

    # ── the page that publishes the states
    cells = census_state_cells(census)
    if len(cells) != len(families):
        bad.append(
            f"the census table has {len(cells)} duplicate-family rows and the ledger has "
            f"{len(families)}: {sorted(set(cells) ^ set(families))}"
        )
    for ident, cell in cells.items():
        state = families.get(ident)
        if state is None:
            bad.append(f"{ident}: the census table has a row the ledger does not")
            continue
        # The token alone, or the token then a human clause. Anything else is a
        # second spelling of the state.
        if cell != state and not cell.startswith(f"{state} — "):
            bad.append(
                f"{ident}: the census State column says {cell!r} and the ledger says "
                f"{state!r}"
            )

    counted = {
        "open": sum(1 for s in families.values() if s == OPEN),
        "resolved": sum(1 for s in families.values() if s == "RESOLVED"),
        "legitimate": sum(1 for s in families.values() if s == "LEGITIMATE_SEPARATION"),
    }
    split = SPLIT.findall(census)
    if len(split) != 1:
        bad.append(
            f"⛔ the census page states the derived split {len(split)} times; it must "
            "state it exactly once, in the form `N open, N resolved, N legitimate "
            "separation`"
        )
    else:
        stated = tuple(int(n) for n in split[0])
        if stated != (counted["open"], counted["resolved"], counted["legitimate"]):
            bad.append(
                f"the census page says {stated} open/resolved/legitimate and the ledger "
                f"says {(counted['open'], counted['resolved'], counted['legitimate'])}"
            )

    # ── holds
    for item in ledger["items"]:
        ident = item["id"]
        for hold in item.get("blocked_by", []):
            if QUESTION_ID.match(f"## {hold}"):
                if hold not in questions:
                    bad.append(
                        f"{ident}: held on {hold}, which `awaiting-maintainer-decision.md` "
                        "does not ask"
                    )
            elif hold not in rows:
                bad.append(
                    f"{ident}: held on {hold!r}, which is neither a `queue.md` row id nor "
                    "a `Q<number>` — a hold that names nothing cannot be discharged"
                )
            elif hold in done:
                bad.append(
                    f"{ident}: held on {hold}, which `queue.md` marks done. A discharged "
                    "gate belongs in `was_blocked_by` with what discharged it"
                )
        for receipt in item.get("was_blocked_by", []):
            if not DISCHARGED.search(receipt):
                bad.append(
                    f"{ident}: a `was_blocked_by` receipt does not say `DISCHARGED "
                    f"<date>`, so it reads like a live gate: {receipt[:60]!r}"
                )
    return bad


# ⛔⛔ THE KNOWN-ANSWER CONTROL, RUN ON EVERY INVOCATION. Every rule here reports
# by staying silent, and a silent rule whose parse has rotted looks exactly like a
# healthy ledger. Each control is the defect this check was written for, so if one
# stops firing the check refuses to report a verdict instead of reporting a
# comforting one.
CONTROL_STATES = {OPEN: "open", "RESOLVED": "gone", "LEGITIMATE_SEPARATION": "fine"}
CONTROL_CLEAN = {
    "duplicate_authority_states": CONTROL_STATES,
    "items": [
        {"id": "DUP-A", "category": FAMILY, "duplicate_authority_state": OPEN,
         "metric_tags": [TAG], "blocked_by": ["Q144"]},
        {"id": "DUP-B", "category": FAMILY, "duplicate_authority_state": "RESOLVED",
         "metric_tags": [], "was_blocked_by": ["A10 — DISCHARGED 2026-09-16"]},
        # Not a family, so it moves no split and carries no tag; it is here to
        # give the STATUS rule something to compare.
        {"id": "STAT-A", "category": "authority", "status": "SOURCE_CONFIRMED",
         "metric_tags": []},
    ],
}
CONTROL_CENSUS = (
    "text: 1 open, 1 resolved, 0 legitimate separation\n"
    f"| DUP-A | family | {OPEN} — with a clause |\n"
    "| DUP-B | family | RESOLVED |\n"
    # ⚠ THE EVIDENCE CELL NARRATES THE OLD TOKEN ON PURPOSE. Shipped rows say
    # things like *"this row was SOURCE_INFERRED because it named no entry
    # point"* beside a column that now reads SOURCE_CONFIRMED, so a rule that
    # looks for a token anywhere on the line reddens a correct page.
    "| STAT-A | a row | it was SOURCE_INFERRED until it was measured | SOURCE_CONFIRMED |\n"
)
#: ⚠ SET FROM A MEASUREMENT, NOT A GUESS -- 108 rows compared on 2026-09-18,
#: floored well below that so ordinary row churn does not trip it and a parse
#: that has stopped finding the column does. The previous floor in this family
#: of checks was guessed at 2000 against 1,917 files and reddened on its author.
STATUS_FLOOR = 95

CONTROL_DONE = {"A10"}
CONTROL_ROWS = {"A10", "ID-PEER"}
CONTROL_QUESTIONS = {"Q144"}


def self_check() -> None:
    def run(ledger, census=CONTROL_CENSUS):
        return check(
            json.loads(json.dumps(ledger)), census, CONTROL_DONE, CONTROL_ROWS,
            CONTROL_QUESTIONS,
        )

    if run(CONTROL_CLEAN):
        raise SystemExit(
            "⛔⛔ THE CONTROL LEDGER, WHICH IS COHERENT, WAS REPORTED AS BROKEN. This "
            f"check would redden a correct ledger: {run(CONTROL_CLEAN)}"
        )
    poisons = {
        "a resolved family that kept the tag": lambda l: l["items"][1]["metric_tags"].append(TAG),
        "an open family that lost the tag": lambda l: l["items"][0]["metric_tags"].clear(),
        "a state outside the vocabulary": lambda l: l["items"][0].update(
            {"duplicate_authority_state": "PROBABLY_FINE"}
        ),
        "a hold on a finished campaign": lambda l: l["items"][0].update({"blocked_by": ["A10"]}),
        "a hold that names nothing": lambda l: l["items"][0].update({"blocked_by": ["vibes"]}),
        "an unasked question": lambda l: l["items"][0].update({"blocked_by": ["Q999"]}),
        "a receipt with no discharge": lambda l: l["items"][1].update(
            {"was_blocked_by": ["A10 candidate-world publication"]}
        ),
        "a status the page contradicts": lambda l: l["items"][2].update(
            {"status": "SOURCE_INFERRED"}
        ),
        "the tag on a non-family item": lambda l: l["items"].append(
            {"id": "AUTH-X", "category": "authority", "metric_tags": [TAG]}
        ),
    }
    for name, poison in poisons.items():
        ledger = json.loads(json.dumps(CONTROL_CLEAN))
        poison(ledger)
        if not run(ledger):
            raise SystemExit(
                f"⛔⛔ THE CONTROL FOR {name!r} DID NOT FIRE, so this check is blind to "
                "the defect it was written for and a clean run means nothing."
            )
    for census, why in (
        ("1 open, 1 resolved, 0 legitimate separation\n", "a table with no rows"),
        (CONTROL_CENSUS.replace("1 resolved", "3 resolved"), "a stated count that disagrees"),
        (CONTROL_CENSUS.replace(f"{OPEN} — with a clause", "RESOLVED"), "a column that disagrees"),
        (CONTROL_CENSUS + "and again: 1 open, 1 resolved, 0 legitimate separation\n",
         "the split stated twice"),
        (CONTROL_CENSUS.replace("| SOURCE_CONFIRMED |", "| DOC_CLAIM |"),
         "a page status the ledger contradicts"),
    ):
        if not run(CONTROL_CLEAN, census):
            raise SystemExit(
                f"⛔⛔ THE CONTROL FOR {why!r} DID NOT FIRE; the census page's copy of the "
                "states is unguarded."
            )


def main() -> int:
    self_check()
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    import check_discharged_holds_are_rewritten as holds

    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    census = CENSUS.read_text(encoding="utf-8")
    # ⚠ IMPORTED, NOT RESPELLED. `queue.md`'s row headers are the authority on
    # which campaigns are finished, and that predicate has been respelled once
    # per guard before now; widening it there widens it here.
    done = holds.rows_marked_done(QUEUE)
    rows = {
        m.group(1)
        for m in (holds.QUEUE_ROW.match(l) for l in QUEUE.read_text(encoding="utf-8").split("\n"))
        if m
    }
    questions = {
        m.group(1)
        for m in (QUESTION_ID.match(l) for l in QUESTIONS.read_text(encoding="utf-8").split("\n"))
        if m
    }
    # ⛔ ANTI-VACUITY, BECAUSE EVERY RULE BELOW REPORTS BY SAYING NOTHING.
    if not done or done == rows:
        raise SystemExit(
            f"⛔⛔ `queue.md` parsed as {len(rows)} rows of which {len(done)} are done; "
            "with no finished rows, or with every row finished, the hold rule cannot fire."
        )
    if len(questions) < 10:
        raise SystemExit(
            f"⛔⛔ only {len(questions)} `Q` ids parsed out of "
            "`awaiting-maintainer-decision.md`; that is a claim about the parser."
        )
    # ⛔ THE SAME, FOR THE STATUS RULE, WHICH SKIPS ANY ROW WHOSE LAST COLUMN
    # IS NOT A STATUS -- so a renamed column, a reordered table or a stray
    # trailing cell turns the whole rule into a no-op that reports success.
    # MEASURED 2026-09-18: 108 of the ledger's 114 status-carrying rows are
    # compared; the other six sit in the `AUTH-*` and `EDIT-*` tables, which
    # end on a different column and publish no evidence class at all.
    compared = len(set(census_status_cells(census)) & {
        i["id"] for i in ledger["items"]
        if isinstance(i.get("id"), str) and i.get("status") in STATUSES
    })
    if compared < STATUS_FLOOR:
        raise SystemExit(
            f"⛔⛔ only {compared} census rows could be compared against the ledger's "
            f"status, below the floor of {STATUS_FLOOR}. The page's tables parsed as "
            "something this check does not recognise, and a rule that compares "
            "nothing passes everything."
        )
    bad = check(ledger, census, done, rows, questions)
    if bad:
        print("the consolidation ledger's status fields disagree:")
        for line in bad:
            print(f"  {line}")
        return 1
    families = [i for i in ledger["items"] if i.get("category") == FAMILY]
    split = {
        s: sum(1 for i in families if i["duplicate_authority_state"] == s)
        for s in ledger["duplicate_authority_states"]
    }
    live = sorted({h for i in ledger["items"] for h in i.get("blocked_by", [])})
    spent = sum(len(i.get("was_blocked_by", [])) for i in ledger["items"])
    print(
        f"ok: {len(families)} duplicate-truth families, {split}, the `{TAG}` tag on "
        f"exactly the {OPEN} ones, and the census page agrees"
    )
    print(f"  live holds: {', '.join(live) or 'none'} ({spent} discharged receipts)")
    print(
        f"  evidence class: {compared} of {len(ledger['items'])} rows compared against "
        "the census page's own column"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
