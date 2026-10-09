#!/usr/bin/env python3
"""Build the release artifact of Outlander, the SDK game (SDK-GAME SG2).

    python3 scripts/package_outlander.py [--out DIR] [--target-dir DIR]
    python3 scripts/package_outlander.py --measure

The artifact is one directory and a tarball of it:

    outlander/
      outlander_visible     the windowed game, release profile
      run.sh                sets BEVY_ASSET_ROOT to its own directory, then runs it
      assets/               the engine files the game reads, and its own art
      MANIFEST.json         engine revision, features, and a sha256 per file

The engine files are the ones `fixtures/external_consumer/package_assets.txt`
names, relative to the engine asset tree. `--measure` writes that list again:
it runs the checkout's release build with `--smoke` under strace and keeps
every engine asset file it opened.

The artifact must stand alone. So the last step runs `run.sh --smoke` from the
artifact under strace, and refuses the artifact if the game opened an asset
file outside it. Without that check, the engine's checkout fallbacks (an
absolute `CARGO_MANIFEST_DIR` path) would find a missing file on the build
machine and the artifact would pass here and fail on any other machine.

Exit codes: 0 the artifact is built and verified; 1 the artifact failed its
verification; 2 a prerequisite is missing (cargo, strace); 3 the build failed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GAME = REPO / "fixtures" / "external_consumer"
ENGINE_ASSETS = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets"
ASSET_LIST = GAME / "package_assets.txt"
BINARY = "outlander_visible"
FEATURES = "visible"
SMOKE_FRAMES = "300"


def fail(code: int, message: str) -> None:
    print(f"package_outlander: {message}", file=sys.stderr)
    raise SystemExit(code)


def build(target_dir: Path) -> Path:
    if shutil.which("cargo") is None:
        fail(2, "cargo is not on PATH")
    command = ["cargo", "build", "--release", "--features", FEATURES, "--bin", BINARY]
    env = dict(os.environ, CARGO_TARGET_DIR=str(target_dir))
    print(f"package_outlander: {' '.join(command)}  (target {target_dir})")
    if subprocess.run(command, cwd=GAME, env=env).returncode != 0:
        fail(3, "the release build failed")
    binary = target_dir / "release" / BINARY
    if not binary.is_file():
        fail(3, f"the build reported success and {binary} does not exist")
    return binary


def opened_assets(command: list[str], cwd: Path, env: dict[str, str]) -> tuple[int, list[str], str]:
    """Run `command` under strace: (exit code, every file opened under an
    `assets/` directory, the program's output)."""
    if shutil.which("strace") is None:
        fail(2, "strace is not on PATH; it is how the artifact proves it stands alone")
    with tempfile.TemporaryDirectory() as scratch:
        log = Path(scratch) / "strace.log"
        result = subprocess.run(
            ["strace", "-f", "-e", "trace=openat,open", "-o", str(log), *command],
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
        )
        opened = set()
        for line in log.read_text(errors="replace").splitlines():
            if "= -1" in line:
                continue
            match = re.search(r'"([^"]*/assets/[^"]*)"', line)
            if match and not match.group(1).endswith("/"):
                opened.add(match.group(1))
    return result.returncode, sorted(opened), result.stdout + result.stderr


def measure(target_dir: Path) -> None:
    binary = build(target_dir)
    code, opened, output = opened_assets([str(binary), "--smoke", SMOKE_FRAMES], GAME, dict(os.environ))
    if code != 0:
        fail(1, f"the checkout's smoke run failed, so it cannot measure a list:\n{output}")
    # The path as opened, not `resolve()`d: generated sprite directories can be
    # symlinks (a worktree links them to its main checkout), and following one
    # moves the file out from under the engine prefix.
    prefix = os.path.abspath(ENGINE_ASSETS) + "/"
    # Without the `.meta` sidecars: which of them a run reads races with the
    # exit (two runs read different subsets of the same PNGs' sidecars), so the
    # list names assets and `assemble` carries each one's sidecar.
    engine = sorted(
        {
            os.path.abspath(path).removeprefix(prefix)
            for path in opened
            if os.path.abspath(path).startswith(prefix) and not path.endswith(".meta")
        }
    )
    ASSET_LIST.write_text(
        "# The engine asset files Outlander's release artifact carries, relative to\n"
        "# crates/ambition_platformer2d_actor_monolith/assets. Written by\n"
        "# `scripts/package_outlander.py --measure` (the files a smoke run opened).\n"
        + "".join(f"{path}\n" for path in engine)
    )
    print(f"package_outlander: {len(engine)} engine asset files -> {ASSET_LIST.relative_to(REPO)}")


def engine_revision() -> str:
    result = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True)
    return result.stdout.strip() or "unknown"


def assemble(binary: Path, out: Path) -> Path:
    root = out / "outlander"
    if root.exists():
        shutil.rmtree(root)
    (root / "assets").mkdir(parents=True)
    shutil.copy2(binary, root / BINARY)
    listed = [
        line.strip()
        for line in ASSET_LIST.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]
    missing = [path for path in listed if not (ENGINE_ASSETS / path).is_file()]
    if missing:
        fail(2, "engine asset files are missing (regenerate them with scripts/regen/sprites.sh):\n  " + "\n  ".join(missing))
    for path in listed:
        target = root / "assets" / path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ENGINE_ASSETS / path, target)
        sidecar = ENGINE_ASSETS / f"{path}.meta"
        if sidecar.is_file():
            shutil.copy2(sidecar, root / "assets" / f"{path}.meta")
    # The game's own tree last: its art wins over an engine file of the same
    # name, as the layered source in a checkout does.
    shutil.copytree(GAME / "assets", root / "assets", dirs_exist_ok=True)
    launcher = root / "run.sh"
    launcher.write_text(
        '#!/bin/sh\n'
        '# The asset root is this directory, wherever it was unpacked.\n'
        'here="$(cd "$(dirname "$0")" && pwd)"\n'
        f'BEVY_ASSET_ROOT="$here" exec "$here/{BINARY}" "$@"\n'
    )
    launcher.chmod(0o755)
    files = sorted(p for p in root.rglob("*") if p.is_file())
    manifest = {
        "game": "outlander",
        "engine_revision": engine_revision(),
        "features": FEATURES,
        "profile": "release",
        "files": {
            str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files
        },
    }
    (root / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return root


def verify(root: Path) -> None:
    # From `/`, so a relative path cannot resolve against the checkout.
    env = dict(os.environ)
    env.pop("BEVY_ASSET_ROOT", None)
    env.pop("CARGO_MANIFEST_DIR", None)
    code, opened, output = opened_assets([str(root / "run.sh"), "--smoke", SMOKE_FRAMES], Path("/"), env)
    for line in output.splitlines():
        if line.startswith("outlander smoke") or line.startswith("  FAILED"):
            print(line)
    inside = os.path.abspath(root) + "/"
    outside = [path for path in opened if not os.path.abspath(path).startswith(inside)]
    if outside:
        fail(1, "the artifact read assets outside itself:\n  " + "\n  ".join(outside))
    if code != 0:
        fail(1, f"the artifact's smoke run failed (exit {code}):\n{output}")
    if not opened:
        fail(1, "the artifact's smoke run opened no asset file, so it proved nothing")
    print(f"package_outlander: verified, {len(opened)} asset files read, all from the artifact")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, default=GAME / "dist")
    parser.add_argument("--target-dir", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", GAME / "target")))
    parser.add_argument("--measure", action="store_true", help="rewrite package_assets.txt from a smoke run")
    args = parser.parse_args()
    if args.measure:
        measure(args.target_dir.resolve())
        return 0
    binary = build(args.target_dir.resolve())
    root = assemble(binary, args.out.resolve())
    verify(root)
    tarball = args.out.resolve() / "outlander.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(root, arcname="outlander")
    print(f"package_outlander: {tarball} ({tarball.stat().st_size / 1048576:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
