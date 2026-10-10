"""The packager writes the bundle the game reads.

`scripts/package_single_exe.py` writes the asset tree after the program, and
`crates/ambition_asset_manager/src/exe_bundle.rs` reads it. Two programs in two
languages hold the one layout, so one small file holds it for both:
`crates/ambition_asset_manager/tests/data/exe_bundle_golden.bin`. The arm here
proves the writer makes those bytes, and the Rust arm
`the_packager_s_bundle_reads_back` proves the reader reads them. A change to
the layout on one side fails one of the two.
"""

import importlib.util
import json
import pathlib
import sys

import pytest

REPO = pathlib.Path(__file__).resolve().parents[2]
GOLDEN = REPO / "crates" / "ambition_asset_manager" / "tests" / "data" / "exe_bundle_golden.bin"

sys.path.insert(0, str(REPO / "scripts"))
_spec = importlib.util.spec_from_file_location(
    "package_single_exe_under_test", REPO / "scripts" / "package_single_exe.py"
)
packager = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = packager
_spec.loader.exec_module(packager)

# The same tree as the Rust arm reads. `PROGRAM\n` is the program.
GOLDEN_PROGRAM = b"PROGRAM\n"
GOLDEN_FILES = {
    "audio/music/a.txt": b"first file\n",
    "sprites/b.bin": bytes(range(7)),
    "worlds/empty.ldtk": b"",
}


def write_tree(tmp_path, program=GOLDEN_PROGRAM, files=GOLDEN_FILES):
    program_path = tmp_path / "program"
    program_path.write_bytes(program)
    sources = {}
    for index, (rel, data) in enumerate(files.items()):
        source = tmp_path / f"source_{index}"
        source.write_bytes(data)
        sources[rel] = source
    return program_path, sources


def test_the_writer_makes_the_bytes_the_game_reads(tmp_path):
    program, sources = write_tree(tmp_path)
    out = tmp_path / "out" / "game"
    packager.write_single_exe(program, sources, out)
    assert out.read_bytes() == GOLDEN.read_bytes()


def test_the_bundle_reads_back_as_the_tree_that_went_in(tmp_path):
    program, sources = write_tree(tmp_path)
    out = tmp_path / "game"
    entries = packager.write_single_exe(program, sources, out)
    payload_start, index = packager.read_index(out)
    assert payload_start == len(GOLDEN_PROGRAM)
    assert index == entries
    data = out.read_bytes()
    assert data[:payload_start] == GOLDEN_PROGRAM
    for rel, (offset, length) in index.items():
        assert data[payload_start + offset : payload_start + offset + length] == GOLDEN_FILES[rel]
    packager.verify_bundle(out, program, sources)


def test_the_file_keeps_the_mode_of_the_program(tmp_path):
    program, sources = write_tree(tmp_path)
    program.chmod(0o755)
    out = tmp_path / "game"
    packager.write_single_exe(program, sources, out)
    assert out.stat().st_mode & 0o777 == 0o755


def test_a_changed_asset_fails_the_read_back(tmp_path):
    program, sources = write_tree(tmp_path)
    out = tmp_path / "game"
    packager.write_single_exe(program, sources, out)
    # The same length, so only the bytes can tell.
    sources["sprites/b.bin"].write_bytes(bytes(range(1, 8)))
    with pytest.raises(SystemExit) as refused:
        packager.verify_bundle(out, program, sources)
    assert refused.value.code == 1


def test_a_program_that_carries_a_bundle_is_not_packaged_again(tmp_path):
    program, sources = write_tree(tmp_path)
    once = tmp_path / "once"
    packager.write_single_exe(program, sources, once)
    with pytest.raises(SystemExit) as refused:
        packager.write_single_exe(once, sources, tmp_path / "twice")
    assert refused.value.code == 1
    assert not (tmp_path / "twice").exists()


def test_a_plain_program_has_no_footer(tmp_path):
    program, _ = write_tree(tmp_path)
    assert packager.read_footer(program) is None


@pytest.mark.parametrize(
    "rel",
    [
        "sprites_0_5x/robot_spritesheet.png",
        "sprites_0_25x/robot_spritesheet.png",
        "sprites_potato/robot_spritesheet.png",
        "sprite_packs/half/ultrapack_0.png",
        "sprite_packs/quarter/ultrapack.json",
        "sprite_packs/potato/ultrapack_3.png",
        "backgrounds/parallax_layers_0_5x/cave_far.png",
        "backgrounds/parallax_layers_potato/cave_far.png",
    ],
)
def test_a_smaller_quality_tier_is_a_tier_file(rel):
    assert packager.is_tier_file(rel)


@pytest.mark.parametrize(
    "rel",
    [
        "sprites/robot_spritesheet.png",
        "sprites/potato_spritesheet.png",
        "sprite_packs/full/ultrapack_0.png",
        "backgrounds/parallax_layers/cave_far.png",
        "audio/music/half/track.ogg",
        "worlds/sandbox.ldtk",
    ],
)
def test_full_resolution_art_is_not_a_tier_file(rel):
    assert not packager.is_tier_file(rel)


def pack_tree(tmp_path, target):
    """A source set with each tier and a four-page pack. `target` has its
    frames on page 2."""
    catalog = {
        "pages": [f"ultrapack_{index}.png" for index in range(4)],
        "targets": {target: {"idle": [{"page": 2}]}, "another_target": {"idle": [{"page": 3}]}},
    }
    catalog_path = tmp_path / "ultrapack.json"
    catalog_path.write_text(json.dumps(catalog))
    blank = tmp_path / "blank"
    blank.write_bytes(b"")
    names = [
        "sprites/robot_spritesheet.png",
        "sprites_0_5x/robot_spritesheet.png",
        "sprite_packs/half/ultrapack_0.png",
        "backgrounds/parallax_layers/cave_far.png",
        "backgrounds/parallax_layers_potato/cave_far.png",
        "audio/music/track.ogg",
    ] + [f"sprite_packs/full/ultrapack_{index}.png" for index in range(4)]
    sources = {name: blank for name in names}
    sources["sprite_packs/full/ultrapack.json"] = catalog_path
    return sources


def a_target_a_prop_row_asks_the_pack_for():
    rows = packager.measure_pack_reachability.PROP_ROWS.read_text()
    targets = packager.measure_pack_reachability.opted_in_targets(rows)
    assert targets, "no prop row asks for the pack, so the reach rule has nothing to keep"
    return sorted(targets)[0]


def test_the_default_tree_keeps_full_resolution_art_and_the_pages_a_consumer_reaches(tmp_path):
    sources = pack_tree(tmp_path, a_target_a_prop_row_asks_the_pack_for())
    assert list(packager.select_assets(sources, "full")) == [
        "audio/music/track.ogg",
        "backgrounds/parallax_layers/cave_far.png",
        # Page 0: each consumer asks for it first. Page 2: the frames of the
        # one target a prop row asks for. Pages 1 and 3: no consumer.
        "sprite_packs/full/ultrapack.json",
        "sprite_packs/full/ultrapack_0.png",
        "sprite_packs/full/ultrapack_2.png",
        "sprites/robot_spritesheet.png",
    ]


def test_the_all_tree_keeps_each_file(tmp_path):
    sources = pack_tree(tmp_path, a_target_a_prop_row_asks_the_pack_for())
    assert list(packager.select_assets(sources, "all")) == sorted(sources)


def test_a_tree_with_no_pack_drops_only_the_tiers(tmp_path):
    sources = pack_tree(tmp_path, "any")
    sources = {rel: path for rel, path in sources.items() if not rel.startswith("sprite_packs/")}
    assert list(packager.select_assets(sources, "full")) == [
        "audio/music/track.ogg",
        "backgrounds/parallax_layers/cave_far.png",
        "sprites/robot_spritesheet.png",
    ]


TRACE = """\
7 openat(AT_FDCWD, "/out/game", O_RDONLY|O_CLOEXEC) = 3
7 openat(AT_FDCWD, "/out/game", O_RDONLY|O_CLOEXEC) = 4
8 openat(AT_FDCWD, "/repo/engine/assets/sprites/robot.png", O_RDONLY|O_CLOEXEC) = 5
8 openat(AT_FDCWD, "/repo/content/assets/worlds/sandbox.ldtk", O_RDONLY) = 6
8 openat(AT_FDCWD, "/repo/engine/assets/sprites/absent.png", O_RDONLY) = -1 ENOENT (No such file or directory)
9 openat(AT_FDCWD, "/somewhere/else/assets/fonts/a.ttf", O_RDONLY) = 7
9 openat(AT_FDCWD, "/usr/lib/x86_64-linux-gnu/libasound.so.2", O_RDONLY|O_CLOEXEC) = 8
9 openat(AT_FDCWD, "/home/player/data/ambition/save.ron", O_WRONLY|O_CREAT, 0666) = 9
"""


def test_the_trace_names_each_asset_the_program_opened_outside_itself():
    own_reads, outside = packager.opens_in_trace(TRACE, "/out/game", ("/repo/engine/assets", "/repo/content/assets"))
    assert own_reads == 2
    # The file that was absent opened nothing. A system library and a save
    # are not assets.
    assert outside == [
        "/repo/content/assets/worlds/sandbox.ldtk",
        "/repo/engine/assets/sprites/robot.png",
        "/somewhere/else/assets/fonts/a.ttf",
    ]


def test_a_program_that_reads_only_itself_has_a_clean_trace():
    clean = "\n".join(line for line in TRACE.splitlines() if "/assets/" not in line)
    assert packager.opens_in_trace(clean, "/out/game", ("/repo/engine/assets",)) == (2, [])
