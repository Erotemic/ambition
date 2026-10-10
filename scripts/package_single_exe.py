#!/usr/bin/env python3
"""Build the game as one file to give a player: the program with its assets inside.

    python3 scripts/package_single_exe.py                     # Linux and Windows
    python3 scripts/package_single_exe.py --platform windows
    python3 scripts/package_single_exe.py --skip-build        # package what is built
    python3 scripts/package_single_exe.py --assets all        # each quality tier

The result is `target/single_exe/ambition-linux-x86_64` and
`target/single_exe/ambition-windows-x86_64.exe`. Each is the `ship` profile
build of `ambition_game_bin` (optimized, link-time optimization, symbols
stripped) with the asset tree written after it. The game finds the tree in its
own file (`crates/ambition_asset_manager/src/exe_bundle.rs` has the layout),
so the player needs no other file and no directory beside it.

The asset tree is the one a packaged build has: the two asset roots as one
flat tree (`package_asset_guard.py`, the `steamdeck` profile). `--assets`
selects how much of it goes in:

    full   (default) the full-resolution art. The smaller quality tiers stay
           out. A player who selects a lower quality gets the full-resolution
           file: the game falls back to it when a tier is absent. The shared
           sprite pack keeps only the pages a consumer can reach
           (`measure_pack_reachability.py`).
    all    each file of each tier, as the Steam Deck package has.

The Windows build is a cross build from Linux. It needs, one time:

    sudo apt-get install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64
    rustup target add x86_64-pc-windows-gnu

The Linux file is one file, and it is not a static program: it loads the GPU
driver, the window system and the sound library of the machine at run time,
as each Linux game does. `--print-needs` lists what it asks the machine for.

The file must stand alone. So the last step for the Linux file runs it with no
window from an empty directory, under strace, with the asset variables
removed, and refuses the file if the game opened an asset outside itself.
Without that check the checkout fallback (an absolute `CARGO_MANIFEST_DIR`
path) finds a missing file on the build machine, and the file passes here and
fails on each other machine. The Windows file is not run: this script reads
its bundle back and compares each byte, and a person must start it on Windows.

Exit codes: 0 each file is built and verified; 1 a file failed its
verification; 2 a prerequisite is missing; 3 a build failed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import measure_pack_reachability  # noqa: E402
import package_asset_guard  # noqa: E402

PACKAGE = "ambition_app"
BINARY = "ambition_game_bin"
PROFILE = "ship"
FEATURES = "desktop_player"
ASSET_PROFILE = "steamdeck"
WINDOWS_TARGET = "x86_64-pc-windows-gnu"
WINDOWS_LINKER = "x86_64-w64-mingw32-gcc"

# The layout of `exe_bundle.rs`. Change both together.
FOOTER_MAGIC = b"AMBNDL01"
FOOTER = struct.Struct("<QQQ8s")

# The smaller quality tiers. The game asks for a tier file first and for the
# full-resolution file when the tier file is absent, so a tree without these
# draws the same game at the highest quality.
TIER_DIRECTORY = re.compile(r"(?:^|/)(?:[a-z_]+_(?:0_5x|0_25x|potato)|sprite_packs/(?:half|quarter|potato))/")


def is_tier_file(rel: str) -> bool:
    return TIER_DIRECTORY.search(rel) is not None


PLATFORMS = {
    "linux": {"target": None, "built": BINARY, "name": "ambition-linux-x86_64"},
    "windows": {"target": WINDOWS_TARGET, "built": f"{BINARY}.exe", "name": "ambition-windows-x86_64.exe"},
}

# The run that proves the file stands alone: the first room of the campaign.
SMOKE_ROOM = "central_hub_complex"
SMOKE_TICKS = "600"


def fail(code: int, message: str) -> None:
    print(f"package_single_exe: {message}", file=sys.stderr)
    raise SystemExit(code)


def say(message: str) -> None:
    print(f"package_single_exe: {message}", flush=True)


def megabytes(size: int) -> str:
    return f"{size / (1024 * 1024):.1f} MiB"


def built_path(target_dir: Path, platform: str) -> Path:
    spec = PLATFORMS[platform]
    base = target_dir / spec["target"] if spec["target"] else target_dir
    return base / PROFILE / spec["built"]


def build(target_dir: Path, platform: str, jobs: int | None) -> Path:
    if shutil.which("cargo") is None:
        fail(2, "cargo is not on PATH")
    spec = PLATFORMS[platform]
    command = [
        "cargo", "build", "-p", PACKAGE, "--bin", BINARY, "--profile", PROFILE,
        "--no-default-features", "--features", FEATURES,
    ]  # fmt: skip
    if jobs:
        command += ["-j", str(jobs)]
    if spec["target"]:
        if shutil.which(WINDOWS_LINKER) is None:
            fail(2, f"{WINDOWS_LINKER} is not on PATH; the module text names the two packages to install")
        command += ["--target", spec["target"]]
    # An optimized build gains nothing from the incremental cache, and the
    # cache of a build this size fills the disk.
    env = dict(os.environ, CARGO_TARGET_DIR=str(target_dir), CARGO_INCREMENTAL="0")
    say(f"{' '.join(command)}  (target {target_dir})")
    if subprocess.run(command, cwd=REPO, env=env).returncode != 0:
        fail(3, f"the {platform} build failed")
    binary = built_path(target_dir, platform)
    if not binary.is_file():
        fail(3, f"the build reported success and {binary} does not exist")
    return binary


def read_footer(path: Path) -> tuple[int, int, int] | None:
    """(payload start, index start, index length) of a file that carries a
    bundle, and `None` for a plain program."""
    size = path.stat().st_size
    if size < FOOTER.size:
        return None
    with path.open("rb") as stream:
        stream.seek(size - FOOTER.size)
        payload_start, index_start, index_len, magic = FOOTER.unpack(stream.read(FOOTER.size))
    if magic != FOOTER_MAGIC:
        return None
    return payload_start, index_start, index_len


def unreachable_pack_pages(sources: dict[str, Path]) -> set[str]:
    """The pages of the full-resolution sprite pack that no consumer can ask
    for. The reach is the one `measure_pack_reachability.py` reports."""
    catalog_rel = "sprite_packs/full/ultrapack.json"
    if catalog_rel not in sources:
        return set()
    targets = measure_pack_reachability.opted_in_targets(measure_pack_reachability.PROP_ROWS.read_text())
    if not targets:
        fail(1, f"no prop row in {measure_pack_reachability.PROP_ROWS} asks for the pack; the reach rule read nothing")
    catalog = json.loads(sources[catalog_rel].read_text())
    used = measure_pack_reachability.pages_used_by(catalog, targets)
    return {
        f"sprite_packs/full/{Path(name).name}"
        for index, name in enumerate(catalog["pages"])
        if index not in used
    }


def select_assets(sources: dict[str, Path], assets: str) -> dict[str, Path]:
    """The files of `sources` (package path to source file) that the `assets`
    choice carries, in the order of their paths."""
    if assets == "all":
        return dict(sorted(sources.items()))
    dropped = unreachable_pack_pages(sources)
    return {rel: path for rel, path in sorted(sources.items()) if not is_tier_file(rel) and rel not in dropped}


def collect_assets(assets: str) -> dict[str, Path]:
    """Package path to source file, for the tree this build carries."""
    # The file set only. `build_contract` also scans each manifest for a path
    # it declares, and that scan is the job of the package audit, not of this
    # script: the bundle holds each file of the two roots, declared or not.
    try:
        source_files = package_asset_guard.collect_source_files(REPO, ASSET_PROFILE)
    except package_asset_guard.AssetContractError as error:
        fail(1, f"the asset tree cannot be packaged:\n{error}")
    return select_assets({rel: item.source for rel, item in source_files.items()}, assets)


def write_single_exe(program: Path, files: dict[str, Path], out: Path) -> dict[str, tuple[int, int]]:
    """Write `program` and then `files` to `out`. Returns path to (offset in
    the payload, length)."""
    if read_footer(program) is not None:
        fail(1, f"{program} carries a bundle already; give this script the program cargo wrote")
    out.parent.mkdir(parents=True, exist_ok=True)
    scratch = out.with_name(out.name + ".part")
    entries: dict[str, tuple[int, int]] = {}
    with scratch.open("wb") as stream:
        with program.open("rb") as source:
            shutil.copyfileobj(source, stream, 1 << 20)
        payload_start = stream.tell()
        for rel, path in files.items():
            offset = stream.tell() - payload_start
            with path.open("rb") as source:
                shutil.copyfileobj(source, stream, 1 << 20)
            entries[rel] = (offset, stream.tell() - payload_start - offset)
        index_start = stream.tell()
        stream.write(struct.pack("<I", len(entries)))
        for rel, (offset, length) in entries.items():
            name = rel.encode("utf-8")
            stream.write(struct.pack("<I", len(name)))
            stream.write(name)
            stream.write(struct.pack("<QQ", offset, length))
        index_len = stream.tell() - index_start
        stream.write(FOOTER.pack(payload_start, index_start, index_len, FOOTER_MAGIC))
    shutil.copymode(program, scratch)
    scratch.replace(out)
    return entries


def read_index(path: Path) -> tuple[int, dict[str, tuple[int, int]]]:
    footer = read_footer(path)
    if footer is None:
        fail(1, f"{path} carries no bundle")
    payload_start, index_start, index_len = footer
    with path.open("rb") as stream:
        stream.seek(index_start)
        index = stream.read(index_len)
    (count,) = struct.unpack_from("<I", index, 0)
    at = 4
    entries = {}
    for _ in range(count):
        (name_len,) = struct.unpack_from("<I", index, at)
        at += 4
        name = index[at : at + name_len].decode("utf-8")
        at += name_len
        offset, length = struct.unpack_from("<QQ", index, at)
        at += 16
        entries[name] = (offset, length)
    if at != len(index):
        fail(1, f"{path}: the index has {len(index) - at} bytes after its last entry")
    return payload_start, entries


def verify_bundle(out: Path, program: Path, files: dict[str, Path]) -> None:
    """Read the bundle back: the program is the bytes cargo wrote, and each
    asset is the bytes of its source file."""
    payload_start, entries = read_index(out)
    if payload_start != program.stat().st_size:
        fail(1, f"{out}: the payload starts at {payload_start} and the program is {program.stat().st_size} bytes")
    if set(entries) != set(files):
        fail(1, f"{out}: the index names {len(entries)} files and the tree has {len(files)}")

    def digest_of(stream, length: int) -> str:
        digest = hashlib.sha256()
        while length:
            block = stream.read(min(length, 1 << 20))
            if not block:
                break
            digest.update(block)
            length -= len(block)
        return digest.hexdigest()

    with out.open("rb") as stream, program.open("rb") as source:
        if digest_of(stream, payload_start) != digest_of(source, payload_start):
            fail(1, f"{out}: the program at its front is not {program}")
        for rel, (offset, length) in entries.items():
            stream.seek(payload_start + offset)
            packed = digest_of(stream, length)
            if length != files[rel].stat().st_size or packed != package_asset_guard.sha256_path(files[rel]):
                fail(1, f"{out}: `{rel}` in the bundle is not the bytes of {files[rel]}")


def library_needs(binary: Path) -> list[str]:
    """The shared libraries the Linux program names in its header. The GPU
    driver, the window system and ALSA are not here: the program loads them by
    name while it runs."""
    if shutil.which("readelf") is None:
        return []
    listing = subprocess.run(["readelf", "-d", str(binary)], capture_output=True, text=True).stdout
    return re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", listing)


def opens_in_trace(trace: str, exe: str, asset_roots: tuple[str, ...]) -> tuple[int, list[str]]:
    """From an strace log of `open`/`openat`: how many times the program
    opened `exe`, and each asset file it opened that is not `exe`. An asset
    file is a file under one of `asset_roots` or under a directory `assets`.
    A call that failed opened nothing."""
    own_reads = 0
    outside = set()
    for line in trace.splitlines():
        if "= -1" in line:
            continue
        match = re.search(r'"([^"]*)"', line)
        if not match:
            continue
        opened = match.group(1)
        if opened == exe:
            own_reads += 1
        elif opened.startswith(asset_roots) or "/assets/" in opened:
            outside.add(opened)
    return own_reads, sorted(outside)


def verify_stands_alone(out: Path) -> None:
    """Run the Linux file with no window from an empty directory, into the
    first room of the game, and refuse it if it opened an asset file outside
    itself."""
    if shutil.which("strace") is None:
        fail(2, "strace is not on PATH; it is how the file proves it stands alone")
    asset_roots = (
        str(REPO / package_asset_guard.ACTOR_ASSET_ROOT),
        str(REPO / package_asset_guard.CONTENT_ASSET_ROOT),
    )
    env = {key: value for key, value in os.environ.items() if key not in ("BEVY_ASSET_ROOT", "CARGO_MANIFEST_DIR")}
    exe = str(out.resolve())
    with tempfile.TemporaryDirectory() as scratch:
        home = Path(scratch) / "home"
        cwd = Path(scratch) / "cwd"
        temp = Path(scratch) / "tmp"
        for directory in (home, cwd, temp):
            directory.mkdir()
        # The saves, the settings and the temporary files of the run go to
        # the scratch directory, not to the person's own.
        env.update(
            HOME=str(home),
            XDG_DATA_HOME=str(home / "data"),
            XDG_CONFIG_HOME=str(home / "config"),
            TMPDIR=str(temp),
            AMBITION_HEADLESS_GAMEPLAY_ROOM=SMOKE_ROOM,
        )
        log = Path(scratch) / "strace.log"
        result = subprocess.run(
            ["strace", "-f", "-e", "trace=openat,open", "-o", str(log), exe, "--headless", "--headless-ticks", SMOKE_TICKS],
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        own_reads, outside = opens_in_trace(log.read_text(errors="replace"), exe, asset_roots)
    if result.returncode != 0:
        fail(1, f"{out} exited {result.returncode} on its run with no window:\n{output[-4000:]}")
    if outside:
        listing = "\n".join(f"  {path}" for path in outside[:40])
        fail(1, f"{out} opened {len(outside)} asset files outside itself:\n{listing}")
    if own_reads < 2:
        fail(1, f"{out} opened itself {own_reads} times on its run, so it did not read its bundle")
    missing = sorted(set(re.findall(r"[Pp]ath not found: ([^\s\"']+)", output)))
    if missing:
        listing = "\n".join(f"  {path}" for path in missing[:40])
        fail(1, f"{out} asked for {len(missing)} asset files it does not carry:\n{listing}")
    if "gameplay_session=true" not in output:
        fail(1, f"{out} did not reach the room `{SMOKE_ROOM}` on its run with no window:\n{output[-4000:]}")
    say(
        f"{out.name}: ran {SMOKE_TICKS} ticks in `{SMOKE_ROOM}` with no window; "
        f"it opened itself {own_reads} times and no asset file outside itself"
    )


def package(platform: str, program: Path, files: dict[str, Path], out_dir: Path) -> Path:
    out = out_dir / PLATFORMS[platform]["name"]
    write_single_exe(program, files, out)
    verify_bundle(out, program, files)
    asset_bytes = out.stat().st_size - program.stat().st_size
    say(
        f"{out}: {megabytes(out.stat().st_size)} "
        f"(program {megabytes(program.stat().st_size)}, {len(files)} asset files {megabytes(asset_bytes)})"
    )
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--platform", choices=["linux", "windows", "both"], default="both")
    parser.add_argument("--assets", choices=["full", "all"], default="full")
    parser.add_argument("--target-dir", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target")))
    parser.add_argument("--out", type=Path, default=None, help="default: <target-dir>/single_exe")
    parser.add_argument("--jobs", "-j", type=int, default=None, help="cargo build jobs")
    parser.add_argument("--skip-build", action="store_true", help="package the programs the last build wrote")
    parser.add_argument("--skip-run", action="store_true", help="do not run the Linux file under strace")
    parser.add_argument("--print-needs", action="store_true", help="list the shared libraries the Linux file names")
    args = parser.parse_args()

    platforms = ["linux", "windows"] if args.platform == "both" else [args.platform]
    target_dir = args.target_dir.resolve()
    out_dir = (args.out or target_dir / "single_exe").resolve()

    programs = {}
    for platform in platforms:
        if args.skip_build:
            programs[platform] = built_path(target_dir, platform)
            if not programs[platform].is_file():
                fail(2, f"{programs[platform]} does not exist; run without --skip-build")
        else:
            programs[platform] = build(target_dir, platform, args.jobs)

    files = collect_assets(args.assets)
    say(f"asset tree `{args.assets}`: {len(files)} files, {megabytes(sum(p.stat().st_size for p in files.values()))}")

    for platform in platforms:
        out = package(platform, programs[platform], files, out_dir)
        if platform == "linux":
            if args.print_needs:
                say("the Linux file names these shared libraries: " + ", ".join(library_needs(out)))
            if not args.skip_run:
                verify_stands_alone(out)
        else:
            say(f"{out.name}: the bundle reads back; this script cannot run it, so start it on Windows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
