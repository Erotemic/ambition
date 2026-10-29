"""The package guard asks the content compiler what the content pack needs.

`package_asset_guard.py` finds the declarations of most manifests with a
pattern match over runtime field names. For a file the content pack lists, that
is a second reader of a typed file, and it read `music_cues.ron` wrongly: a
section's `path:` is relative to its cue's `asset_root`, the pattern took it as
a package path, and every package build refused six files that were present. It
also could not see a music track whose file path comes from its id.

So a pack file is not pattern-matched. Its declarations are the lines of
`ambition_content --package-inputs`. These tests stand a script in for that
binary (`AMBITION_CONTENT_BIN`) and check what the guard does with its answer;
`package_inputs_are_the_paths_the_schemas_resolved` in
`crates/ambition_content_cli/tests/` holds the binary to the same lines.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import package_asset_guard as guard  # noqa: E402

PRESENT = "audio/music/generated/siege/adaptive/loop/loop.full.ogg"
ABSENT = "audio/music/generated/unrendered/full.ogg"


@pytest.fixture
def repo(tmp_path, monkeypatch):
    """A two-root tree whose pack lists one cue file, and a compiler that
    answers for it."""
    actors = tmp_path / guard.CONTENT_ASSET_ROOT
    content = tmp_path / guard.CONTENT_ASSET_ROOT
    (actors / PRESENT).parent.mkdir(parents=True)
    (actors / PRESENT).write_bytes(b"ogg")
    (content / "audio").mkdir(parents=True, exist_ok=True)
    (content / "pack.ron").write_text('(sources: [(path: "audio/cues.ron", schema: "music_cue_catalog")])\n')
    # The field the pattern match misread: relative to `asset_root`.
    (content / "audio" / "cues.ron").write_text(
        '(asset_root: "audio/music/generated/siege", path: "adaptive/loop/loop.full.ogg")\n'
    )
    answer = tmp_path / "answer.txt"
    compiler = tmp_path / "compiler.py"
    compiler.write_text(f"import sys\nsys.stdout.write(open({str(answer)!r}).read())\n")
    monkeypatch.setenv(guard.CONTENT_COMPILER_ENV, f"{sys.executable} {compiler}")

    def answer_with(*assets: str) -> Path:
        lines = ["source\taudio/cues.ron"] + [f"asset\t{path}\tsiege:music_cue/x" for path in assets]
        answer.write_text("\n".join(lines) + "\n")
        return tmp_path

    return answer_with


def test_a_pack_file_is_read_by_the_compiler_and_not_by_the_pattern(repo):
    contract, _ = guard.build_contract(repo(PRESENT), "android")
    assert "adaptive/loop/loop.full.ogg" not in contract.entries
    assert "pack-asset:siege:music_cue/x" in contract.entries[PRESENT].origins
    assert guard.PACK_SOURCE_ORIGIN in contract.entries["audio/cues.ron"].origins


def test_an_absent_pack_asset_is_reported_and_refused_only_when_strict(repo, capsys):
    tree = repo(PRESENT, ABSENT)
    contract, _ = guard.build_contract(tree, "android")
    assert ABSENT not in contract.entries
    assert ABSENT in capsys.readouterr().err
    with pytest.raises(guard.AssetContractError, match="unrendered"):
        guard.build_contract(tree, "android", strict_pack_assets=True)


def test_a_pack_the_compiler_refuses_stops_the_package(repo, monkeypatch):
    tree = repo(PRESENT)
    monkeypatch.setenv(guard.CONTENT_COMPILER_ENV, f"{sys.executable} -c 'raise SystemExit(1)'")
    with pytest.raises(guard.AssetContractError, match="refused"):
        guard.build_contract(tree, "android")
