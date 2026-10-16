"""The variant pass builds on the job cap's processes, the same files.

After a full re-render it was 554 s on one process, and two tier renders
(`alice`, `goblin`) were 220 s of it: 98 s on six processes, with each
sheet's tier render a task of its own (measured 2026-10-15)."""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "lib"))

pytest.importorskip("PIL")
pytest.importorskip("yaml")

from PIL import Image  # noqa: E402

import generate_visual_quality_variants as g  # noqa: E402


def test_the_tasks_run_in_worker_processes(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.setattr(g, "_variant_task", lambda task, *rest: {"pid": (os.getpid(), 0, 0)})
    tasks = [("loose", index, ()) for index in range(4)]
    pids = [counts["pid"][0] for _, counts in g._run_tasks(tasks, tmp_path, tmp_path, False, 2, g.PhaseClock())]
    assert len(pids) == 4 and os.getpid() not in pids, pids
    # Control: one job runs each task in this process.
    pids = [counts["pid"][0] for _, counts in g._run_tasks(tasks, tmp_path, tmp_path, False, 1, g.PhaseClock())]
    assert pids == [os.getpid()] * 4


def test_the_files_do_not_depend_on_the_number_of_processes(tmp_path: Path) -> None:
    built = {}
    for jobs in (1, 3):
        root = tmp_path / f"root{jobs}"
        (root / "sprites" / "props").mkdir(parents=True)
        for index in range(5):
            image = Image.new("RGBA", (64 + 8 * index, 48), (30 * index, 90, 160, 255))
            image.save(root / "sprites" / "props" / f"prop_{index}.png")
        g.generate_sprite_variants(root, jobs=jobs)
        built[jobs] = {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in sorted(root.rglob("*"))
            if path.is_file() and not path.is_relative_to(root / "sprites")
        }
    assert len(built[1]) == 5 * len(g.VARIANTS), sorted(built[1])
    assert built[3] == built[1]


def test_the_publish_passes_the_job_cap() -> None:
    text = (REPO / "scripts/regen/sprites.sh").read_text()
    assert '--sprites-only --jobs "$AMBITION_SPRITE_JOBS"' in text
