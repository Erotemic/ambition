"""Publication decisions use accepted generation evidence and required outputs."""

from __future__ import annotations

import hashlib
import importlib.util
import subprocess
import json
import sys
import os
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
    (renderer / "ambition_sprite2d_renderer").symlink_to(REPO / "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer", target_is_directory=True)
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
            "--installs", str(installs_tsv), "--portraits", str(portraits_tsv),
            *extra, "hero",
        ],
        capture_output=True, text=True, check=True,
    )
    return dict(line.split("\t")[::-1] for line in result.stdout.splitlines())


def test_only_an_accepted_generation_receipt_can_make_a_target_current(tmp_path):
    tree = _tree(tmp_path)
    sheet = tree["sprites"] / "hero_spritesheet.png"
    sheet.write_bytes(b"pixels")
    sys.path.insert(0, str(REPO / 'tools/ambition_sprite2d_renderer'))
    from ambition_sprite2d_renderer.products import generation
    source = tree['repo'] / 'data/a.txt'
    inputs = [{'path':'data/a.txt','sha256':hashlib.sha256(source.read_bytes()).hexdigest()}]
    record = {'schema':'ambition.art_provenance.v1', 'generation':generation(inputs, [], {}, complete=True),
              'outputs':[{'path':sheet.name,'sha256':hashlib.sha256(sheet.read_bytes()).hexdigest()}]}
    directory = tree['sprites'].parent.parent / 'asset_provenance/sprites'
    directory.mkdir(parents=True)
    receipt = directory / 'hero.json'
    receipt.write_text(json.dumps(record))
    installs = "hero\thero_spritesheet.png\n"
    assert _verdicts(tree, installs=installs) == {'hero':'fresh'}
    assert _verdicts(tree, '--force', installs=installs) == {'hero':'stale'}
    assert _verdicts(tree, installs=installs, portraits='hero\tportraits/hero.png\n') == {'hero':'stale'}
    sheet.write_bytes(b'changed')
    assert _verdicts(tree, installs=installs) == {'hero':'stale'}
    sheet.write_bytes(b'pixels')
    source.write_text('new source')
    assert _verdicts(tree, installs=installs) == {'hero':'stale'}
    receipt.unlink()
    assert _verdicts(tree, installs=installs) == {'hero':'stale'}
