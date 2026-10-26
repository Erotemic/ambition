"""A focused publish (`sprites.sh --target NAME`) packs the runtime groups.

It packed no Ultrapack, so a focused publish of a runtime-group target
(`intro_cart`, which a prop row draws from the pack) left the pack on the old
art until the next full publish. A tier packs only the runtime groups, in
about a second. The test runs the script's own focused block with a renderer
stub that records each call.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"


def _focused_pack_block() -> str:
    text = REGEN.read_text()
    focused = text.index('if [ "${#target_names[@]}" -gt 0 ]; then\n    validate_sprite_targets')
    start = text.index('    if [ "${AMBITION_ULTRAPACK:-1}" != "0" ]; then', focused)
    end = text.index('    echo "==> staging root → runtime roots"', start)
    return text[start:end]


def _tier_table() -> str:
    text = REGEN.read_text()
    start = text.index("ultrapack_tiers=(\n")
    return text[start : text.index(")\n", start) + 2]


def test_a_focused_publish_packs_each_tier_of_the_runtime_groups(tmp_path: Path) -> None:
    calls = tmp_path / "calls"
    keys = tmp_path / "renderer" / ".cache" / "ultrapack"
    keys.mkdir(parents=True)
    for tier in ("full", "potato"):
        (keys / f"{tier}.key").write_text("old inputs\n")
    script = f"""
set -euo pipefail
run_renderer_python() {{ echo "$*" >> {calls}; echo "packed $1"; }}
renderer_dir={tmp_path / "renderer"}
publish_asset_root={tmp_path / "assets"}
sprites_dir={tmp_path / "assets" / "sprites"}
ultrapack_plan=(--pack-plan plan.yaml)
{_tier_table()}
{_focused_pack_block()}
"""
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    lines = calls.read_text().splitlines()
    assert [line.split()[0] for line in lines] == [
        f"ultrapack-{tier}" for tier in ("full", "half", "quarter", "potato")
    ], lines
    for line, tier in zip(lines, ("full", "half", "quarter", "potato")):
        assert "--runtime-only" in line.split(), line
        assert f"--out {tmp_path / 'assets' / 'sprite_packs' / tier}" in line, line
    # The next full publish must run each tier again, not trust an old key.
    assert not list(keys.glob("*.key")), list(keys.glob("*.key"))
    assert "packed ultrapack-full" in out, out


def test_a_focused_publish_writes_the_staging_root_and_swaps_it_in() -> None:
    """A focused run wrote the runtime roots in place, so one that stopped
    part way left a mixed set. It stages as a full publish does."""
    text = REGEN.read_text()
    focused = text.index('if [ "${#target_names[@]}" -gt 0 ]; then\n    validate_sprite_targets')
    end = text.index("    print_regen_timings\n    exit 0\n", focused)
    block = text[focused:end]
    begin = block.index('    begin_staging\n    use_publish_root "$staging_root"\n')
    for writer in ('publish-many', "generate_visual_quality_variants.py", "--runtime-only"):
        assert begin < block.index(writer) < block.index("    finish_staging\n"), writer
    assert '--asset-root "$publish_asset_root"' in block
    assert '--out "$publish_asset_root/sprite_packs/$tname"' in block
    assert block.index("    finish_staging\n") < block.index('    use_publish_root "$asset_root"\n')
