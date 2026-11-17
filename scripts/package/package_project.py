#!/usr/bin/env python3
"""Package an authored project with a prebuilt desktop host.

The Rust exporter reads project, pack, and library declarations. This command
uses the existing byte contract and file-open probe to verify the artifact.
It does not run Cargo.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from package_asset_guard import AssetContract, ContractEntry, audit_tree, sha256_path, walk_package_tree, write_json_atomic
from package_outlander import opened_assets

REPO = Path(__file__).resolve().parents[2]


def verify(root: Path, ticks: int, presentation_frames: int) -> dict:
    contract = AssetContract.from_json(json.loads((root / "CONTRACT.json").read_text()))
    for name in ("CONTRACT.json", "VERIFIED.json"):
        if (root / name).is_file():
            contract.entries[name] = ContractEntry(name)
    audit_tree(contract, root, reject_extras=True)
    expected = json.loads((root / "project-export.json").read_text())
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("AMBITION_", "CARGO_")) and key != "BEVY_ASSET_ROOT"}
    env["PATH"] = "/nonexistent"
    opened_inputs = set()
    reports = []
    modes = [("--headless-ticks", ticks)]
    if presentation_frames:
        modes.append(("--presentation-frames", presentation_frames))
    for flag, count in modes:
        code, opened, output = opened_assets([str(root / "run.sh"), flag, str(count)], Path("/"), env)
        outside = [path for path in opened if not Path(path).is_relative_to(root)]
        if outside:
            raise RuntimeError("the host read game inputs outside the artifact:\n  " + "\n  ".join(outside)
                               + "\nBuild a desktop_player host with embedded installed content and maps.")
        if code:
            raise RuntimeError(f"the artifact failed with status {code}:\n{output}")
        if not opened:
            raise RuntimeError("the probe opened no game input, so it did not prove independent loading")
        if f"project: {expected['project']}" not in output:
            raise RuntimeError("the host did not report the exported project identity")
        for selected in expected["implementations"]:
            key = f"{selected['provider']}::{selected['module']}"
            entries = [line for line in output.splitlines() if line.startswith(f"  {key}/")]
            runner = "[native]" if selected["implementation"] == "Linked" else "[loaded "
            if not any(runner in line for line in entries):
                raise RuntimeError(f"the running artifact did not select {key} as {selected['implementation']}")
        opened_inputs.update(opened)
        reports.append({"mode": flag, "count": count, "opened_inputs": len(opened)})
    generations = [line for line in output.splitlines() if line.startswith("module\t")]
    return {"project": expected["project"], "pack": expected["pack"], "ticks": ticks,
            "exit": code, "opened_inputs": sorted(opened_inputs), "probes": reports,
            "presentation_frames": presentation_frames, "generation": generations,
            "host_sha256": sha256_path(root / "game")}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path)
    parser.add_argument("--host", type=Path)
    parser.add_argument("--exporter", type=Path, default=REPO / "target/debug/project_export")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--ticks", type=int, default=30)
    parser.add_argument("--presentation-frames", type=int, default=0,
                        help="also run this many no-window frames; the host must support --presentation-frames")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    out = args.out.resolve()
    if args.verify_only:
        report = verify(out, args.ticks, args.presentation_frames)
        write_json_atomic(out / "VERIFIED.json", report)
        print(f"verified {report['project']}: {len(report['opened_inputs'])} inputs from the artifact")
        return 0
    if args.project is None or args.host is None:
        parser.error("--project and --host are required when creating a package")
    if out.exists():
        parser.error(f"{out} already exists; select a new output, or use --verify-only")
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f".{out.name}-", dir=out.parent) as temporary:
        staged = Path(temporary) / "game"
        subprocess.run([str(args.exporter.resolve()), "--project", str(args.project.resolve()),
                        "--out", str(staged)], check=True)
        shutil.copy2(args.host.resolve(), staged / "game")
        launcher = staged / "run.sh"
        launcher.write_text('#!/bin/sh\nhere="$(CDPATH= cd -- "${0%/*}" && pwd)"\n'
                            'export BEVY_ASSET_ROOT="$here"\n'
                            'exec "$here/game" --project "$here/project.ron" "$@"\n')
        launcher.chmod(0o755)
        files = walk_package_tree(staged)
        entries = {name: ContractEntry(name, sha256_path(path), path.stat().st_size,
                                      {"selected-project"}) for name, path in files.items()}
        contract = AssetContract("desktop-project", entries, ("project",))
        write_json_atomic(staged / "CONTRACT.json", contract.as_json())
        verify(staged, args.ticks, args.presentation_frames)
        staged.rename(out)
    report = verify(out, args.ticks, args.presentation_frames)
    write_json_atomic(out / "VERIFIED.json", report)
    print(f"packaged {report['project']}: {len(entries)} files, {len(report['opened_inputs'])} opened inputs")
    print(out / "run.sh")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"package_project: {error}")
