"""The status-agreement check has to FIRE, on the data it ships against.

⛔ Every rule in that module reports by staying SILENT, so a rotted parse and a
coherent ledger produce the same output. The module runs its own controls on
every invocation; these arms pin them from outside, so a future edit that
loosens a rule reddens here rather than going quiet.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import check_consolidation_ledger_states_are_live as mod  # noqa: E402


def run(mutate=None, census: str | None = None) -> list[str]:
    ledger = json.loads(json.dumps(mod.CONTROL_CLEAN))
    if mutate is not None:
        mutate(ledger)
    return mod.check(
        ledger,
        mod.CONTROL_CENSUS if census is None else census,
        mod.CONTROL_DONE,
        mod.CONTROL_ROWS,
        mod.CONTROL_QUESTIONS,
    )


def test_a_coherent_ledger_is_green():
    # ⛔ THE CONTROL THAT MATTERS MOST. A check that flags everything also
    # "catches" every poison below, and would be unusable in a lane.
    assert run() == []


def test_a_resolved_family_that_kept_the_tag_is_caught():
    # The shipped defect, exactly: three families declared closed in prose while
    # `metric_tags` still made the campaign's headline count say four.
    found = run(lambda l: l["items"][1]["metric_tags"].append(mod.TAG))
    assert any("DUP-B" in line and mod.TAG in line for line in found), found


def test_an_open_family_that_dropped_the_tag_is_caught():
    # The same disagreement the other way round, which would UNDERCOUNT the
    # campaign -- the direction nobody notices.
    assert run(lambda l: l["items"][0]["metric_tags"].clear())


def test_a_state_outside_the_declared_vocabulary_is_caught():
    assert run(lambda l: l["items"][0].update({"duplicate_authority_state": "PROBABLY_FINE"}))


def test_the_page_may_not_disagree_with_the_ledger_it_publishes():
    census = mod.CONTROL_CENSUS.replace(f"{mod.OPEN} — with a clause", "RESOLVED")
    assert run(census=census)


def test_a_state_token_may_carry_a_human_clause():
    # ⚠ THE COST OF THE RULE ABOVE, PINNED. The column has to stay readable, so
    # `OPEN_PRESSURE — known transitional layer` is legal and a bare second
    # spelling is not. Without this arm the obvious tightening is "cell == state",
    # which would strip the page of every explanation it has.
    census = mod.CONTROL_CENSUS.replace(
        f"{mod.OPEN} — with a clause", f"{mod.OPEN} — ruling asked as `Q144`"
    )
    assert run(census=census) == []


def test_a_derived_split_stated_once_and_wrongly_is_caught():
    assert run(census=mod.CONTROL_CENSUS.replace("1 resolved", "3 resolved"))


def test_a_derived_split_stated_twice_is_caught():
    # ⛔ TWO COPIES OF THE DERIVED NUMBER IS THE DEFECT ITSELF, so agreeing with
    # the ledger twice is not good enough.
    assert run(census=mod.CONTROL_CENSUS + "again: 1 open, 1 resolved, 0 legitimate separation\n")


def test_a_hold_on_a_finished_campaign_is_caught():
    assert run(lambda l: l["items"][0].update({"blocked_by": ["A10"]}))


def test_a_hold_that_names_nothing_is_caught():
    # `peer-stable identity correction` was real and named no page; a hold nobody
    # can resolve can never be discharged.
    assert run(lambda l: l["items"][0].update({"blocked_by": ["vibes"]}))


def test_a_receipt_that_does_not_say_it_is_spent_is_caught():
    assert run(lambda l: l["items"][1].update({"was_blocked_by": ["A10 candidate publication"]}))


def test_the_tag_outside_the_family_category_is_caught():
    # The count is over one category; a tag anywhere else inflates it silently.
    assert run(
        lambda l: l["items"].append({"id": "AUTH-X", "category": "authority", "metric_tags": [mod.TAG]})
    )


def test_the_modules_own_controls_pass():
    mod.self_check()


def test_the_shipped_ledger_and_page_agree():
    # ⭐ THE ARM THAT MAKES THIS A RATCHET RATHER THAN A UNIT TEST: it runs
    # against the real ledger, the real census page and the real `queue.md`.
    assert mod.main() == 0


def test_a_ledger_row_named_nowhere_on_the_page_is_a_failure():
    """⛔⛤ THE FIRST DRAFT OF THIS CHECK WAS A SUBSTRING TEST AND ITS POISON PASSED.

    `item["id"] not in census` sees `CRATE-BODY-SEED` inside a page row renamed
    to `CRATE-BODY-SEED-POISONED`, so a row that no longer names the id read as
    present. These ids share prefixes by construction — `CRATE-*`, `ORDER-*`,
    `DUP-*` — so the failure is not hypothetical.

    Both directions are pinned: a SUPERSTRING must not satisfy the check, and a
    real removal must fail it. The first is the one that was broken.
    """
    victim = next(
        item["id"] for item in mod.CONTROL_CLEAN["items"] if item.get("id")
    )

    renamed = mod.CONTROL_CENSUS.replace(victim, f"{victim}-POISONED")
    assert [m for m in run(census=renamed) if "named nowhere" in m], (
        f"a page row renamed to a SUPERSTRING of {victim} still satisfied the check"
    )

    removed = mod.CONTROL_CENSUS.replace(victim, "XX-GONE")
    assert [m for m in run(census=removed) if "named nowhere" in m], (
        f"removing {victim} from the page did not fail the check"
    )


def test_a_ledger_status_the_page_contradicts_is_caught():
    """⛔⛤ FOURTEEN OF 114 SHIPPED ROWS DISAGREED, MEASURED 2026-09-18.

    The presence rule above landed a day earlier and explicitly declined to
    compare the STATUS token, because the page's tables differ between
    sections. Three disagreements were then found by hand, one at a time,
    while doing unrelated work; comparing all of them found eleven more.

    Every one was the same way round -- page `SOURCE_CONFIRMED`, ledger
    `SOURCE_INFERRED` -- which is the tell. Nobody downgrades a row by
    accident. The page is where a measurement gets written and the ledger is
    the copy that has to be remembered separately.
    """
    found = run(lambda l: l["items"][2].update({"status": "SOURCE_INFERRED"}))
    assert any("STAT-A" in line and "SOURCE_CONFIRMED" in line for line in found), found


def test_an_evidence_cell_that_narrates_an_old_token_is_not_a_disagreement():
    """⚠ THE COST OF THE RULE ABOVE, AND THE REASON IT IS ANCHORED.

    The obvious implementation -- a line naming the id with a status token
    somewhere on it -- reddens a correct page, because shipped evidence cells
    say things like *"this row was SOURCE_INFERRED because it named no entry
    point"* beside a column that now reads `SOURCE_CONFIRMED`. Only the last
    cell is the claim; the rest is prose about it.
    """
    assert run() == []
    census = mod.CONTROL_CENSUS.replace(
        "it was SOURCE_INFERRED until it was measured",
        "DOC_CLAIM and NEEDS_RUNTIME_VERIFICATION were both considered",
    )
    assert run(census=census) == [], "prose mentioning other tokens read as the row's status"


def test_prose_naming_an_id_is_not_a_row_that_can_disagree():
    # 16 shipped lines mention an id in running prose. A bullet that says
    # `- `STAT-A` is DOC_CLAIM for now` is commentary, not the table.
    census = mod.CONTROL_CENSUS + "- `STAT-A` was DOC_CLAIM before anyone measured it\n"
    assert run(census=census) == []


def test_a_second_id_inside_another_rows_evidence_does_not_leak():
    # `ORDER-MECHANICAL-EDIT` is named in three shipped rows that are not its
    # own. A row's subject is its first cell and nothing else.
    census = mod.CONTROL_CENSUS + (
        "| OTHER-ROW | see also STAT-A, which differs | NEEDS_RUNTIME_VERIFICATION |\n"
    )
    assert run(census=census) == []


def test_a_row_whose_last_column_is_not_a_status_is_skipped_not_guessed():
    # Two whole shipped tables (`AUTH-*`, `EDIT-*`) end on a different column.
    # Skipping them is correct; guessing at them would invent 27 findings.
    census = mod.CONTROL_CENSUS.replace("| SOURCE_CONFIRMED |", "| combat, HUD, abilities |")
    assert run(census=census) == []


def test_the_status_cells_parser_finds_the_shipped_page():
    """⛔ THE ANTI-VACUITY ARM. Skipping a row and failing to parse one are the
    same silence, so the count is pinned from outside as well as floored inside.
    """
    cells = mod.census_status_cells(mod.CENSUS.read_text(encoding="utf-8"))
    assert len(cells) >= mod.STATUS_FLOOR, (
        f"only {len(cells)} census rows parsed as carrying an evidence class"
    )
    assert all(v for v in cells.values())
