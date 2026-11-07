"""The planning control-plane check protects content, not line counts."""

from __future__ import annotations

from pathlib import Path

from scripts.checks import check_planning_docs_survive as guard


def test_the_live_control_plane_is_intact_today():
    assert guard.main() == 0


def test_every_named_document_has_required_sections_and_body():
    assert "docs/planning/queue.md" in guard.LIVE_CONTROL_PLANE
    assert "docs/planning/awaiting-maintainer-decision.md" in guard.LIVE_CONTROL_PLANE
    assert not guard.findings_for(guard.REPO, guard.LIVE_CONTROL_PLANE)


def test_a_short_but_complete_document_is_valid(tmp_path: Path):
    rel = "docs/planning/queue.md"
    path = tmp_path / rel
    path.parent.mkdir(parents=True)
    path.write_text("# The queue\nThere are no active tasks.\n", encoding="utf-8")
    assert guard.findings_for(tmp_path, {rel: ("# The queue",)}) == []


def test_missing_file_heading_or_body_is_reported(tmp_path: Path):
    rel = "docs/planning/status.md"
    named = {rel: ("# Planning status", "## Current execution")}
    path = tmp_path / rel
    path.parent.mkdir(parents=True)
    assert "missing" in " ".join(guard.findings_for(tmp_path, named))

    path.write_text("# Planning status\nA status.\n", encoding="utf-8")
    assert "heading" in " ".join(guard.findings_for(tmp_path, named))

    path.write_text("# Planning status\n## Current execution\n", encoding="utf-8")
    assert "body" in " ".join(guard.findings_for(tmp_path, named))

    path.write_text("# Planning status\n## Current execution\nNo active tasks.\n", encoding="utf-8")
    assert guard.findings_for(tmp_path, named) == []
