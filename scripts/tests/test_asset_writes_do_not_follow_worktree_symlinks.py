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


# ---------------------------------------------------------------------------
# ⛔⛔ THE SFX BANK WAS THE UNGUARDED ROAD, AND THIS FILE DID NOT LOOK AT IT.
#
# Everything above tests `generate_visual_quality_variants.py`. Jon's 2026-09-03
# review found the guard was one publisher wide: `scripts/regen/sfx.sh` aims
# `tools/ambition_sfx_pack/pack.py` at
# `crates/ambition_platformer2d_actor_monolith/assets/audio/sfx.bank`, which is
# inside `mirror_assets_for_worktree.py`'s MIRRORED_TREES — and the packer wrote
# it with a bare `out_path.open("wb")`, straight through the link.
#
# ⭐ The lesson worth more than the fix: this file's own docstring says the
# hazard belongs to "a writer publishing into this tree", and then tested ONE
# writer. A guard scoped to the road where the bug was found does not cover the
# invariant it claims.
#
# ⚠ These are BEHAVIOURAL, not source-text scans — they build a real symlink,
# run the real publisher, and read the target's bytes. AGENTS.md declines
# source-text meta-test machinery, and a grep for a helper's NAME would pass on
# a call site that passed the wrong path anyway.
# ---------------------------------------------------------------------------

PACK = REPO / "tools/ambition_sfx_pack/pack.py"


def load_packer():
    if not PACK.exists():
        pytest.skip(f"{PACK} is absent")
    spec = importlib.util.spec_from_file_location("sfx_pack", PACK)
    module = importlib.util.module_from_spec(spec)
    sys.modules["sfx_pack"] = module
    spec.loader.exec_module(module)
    return module


def _mirrored(tmp_path: Path, name: str) -> tuple[Path, Path]:
    """A worktree path symlinked at a main-checkout file, as the mirror makes it."""
    main = tmp_path / "main"
    work = tmp_path / "worktree"
    main.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    target = main / name
    target.write_bytes(b"MAIN CHECKOUT BYTES")
    link = work / name
    link.symlink_to(target)
    return target, link


def test_writing_the_bank_does_not_reach_through_the_mirror(tmp_path):
    """The premise is asserted first: without the guard this WOULD have written
    through. A test that only checks the fixed path passes on a system where
    writes never follow links, which proves nothing."""
    packer = load_packer()
    target, link = _mirrored(tmp_path, "sfx.bank")

    # PREMISE: an unguarded open-for-write follows the link.
    probe_target, probe_link = _mirrored(tmp_path / "probe", "sfx.bank")
    with probe_link.open("wb") as fh:
        fh.write(b"THROUGH")
    assert probe_target.read_bytes() == b"THROUGH", (
        "an unguarded write did NOT follow the symlink on this filesystem, so "
        "the test below would pass for the wrong reason"
    )
    assert probe_link.is_symlink()

    packer.write_bank(link, [])

    assert target.read_bytes() == b"MAIN CHECKOUT BYTES", (
        "write_bank reached through the mirror and rewrote the MAIN checkout's "
        "sfx.bank — the file every other session builds and gates from"
    )
    assert not link.is_symlink(), "the worktree must now own a real bank"
    assert link.read_bytes()[:8] == b"AMBNDSFX"


def test_writing_the_dump_does_not_reach_through_the_mirror(tmp_path):
    """The dump is the second write site and shipped unguarded beside the bank —
    two roads out of one function pair, which is how one gets fixed alone."""
    packer = load_packer()
    target, link = _mirrored(tmp_path, "sfx.bank.dump")

    packer.write_dump(link, [], Path("sfx.bank"))

    assert target.read_bytes() == b"MAIN CHECKOUT BYTES"
    assert not link.is_symlink()
    assert "dump" in link.read_text()
