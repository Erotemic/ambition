#!/usr/bin/env python3
"""Render line_profiler ``.lprof`` databases into adjacent text reports.

Kernprof writes the binary database. This helper performs the explicit second
step that creates ``.txt`` reports and a compact per-directory index. It is safe
to rerun and can process an interrupted profile directory after the fact.
"""
from __future__ import annotations

import argparse
import datetime as datetime_mod
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterable
from zoneinfo import ZoneInfo

NY_TZ = ZoneInfo("America/New_York")


def _atomic_write(path: Path, text: str) -> None:
    tmp = path.with_name(path.name + f".tmp-{os.getpid()}")
    tmp.write_text(text, encoding="utf8")
    tmp.replace(path)


def _iter_profiles(inputs: Iterable[Path], newer_than: Path | None) -> list[Path]:
    threshold = newer_than.stat().st_mtime_ns if newer_than is not None else None
    found: set[Path] = set()
    for item in inputs:
        if item.is_file() and item.suffix == ".lprof":
            candidates = [item]
        elif item.is_dir():
            candidates = item.rglob("*.lprof")
        else:
            continue
        for candidate in candidates:
            candidate = candidate.resolve()
            if threshold is not None and candidate.stat().st_mtime_ns <= threshold:
                continue
            found.add(candidate)
    return sorted(found, key=lambda p: (p.parent.as_posix(), p.name))


def _phase_name(path: Path) -> str:
    match = re.match(r"^(?P<label>.+?)\.[A-Za-z0-9]{6}\.lprof$", path.name)
    return match.group("label") if match else path.stem


def _summary_lines(text: str, limit: int = 15) -> list[str]:
    rows: list[tuple[float, str]] = []
    for line in text.splitlines():
        match = re.match(r"^\s*([0-9]+(?:\.[0-9]+)?)\s+seconds?\s+-\s+(.+)$", line)
        if match:
            rows.append((float(match.group(1)), match.group(2).strip()))
    rows.sort(reverse=True)
    return [f"{seconds:9.3f}s  {description}" for seconds, description in rows[:limit]]


def render_profile(path: Path, *, force: bool = False) -> tuple[Path, str]:
    report = path.with_suffix(".txt")
    if not force and report.exists() and report.stat().st_mtime_ns >= path.stat().st_mtime_ns:
        return report, report.read_text(encoding="utf8", errors="replace")
    command = [sys.executable, "-m", "line_profiler", "-tmz", str(path)]
    proc = subprocess.run(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode:
        raise RuntimeError(
            f"line_profiler report command failed for {path} with status "
            f"{proc.returncode}:\n{proc.stdout}"
        )
    _atomic_write(report, proc.stdout)
    return report, proc.stdout


def write_indexes(rendered: list[tuple[Path, Path, str]]) -> list[Path]:
    grouped: dict[Path, list[tuple[Path, Path, str]]] = defaultdict(list)
    for lprof, report, text in rendered:
        grouped[lprof.parent].append((lprof, report, text))
    indexes: list[Path] = []
    for parent, items in grouped.items():
        now = datetime_mod.datetime.now(NY_TZ)
        lines = [
            "Ambition sprite line-profile reports",
            f"Generated: {now.isoformat()}",
            "",
        ]
        for lprof, report, text in sorted(items, key=lambda row: row[0].name):
            lines.extend(
                [
                    f"[{_phase_name(lprof)}]",
                    f"binary: {lprof.name}",
                    f"text:   {report.name}",
                ]
            )
            summary = _summary_lines(text)
            if summary:
                lines.append("slowest profiled functions (inclusive times):")
                lines.extend(f"  {line}" for line in summary)
            lines.append("")
        index = parent / "profile-index.txt"
        _atomic_write(index, "\n".join(lines).rstrip() + "\n")
        indexes.append(index)
    return indexes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path, help=".lprof files or directories")
    parser.add_argument("--newer-than", type=Path, help="only render profiles newer than marker")
    parser.add_argument("--force", action="store_true", help="replace current reports")
    args = parser.parse_args(argv)

    profiles = _iter_profiles(args.paths, args.newer_than)
    if not profiles:
        print("no .lprof databases found", file=sys.stderr)
        return 1
    rendered: list[tuple[Path, Path, str]] = []
    failures: list[tuple[Path, Exception]] = []
    for path in profiles:
        try:
            report, text = render_profile(path, force=args.force)
        except Exception as ex:  # Preserve every binary profile if one report fails.
            failures.append((path, ex))
            print(f"warning: {ex}", file=sys.stderr)
        else:
            rendered.append((path, report, text))
            print(f"[profile] text report -> {report}")
    for index in write_indexes(rendered):
        print(f"[profile] index -> {index}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
