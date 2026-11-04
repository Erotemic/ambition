"""Exercise ledger path, definition, and storage-kind checks."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from scripts.checks import check_consolidation_ledger_still_resolves as guard  # noqa: E402


def test_the_real_ledger_resolves() -> None:
    """The shipped corpus, which is the only case that can rot on its own."""
    assert guard.main() == 0


def test_a_cited_path_that_does_not_exist_is_found(monkeypatch, tmp_path, capsys) -> None:
    ledger = {
        "items": [
            {
                "id": "PLANTED",
                "current_truth": "",
                "source_paths": ["crates/does_not_exist/src/lib.rs"],
            }
        ]
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    assert "no longer exist" in capsys.readouterr().out


def test_a_name_with_no_definition_is_found(monkeypatch, tmp_path, capsys) -> None:
    ledger = {
        "items": [
            {
                "id": "PLANTED",
                "current_truth": "TheVanishedOwner replaced it.",
                "source_paths": [],
            }
        ]
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "TheVanishedOwner" in out and "PLANTED" in out


def test_a_name_owned_by_another_crate_is_declined(monkeypatch, tmp_path) -> None:
    """⛔ THE DECLINE THAT MATTERS. `ResMut` is Bevy's; a guard reading only this
    workspace can never find its definition, and reporting it is noise."""
    ledger = {
        "items": [
            {"id": "PLANTED", "current_truth": "ResMut and TypeId are used here.", "source_paths": []}
        ],
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 0


def test_an_empty_ledger_is_refused(monkeypatch, tmp_path, capsys) -> None:
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps({"items": []}), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    assert guard.main() == 1
    assert "empty corpus" in capsys.readouterr().out


def test_an_item_claiming_a_resource_names_a_resource():
    """⛔ THE EXPENSIVE FAILURE, ONE CHECKABLE SLICE OF IT.

    This guard's docstring says a `current_truth` can go stale while every path
    exists and every type compiles. That is true, and one narrow slice of it IS
    mechanical: an item whose `representation` is exactly "Resource" must name a
    type that derives `Resource`. MEASURED 2026-09-16: eight of nine were right,
    and `AUTH-CONTENT-BINDING` — marked SOURCE_CONFIRMED — recorded
    `ActiveContentBinding` as a Resource when it is a Component.
    """
    items = json.loads(guard.LEDGER.read_text(encoding="utf-8"))["items"]
    claiming = [i for i in items if str(i.get("representation")) == "Resource"]
    assert len(claiming) >= 3, "too few items claim a Resource for this to witness"


def test_the_declaration_reader_tells_a_resource_from_a_component():
    """⛔ THE CONTROL. If this returned the same answer for both, the rule could

    never disagree with anything.
    """
    source = {
        "a.rs": "#[derive(Resource, Clone)]\npub struct AlphaThing {}\n",
        "b.rs": "#[derive(bevy::prelude::Component)]\npub struct BetaThing(u8);\n",
        "c.rs": "pub struct GammaThing;\n",
    }
    assert guard.declaration_kinds(source, "AlphaThing") == ["Resource"]
    assert guard.declaration_kinds(source, "BetaThing") == ["Component"]
    assert guard.declaration_kinds(source, "GammaThing") == ["neither"]
    # ⚠ And a type it cannot find returns NOTHING, which the rule treats as a
    # claim about the scan rather than a finding.
    assert guard.declaration_kinds(source, "DeltaThing") == []


def test_the_ledgers_own_prose_is_swept_for_a_stale_imperative():
    """⛔ THE LEDGER IS JSON AND THE MARKDOWN SWEEP NEVER SAW IT.

    `PUB-ROOM-REPLACEMENT`'s CENSUS row already said a refusal leaves N
    untouched while its LEDGER item still said failure could occur after partial
    live replacement. Two copies of one claim, drifted apart, one of them swept.
    """
    items = json.loads(guard.LEDGER.read_text(encoding="utf-8"))["items"]
    assert guard.stale_mood(items) == []


def test_that_sweep_imports_its_pattern_rather_than_restating_it():
    """⭐ ONE AUTHORITY FOR THE GRAMMAR, TWO ARTIFACTS SCANNED. If this module

    grew its own copy of the mood regex, widening one would silently leave the
    other behind — which is the defect the rule itself exists to catch.
    """
    from scripts.checks import check_discharged_holds_are_rewritten as holds

    planted = [
        {
            "id": "PLANTED",
            "consolidation_hypothesis": "A10 should make this one publication.",
        }
    ]
    assert holds.rows_marked_done(guard.REPO / "docs/planning/queue.md")
    assert guard.stale_mood(planted), "the imported pattern no longer fires"


def _planted(tmp_path, monkeypatch, truth: str):
    """Create a ledger for one semantic claim."""
    ledger = {
        "items": [{"id": "PLANTED", "current_truth": truth, "source_paths": []}],
    }
    planted = tmp_path / "ledger.json"
    planted.write_text(json.dumps(ledger), encoding="utf-8")
    monkeypatch.setattr(guard, "LEDGER", planted)
    return planted


def test_a_macro_generated_type_resolves(monkeypatch, tmp_path) -> None:
    """⛔⛤ `LoadId` SAT IN `EXTERNAL` AND THE PREMISE WAS FALSE, MEASURED 2026-09-18.

    `EXTERNAL` means "owned by Bevy, std or a crate outside this workspace".
    `ambition_load` is a workspace crate and `LoadId` is one of its types; no
    definition was findable because the scan reads SOURCE, not expansions, and
    the declaration is `pub struct $name(String)` inside `string_id!`.

    ⇒ The exemption suppressed the right name for the wrong reason, which is
    the failure mode an exemption list has: it looks identical to the case it
    claims to be. One macro generates ELEVEN types across three crates, and
    only this one happened to be cited.
    """
    _planted(tmp_path, monkeypatch, "LoadId correlates the load transaction.")
    assert guard.main() == 0


def test_a_generated_name_is_not_resolved_by_the_macro_being_present(monkeypatch, tmp_path) -> None:
    # ⚠ THE CONTROL. `string_id!` appears in the corpus, so a rule that merely
    # noticed the macro would green every name. It is the INVOCATION that
    # defines, and a type the macro never generated must still be unresolved.
    _planted(tmp_path, monkeypatch, "NeverGeneratedId correlates nothing.")
    assert guard.main() == 1


def test_a_name_recorded_as_deleted_is_declined(monkeypatch, tmp_path) -> None:
    """A row may RECORD a road that is gone; `architecture-census.md` spells the
    same escape as a `cite-ok` comment."""
    assert "SpawnPlayerCloneRequest" in guard.DELETED
    _planted(tmp_path, monkeypatch, "SpawnPlayerCloneRequest was the third seam.")
    assert guard.main() == 0


def test_a_deleted_name_that_came_back_is_a_finding(monkeypatch, tmp_path, capsys) -> None:
    """⛔ THE EXEMPTION, CHECKED IN THE DIRECTION THAT CAN SURPRISE YOU.

    A suppression list is honest only while its premise holds, and this one's
    premise is "the definition is gone". If the type returns, every row citing
    it is describing something live again — and a list that only ever
    suppresses is exactly what would hide that.
    """
    monkeypatch.setitem(guard.DELETED, "ProjectileSpawnRequest", "ffffffff")
    _planted(tmp_path, monkeypatch, "Nothing is cited here.")
    assert guard.main() == 1
    assert "ProjectileSpawnRequest" in capsys.readouterr().out


def test_the_external_set_names_nothing_this_workspace_defines() -> None:
    """⛔ THE RULE THAT WOULD HAVE CAUGHT `LoadId`, NOW STANDING.

    An `EXTERNAL` entry asserts the name is foreign. A workspace definition of
    it — spelled out, or generated by a macro in `GENERATOR` — contradicts that,
    and the entry belongs somewhere else.
    """
    source = "\n".join(guard.workspace_source()[1])
    generated = set(guard.GENERATOR.findall(source))
    for name in guard.EXTERNAL:
        assert name not in generated, f"{name} is generated by a workspace macro"
        assert not re.search(rf"\b(struct|enum|trait|type|fn)\s+{name}\b", source), (
            f"{name} is declared in this workspace and is not EXTERNAL"
        )
