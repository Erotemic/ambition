#!/usr/bin/env python3
"""Slice a profiling bundle's capture into labelled time chunks.

`perf`'s `--time a%-b%` filter already produced one flat report per chunk into
`perf_windows/`. This pairs each of those with the game's own log lines from the
same window -- room/boss/title/session markers, frame spikes, and the workload
census rows -- so "what got slow when I entered the room" is a diff between two
chunks rather than a whole-run average.
"""

from __future__ import annotations

import os
import re
import sys

STAMP = re.compile(r"^\[\s*([0-9.]+)s\]\s?(.*)$")
PERCENT = re.compile(r"\s*[0-9]+(\.[0-9]+)?%")
ADAPTER = re.compile(r"AdapterInfo \{[^}]*\}")


def main(argv: list[str]) -> int:
    if len(argv) != 4:
        print("usage: profile_timeline.py <bundle-dir> <chunks> <marker-regex>", file=sys.stderr)
        return 2
    out, chunks, marker = argv[1], int(argv[2]), argv[3]
    pattern = re.compile(marker, re.I)

    events: list[tuple[float, str]] = []
    adapter = ""
    for name in ("game-stderr-stamped.txt", "game-stdout-stamped.txt"):
        path = os.path.join(out, name)
        if not os.path.exists(path):
            continue
        with open(path, errors="replace", encoding="utf-8") as handle:
            for line in handle:
                match = STAMP.match(line)
                if match:
                    events.append((float(match.group(1)), match.group(2).rstrip()))
                if not adapter:
                    found = ADAPTER.search(line)
                    if found:
                        adapter = found.group(0)
    events.sort(key=lambda event: event[0])
    total = max((at for at, _ in events), default=0.0)
    markers = [(at, text) for at, text in events if pattern.search(text)]

    lines = [
        "# Timeline profile",
        "",
        f"Observed span: {total:.1f}s, sliced into {chunks} chunks.",
        f"Marker regex: `{marker}`",
        "",
        "Chunk boundaries assume log time tracks trace time from launch;",
        "if the game exited early or idled past the last log line, edges skew.",
        "",
    ]
    if adapter:
        software = (
            "device_type: Cpu" in adapter
            or "llvmpipe" in adapter
            or "lavapipe" in adapter
            or "swiftshader" in adapter.lower()
        )
        if software:
            lines += [
                "> **SOFTWARE RENDERING.** This run had no GPU, so the per-chunk symbols",
                "> below are mostly the CPU rasterizer's unsymbolized JIT frames, not",
                "> your code. See the Renderer section of summary.md and",
                "> host-environment.txt before drawing conclusions.",
                "",
            ]
        else:
            lines += [f"Renderer: `{adapter}`", ""]

    for index in range(chunks):
        low, high = total * index / chunks, total * (index + 1) / chunks
        lines.append(f"## Chunk {index}: {low:.1f}s - {high:.1f}s")
        carried = [text for at, text in markers if at < low]
        if carried:
            lines.append(f"Carried context: `{carried[-1][:200]}`")
        inside = [(at, text) for at, text in markers if low <= at < high]
        if inside:
            lines += ["", "Markers:", "```text"]
            shown = inside if len(inside) <= 10 else inside[:5] + inside[-5:]
            for at, text in shown:
                lines.append(f"{at:9.3f}s {text[:200]}")
            if len(inside) > 10:
                lines.append(f"... {len(inside) - 10} more marker lines omitted ...")
            lines.append("```")
        else:
            lines.append("(no marker lines in this window)")

        report = ""
        for candidate in (
            os.path.join(out, "perf_windows", f"chunk-{index:02d}.txt"),
            os.path.join(out, f"perf-chunk-{index:02d}.txt"),
        ):
            if os.path.exists(candidate):
                with open(candidate, errors="replace", encoding="utf-8") as handle:
                    report = handle.read()
                break
        top = [line[:200] for line in report.splitlines() if PERCENT.match(line)][:15]
        if top:
            lines += ["", "Top symbols:", "```text"] + top + ["```"]
        lines.append("")

    with open(os.path.join(out, "timeline.md"), "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
