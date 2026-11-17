#!/usr/bin/env python3
"""Measure a bolt-speed edit through a restarted simulation in each backend.

Run profile.py first. This command changes only the example's speed constant
and WASM output, then restores them. Do not edit those files during this run.
Native measurement builds the release host, then the behavior probe. Rhai and
WASM use the existing probe executable. No pixel-readiness claim is made.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
GAME = ROOT / "examples/projects/pulse_forge"
BUILD = ["cargo", "test", "--release", "-p", "pulse_forge_host", "--no-default-features",
         "--features", "desktop_player", "--test", "profile", "--no-run", "--message-format", "json", "--jobs", "3"]
HOST_BUILD = ["cargo", "build", "--release", "-p", "pulse_forge_host", "--no-default-features",
              "--features", "desktop_player", "--bin", "pulse_forge_host", "--jobs", "3"]
MODULE_BUILD = [str(ROOT / "scripts/regen/build_extension_modules.sh"), "pulse_forge_modules"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "target/pulse-forge-edits")
    parser.add_argument("--samples", type=int, default=3)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("samples must be positive")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_INCREMENTAL="0")
    script = GAME / "modules/pulse_forge.rhai"
    rust = GAME / "portable/src/lib.rs"
    wasm = GAME / "build/pulse_forge.wasm"
    originals = {path: path.read_bytes() for path in (script, rust, wasm)}
    assert b"const BOLT_SPEED = 240.0;" in originals[script], "restore the baseline Rhai speed first"
    assert b"const BOLT_SPEED: f32 = 240.0;" in originals[rust], "restore the baseline Rust speed first"

    def build(command: list[str], label: str) -> tuple[float, str]:
        started = time.monotonic()
        with (out / f"{label}.log").open("w") as log:
            subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        return time.monotonic() - started, (out / f"{label}.log").read_text()

    def executable(output: str) -> str:
        for line in reversed(output.splitlines()):
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get("target", {}).get("name") == "profile" and row.get("executable"):
                return row["executable"]
        raise RuntimeError("Cargo did not report the behavior probe executable")

    controls = {}
    build(MODULE_BUILD, "module-warm")
    build(HOST_BUILD, "host-warm")
    build(BUILD, "probe-warm")
    controls["module_noop_s"], _ = build(MODULE_BUILD, "module-noop")
    controls["release_host_noop_s"], _ = build(HOST_BUILD, "host-noop")
    controls["probe_noop_s"], output = build(BUILD, "probe-noop")
    probe = executable(output)
    samples = []
    try:
        for index in range(args.samples):
            for backend in ("rhai", "wasm", "linked"):
                name = f"{index}-{backend}"
                started = time.monotonic()
                target = script if backend == "rhai" else rust
                target.write_bytes(originals[target].replace(b"= 240.0;", b"= 241.0;"))
                build_s = probe_build_s = 0.0
                if backend == "wasm":
                    build_s, _ = build(MODULE_BUILD, f"{name}-module")
                    wasm.write_bytes((ROOT / "target/extension-modules/wasm32-unknown-unknown/release/pulse_forge_modules.wasm").read_bytes())
                elif backend == "linked":
                    build_s, _ = build(HOST_BUILD, f"{name}-host")
                    probe_build_s, output = build(BUILD, f"{name}-probe")
                    probe = executable(output)
                child_env = dict(env, PROFILE_BACKEND=backend, PROFILE_CYCLES="1", PROFILE_EXPECT_SPEED="241",
                                 PROFILE_NO_COUNTERS="1", PROFILE_OUT=str(out / f"{name}.json"))
                observed_s = None
                with (out / f"{name}-run.log").open("w") as log:
                    process = subprocess.Popen([probe, "--ignored", "--exact", "profile_promotion_workload", "--nocapture"],
                                               cwd=ROOT, env=child_env, stdout=subprocess.PIPE,
                                               stderr=subprocess.STDOUT, text=True)
                    for line in process.stdout:
                        log.write(line)
                        if line.startswith("OBSERVED_SPEED 241"):
                            observed_s = time.monotonic() - started
                    if process.wait() or observed_s is None:
                        raise RuntimeError(f"{name}: the changed speed did not reach the simulation; see its run log")
                samples.append({"backend": backend, "build_s": build_s, "probe_build_s": probe_build_s,
                                "edit_to_observed_s": observed_s,
                                "code_sha256": hashlib.sha256(wasm.read_bytes() if backend == "wasm" else
                                    Path(probe).read_bytes() if backend == "linked" else script.read_bytes()).hexdigest()})
                print(f"{name}: {observed_s:.3f} s to changed projectile", flush=True)
                for path, data in originals.items():
                    path.write_bytes(data)
                if backend == "wasm":
                    build(MODULE_BUILD, f"{name}-restore")
                elif backend == "linked":
                    build(HOST_BUILD, f"{name}-restore-host")
                    _, output = build(BUILD, f"{name}-restore-probe")
                    probe = executable(output)
    finally:
        for path, data in originals.items():
            path.write_bytes(data)
        build(MODULE_BUILD, "final-restore-module")
        build(HOST_BUILD, "final-restore-host")
        build(BUILD, "final-restore-probe")
    (out / "report.json").write_text(json.dumps({"controls": controls, "samples": samples,
        "mode": "restart; observed projectile velocity; native total includes separate probe build",
        "commands": {"module": MODULE_BUILD, "host": HOST_BUILD, "probe": BUILD}}, indent=2) + "\n")


if __name__ == "__main__":
    main()
