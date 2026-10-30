"""The parts the gate districts' room generators share.

`pirate_port/make_specs.py` and `ninja_village/make_specs.py` write each room
of their district as an `area create` spec and, with `--apply`, make the rooms
in the district's own world file. This module writes the spec text (one
entity a line) and runs the tool.

The robot's numbers (`platformer_defaults.ron`, measured in the game): a jump
rises 83 px, a jump and a double jump 139 px, a run is 270 px/s, the body is
30 x 48. A step up of 64 px or less is one jump.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

WORLDS = default_worlds_dir()
SANDBOX_LDTK = WORLDS / "sandbox.ldtk"
HUB_ROOM = "central_hub_complex"
WALL = 16


def _value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return f"{v:.1f}" if v == int(v) else repr(v)
    if isinstance(v, int):
        return str(v)
    return '"' + str(v).replace('"', '\\"') + '"'


def entity(etype, x, y, w, h, **fields) -> str:
    field_text = ", ".join(f"{k}: {_value(v)}" for k, v in fields.items())
    return f'( type: "{etype}", px: [{round(x)}, {round(y)}], size: [{round(w)}, {round(h)}], fields: ( {field_text} ) ),'


def solid(name, x, y, w, h):
    return entity("Solid", x, y, w, h, name=name)


def ledge(name, x, y, w):
    """A one-way platform whose top is at `y`."""
    return entity("OneWayPlatform", x, y, w, 16, name=name)


def breakable_wall(id, x, y, w, h, look="", hp=3, name=None):
    """A solid wall that breaks after `hp` hits and stays broken in the save
    (`respawn: Persistent`). `id` keys the save: it must be unique in the
    world file and stay the same each time the room is made. `look` is the
    prop sheet that draws it; empty draws the generic breakable sprite."""
    return entity("BreakablePlatform", x, y, w, h, id=id, name=name or id.replace("_", " "),
                  max_hp=float(hp), respawn="Persistent", collision="Solid", trigger="OnHit", look=look)


def door(zone, name, x, floor, to_room, to_zone, bidirectional=True):
    """A door standing on `floor`, its left at `x`."""
    return entity("LoadingZone", x, floor - 96, 48, 96, id=zone, name=name, activation="Door",
                  target_room=to_room, target_zone=to_zone, bidirectional=bidirectional)


def pad(zone, name, x, floor):
    """An arrival pad: a zone that a way in names and that leads nowhere."""
    return entity("LoadingZone", x, floor - 96, 48, 96, id=zone, name=name, activation="Door",
                  target_room="", target_zone="", bidirectional=False)


def edge(zone, name, x, floor, to_room, to_zone, h=96):
    """An edge exit 16 wide at `x`, from `floor` up `h`."""
    return entity("LoadingZone", x, floor - h, 16, h, id=zone, name=name, activation="EdgeExit",
                  target_room=to_room, target_zone=to_zone, bidirectional=True)


def start(rid, x, floor):
    return entity("PlayerStart", x - 14, floor - 46, 28, 46, name=f"{rid}_spawn")


def resident(character, dialogue, x, floor, brain="stand_still", w=32, h=64):
    """A resident who stands at `x` on `floor` and talks: `dialogue` is the
    Yarn node."""
    return entity("NpcSpawn", x - w // 2, floor - h, w, h, character_id=character, prompt="Talk",
                  dialogue_id=dialogue, brain_override=brain)


def sign(name, x, y, text, w=360):
    """A line of writing with its middle over `x`."""
    return entity("DebugLabel", x - w // 2, y, w, 24, name=name, text=text, category="Custom")


def prop(name, kind, x, y, w, h):
    return entity("Prop", x, y, w, h, name=name, kind=kind)


def water(wid, x, y, w, h):
    return entity("WaterVolume", x, y, w, h, id=wid, name=wid.replace("_", " "))


def room(rid, entities, *, at, size, biome, music, bg, title=None, generator, zoom=None, extra=""):
    """The spec text of a room: its camera and title are added here."""
    world_x, world_y = at
    w, h = size
    cam = {"zoom": zoom} if zoom else {}
    body = list(entities) + [entity("CameraZone", 0, 0, w, h, id=f"{rid}_camera", name=f"{rid}_camera", mode="Default", **cam)]
    if title:
        body.append(sign(f"{rid}_title", w // 2, 40, title, min(900, w - 140)))
    text = "\n        ".join(body)
    return f"""// Written by {generator}. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{bg}",
    biome: "{biome}",
    parallax_theme: "{biome}",
    music_track: "{music}",{extra}
    entities: [
        {text}
    ],
    connect_to: [],
)
"""


def run(*args) -> tuple[int, str]:
    done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", *args], cwd=TOOLS, capture_output=True, text=True)
    print(done.stdout + done.stderr)
    return done.returncode, done.stdout + done.stderr


def ensure_world(world: Path, uid_offset: int) -> int:
    """Scaffold the world file when it is not there yet."""
    if world.exists():
        return 0
    code, _ = run("world", "init", str(world), "--source", str(SANDBOX_LDTK), "--uid-offset", str(uid_offset))
    return code


def make_rooms(world: Path, specs: list[Path], keep_position: bool = False) -> int:
    """Make each room again from its spec. Each room's doors name the other
    rooms, so the world is whole only after the last one: until then it is
    enough that each was written. `keep_position`: a room that exists stays
    where the LDtk editor has it (`area create --keep-position`)."""
    keep = ["--keep-position"] if keep_position else []
    for path in specs:
        code, out = run("area", "create", str(path), "--ldtk", str(world), "--replace-existing", *keep)
        if code != 0 and "wrote new level" not in out and "replaced level" not in out:
            return code
    return 0


def add_entities(yaml_text: str, ldtk: Path, scratch: Path, *, unless: str | None = None) -> int:
    """`entity add` of `yaml_text` into `ldtk`, unless `unless` is already in
    the file (an id the yaml adds)."""
    if unless and unless in ldtk.read_text():
        return 0
    scratch.write_text(yaml_text)
    code, _ = run("entity", "add", str(scratch), "--ldtk", str(ldtk), "--in-place")
    scratch.unlink()
    return code


def paint(world: Path, room_id: str, layer: str, value: int, x: int, y: int, w: int, h: int) -> int:
    code, _ = run("intgrid", "paint", "--ldtk", str(world), "--level", room_id, "--layer", layer,
                  "--value", str(value), "--px", f"{x},{y}", "--size", f"{w},{h}")
    return code
