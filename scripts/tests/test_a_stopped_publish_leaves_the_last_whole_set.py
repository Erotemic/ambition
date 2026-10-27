"""A sprite publish writes into a staging root and swaps it in at the end.

So a publish that stops part way leaves the runtime roots as the last whole
set, and the next run continues the staged copy. The tests call the
functions `scripts/regen/sprites.sh` calls (`scripts/lib/sprite_staging.sh`)
on a small tree, and check the order of the calls in the script.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LIB = REPO / "scripts/lib/sprite_staging.sh"
REGEN = REPO / "scripts/regen/sprites.sh"


def _bash(tmp_path: Path, body: str, *, provenance=False) -> str:
    script = f"""
set -euo pipefail
asset_root={tmp_path / "assets"}
staging_root={tmp_path / "stage"}
mirror_root={tmp_path / "mirror"}
{("staged_provenance_root=" + str(tmp_path / "records")) if provenance else ""}
source {LIB}
{body}
"""
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout


def _tree(tmp_path: Path) -> Path:
    assets = tmp_path / "assets"
    (assets / "sprites").mkdir(parents=True)
    (assets / "sprites" / "sandbag_spritesheet.png").write_text("old sheet")
    (assets / "sprites" / "sandbag_parts.ron").write_text("old flipbook")
    (assets / "sprite_packs" / "full").mkdir(parents=True)
    (assets / "sprite_packs" / "full" / "ultrapack.json").write_text("old pack")
    return assets


def test_a_stopped_run_leaves_the_runtime_roots_whole_and_the_next_run_continues(tmp_path: Path) -> None:
    assets = _tree(tmp_path)
    stage = tmp_path / "stage"
    # The run publishes one of two files, then stops.
    _bash(tmp_path, 'begin_staging; echo "new sheet" > "$staging_root/sprites/sandbag_spritesheet.png"')
    assert (assets / "sprites" / "sandbag_spritesheet.png").read_text() == "old sheet"
    assert (assets / "sprites" / "sandbag_parts.ron").read_text() == "old flipbook"

    # The next run keeps what the stopped run published and swaps the set in.
    out = _bash(
        tmp_path,
        'begin_staging; echo "new flipbook" > "$staging_root/sprites/sandbag_parts.ron"; finish_staging',
    )
    assert "continuing the copy a stopped run left" in out, out
    assert (assets / "sprites" / "sandbag_spritesheet.png").read_text() == "new sheet\n"
    assert (assets / "sprites" / "sandbag_parts.ron").read_text() == "new flipbook\n"
    assert (assets / "sprite_packs" / "full" / "ultrapack.json").read_text() == "old pack"
    # Control: a root the runtime did not have is made, and staging is gone.
    assert (assets / "sprites_potato").is_dir()
    assert not stage.exists()


def test_a_swap_that_stopped_part_way_is_repaired_by_the_next_run(tmp_path: Path) -> None:
    assets = _tree(tmp_path)
    _bash(tmp_path, 'begin_staging; echo "new sheet" > "$staging_root/sprites/sandbag_spritesheet.png"')
    # The swap moved `sprites` and stopped before the other roots.
    stage = tmp_path / "stage"
    (assets / "sprites").rename(stage / "sprites.old")
    (stage / "sprites").rename(assets / "sprites")

    _bash(tmp_path, "begin_staging")
    assert (stage / "sprites" / "sandbag_spritesheet.png").read_text() == "new sheet\n"
    assert (stage / "sprite_packs" / "full" / "ultrapack.json").read_text() == "old pack"
    _bash(tmp_path, "finish_staging")
    assert (assets / "sprites" / "sandbag_spritesheet.png").read_text() == "new sheet\n"
    assert not stage.exists()


def test_the_next_run_starts_from_the_set_the_swap_replaced(tmp_path: Path) -> None:
    """A copy of the 1.6 GB of roots is 26 s per run. The swap keeps the set
    it replaces, and the next run copies into it only what differs."""
    assets = _tree(tmp_path)
    (assets / "sprites" / "robot_spritesheet.png").write_text("robot")
    _bash(tmp_path, 'begin_staging; echo "new sheet" > "$staging_root/sprites/sandbag_spritesheet.png"; finish_staging')
    mirror = tmp_path / "mirror"
    assert (mirror / "sprites" / "sandbag_spritesheet.png").read_text() == "old sheet"
    kept = (mirror / "sprites" / "robot_spritesheet.png").stat().st_ino
    # Between the runs, the runtime root loses a file.
    (assets / "sprites" / "sandbag_parts.ron").unlink()

    _bash(tmp_path, "begin_staging")
    stage = tmp_path / "stage"
    # The staged copy is the runtime set, file for file.
    assert sorted(p.name for p in (stage / "sprites").iterdir()) == ["robot_spritesheet.png", "sandbag_spritesheet.png"]
    assert (stage / "sprites" / "sandbag_spritesheet.png").read_text() == "new sheet\n"
    assert (stage / "sprite_packs" / "full" / "ultrapack.json").read_text() == "old pack"
    # It is the mirror, moved: a file that did not change was not copied.
    assert (stage / "sprites" / "robot_spritesheet.png").stat().st_ino == kept
    assert not (mirror / "sprites").exists()


def test_the_publish_swaps_after_the_sprite_phases_and_before_the_ldtk_phases() -> None:
    text = REGEN.read_text()
    swap = text.index('phase "staging root → runtime roots"')
    begin = text.index("begin_staging\nuse_publish_root \"$staging_root\"")
    # Every phase that writes a runtime root writes the staged one.
    for writer in (
        'phase "entity sprites',
        'phase "Ultrapack',
        'phase "Publish boundary',
        "if ! run_quality_variants; then\n    regen_failures+=",
    ):
        assert begin < text.index(writer) < swap, writer
    assert '--asset-root "$publish_asset_root"' in text
    assert 'pack_root="$publish_asset_root/sprite_packs"' in text
    assert 'mirror_root="$renderer_dir/.cache/publish_mirror"' in text
    # The LDtk phases write sheet paths into tracked files: they read the
    # runtime roots, after the swap.
    for reader in ('phase "LDtk editor-icon atlas:"', 'phase "LDtk sprite tilesets:"'):
        assert text.index('use_publish_root "$asset_root"', swap) < text.index(reader), reader
    # Only a set that every sprite phase passed is swapped in.
    assert 'if [ "${#regen_failures[@]}" -eq 0 ]; then\n    finish_staging' in text


def test_a_failed_publish_keeps_the_accepted_receipt_with_the_live_pixels(tmp_path):
    assets = _tree(tmp_path)
    records = tmp_path / 'records/sprites'
    records.mkdir(parents=True)
    receipt = records / 'sandbag.json'
    receipt.write_text('old accepted receipt')
    _bash(tmp_path, 'begin_staging; echo "new sheet" > "$staging_root/sprites/sandbag_spritesheet.png"; echo "new receipt" > "$staging_root/_art_provenance/sprites/sandbag.json"', provenance=True)
    assert receipt.read_text() == 'old accepted receipt'
    assert (assets / 'sprites/sandbag_spritesheet.png').read_text() == 'old sheet'
    _bash(tmp_path, 'begin_staging; finish_staging', provenance=True)
    assert receipt.read_text() == 'new receipt\n'
    assert (assets / 'sprites/sandbag_spritesheet.png').read_text() == 'new sheet\n'
