"""A local preview composes the release file set without the release audit.

`compose --policy local` (the served-web development loop) takes the same
selected files a release does, from metadata: it hashes no input, checks the
composed tree by membership, and reports a missing declared input instead of
refusing. Its contract records no digests, so no package can be audited
against it. `--policy release` is unchanged.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from scripts.package import package_asset_guard as guard  # noqa: E402


@pytest.fixture
def tree(tmp_path, monkeypatch):
    """A content root with one sheet and its image, and a compiler with no
    pack inputs."""
    root = tmp_path / guard.CONTENT_ASSET_ROOT
    (root / "sprites").mkdir(parents=True)
    (root / "sprites/hero.png").write_bytes(b"pixels")
    (root / "sprites/hero_spritesheet.ron").write_text('(image: "hero.png")\n')
    (root / "pack.ron").write_text("(sources: [])\n")
    compiler = tmp_path / "compiler.py"
    compiler.write_text("")
    monkeypatch.setenv(guard.CONTENT_COMPILER_ENV, f"{sys.executable} {compiler}")
    return tmp_path


def compose(repo: Path, policy: str, out: Path) -> int:
    return guard.main([
        "compose", "--repo", str(repo), "--profile", "web", "--materialize", "link", "--policy", policy,
        "--output", str(out / "assets"), "--contract", str(out / "contract.json"),
        "--hash-manifest", str(out / "sha256"),
    ])


def test_local_composes_the_release_file_set_without_hashing(tree, tmp_path, monkeypatch):
    assert compose(tree, "release", tmp_path / "release") == 0
    hashed: list[Path] = []
    real = guard.sha256_path
    monkeypatch.setattr(guard, "sha256_path", lambda path: hashed.append(path) or real(path))
    assert compose(tree, "local", tmp_path / "local") == 0
    assert hashed == [], f"a local composition hashed {hashed}"
    release = guard.load_contract(tmp_path / "release/contract.json")
    local = guard.load_contract(tmp_path / "local/contract.json")
    assert set(local.entries) == set(release.entries) and "sprites/hero.png" in local.entries
    assert all(entry.sha256 is None for entry in local.entries.values())
    assert all(entry.sha256 for entry in release.entries.values())
    assert (tmp_path / "release/sha256").is_file() and not (tmp_path / "local/sha256").exists()


def test_a_missing_declared_input_is_reported_by_a_preview_and_refused_by_a_release(tree, tmp_path, capsys):
    (tree / guard.CONTENT_ASSET_ROOT / "sprites/hero.png").unlink()
    assert compose(tree, "release", tmp_path / "release") != 0
    assert "sprites/hero.png" in capsys.readouterr().err
    assert compose(tree, "local", tmp_path / "local") == 0
    err = capsys.readouterr().err
    assert "WARNING (local)" in err and "sprites/hero.png" in err and "declared by" in err
    assert not (tmp_path / "local/assets/sprites/hero.png").exists()


def test_no_package_is_audited_against_a_local_contract(tree, tmp_path, capsys):
    assert compose(tree, "local", tmp_path / "local") == 0
    code = guard.main(["audit-tree", "--contract", str(tmp_path / "local/contract.json"),
                       "--asset-root", str(tmp_path / "local/assets")])
    assert code != 0 and "Compose with --policy release" in capsys.readouterr().err
