"""The authoring-coverage census refuses an empty join and an unnamed sprite stage.

⛔⛤ **AN EMPTY JOIN READS AS GOOD NEWS.** This script joins a recording's per-move
ratios to the `.spec.json` that decides each box. If the recording and the specs
stop naming the same clips — a renamed clip, a re-pointed stage, a take of a
different roster — the join produces nothing, and "0 moves are bone-derived with
no inflate" reads as *every move is tuned* rather than *these two inputs do not
meet*. Same family as an all-zero ratio table.

⛔⛤ **AND A NEW SPRITE STAGE MUST BE AN ERROR, NOT A SILENT ABSENCE.** The
stage→character map has no machine-readable source, so a stage nobody adds a row
for would drop its whole roster out of every table below — which also reads as
"those moves are fine". ⇒ It refuses.

⚠ I PUBLISHED 93/62 FROM AN AD-HOC JOIN AND THIS SCRIPT SAYS 97/64. That is the
whole reason the rule here is to COMMIT the script: two answers to one question
and the reproducible one wins.
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
import sys

import pytest

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
_loader = importlib.util.spec_from_file_location(
    "measure_hitbox_authoring_coverage",
    REPO / "scripts" / "measure_hitbox_authoring_coverage.py",
)
cov = importlib.util.module_from_spec(_loader)
_loader.loader.exec_module(cov)


def test_every_sprite_stage_is_named_in_the_character_map():
    """⛔ THE REFUSAL'S SUBJECT MUST EXIST: this is what makes adding a stage an
    error rather than a silently missing roster."""
    stages = {p.name for p in cov.STAGES.iterdir() if p.is_dir()}
    assert stages, f"no sprite stage found under {cov.STAGES} — the scan is not reaching the tree"
    unnamed = sorted(stages - set(cov.STAGE_CHARACTERS))
    assert not unnamed, (
        f"sprite stage(s) {unnamed} have no row in STAGE_CHARACTERS, so every move "
        "they decide would be missing from the census — which reads as 'those "
        "moves are fine'."
    )


def test_the_authored_tables_still_name_a_clip_per_move():
    """⛔ THE OTHER HALF OF THE JOIN. If the tables stop spelling `clip:` the way
    this reads them, every row drops out and the census reports an empty problem."""
    clips = cov.clip_of_move()
    # ⚠ MEASURED FLOOR: 372 moves carry a clip at 2026-09-12.
    assert len(clips) > 250, (
        f"only {len(clips)} authored move(s) resolve to a clip (372 at 2026-09-12) "
        "— the table reader has drifted from the tables"
    )


def test_an_empty_join_is_refused_rather_than_reported(tmp_path, monkeypatch):
    """⭐ THE REFUSAL ITSELF, EXERCISED. An unexercised `raise` is a claim about a
    branch nobody has taken."""
    take = tmp_path / "takes.json"
    take.write_text(json.dumps({"takes": []}))
    monkeypatch.setattr(sys, "argv", ["cov", str(take)])
    with pytest.raises(SystemExit) as caught:
        cov.main()
    assert "REFUSING TO REPORT" in str(caught.value), caught.value
