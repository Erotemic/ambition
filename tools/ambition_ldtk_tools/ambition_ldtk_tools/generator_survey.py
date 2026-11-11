"""Which rooms a generator still makes as its world holds them.

`level ownership survey` replays a generator's specs alone. Most generators
also edit their rooms in place after `area create` (they paint ladders, add
entities), so that replay calls a room the generator still makes "changed".
This runs the whole generator instead: its `--apply`, on the live worlds,
with the specs it last wrote when it keeps them beside it (a generator in the
map submodule makes them again; its rooms are the levels of those specs). It fingerprints every
level before and after, then puts the worlds (and kept specs) back from git.

    uv run --project tools/ambition_ldtk_tools python -m ambition_ldtk_tools.generator_survey \
        game/ambition_map_assets/ambition_content/generators/<generator>/make_specs.py [--settle]

`--settle` then records each room the run left as it was as generated, and
adopts (LDtk-owned) each room of the generator's that the run changed (it was
edited after the generator ran, by hand or by another script) or did not
reach. A room already adopted stays adopted.

A sprite missing from this machine's art publication does not stop the run:
the committed specs already hold its placement. The worlds and the specs must
be clean in git; the run refuses otherwise.
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import re
import runpy
import subprocess
import sys
from pathlib import Path

from ambition_ldtk_tools import generated_record
from ambition_ldtk_tools.ldtk.paths import default_worlds_dir

PACKAGE_ROOT = Path(__file__).resolve().parents[1]


def _worlds() -> Path:
    """The folder the world files ARE in. `game/*/assets/worlds/` holds
    symlinks into the map submodule; git there sees the links, not the files,
    so a check or a restore there would see and undo nothing."""
    folders = {world.resolve().parent for world in default_worlds_dir().glob("*.ldtk")}
    if len(folders) != 1:
        raise SystemExit(f"the worlds are not in one folder: {sorted(map(str, folders))}")
    return folders.pop()


def _snapshot() -> dict[str, dict[str, str]]:
    return {
        world.name: {level["identifier"]: generated_record.fingerprint(level) for level in json.loads(world.read_text())["levels"]}
        for world in sorted(_worlds().glob("*.ldtk"))
    }


def _git(cwd: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True).stdout


def _folders(generator: Path) -> list[Path]:
    """What a run writes that the survey puts back: the worlds, and the
    generator's folder when it keeps its specs there."""
    return [_worlds(), *([generator.parent] if any(generator.parent.glob("*.ron")) else [])]


def _dirty(generator: Path) -> str:
    return "".join(_git(folder, "status", "--porcelain", ".") for folder in _folders(generator))


def _restore(generator: Path) -> None:
    for folder in _folders(generator):
        _git(folder, "checkout", "--", ".")
        _git(folder, "clean", "-fdq", ".")


def _level_id(spec_text: str) -> str | None:
    m = re.search(r'level_id:\s*"?([A-Za-z0-9_]+)', spec_text)
    return m.group(1) if m else None


def _run(generator: Path) -> tuple[int, str, set[str]]:
    """Run `generator --apply` in this process, with its committed specs if
    it keeps them. Returns its exit, its output, and the levels of the specs
    it wrote or kept (its own rooms)."""
    # As `python <generator>` would have it: its own folder first, then the
    # shared generator code beside it.
    sys.path[:0] = [str(generator.parent), str(generator.parent.parent), str(PACKAGE_ROOT / "specs")]
    import district_scenery

    body_box = district_scenery.body_box

    def lenient_body_box(kind):
        try:
            return body_box(kind)
        except SystemExit:
            return (0, 0, 1, 1)

    district_scenery.body_box = lenient_body_box
    write_text = Path.write_text
    own: set[str] = set()

    def keep_committed_specs(self, *args, **kwargs):
        if self.suffix == ".ron":
            committed = self.resolve().parent == generator.parent and self.exists()
            if level_id := _level_id(self.read_text() if committed else str(args[0]) if args else ""):
                own.add(level_id)
            if committed:
                return len(args[0]) if args else 0
        return write_text(self, *args, **kwargs)

    out = io.StringIO()
    argv, cwd = sys.argv, Path.cwd()
    Path.write_text = keep_committed_specs
    sys.argv = [str(generator), "--apply"]
    code = 0
    try:
        os.chdir(PACKAGE_ROOT)
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            runpy.run_path(str(generator), run_name="__main__")
    except SystemExit as exit_:
        code = exit_.code if isinstance(exit_.code, int) else (0 if exit_.code is None else 1)
    finally:
        Path.write_text = write_text
        district_scenery.body_box = body_box
        sys.argv = argv
        os.chdir(cwd)
    return code, out.getvalue(), own


def survey(generator: Path) -> dict[str, list[tuple[str, str]]]:
    """Per world: (level, `reproduced` | `DIFFERS` | `NOT MADE` | `TOUCHED`).
    NOT MADE is a level of the generator's that this run did not reach;
    TOUCHED is a level of another generator's that this one changed."""
    generator = generator.resolve()
    if dirty := _dirty(generator):
        raise SystemExit(f"refused: the worlds or the generator's folder are not clean in git:\n{dirty}")
    before = _snapshot()
    try:
        # The ownership guard refuses a run over an edited or adopted room
        # before it makes anything. A survey measures what the generator
        # makes, on worlds it puts back, so it runs without the records
        # (restored with the worlds); settling keeps an adopted room adopted.
        for record in _worlds().glob("*.generated.json"):
            record.unlink()
        code, out, own = _run(generator)
        after = _snapshot()
    finally:
        _restore(generator)
    made = set(re.findall(r"wrote new level '([^']+)'", out))
    if not made:
        raise SystemExit(f"{generator}: the run made no level (exit {code}):\n{out[-2000:]}")
    if code != 0:
        # It made rooms and then stopped (often its last `validate`, on a
        # finding elsewhere in the world): the rooms it made are measured; a
        # room of its own it did not reach is NOT MADE.
        print(f"note: {generator.name} exited {code} after making {len(made)} level(s)", file=sys.stderr)
    rows: dict[str, list[tuple[str, str]]] = {}
    for world, levels in after.items():
        for level_id, print_ in sorted(levels.items()):
            was = before.get(world, {}).get(level_id)
            if level_id in own and was is not None:
                verdict = "NOT MADE" if level_id not in made else "reproduced" if was == print_ else "DIFFERS"
                rows.setdefault(world, []).append((level_id, verdict))
            elif was != print_ and was is not None:
                rows.setdefault(world, []).append((level_id, "TOUCHED"))
    return rows


def main(argv=None) -> int:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("generator", type=Path)
    parser.add_argument("--settle", action="store_true")
    args = parser.parse_args(argv)
    rows = survey(args.generator)
    for world, verdicts in rows.items():
        print(f"## {world}")
        for level_id, verdict in verdicts:
            print(f"{verdict:12} {level_id}")
    if args.settle:
        for world, verdicts in rows.items():
            path = _worlds() / world
            generated_record.record(path, [level_id for level_id, verdict in verdicts if verdict == "reproduced"])
            data = generated_record.load(path)
            adopted = (level_id for level_id, verdict in verdicts if verdict in ("DIFFERS", "NOT MADE"))
            data["ldtk_owned"] = list(dict.fromkeys([*data["ldtk_owned"], *adopted]))
            for level_id in data["ldtk_owned"]:
                data["generated"].pop(level_id, None)
            generated_record.save(path, data)
            print(f"settled {world}: {generated_record.record_path(path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
