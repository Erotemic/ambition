"""The anchor guard must run in a lane, and must be able to FAIL."""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from scripts.checks import check_planning_anchors_resolve as guard  # noqa: E402


def test_every_planning_pointer_resolves_today():
    assert guard.main() == 0


def test_the_slug_matches_github_for_a_heading_with_an_emoji_and_a_dash():
    """⛔ THE EXACT SHAPE THAT BROKE. `— ✅ DONE` collapses to `--done`: the em

    dash and the emoji vanish, the spaces around them become hyphens. A guard
    that models this wrongly reports a false RED on every decorated heading, and
    a false red is obeyed faster than a false green is questioned.
    """
    assert (
        guard.slug("### A10 — candidate world / last-good-world publication — ✅ DONE")
        == "a10--candidate-world--last-good-world-publication---done"
    )


def test_a_plain_heading_slugs_the_obvious_way():
    assert guard.slug("## Priority table") == "priority-table"


def test_anchors_are_read_from_headings_only():
    """⚠ A line mentioning `#` mid-sentence is not a heading."""
    tmp = pathlib.Path(guard.PLANNING / "README.md")
    found = guard.anchors(tmp)
    assert found, "the planning README declares no headings; the reader is broken"
    assert all(" " not in a for a in found)


def test_the_reachability_rule_catches_orphans():
    assert guard.orphans() == []


def test_a_same_file_pointer_is_in_the_population():
    assert not guard.SAME_FILE_LINK.findall("see [it](other-page.md#a-heading)")
    assert guard.SAME_FILE_LINK.findall("see [it](#a-heading)") == ["a-heading"]


def test_a_small_valid_tree_with_no_anchor_links_passes(tmp_path, monkeypatch):
    planning = tmp_path / "docs/planning"
    recipes = tmp_path / "docs/recipes"
    planning.mkdir(parents=True)
    recipes.mkdir(parents=True)
    (planning / "README.md").write_text("# Planning\n", encoding="utf-8")
    (recipes / "index.md").write_text("# Recipes\n", encoding="utf-8")
    monkeypatch.setattr(guard, "ROOT", tmp_path)
    monkeypatch.setattr(guard, "PLANNING", planning)
    assert guard.main() == 0


def test_missing_target_and_broken_heading_fail_even_in_a_small_tree(tmp_path, monkeypatch):
    planning = tmp_path / "docs/planning"
    recipes = tmp_path / "docs/recipes"
    planning.mkdir(parents=True)
    recipes.mkdir(parents=True)
    (recipes / "index.md").write_text("# Recipes\n", encoding="utf-8")
    (planning / "README.md").write_text(
        "# Planning\n[Missing](missing.md#section)\n", encoding="utf-8"
    )
    monkeypatch.setattr(guard, "ROOT", tmp_path)
    monkeypatch.setattr(guard, "PLANNING", planning)
    assert guard.main() == 1

    (planning / "missing.md").write_text("# Other\n", encoding="utf-8")
    assert guard.main() == 1

    (planning / "missing.md").write_text("# Section\n", encoding="utf-8")
    assert guard.main() == 0


def test_missing_entry_point_fails(tmp_path, monkeypatch):
    planning = tmp_path / "docs/planning"
    planning.mkdir(parents=True)
    (planning / "README.md").write_text("# Planning\n", encoding="utf-8")
    monkeypatch.setattr(guard, "ROOT", tmp_path)
    monkeypatch.setattr(guard, "PLANNING", planning)
    assert guard.main() == 1


def test_orphan_page_is_detected_without_a_population_floor(tmp_path, monkeypatch):
    planning = tmp_path / "docs/planning"
    recipes = tmp_path / "docs/recipes"
    planning.mkdir(parents=True)
    recipes.mkdir(parents=True)
    (planning / "README.md").write_text("# Planning\n", encoding="utf-8")
    (recipes / "index.md").write_text("# Recipes\n", encoding="utf-8")
    (planning / "work.md").write_text("# Work\n", encoding="utf-8")
    monkeypatch.setattr(guard, "ROOT", tmp_path)
    monkeypatch.setattr(guard, "PLANNING", planning)
    assert guard.main() == 1
    (planning / "README.md").write_text("# Planning\n[Work](work.md)\n", encoding="utf-8")
    assert guard.main() == 0
