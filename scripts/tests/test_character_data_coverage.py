"""The two-manifest-shape trap, pinned.

`measure_character_data_coverage.py` answers two questions Jon's reports keep
asking: which sheets carry the knockdown rows, and which catalog rows author
`standing_height`.

⛔⛔ SPRITE MANIFESTS COME IN TWO SHAPES. One lists animations as
`body_metrics.animations` MAP KEYS (`"knockdown": (...)`), the other names them
in ROW FIELDS (`animation: "knockdown"`). A regex for the first form reported 4
sheets where 13 carry the rows, and called `officer` — a file plainly containing
all four words — empty. That would have read as a REGRESSION from the doc's
count of 10, in a week with no art work.

⚠ And the catalog must be counted per ROW, not per line: `grep -c` picks up
comments and inflates the number.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_character_data_coverage.py"


def load():
    spec = importlib.util.spec_from_file_location("coverage", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_both_manifest_shapes_are_recognised():
    """⭐ THE WHOLE POINT. Either shape alone is a real manifest in this tree."""
    module = load()
    as_map_key = '(target: "x", body_metrics: Some((animations: {"knockdown": (hurtbox: None)})))'
    as_row_field = '(target: "y", rows: [(animation: "knockdown", frames: 4)])'
    assert module.sheet_has_row(as_map_key, "knockdown"), "map-key form missed"
    assert module.sheet_has_row(as_row_field, "knockdown"), (
        "row-field form missed — this is the shape that reported `officer` empty "
        "while it contained all four rows"
    )


def test_a_near_miss_name_is_not_counted():
    """`getup` must not be satisfied by `getup_attack` appearing, or every sheet
    with the attack row reads as carrying both."""
    module = load()
    only_attack = '(rows: [(animation: "getup_attack", frames: 3)])'
    assert module.sheet_has_row(only_attack, "getup_attack")
    assert not module.sheet_has_row(only_attack, "getup"), (
        "`getup` matched inside `getup_attack`; the two are separate rows"
    )


def test_the_catalog_is_read_per_row_not_per_line(tmp_path, monkeypatch):
    """⛔ A commented-out height must not count, and a row without one must be
    reported as silent rather than dropped."""
    module = load()
    catalog = tmp_path / "character_catalog.ron"
    catalog.write_text(
        '(\n    characters: {\n'
        '        "has_one": (\n            standing_height: Some(60.5),\n        ),\n'
        '        "commented": (\n            // standing_height: Some(99.0),\n        ),\n'
        '        "silent": (\n            display_name: "x",\n        ),\n'
        '    },\n)\n'
    )
    monkeypatch.setattr(module, "CATALOG", catalog)
    authored, silent = module.authored_heights()
    assert authored == {"has_one": 60.5}
    assert sorted(silent) == ["commented", "silent"], (
        "a commented-out height must count as SILENT, not as authored"
    )


def test_the_live_tree_sees_both_shapes_at_once():
    """⭐ POSITIVE CONTROL. If the count collapses to the handful only one shape
    produces, the two-shape handling has regressed."""
    module = load()
    if not module.SHEETS.is_dir():
        pytest.skip("the sprite tree is gitignored generated output; absent here")
    rows = module.knockdown_coverage()
    full = [n for n, got in rows.items() if len(got) == len(module.KNOCKDOWN_ROWS)]
    assert len(rows) > 100, f"only {len(rows)} sheets scanned"
    assert len(full) >= 10, (
        f"only {len(full)} sheets carry all four knockdown rows. Ten were counted "
        "on 2026-08-26 and thirteen on 2026-09-02; a sharp drop is far more "
        "likely to be the map-key/row-field regex than lost art"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
