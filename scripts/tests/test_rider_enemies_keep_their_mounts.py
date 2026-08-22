"""Guard authored rider-to-mount links in LDtk content.

Every rider-like brain must name a live `mounted_on` entity whose authored box
overlaps the rider. Geometry is the invariant rather than a count or equal origin,
so new rider pairs are covered and riders standing on a mount's back remain valid."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]

#: The map submodule. Every `.ldtk` under it is authored content, reached from
#: `game/<crate>/assets/worlds` through tracked symlinks.
MAP_ASSETS = REPO / "game" / "ambition_map_assets"

#: A brain whose name says it rides something. Substring match on purpose — the
#: authored values are `pirate_shark_rider`, `pirate_heavy_shark_rider`,
#: `PhaseScript:gnu_ton_rider`, and the pre-`5e4d6448e` fused spellings were
#: `pirate_on_shark` / `pirate_heavy_on_shark`.
#: matching the CAPABILITY the name claims, not an enumerated list, so a new
#: rider archetype is covered the day it is authored.
RIDER_MARKERS = ("_rider", "_on_shark")

#: The field that carries the link.
MOUNT_FIELD = "mounted_on"


class Spawn:
    """One authored entity instance, with the geometry the invariant needs."""

    def __init__(self, world: str, level: str, entity: dict) -> None:
        self.world = world
        self.level = level
        self.iid = entity.get("iid")
        self.identifier = entity.get("__identifier")
        self.px = tuple(entity.get("px") or (0, 0))
        self.size = (entity.get("width") or 0, entity.get("height") or 0)
        self.fields = {
            f["__identifier"]: f.get("__value") for f in entity.get("fieldInstances", [])
        }

    @property
    def brain(self) -> str:
        return self.fields.get("brain") or ""

    @property
    def is_rider(self) -> bool:
        return any(marker in self.brain for marker in RIDER_MARKERS)

    @property
    def mount_iid(self) -> str | None:
        """The `entityIid` this instance's `mounted_on` names, if it has one.

        ⚠ LDtk stores a set EntityRef as `{entityIid, layerIid, levelIid,
        worldIid}` and an unset one as `null`; some exporters flatten it to the
        bare iid string. `field_entity_ref` (Rust) reads both, so this does too —
        a check that only understood one shape would go green on the other.
        """
        value = self.fields.get(MOUNT_FIELD)
        if isinstance(value, dict):
            return value.get("entityIid") or None
        if isinstance(value, str) and value:
            return value
        return None

    def overlaps(self, other: "Spawn") -> bool:
        ax, ay = self.px
        aw, ah = self.size
        bx, by = other.px
        bw, bh = other.size
        return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah

    def __str__(self) -> str:
        return (
            f"{self.world}:{self.level} {self.identifier} {self.iid} "
            f"{self.fields.get('name')!r} brain={self.brain!r} "
            f"px={list(self.px)} size={list(self.size)}"
        )


def _levels() -> list[tuple[str, str, list[Spawn]]]:
    """`(world, level, spawns)` for every authored level in the map submodule.

    ⚠ parsed, not grepped: the worlds are submodule content reached through
    symlinks, and a `grep -r` skips those.
    """
    out: list[tuple[str, str, list[Spawn]]] = []
    for path in sorted(MAP_ASSETS.rglob("*.ldtk")):
        data = json.loads(path.read_text(encoding="utf8"))
        for level in data.get("levels", []):
            level_id = level.get("identifier", "?")
            spawns = [
                Spawn(path.name, level_id, entity)
                for layer in level.get("layerInstances", [])
                for entity in layer.get("entityInstances", [])
            ]
            out.append((path.name, level_id, spawns))
    return out


def _riders() -> list[Spawn]:
    return [s for _, _, spawns in _levels() for s in spawns if s.is_rider]


@pytest.fixture(scope="module")
def worlds_present() -> None:
    """Skip when map assets are absent from a non-recursive checkout."""
    if not MAP_ASSETS.is_dir() or not any(MAP_ASSETS.rglob("*.ldtk")):
        pytest.skip("game/ambition_map_assets is not checked out")


def test_there_are_rider_enemies_at_all(worlds_present: None) -> None:
    """⛔ kills the vacuous pass.

    Every assertion below iterates the rider set. If a rename retires
    `_rider`/`_on_shark`, this file would go green while checking nothing — the
    exact shape of a check that cannot fail. Then `RIDER_MARKERS` is what needs
    updating, not this test deleting.
    """
    assert _riders(), (
        f"no spawn brain matches {RIDER_MARKERS}, so every other assertion "
        f"in this file is vacuous. Either the rider archetypes were renamed — "
        f"update RIDER_MARKERS — or they were removed, in which case delete this "
        f"file and say so."
    )


def test_every_rider_rides_something(worlds_present: None) -> None:
    """A rider brain must carry a `mounted_on` entity-ref.

    ⚠ this is the assertion an EDITOR SESSION breaks, which is the whole reason
    it exists — opening a world to move one platform can null every `EntityRef`
    in the FILE, not just in the level being edited.
    """
    unmounted = [str(rider) for rider in _riders() if rider.mount_iid is None]
    assert not unmounted, (
        f"{len(unmounted)} rider(s) have no mount:\n  " + "\n  ".join(unmounted) + "\n\n"
        f"An LDtk editor session nulls every `EntityRef` in a world file — see "
        f"this file's docstring and queue D49. ⛔ Do NOT satisfy this by deleting "
        f"riders; re-author the `{MOUNT_FIELD}` refs (the tool verb is "
        f"`entity set-field`, which resolves a target iid into a full EntityRef)."
    )


def test_every_mount_ref_lands_on_a_body_the_rider_is_touching(
    worlds_present: None,
) -> None:
    """⭐ The link has to be a link: it resolves, and the two bodies TOUCH.

    Three failures collapse into one assertion — a ref naming an iid that no
    longer exists, a ref naming an entity in some other level, and a ref naming
    a real entity the rider is nowhere near (the shape a careless repair takes,
    because every pirate is interchangeable in the JSON and only one of the four
    sharks is under any given one).
    """
    broken: list[str] = []
    checked = 0
    for world, level, spawns in _levels():
        by_iid = {s.iid: s for s in spawns}
        for spawn in spawns:
            mount_iid = spawn.mount_iid
            if mount_iid is None:
                continue
            checked += 1
            mount = by_iid.get(mount_iid)
            if mount is None:
                broken.append(
                    f"{spawn} -> {MOUNT_FIELD} names {mount_iid!r}, which is not "
                    f"an entity in {world}:{level}"
                )
                continue
            if not spawn.overlaps(mount):
                broken.append(
                    f"{spawn} -> rides {mount_iid!r} at px={list(mount.px)} "
                    f"size={list(mount.size)}, which its authored box does not "
                    f"touch. A rider sits ON its mount."
                )
    assert not broken, "a mount ref does not land on its mount:\n  " + "\n  ".join(
        broken
    )
    # THE FLOOR, and it is derived rather than typed in. A "for every ref …"
    # check reads exactly like a pass when there are no refs, which is precisely
    # the state the editor left `sandbox.ldtk` in for a month. Every rider brain
    # carries one ref, so the number of refs this loop saw can never honestly be
    # under the number of riders in the tree.
    riders = len(_riders())
    assert checked >= riders, (
        f"only {checked} `{MOUNT_FIELD}` ref(s) reached this check but the tree "
        f"authors {riders} rider(s), so it is looking at fewer pairs than exist "
        f"and its silence means nothing"
    )
