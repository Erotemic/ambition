#!/usr/bin/env python3
"""Compare the same promotion input in separate release test processes.

Build the WASM module first. This command builds the benchmark host once,
then runs each backend with and without invocation counters. It writes raw
samples and build metadata. It does not change game source files.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "target/pulse-forge-profile")
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--cycles", type=int, default=10)
    parser.add_argument("--jobs", type=int, default=3)
    args = parser.parse_args()
    if min(args.samples, args.cycles, args.jobs) < 1:
        parser.error("samples, cycles, and jobs must be positive")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(ROOT / "scripts/setup/target_bindmount.sh"), "--check"], cwd=ROOT, check=True)
    env = dict(os.environ, CARGO_INCREMENTAL="0")
    command = ["cargo", "test", "--release", "-p", "pulse_forge_host", "--no-default-features",
               "--features", "desktop_player", "--test", "profile",
               "--no-run", "--message-format", "json", "--jobs", str(args.jobs)]
    started = time.monotonic()
    with (out / "build.jsonl").open("w") as log:
        build = subprocess.run(command, cwd=ROOT, env=env, text=True, stdout=log, stderr=subprocess.STDOUT)
    build.check_returncode()
    build_seconds = time.monotonic() - started
    executable = None
    for line in (out / "build.jsonl").read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("reason") == "compiler-artifact" and row.get("target", {}).get("name") == "profile":
            executable = row.get("executable") or executable
    if executable is None:
        raise SystemExit("Cargo did not report the profile test executable")
    samples = []
    for index in range(args.samples):
        backends = ["rhai", "wasm", "linked"]
        backends = backends[index % 3:] + backends[:index % 3]
        for backend in backends:
            for counters in (True, False):
                name = f"{index}-{backend}-{'counted' if counters else 'plain'}"
                sample = out / f"{name}.json"
                child_env = dict(env, PROFILE_BACKEND=backend, PROFILE_CYCLES=str(args.cycles), PROFILE_OUT=str(sample))
                child_env.pop("PROFILE_NO_COUNTERS", None)
                if not counters:
                    child_env["PROFILE_NO_COUNTERS"] = "1"
                with (out / f"{name}.log").open("w") as log:
                    subprocess.run([executable, "--ignored", "--exact", "profile_promotion_workload", "--nocapture"],
                                   cwd=ROOT, env=child_env, stdout=log, stderr=subprocess.STDOUT, check=True)
                samples.append(json.loads(sample.read_text()))
                print(f"{name}: {samples[-1]['step_mean_us']:.1f} us/step", flush=True)
    metadata = {
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "diff": subprocess.check_output(["git", "diff", "--stat"], cwd=ROOT, text=True),
        "platform": platform.platform(), "cpu": Path("/proc/cpuinfo").read_text().split("model name", 1)[-1].splitlines()[0],
        "rustc": subprocess.check_output(["rustc", "-Vv"], cwd=ROOT, text=True),
        "command": command, "build_seconds": build_seconds, "CARGO_INCREMENTAL": "0",
        "host_sha256": hashlib.sha256(Path(executable).read_bytes()).hexdigest(),
        "input_sha256": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                         for path in (ROOT / "examples/projects/pulse_forge/modules/pulse_forge.rhai",
                                      ROOT / "examples/projects/pulse_forge/modules/sentinel.rhai",
                                      ROOT / "examples/projects/pulse_forge/build/pulse_forge.wasm",
                                      ROOT / "examples/projects/pulse_forge/portable/src/lib.rs")},
        "samples": samples,
    }
    (out / "report.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(out / "report.json")


if __name__ == "__main__":
    main()
