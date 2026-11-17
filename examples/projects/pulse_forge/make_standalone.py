#!/usr/bin/env python3
"""Make an independent Pulse Forge directory with its Rust source workspace.

The SDK source is an explicit build input. The runtime package needs no SDK
checkout. This command reuses project_export for the authored runtime files.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import subprocess
import tomllib

try:
    import toml
except ImportError:
    raise SystemExit("make_standalone needs the TOML writer: python3 -m pip install toml")

EXAMPLE = Path(__file__).resolve().parent
SDK = EXAMPLE.parents[2]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sdk", type=Path, default=SDK)
    parser.add_argument("--exporter", type=Path, default=SDK / "target/debug/project_export")
    args = parser.parse_args()
    out, sdk = args.out.resolve(), args.sdk.resolve()
    subprocess.run([str(args.exporter.resolve()), "--project", str(EXAMPLE / "project-rhai.ron"),
                    "--out", str(out)], check=True)
    sdk_manifest = tomllib.loads((sdk / "Cargo.toml").read_text())
    base = sdk_manifest["workspace"]
    workspace = {"workspace": {"members": ["portable", "host"], "resolver": base.get("resolver", "2"),
                               "package": base["package"], "dependencies": base["dependencies"]},
                 "patch": sdk_manifest.get("patch", {})}
    (out / "Cargo.toml").write_text(toml.dumps(workspace))
    shutil.copy2(sdk / "Cargo.lock", out / "Cargo.lock")
    for crate in ("portable", "host"):
        source = EXAMPLE / crate
        destination = out / crate
        shutil.copytree(source / "src", destination / "src")
        manifest = tomllib.loads((source / "Cargo.toml").read_text())
        manifest.pop("dev-dependencies", None)
        for dependency in manifest.get("dependencies", {}).values():
            if not isinstance(dependency, dict) or "path" not in dependency:
                continue
            resolved = (source / dependency["path"]).resolve()
            if resolved.is_relative_to(EXAMPLE):
                continue
            relative = resolved.relative_to(SDK)
            dependency["path"] = str(sdk / relative)
        (destination / "Cargo.toml").write_text(toml.dumps(manifest))
    print(f"standalone source: {out}; SDK build input: {sdk}")


if __name__ == "__main__":
    main()
