#!/usr/bin/env python3
"""Capture a Linux ALSA process at 48 kHz stereo float, with a time limit."""

from __future__ import annotations

import argparse
import array
import json
import math
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import tempfile
import time

RATE = 48000
FRAME_BYTES = 8


def read_pcm(path: Path, seconds: float) -> None:
    start = time.monotonic()
    count = 0
    limit = int(seconds * RATE) * FRAME_BYTES
    with path.open("wb") as output:
        while count < limit:
            block = os.read(0, min(4096, limit - count))
            if not block:
                break
            output.write(block)
            count += len(block)
            # The null PCM has no device clock. Pipe pressure sets its rate.
            delay = count / (RATE * FRAME_BYTES) - (time.monotonic() - start)
            if delay > 0:
                time.sleep(delay)
    path.with_suffix(path.suffix + ".done").write_text(str(count))


def report(path: Path) -> dict:
    samples = array.array("f")
    samples.frombytes(path.read_bytes())
    if sys.byteorder != "little":
        samples.byteswap()
    if not samples or len(samples) % 2:
        raise ValueError("The capture has no complete stereo frames")
    if not all(math.isfinite(value) for value in samples):
        raise ValueError("The capture has non-finite samples")
    return {
        "path": str(path),
        "format": "FLOAT_LE",
        "sample_rate": RATE,
        "channels": 2,
        "frames": len(samples) // 2,
        "seconds": len(samples) / (RATE * 2),
        "peak": max(abs(value) for value in samples),
        "rms": math.sqrt(sum(value * value for value in samples) / len(samples)),
        "output": "software capture; no physical speaker or listening claim",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=30.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or not math.isfinite(args.seconds) or args.seconds <= 0:
        parser.error("Supply a command and a finite positive capture time")
    if sys.platform != "linux":
        parser.error("This capture uses Linux ALSA")
    path = args.out.resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    done = path.with_suffix(path.suffix + ".done")
    if path.exists() or done.exists():
        parser.error("Select a new output path")
    reader = shlex.join([
        sys.executable, str(Path(__file__).resolve()), "--read", str(path), str(args.seconds),
    ])
    with tempfile.TemporaryDirectory(prefix="ambition-alsa-") as directory:
        config = Path(directory) / "asound.conf"
        config.write_text(
            'pcm.capture_sink { type file slave.pcm { type null } '
            f'file {json.dumps("|" + reader)} format "raw" }}\n'
            'pcm.!default { type plug slave { pcm "capture_sink" '
            'format FLOAT_LE rate 48000 channels 2 } '
            'hint.description "Ambition software capture" }\n'
        )
        process = subprocess.Popen(
            command, env={**os.environ, "ALSA_CONFIG_PATH": str(config)},
            start_new_session=True,
        )
        try:
            result = process.wait(timeout=args.seconds + 5)
        except (subprocess.TimeoutExpired, KeyboardInterrupt):
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            raise
        deadline = time.monotonic() + 2
        while not done.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        if not done.exists():
            raise RuntimeError("The command did not close a native audio capture")
        summary = report(path)
        summary["command_exit_code"] = result
        path.with_suffix(path.suffix + ".json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary, indent=2))
        return result


if __name__ == "__main__":
    if sys.argv[1:2] == ["--read"]:
        read_pcm(Path(sys.argv[2]), float(sys.argv[3]))
    else:
        sys.exit(main())
