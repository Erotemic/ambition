"""Regenerating an asset in a worktree must not write into the main checkout.

`scripts/mirror_assets_for_worktree.py` symlinks a worktree's generated assets
at the main checkout's copies, file by file, and says why in as many words:

    mirror the files individually and a regenerated sprite lands as a REAL file
    in the worktree, replacing that one symlink, while every other asset still
    points at the shared copy — and the main checkout never sees it. Directory
    links would send that write straight into main's assets, which is exactly
    the accident this exists to prevent.

⛔⛔ THAT IS ONLY TRUE IF THE WRITER UNLINKS FIRST, AND IT DID NOT.
`Image.save()` and `Path.write_text()` open the destination for writing, and an
open-for-write FOLLOWS a symlink — so the bytes land in the shared main
checkout. Verified 2026-09-02 rather than reasoned about: a plain
`open(link, "wb")` replaced the target's contents and left the link in place.

⚠ The main checkout is what other sessions build and gate from. This is the
difference between regenerating your own assets and silently editing everyone
else's — and the mirror's docstring actively reassures a reader that it cannot
happen, which is what makes it worth a test rather than a comment.
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
SCRIPT = REPO / "scripts/generate_visual_quality_variants.py"


def load():
    if not SCRIPT.exists():
        pytest.skip(f"{SCRIPT} is absent")
    spec = importlib.util.spec_from_file_location("variants", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    # ⛔ REGISTER BEFORE EXEC. Without this the generator's dataclasses raise
    # `AttributeError: 'NoneType' object has no attribute '__dict__'` while
    # resolving their own module — and a loader that swallowed that into a SKIP
    # made three of this file's five tests silently not run, which is the
    # failure mode the file exists to warn about.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_a_plain_write_really_does_follow_a_symlink(tmp_path):
    """⭐ THE PREMISE, MEASURED. Without this the test below could pass because
    writes never followed links in the first place, and would be pinning
    nothing. This is the behaviour that makes the guard necessary.
    """
    shared = tmp_path / "main.png"
    shared.write_bytes(b"SHARED")
    link = tmp_path / "worktree.png"
    link.symlink_to(shared)

    with link.open("wb") as handle:
        handle.write(b"REGENERATED")

    assert shared.read_bytes() == b"REGENERATED", (
        "premise: an open-for-write follows the symlink into the shared file"
    )
    assert link.is_symlink(), "and leaves the link in place, so nothing looks wrong"


def test_the_guard_replaces_the_link_instead_of_the_target(tmp_path):
    module = load()
    shared = tmp_path / "main.png"
    shared.write_bytes(b"SHARED")
    link = tmp_path / "worktree.png"
    link.symlink_to(shared)

    module.break_symlink_before_write(link)
    assert not link.exists(), "the link is gone, so the write creates a real file"

    with link.open("wb") as handle:
        handle.write(b"REGENERATED")

    assert shared.read_bytes() == b"SHARED", (
        "the MAIN checkout's bytes must be untouched — this is the whole point"
    )
    assert link.read_bytes() == b"REGENERATED"
    assert not link.is_symlink(), "the worktree now owns a real file"


def test_the_guard_leaves_an_ordinary_file_alone(tmp_path):
    """It must not delete a real file it was about to overwrite — that would
    turn a harmless rewrite into a window where the asset does not exist."""
    module = load()
    real = tmp_path / "real.png"
    real.write_bytes(b"ORIGINAL")
    module.break_symlink_before_write(real)
    assert real.exists() and real.read_bytes() == b"ORIGINAL"


def test_the_guard_tolerates_a_path_that_does_not_exist_yet(tmp_path):
    module = load()
    module.break_symlink_before_write(tmp_path / "never_written.png")


def test_every_write_site_in_the_generator_is_guarded():
    """⛔ A GUARD CALLED AT TWO OF THREE WRITE SITES IS NOT A GUARD. The pages,
    the resized loose PNGs and the manifest all land in the same mirrored tree.
    """
    if not SCRIPT.exists():
        pytest.skip("the generator is absent")
    text = SCRIPT.read_text()
    for write in ["resized.save(", "page.save(", "ron_dst.write_text("]:
        index = text.index(write)
        window = text[max(0, index - 240) : index]
        assert "break_symlink_before_write" in window, (
            f"the write at `{write}` is not preceded by the symlink guard, so "
            "regenerating in a worktree writes through to the main checkout"
        )
