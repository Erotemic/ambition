"""**A rider brain without a mount is a rider standing in the air.**

ADR 0020 links a rider to its mount through an LDtk `EntityRef` field,
`mounted_on`. Jon, 2026-08-08, from play: *"The pirates in the pirate sky no
longer ride their sharks."*

⛔ **They were authored and then silently destroyed by an EDITOR SESSION.**
`5e4d6448e` (2026-07-05) created four linked pairs in `pirate_sky_lookout` —
verified by reading the blob at that commit. `6e48e5988` (2026-07-06, *"modify
falling sand room"*, a 1505-line rewrite of `sandbox.ldtk` made while editing an
**unrelated level**) brought every one back as:

    { "__identifier": "mounted_on", "__type": "EntityRef", "__value": null, … }

`intro.ldtk` proves the pattern from the other side: its entities carry **no
`mounted_on` field at all** and its three refs still work, because it has not
been through an editor session since.

⭐ **Nothing errored, and nothing noticed for a month.** The standing rule about
never rewriting a `.ldtk` is aimed at TOOLS; here the writer was the editor,
which is where authoring is supposed to happen. That is why this is a test and
not a comment.

## Why a RATCHET and not a flat assertion

The four broken refs are **not repaired yet, deliberately** — the queue row
(D49) says not to re-author them until somebody knows what the editor did to
them, because a repair the next session deletes is worse than the bug. So this
pins the known-bad set and fails when it GROWS.

⚠ **That makes the number in `KNOWN_UNMOUNTED` a defect count, not a
baseline to keep comfortable.** When the refs are repaired it goes to zero and
this becomes the flat assertion it wants to be.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]

#: Where the worlds really live (reached from `game/` through tracked symlinks).
WORLDS = REPO / "game" / "ambition_map_assets" / "ambition_content" / "worlds"

#: A brain whose name says it rides something. Substring match on purpose — the
#: authored values are `pirate_shark_rider`, `pirate_heavy_shark_rider`, and the
#: pre-`5e4d6448e` fused spellings were `pirate_on_shark` / `pirate_heavy_on_shark`.
#: ⚠ matching the CAPABILITY the name claims, not an enumerated list, so a new
#: rider archetype is covered the day it is authored.
RIDER_MARKERS = ("_rider", "_on_shark")

#: ⛔ THE DEFECT COUNT, NOT A BUDGET. Every entry is a rider drawn without the
#: mount it was authored with, and the goal is zero. See the module docstring for
#: why they are not repaired yet.
KNOWN_UNMOUNTED = 4


def _enemy_spawns() -> list[tuple[str, str, dict]]:
    """`(world, level, fields)` for every `EnemySpawn` in every authored world.

    ⚠ parsed, not grepped: the worlds are gitignored-adjacent submodule content
    reached through symlinks, and a `grep -r` skips both.
    """
    rows: list[tuple[str, str, dict]] = []
    for path in sorted(WORLDS.glob("*.ldtk")):
        data = json.loads(path.read_text(encoding="utf8"))
        for level in data.get("levels", []):
            for layer in level.get("layerInstances", []):
                for entity in layer.get("entityInstances", []):
                    if entity.get("__identifier") != "EnemySpawn":
                        continue
                    fields = {
                        f["__identifier"]: f["__value"]
                        for f in entity.get("fieldInstances", [])
                    }
                    rows.append((path.name, level.get("identifier", "?"), fields))
    return rows


def _riders() -> list[tuple[str, str, dict]]:
    return [
        row
        for row in _enemy_spawns()
        if any(marker in (row[2].get("brain") or "") for marker in RIDER_MARKERS)
    ]


@pytest.fixture(scope="module")
def worlds_present() -> None:
    """⚠ SKIP rather than fail when the submodule is absent.

    A dangling world symlink is the DESIGNED signal for "you did not run
    `git submodule update --init`" (Jon, 2026-08-08), and every worktree agent
    hits it. Failing here would turn a deliberate state into a red on every
    non-recursive clone.
    """
    if not WORLDS.is_dir() or not any(WORLDS.glob("*.ldtk")):
        pytest.skip("game/ambition_map_assets is not checked out")


def test_there_are_rider_enemies_at_all(worlds_present: None) -> None:
    """⛔ kills the vacuous pass.

    Every assertion below iterates the rider set. If a rename retires
    `_rider`/`_on_shark`, this file would go green while checking nothing — the
    exact shape of a check that cannot fail. Then `RIDER_MARKERS` is what needs
    updating, not this test deleting.
    """
    assert _riders(), (
        f"no EnemySpawn brain matches {RIDER_MARKERS}, so every other assertion "
        f"in this file is vacuous. Either the rider archetypes were renamed — "
        f"update RIDER_MARKERS — or they were removed, in which case delete this "
        f"file and say so."
    )


def test_no_new_rider_loses_its_mount(worlds_present: None) -> None:
    """The ratchet. A rider brain must carry a `mounted_on` entity-ref.

    ⚠ this is the assertion an EDITOR SESSION breaks, which is the whole reason
    it exists — opening a world to move one platform can null every `EntityRef`
    in the FILE, not just in the level being edited.
    """
    unmounted = [
        f"{world}:{level} {fields.get('name')!r} brain={fields.get('brain')!r}"
        for world, level, fields in _riders()
        if not fields.get("mounted_on")
    ]
    assert len(unmounted) <= KNOWN_UNMOUNTED, (
        f"{len(unmounted)} rider enemies have no mount, up from the known "
        f"{KNOWN_UNMOUNTED}:\n  " + "\n  ".join(unmounted) + "\n\n"
        f"An LDtk editor session nulls every `EntityRef` in a world file — see "
        f"this file's docstring and queue D49. ⛔ Do NOT lower this by deleting "
        f"riders; re-author the `mounted_on` refs."
    )


def test_the_known_breakage_has_not_been_silently_repaired(worlds_present: None) -> None:
    """⭐ The other half of a ratchet, and the half people forget.

    If somebody fixes the four refs, this fails and tells them to set
    `KNOWN_UNMOUNTED = 0` — which converts the ratchet into the flat assertion it
    always wanted to be. Without this, a repair leaves a permanent allowance for
    four broken riders and the next four slip in under it.
    """
    unmounted = [row for row in _riders() if not row[2].get("mounted_on")]
    assert len(unmounted) == KNOWN_UNMOUNTED, (
        f"only {len(unmounted)} rider enemies are unmounted, but this file still "
        f"allows {KNOWN_UNMOUNTED}. Somebody repaired them — thank you. Set "
        f"`KNOWN_UNMOUNTED = {len(unmounted)}` so the allowance does not outlive "
        f"the defect it was written for."
    )
