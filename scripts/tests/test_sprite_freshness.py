"""`scripts/lib/sprite_freshness.py` answers what `publish_cached_batch` asked
one target at a time: the key is the bash `unit_key`, byte for byte, and a
target is fresh only when its key, its outputs and its portraits all hold."""

from __future__ import annotations

import hashlib
import importlib.util
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("sprite_freshness", REPO / "scripts/lib/sprite_freshness.py")
freshness = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(freshness)


def _tree(tmp_path: Path) -> dict:
    repo = tmp_path / "repo"
    renderer = repo / "renderer"
    cache = renderer / ".cache"
    (cache / "reads").mkdir(parents=True)
    (cache / "sheets").mkdir()
    (renderer / "pkg").mkdir()
    (renderer / "pkg/core.py").write_text("core\n")
    (renderer / "pkg/hero.py").write_text("hero\n")
    (renderer / "pkg/villain.py").write_text("villain\n")
    (cache / "target_closures.tsv").write_text(
        "hero\tpkg/hero.py\nhero\tpkg/core.py\nvillain\tpkg/villain.py\nvillain\tpkg/core.py\nvillain\tpkg/gone.py\n"
    )
    (repo / "data/hero").mkdir(parents=True)
    (repo / "data/hero/pose.json").write_text("{}\n")
    (repo / "data/hero/.hidden").write_text("x\n")
    (repo / "data/b.txt").write_text("b\n")
    (repo / "data/a.txt").write_text("a\n")
    (cache / "reads/hero.reads").write_text(
        "f data/b.txt\nf data/a.txt\nf data/a.txt\nf data/missing.txt\nd data/hero\nd data/none\n"
    )
    (cache / "reads/villain.reads").write_text("f data/a.txt\n")
    sprites = repo / "sprites"
    sprites.mkdir()
    return {"repo": repo, "renderer": renderer, "cache": cache, "sprites": sprites}


def _bash_unit_key(tree: dict, target: str) -> str:
    script = f"""
set -u
repo_root={tree['repo']}
renderer_dir={tree['renderer']}
cache_dir={tree['cache']}
target_reads_dir=$cache_dir/reads
target_closures_file=$cache_dir/target_closures.tsv
unit_core_fingerprint=core123
source {REPO}/scripts/lib/sprite_cache_key.sh
unit_key {target}
"""
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout.strip()


def _python_unit_key(tree: dict, target: str):
    return freshness.unit_key(
        freshness.Hasher(),
        target=target,
        core="core123",
        repo_root=tree["repo"],
        renderer_dir=tree["renderer"],
        reads_dir=tree["cache"] / "reads",
        closure_of=freshness.closures(tree["cache"] / "target_closures.tsv"),
    )


def test_the_python_unit_key_is_the_bash_unit_key(tmp_path):
    """A stored key is compared as text, so a key one byte off makes every
    sheet stale. The fixture has what the bash side treats specially: a
    missing closure file, a missing read, a duplicate read, an unsorted
    record, a hidden file in a read directory and a directory that is gone."""
    tree = _tree(tmp_path)
    for target in ("hero", "villain"):
        assert _python_unit_key(tree, target) == _bash_unit_key(tree, target), target
    # A target with no read record has a random bash key: nothing can equal it.
    assert _python_unit_key(tree, "nobody") is None
    # Control: the key moves with a read file's bytes on both sides.
    before = _bash_unit_key(tree, "hero")
    (tree["repo"] / "data/a.txt").write_text("changed\n")
    assert _bash_unit_key(tree, "hero") != before
    assert _python_unit_key(tree, "hero") == _bash_unit_key(tree, "hero")


def _verdicts(tree: dict, *extra: str, installs: str, portraits: str = "") -> dict:
    installs_tsv = tree["repo"] / "installs.tsv"
    portraits_tsv = tree["repo"] / "portraits.tsv"
    installs_tsv.write_text(installs)
    portraits_tsv.write_text(portraits)
    result = subprocess.run(
        [
            "python3", str(REPO / "scripts/lib/sprite_freshness.py"),
            "--repo-root", str(tree["repo"]), "--renderer-dir", str(tree["renderer"]),
            "--cache-dir", str(tree["cache"]), "--sprites-dir", str(tree["sprites"]),
            "--core", "core123", "--installs", str(installs_tsv), "--portraits", str(portraits_tsv),
            *extra, "hero",
        ],
        capture_output=True, text=True, check=True,
    )
    return dict(line.split("\t")[::-1] for line in result.stdout.splitlines())


def test_a_target_is_fresh_only_when_its_key_outputs_and_portraits_hold(tmp_path):
    tree = _tree(tmp_path)
    sheet = tree["sprites"] / "hero_spritesheet.png"
    sheet.write_bytes(b"pixels")
    digest = hashlib.sha256(b"pixels").hexdigest()
    entry = tree["cache"] / "sheets/hero"
    entry.write_text(f"{_bash_unit_key(tree, 'hero')}\n{digest} hero_spritesheet.png\n")
    installs = "hero\thero_spritesheet.png\n"
    assert _verdicts(tree, installs=installs) == {"hero": "fresh"}
    assert _verdicts(tree, "--force", installs=installs) == {"hero": "stale"}
    assert _verdicts(tree, installs=installs, portraits="hero\tportraits/hero.png\n") == {"hero": "stale"}
    assert _verdicts(tree, "--portraits-failed", installs=installs) == {"hero": "stale"}
    sheet.write_bytes(b"another robot")
    assert _verdicts(tree, installs=installs) == {"hero": "stale"}, "an output that changed is stale"
    sheet.write_bytes(b"pixels")
    entry.write_text(f"{'0' * 64}\n{digest} hero_spritesheet.png\n")
    assert _verdicts(tree, installs=installs) == {"hero": "stale"}, "a stored key that differs is stale"
    entry.write_text(f"{_bash_unit_key(tree, 'hero')}\n")
    assert _verdicts(tree, installs=installs) == {"hero": "stale"}, "an entry with no digests is stale"
