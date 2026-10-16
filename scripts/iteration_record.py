#!/usr/bin/env python3
"""Time a content edit to the host's acknowledgment of the generation it made.

Fast-iteration I0 (`docs/planning/engine/fast-iteration-implementation.md`),
output schema M0 (`docs/planning/engine/extension-iteration-evidence.md`).

The recorder starts the host command you name and reads its stdout. A host that
reloads a pack prints one acknowledgment line for each event
(`content::live_reload`):

    [live-reload] event=watching pack=<id> generation=<hex>
    [live-reload] event=staged pack=<id> generation=<hex> prepare_ms=<ms>
    [live-reload] event=published pack=<id> generation=<hex>
    [live-reload] event=refused pack=<id> generation=<hex> prepare_ms=<ms>

For each sample the recorder writes the edit, waits for a published generation
that differs from the running one, then writes the original bytes back and
waits for the original generation again. Before the samples it touches the file
with no change in its bytes and requires that nothing is published.

It refuses the run (exit 1, no summary) when:

- the host acknowledges no new generation within the timeout (it did not load
  the edit);
- the host acknowledges the running generation as the answer to an edit (an old
  acknowledgment is not a faster iteration);
- the host refuses the edit;
- a revert does not give back the generation the run started from;
- a save with no change publishes a generation.

The loop runs no build command, so `compile_seconds` is 0 and
`host_link_invocations` is 0 by construction; build the host before you run
this. The raw rows go to `target/iteration_reports/<stamp>/records.jsonl`.

Usage::

    cargo build -p ambition_demo_smash_app --bin smash_demo
    python scripts/iteration_record.py \\
        --host target/debug/smash_demo --ticks 1000000000 \\
        --pack ambition_demo_smash \\
        --file game/ambition_demo_smash/assets/data/smash_rules.ron \\
        --old 'X' --new 'Y' --samples 5
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import queue
import re
import shlex
import statistics
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / "target" / "iteration_reports"

ACK = re.compile(
    r"\[live-reload\] event=(?P<event>\w+) pack=(?P<pack>\S+) generation=(?P<generation>[0-9a-f]+)"
    r"(?: prepare_ms=(?P<prepare_ms>[0-9.]+))?"
)


class Refused(Exception):
    """The run cannot give a row that means what the schema says."""


@dataclass
class Ack:
    at: float
    event: str
    pack: str
    generation: str
    prepare_ms: float | None


class Host:
    """The host process and its acknowledgments, read on a thread."""

    def __init__(self, command: list[str], pack: str) -> None:
        self.pack = pack
        self.lines: "queue.Queue[Ack]" = queue.Queue()
        self.process = subprocess.Popen(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            match = ACK.search(line)
            if match and match["pack"] == self.pack:
                prepare = match["prepare_ms"]
                self.lines.put(
                    Ack(
                        at=time.monotonic(),
                        event=match["event"],
                        pack=match["pack"],
                        generation=match["generation"],
                        prepare_ms=float(prepare) if prepare else None,
                    )
                )

    def next(self, timeout: float) -> Ack | None:
        try:
            return self.lines.get(timeout=timeout)
        except queue.Empty:
            return None

    def stop(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()[:16]


def wait_for(host: Host, events: set[str], timeout: float) -> Ack | None:
    """The next acknowledgment of one of `events`, skipping the others."""
    deadline = time.monotonic() + timeout
    while (left := deadline - time.monotonic()) > 0:
        ack = host.next(left)
        if ack is None:
            return None
        if ack.event in events:
            return ack
    return None


def write(path: Path, data: bytes) -> float:
    """Write `data` and return when it was written. The mtime moves forward a
    second, so a host that compares mtimes sees a change even within one tick of
    the filesystem's clock."""
    path.write_bytes(data)
    later = time.time() + 1.0
    os.utime(path, (later, later))
    return time.monotonic()


def change(host: Host, path: Path, data: bytes, running: str, timeout: float) -> tuple[Ack, Ack, float]:
    """Write `data`; return the staged and published acknowledgments of a new
    generation, and when the edit was written."""
    written = write(path, data)
    staged = wait_for(host, {"staged", "refused", "published"}, timeout)
    if staged is None:
        raise Refused(f"the host acknowledged no generation within {timeout} s: it did not load the edit")
    if staged.event == "refused":
        raise Refused("the host refused the edit; read its log for the file and the field")
    if staged.generation == running:
        raise Refused(f"the host acknowledged the running generation {running} as the answer to an edit")
    published = staged if staged.event == "published" else wait_for(host, {"published"}, timeout)
    if published is None:
        raise Refused(f"the host staged {staged.generation} and published nothing within {timeout} s")
    if published.generation != staged.generation:
        raise Refused(f"the host staged {staged.generation} and published {published.generation}")
    return staged, published, written


def row(args, sample: int, kind: str, data: bytes, staged: Ack, published: Ack, written: float) -> dict:
    return {
        "source_commit": git("rev-parse", "HEAD"),
        "dirty_paths": len(git("status", "--porcelain").splitlines()),
        "edit_class": args.edit_class,
        "sample_kind": kind,
        "manifest_and_feature_args": args.host_command,
        "toolchain_target_profile": None,
        "cache_state": "warm",
        "sample_index": sample,
        "edited_file": str(args.file),
        "changed_input_digest": digest(data),
        "artifact_digest": None,
        "admitted_generation_digest": published.generation,
        "compile_seconds": 0.0,
        "module_or_builder_link_seconds": 0.0,
        "host_link_invocations": 0,
        "prepare_seconds": None if staged.prepare_ms is None else staged.prepare_ms / 1000.0,
        "activate_seconds": round(published.at - written, 4),
        "first_observed_result_seconds": None,
        "peak_rss_bytes": None,
        "status": "measured",
        "machine": platform.node(),
    }


def git(*argv: str) -> str:
    return subprocess.run(["git", *argv], cwd=ROOT, capture_output=True, text=True).stdout.strip()


def run(args) -> list[dict]:
    path = (ROOT / args.file).resolve()
    original = path.read_bytes()
    old, new = args.old.encode(), args.new.encode()
    if original.count(old) != 1:
        raise Refused(f"{args.file} holds {original.count(old)} copies of --old; it must hold exactly one")
    edited = original.replace(old, new)
    host = Host(args.host_command, args.pack)
    rows: list[dict] = []
    try:
        watching = wait_for(host, {"watching"}, args.start_timeout)
        if watching is None:
            raise Refused(f"the host did not say it watches the pack `{args.pack}` within {args.start_timeout} s")
        baseline = watching.generation
        # A save with no change must publish nothing.
        write(path, original)
        quiet = wait_for(host, {"staged", "published"}, args.quiet)
        if quiet is not None:
            raise Refused(f"a save with no change gave a `{quiet.event}` of generation {quiet.generation}")
        for sample in range(1, args.samples + 1):
            staged, published, written = change(host, path, edited, baseline, args.timeout)
            rows.append(row(args, sample, "edit", edited, staged, published, written))
            staged, published, written = change(host, path, original, published.generation, args.timeout)
            if published.generation != baseline:
                raise Refused(
                    f"the revert gave generation {published.generation}, not the generation "
                    f"{baseline} the run started from"
                )
            rows.append(row(args, sample, "revert", original, staged, published, written))
    finally:
        path.write_bytes(original)
        host.stop()
    return rows


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--host", required=True, help="the host executable, already built")
    parser.add_argument("--host-arg", action="append", default=[], help="an argument for the host (repeatable)")
    parser.add_argument("--ticks", type=int, help="pass --ticks N to a headless demo host")
    parser.add_argument("--pack", required=True, help="the pack id the acknowledgments name")
    parser.add_argument("--file", required=True, type=Path, help="the source to edit, from the repo root")
    parser.add_argument("--old", required=True, help="text the file holds exactly once")
    parser.add_argument("--new", required=True, help="what the edit puts in its place")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--edit-class", default="pack-data")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--start-timeout", type=float, default=120.0)
    parser.add_argument("--quiet", type=float, default=3.0, help="seconds a no-change save must stay silent")
    parser.add_argument("--out", type=Path, help="the report directory (default target/iteration_reports/<stamp>)")
    args = parser.parse_args(argv)
    args.host_command = [args.host, *args.host_arg] + (["--ticks", str(args.ticks)] if args.ticks else [])
    try:
        rows = run(args)
    except Refused as why:
        print(f"[iteration_record] REFUSED: {why}", file=sys.stderr)
        return 1
    out = args.out or REPORTS / time.strftime("%Y%m%dT%H%M%S")
    out.mkdir(parents=True, exist_ok=True)
    with (out / "records.jsonl").open("w") as records:
        for record in rows:
            records.write(json.dumps(record) + "\n")
    for kind in ("edit", "revert"):
        seconds = [r["activate_seconds"] for r in rows if r["sample_kind"] == kind]
        prepare = [r["prepare_seconds"] for r in rows if r["sample_kind"] == kind and r["prepare_seconds"] is not None]
        print(
            f"[iteration_record] {kind}: {len(seconds)} samples, edit to published generation "
            f"median {statistics.median(seconds):.3f} s (range {min(seconds):.3f}-{max(seconds):.3f} s); "
            f"host compile of the pack median "
            f"{statistics.median(prepare) * 1000 if prepare else float('nan'):.1f} ms"
        )
    print(f"[iteration_record] host: {shlex.join(args.host_command)}; rows: {out / 'records.jsonl'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
