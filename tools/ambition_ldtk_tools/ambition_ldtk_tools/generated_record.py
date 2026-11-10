"""Which LDtk levels a generator owns, and whether someone edited them.

A generator (a `make_specs.py`) writes area specs and applies them with
`area create --replace-existing`, which builds the whole level again. An edit
made in the LDtk editor inside that level is lost. This module keeps a record
beside each world, `<world>.generated.json`, so that does not happen in
silence:

- After a generator applies its rooms, it records a fingerprint of each level
  as it left it (`record`). `district_kit.make_rooms` does this.
- Before `area create` replaces a level, it compares the level with its
  record (`check`). A level that changed since the record was edited by hand
  (or by a tool other than its generator), and `area create` refuses to
  replace it.
- An author who takes a level over marks it LDtk-owned (`adopt`). Its
  generator then skips it, and the LDtk file is the truth for it.

The fingerprint leaves out what an edit cannot lose: the level's place in the
world (the LDtk editor owns placement, see `area create --keep-position`),
uids, iids and the tiles LDtk derives from IntGrid values.

Usage:

    python -m ambition_ldtk_tools level ownership status --ldtk <world>
    python -m ambition_ldtk_tools level ownership adopt <level>... --ldtk <world>
    python -m ambition_ldtk_tools level ownership release <level>... --ldtk <world>
    python -m ambition_ldtk_tools level ownership survey <spec>... --ldtk <world> [--settle]

`survey` answers, for a world no generator has recorded yet, which of its
levels a generator still makes: it applies the specs, in order, to a copy of
the world and compares each level it made with the live one. A level that
matches can be recorded as generated; one that differs was edited after its
generator ran, and recording it would discard that edit at the next apply.
It replays `area create` alone, not what a generator does to its rooms after
(painting ladders, adding entities), so it calls such a room changed:
`python -m ambition_ldtk_tools.generator_survey` runs the whole generator.
`--settle` acts on the answer: each reproduced level is recorded as
generated, and each that differs is adopted (LDtk-owned).
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

SCHEMA = "ambition.ldtk_generated_record.v1"


def record_path(ldtk: Path) -> Path:
    """The record of `ldtk`. It sits beside the real file (a world under
    `game/*/assets/worlds/` is a symlink into the map submodule)."""
    real = Path(ldtk).resolve()
    return real.with_name(real.stem + ".generated.json")


def load(ldtk: Path) -> dict:
    path = record_path(ldtk)
    if not path.exists():
        return {"schema": SCHEMA, "generated": {}, "ldtk_owned": []}
    data = json.loads(path.read_text())
    if data.get("schema") != SCHEMA:
        raise SystemExit(f"{path}: unknown schema {data.get('schema')!r}")
    data.setdefault("generated", {})
    data.setdefault("ldtk_owned", [])
    return data


def save(ldtk: Path, data: dict) -> None:
    data["ldtk_owned"] = sorted(set(data["ldtk_owned"]))
    data["generated"] = dict(sorted(data["generated"].items()))
    record_path(ldtk).write_text(json.dumps(data, indent=2) + "\n")


def _plain(value):
    """`value` with every whole float as an int, so that 64 and 64.0 (which
    the editor and the tools can write for one number) are equal."""
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, list):
        return [_plain(v) for v in value]
    if isinstance(value, dict):
        return {k: _plain(v) for k, v in value.items()}
    return value


def _fields(instances) -> list:
    return sorted([f.get("__identifier"), _plain(f.get("__value"))] for f in instances or [])


def fingerprint(level: dict) -> str:
    """A hash of what an edit inside `level` can change."""
    layers = []
    for layer in level.get("layerInstances") or []:
        entities = sorted(
            [
                e.get("__identifier"),
                _plain(e.get("px")),
                _plain(e.get("width")),
                _plain(e.get("height")),
                _fields(e.get("fieldInstances")),
            ]
            for e in layer.get("entityInstances") or []
        )
        tiles = sorted([_plain(t.get("px")), _plain(t.get("src")), t.get("f"), t.get("t")] for t in layer.get("gridTiles") or [])
        layers.append([layer.get("__identifier"), layer.get("intGridCsv") or [], tiles, entities])
    layers.sort(key=lambda row: str(row[0]))
    body = [_plain(level.get("pxWid")), _plain(level.get("pxHei")), _fields(level.get("fieldInstances")), layers]
    return hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode()).hexdigest()[:16]


def _levels(ldtk: Path) -> dict:
    return {level["identifier"]: level for level in json.loads(Path(ldtk).read_text()).get("levels", [])}


def record(ldtk: Path, level_ids) -> None:
    """Record each of `level_ids` as its generator leaves it now."""
    data = load(ldtk)
    levels = _levels(ldtk)
    for level_id in level_ids:
        if level_id in levels:
            data["generated"][level_id] = fingerprint(levels[level_id])
    save(ldtk, data)


def check(ldtk: Path, level: dict) -> str | None:
    """Why `area create` may not replace `level` in `ldtk`, or None."""
    data = load(ldtk)
    level_id = level.get("identifier")
    if level_id in data["ldtk_owned"]:
        return (
            f"level {level_id!r} is LDtk-owned ({record_path(ldtk).name}): the LDtk file is its truth. "
            f"Edit it in LDtk, or `level ownership release {level_id}` to give it back to its generator."
        )
    was = data["generated"].get(level_id)
    if was is not None and was != fingerprint(level):
        return (
            f"level {level_id!r} changed since its generator last wrote it: an edit in LDtk would be lost. "
            f"Keep the edit with `level ownership adopt {level_id}` (the generator then skips the level) "
            f"or move it into the generator; `--overwrite-hand-edits` discards it."
        )
    return None


def status(ldtk: Path) -> list[tuple[str, str]]:
    """Each level of `ldtk` and its owner: `generated`, `HAND-EDITED`,
    `ldtk-owned` or `unrecorded` (no generator has recorded it)."""
    data = load(ldtk)
    rows = []
    for level_id, level in _levels(ldtk).items():
        if level_id in data["ldtk_owned"]:
            rows.append((level_id, "ldtk-owned"))
        elif level_id not in data["generated"]:
            rows.append((level_id, "unrecorded"))
        elif data["generated"][level_id] == fingerprint(level):
            rows.append((level_id, "generated"))
        else:
            rows.append((level_id, "HAND-EDITED"))
    return rows


def _create(spec: Path, world: Path, package_root: Path):
    import subprocess
    import sys

    return subprocess.run(
        [sys.executable, "-m", "ambition_ldtk_tools", "area", "create", str(spec), "--ldtk", str(world),
         "--replace-existing", "--keep-position"],
        cwd=package_root, capture_output=True, text=True,
    )


def survey(ldtk: Path, specs: list[Path]) -> list[tuple[str, str]]:
    """Each level the `specs` make, and whether the live `ldtk` holds it as
    they make it: `reproduced`, `DIFFERS`, `absent` (not in the live
    world) or `REFUSED` (`area create` will not make it in this world now). The live world is not written."""
    import shutil
    import tempfile

    from ambition_ldtk_tools import ron_parse

    live = _levels(ldtk)
    package_root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory() as scratch:
        copy = Path(scratch) / Path(ldtk).resolve().name
        shutil.copyfile(Path(ldtk).resolve(), copy)
        made, refused = [], []
        for n, spec in enumerate(specs):
            text = Path(spec).read_text()
            level_id = str(ron_parse.load(text)["level_id"])
            # A link into another world file places its door in that file,
            # which the copy does not hold: make the level without it.
            local = Path(scratch) / f"{n}_{Path(spec).name}"
            local.write_text(text)
            done = _create(local, copy, package_root)
            out = done.stdout + done.stderr
            if done.returncode != 0 and "connect_to target_room" in out and "not found" in out:
                import re

                local.write_text(re.sub(r"connect_to: \[.*?\n    \],", "connect_to: [],", text, flags=re.S))
                done = _create(local, copy, package_root)
                out = done.stdout + done.stderr
            if done.returncode != 0 and "wrote new level" not in out and "replaced level" not in out:
                # The spec cannot be made in this world any more (a door it
                # places collides with what the world holds now).
                refused.append(level_id)
                continue
            made.append(level_id)
        after = _levels(copy)
    rows = [(level_id, "REFUSED") for level_id in refused]
    for level_id in made:
        if level_id not in live:
            rows.append((level_id, "absent"))
        elif fingerprint(live[level_id]) == fingerprint(after[level_id]):
            rows.append((level_id, "reproduced"))
        else:
            rows.append((level_id, "DIFFERS"))
    return rows


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["status", "adopt", "release", "survey"])
    parser.add_argument("levels", nargs="*")
    parser.add_argument("--ldtk", type=Path, required=True)
    parser.add_argument("--settle", action="store_true", help="survey: record what is reproduced, adopt what differs")
    args = parser.parse_args(argv)
    if args.action == "status":
        rows = status(args.ldtk)
        for level_id, owner in rows:
            print(f"{owner:12} {level_id}")
        edited = [level_id for level_id, owner in rows if owner == "HAND-EDITED"]
        print(f"{len(rows)} level(s); {len(edited)} hand-edited since their generator wrote them")
        return 1 if edited else 0
    if args.action == "survey":
        if not args.levels:
            parser.error("survey needs the generator's area specs, in the order it applies them")
        rows = survey(args.ldtk, [Path(spec) for spec in args.levels])
        for level_id, verdict in rows:
            print(f"{verdict:12} {level_id}")
        differs = [level_id for level_id, verdict in rows if verdict != "reproduced"]
        print(f"{len(rows)} level(s); {len(differs)} not as the specs make them")
        if args.settle:
            record(args.ldtk, [level_id for level_id, verdict in rows if verdict == "reproduced"])
            data = load(args.ldtk)
            data["ldtk_owned"] = [*data["ldtk_owned"], *(level_id for level_id, verdict in rows if verdict in ("DIFFERS", "REFUSED"))]
            for level_id in data["ldtk_owned"]:
                data["generated"].pop(level_id, None)
            save(args.ldtk, data)
            print(f"settled: recorded {len(rows) - len(differs)}, adopted {len(differs)} ({record_path(args.ldtk)})")
            return 0
        return 1 if differs else 0
    if not args.levels:
        parser.error(f"{args.action} needs at least one level")
    data = load(args.ldtk)
    known = _levels(args.ldtk)
    missing = [level_id for level_id in args.levels if level_id not in known]
    if missing:
        raise SystemExit(f"{args.ldtk} has no level {', '.join(missing)}")
    if args.action == "adopt":
        data["ldtk_owned"] = [*data["ldtk_owned"], *args.levels]
    else:
        data["ldtk_owned"] = [level_id for level_id in data["ldtk_owned"] if level_id not in args.levels]
        # The generator's next apply writes the level again; until then there is
        # no fingerprint of its own to compare with.
        for level_id in args.levels:
            data["generated"].pop(level_id, None)
    save(args.ldtk, data)
    print(f"{args.action}: {', '.join(args.levels)} ({record_path(args.ldtk)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
