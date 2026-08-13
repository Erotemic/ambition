"""Every level field the ENGINE reads must be DECLARED by every project.

A field the runtime reads and no project declares is a channel that cannot be
authored. That is not hypothetical: `RoomMetadata::mode` — the field a hosted
demo's entire ruleset gates on — documented itself as "authored as the LDtk
level string field `mode`" for months while no `.ldtk` in the repo declared it
and no level set it. Every mode in the game is assigned in Rust. The reader was
reading a field nothing could write, and nothing noticed, because the failure
mode of an undeclared field is silence: `field_i32` returns `None`, the
composer takes its default, and the room looks fine.

So this walks the shipped projects and asserts the declarations exist. It is
deliberately about the DEFS and not about any level's values — authoring a
number is a design decision per room, but being ABLE to author it is a
property of the toolchain, and that is what rots.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]

# Level fields read by `LdtkLevel::level_metadata` in
# `crates/ambition_platformer2d_ldtk/src/project.rs`. Adding a reader there without
# adding the def here (and to the projects) recreates the `mode` bug.
ENGINE_READ_LEVEL_FIELDS = {
    "activeArea",
    "biome",
    "music_track",
    "ambient_profile",
    "visual_theme",
    "gallery",
    "mode",
    "blast_margin",
    "side_blast_margin",
    "ceiling_blast_margin",
}


#: ⛔⛔ `.claude/worktrees/` holds abandoned agent worktrees, and a repo-wide walk
#: finds every world TWICE — once really and once as a stale copy whose submodule
#: files were never checked out. That is how this test started failing on six
#: projects that were perfectly fine (2026-08-13): the six failures were
#: `FileNotFoundError` on paths under a worktree, not a missing field def
#: anywhere. The same contamination made a first census of the authoring specs
#: read four times its true size.
#:
#: ⚠ worse than the noise is the quiet case — a stale worktree whose `.ldtk` DOES
#: exist gets validated as though it shipped, so a real project's regression
#: hides behind an old copy's passing.
EXCLUDED_DIRS = frozenset({"target", "__pycache__", ".claude", ".git"})


def ldtk_projects() -> list[Path]:
    #: ⚠ every world is reachable by TWO paths — `ambition_content/assets/worlds`
    #: is a directory of symlinks into the `ambition_map_assets` submodule where
    #: the files really live. Resolving collapses the pair to one project instead
    #: of testing each world twice under two ids.
    by_real_path = {
        p.resolve(): p
        for p in REPO_ROOT.rglob("*.ldtk")
        if EXCLUDED_DIRS.isdisjoint(p.parts)
    }
    found = sorted(by_real_path.values())
    assert found, "no .ldtk projects found; this test would pass vacuously"
    return found


@pytest.mark.parametrize("project", ldtk_projects(), ids=lambda p: p.stem)
def test_every_project_declares_the_fields_the_engine_reads(project: Path):
    data = json.loads(project.read_text())
    declared = {f.get("identifier") for f in data["defs"].get("levelFields", [])}
    missing = sorted(ENGINE_READ_LEVEL_FIELDS - declared)
    assert not missing, (
        f"{project.name} does not declare {missing}. The engine reads these level "
        "fields, so an undeclared one is a channel nobody can author — and it "
        "fails SILENTLY, because the reader just returns None and the composer "
        "takes its default. Add it with "
        "`ambition_ldtk_tools level add-field-def <name> --type <T>`."
    )
