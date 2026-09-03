"""The pack census must refuse a parser failure instead of publishing it.

`measure_pack_reachability.py` answers "how much of the shared sprite pack can
anything reach?" by intersecting the packed targets with the prop rows that opt
into the pack road. Today that is ONE target (`intro_cart`) against 197 packed,
and 98.8% of the pages on this machine sit where no consumer can ask for them.

⛔⛔ THAT HEADLINE IS ALSO WHAT A BROKEN REGEX PRODUCES. If the opt-in table's
shape drifts and the match returns nothing, the script computes "0 targets
reachable" and reports an even more dramatic version of a real finding — with
no symptom distinguishing it from the truth. The refusal is the whole guard,
so it is what these tests pin.

⭐ THE MEGABYTES ARE MACHINE-LOCAL AND ARE NOT ASSERTED HERE. `sprite_packs/`
is gitignored generated output and a worktree symlinks the main checkout's
copies. Every test below runs on fixtures or on committed Rust.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_pack_reachability.py"


def load():
    spec = importlib.util.spec_from_file_location("pack_reach", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# The real tuple shape, copied from `intro_prop_sprite_rows()` rather than
# recalled: a 4-tuple whose last element is `Some("target")` or `None`.
ROWS = """
        (
            "intro_cart",
            "intro_cart_spritesheet.png",
            intro_sheet("intro_cart", t),
            Some("intro_cart"),
        ),
        (
            "lab_power_core",
            "creator_lab_props_spritesheet.png",
            intro_sheet("power_core", t),
            None,
        ),
"""


def test_it_reads_the_opt_in_rows_and_ignores_the_none_rows():
    found = load().opted_in_targets(ROWS)
    assert found == {"intro_cart"}, (
        "a `None` row uses its per-target sheet and must not count as pack reach"
    )


def test_the_real_table_still_parses():
    """⭐ POSITIVE CONTROL against the committed source. The refusal below only
    protects against a regex that matches nothing; this catches the day the
    table changes shape, which is the event that would trigger it."""
    module = load()
    if not module.PROP_ROWS.exists():
        pytest.skip("the intro prop table is absent")
    found = module.opted_in_targets(module.PROP_ROWS.read_text())
    assert found, (
        "no pack opt-in row parsed from the committed table — the regex is "
        "stale, and the census would report the whole pack unreachable"
    )


def test_page_zero_counts_as_reachable_even_when_no_frame_uses_it():
    """The catalog registers `ultrapack_0.png` per tier as the profile-gated
    entry every consumer resolves before its siblings, so it loads regardless.
    Counting only frame-referenced pages would understate the reach."""
    module = load()
    catalog = {"targets": {"cart": {"idle": [{"page": 4}, {"page": 5}]}}}
    assert module.pages_used_by(catalog, {"cart"}) == {0, 4, 5}


def test_an_unpacked_target_contributes_no_pages():
    module = load()
    catalog = {"targets": {"cart": {"idle": [{"page": 3}]}}}
    assert module.pages_used_by(catalog, {"not_in_the_pack"}) == {0}


def _run(env_repo: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(SCRIPT)], capture_output=True, text=True, cwd=env_repo
    )


def test_it_refuses_when_the_pack_directory_is_absent(tmp_path, monkeypatch):
    """A fresh checkout has no packs. Reporting 0 MB would read as 'the pack
    costs nothing', which is the opposite of what absence means here."""
    module = load()
    monkeypatch.setattr(module, "PACKS", tmp_path / "nope")
    code = module.main([])
    assert code == 2


def test_it_refuses_when_no_opt_in_row_parses(tmp_path, monkeypatch, capsys):
    """⛔ THE POISON. A table the regex cannot read must produce a refusal, not
    a 100%-unreachable headline."""
    module = load()
    packs = tmp_path / "sprite_packs" / "full"
    packs.mkdir(parents=True)
    (packs / "ultrapack.json").write_text(
        json.dumps({"pages": ["ultrapack_0.png"], "targets": {"cart": {}}})
    )
    rows = tmp_path / "sprites.rs"
    rows.write_text("rows that the regex cannot read")
    monkeypatch.setattr(module, "PACKS", tmp_path / "sprite_packs")
    monkeypatch.setattr(module, "PROP_ROWS", rows)

    code = module.main([])
    out = capsys.readouterr().out
    assert code == 2, "a parser failure must not exit 0"
    assert "NO PACK OPT-IN ROWS PARSED" in out
    assert "manufactured by a broken match" in out, (
        "the message must name the failure mode, or the next reader will treat "
        "the refusal as a real finding about the pack"
    )


def test_measure_reports_reachable_bytes_against_total(tmp_path, monkeypatch):
    module = load()
    tier = tmp_path / "full"
    tier.mkdir(parents=True)
    (tier / "ultrapack.json").write_text(
        json.dumps(
            {
                "pages": ["ultrapack_0.png", "ultrapack_1.png", "ultrapack_2.png"],
                "targets": {"cart": {"idle": [{"page": 2}]}, "ghost": {}},
            }
        )
    )
    for index, size in enumerate([10, 100, 1000]):
        (tier / f"ultrapack_{index}.png").write_bytes(b"x" * size)
    monkeypatch.setattr(module, "PACKS", tmp_path)
    monkeypatch.setattr(module, "TIERS", ["full"])

    row = module.measure({"cart"})["tiers"][0]
    assert row["packed_targets"] == 2, "the pack holds two targets"
    assert row["bytes"] == 1110
    assert row["reachable_bytes"] == 1010, "page 0 plus cart's page 2, not page 1"
    assert row["reachable_pages"] == 2


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
