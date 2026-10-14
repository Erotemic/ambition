"""The per-sheet cache key of `scripts/regen/sprites.sh` (`scripts/lib/sprite_cache_key.sh`).

The functions run as the publish runs them, on a small renderer tree: two
targets with a rig each, and a helper that the core imports. The closure map
and the core's files are written here as `sprite_cache_key_prepare` would
write them (that step asks the renderer's registry, which this tree has not).
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
LIB = REPO / "scripts" / "lib" / "sprite_cache_key.sh"
PKG = "ambition_sprite2d_renderer"


@pytest.fixture
def renderer(tmp_path: Path) -> Path:
    root = tmp_path / "renderer"
    for rel, text in {
        f"{PKG}/__init__.py": "",
        f"{PKG}/core/__init__.py": "",
        f"{PKG}/core/draw.py": "from ..targets.fam import _shared\n",
        f"{PKG}/targets/__init__.py": "",
        f"{PKG}/targets/fam/__init__.py": "",
        f"{PKG}/targets/fam/_shared.py": "S = 1\n",
        f"{PKG}/targets/fam/_rig_a.py": "A = 1\n",
        f"{PKG}/targets/fam/_rig_b.py": "B = 1\n",
        f"{PKG}/targets/fam/alpha.py": "from ._rig_a import A\n",
        f"{PKG}/targets/fam/beta.py": "from ._rig_b import B\n",
    }.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    fam = f"{PKG}/targets/fam"
    (tmp_path / "closures.tsv").write_text(
        f"alpha\t{fam}/alpha.py\nalpha\t{fam}/_rig_a.py\nbeta\t{fam}/beta.py\nbeta\t{fam}/_rig_b.py\n"
    )
    return root


def keys(renderer: Path) -> dict[str, str]:
    """`unit_key` of each target, by the library's own functions."""
    script = f"""
        set -euo pipefail
        source {LIB}
        renderer_dir={renderer}
        target_closures_file={renderer.parent / 'closures.tsv'}
        core_target_files={PKG}/targets/fam/_shared.py
        core_shared_fingerprint="$(compute_core_shared)"
        printf 'alpha %s\\n' "$(unit_key alpha)"
        printf 'beta %s\\n' "$(unit_key beta)"
    """
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    return dict(line.split() for line in out.splitlines())


def edit(renderer: Path, rel: str) -> None:
    path = renderer / PKG / "targets" / "fam" / rel
    path.write_text(path.read_text() + "# edited\n")


def test_an_edit_to_one_targets_rig_makes_only_that_target_stale(renderer: Path) -> None:
    """The defect: every `_*.py` helper was in the shared core, so the crew's
    rig made all 271 sheets stale, a republish of about one hour."""
    before = keys(renderer)
    edit(renderer, "_rig_a.py")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"], "the edit to alpha's rig did not make alpha stale"
    assert after["beta"] == before["beta"], "the edit to alpha's rig made beta stale"


def test_an_edit_to_a_target_module_makes_it_stale(renderer: Path) -> None:
    before = keys(renderer)
    edit(renderer, "beta.py")
    after = keys(renderer)
    assert after["beta"] != before["beta"] and after["alpha"] == before["alpha"]


def test_an_edit_to_a_helper_the_core_imports_makes_every_target_stale(renderer: Path) -> None:
    """The control: the core runs for every target."""
    before = keys(renderer)
    edit(renderer, "_shared.py")
    after = keys(renderer)
    assert after["alpha"] != before["alpha"] and after["beta"] != before["beta"]
