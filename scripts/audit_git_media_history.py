#!/usr/bin/env python3
"""Audit a Git repository history for checked-in media assets.

This scans files reachable from all refs, not just the current checkout, so it
can find media assets that were committed and later deleted. The audit is based
on historical file paths and flags media-looking extensions such as .png, .jpg,
.mp3, .wav, .mp4, etc. It intentionally does not flag arbitrary large
non-media files; for example, .ldtk project files are ignored by default.

Examples:
    python scripts/audit_git_media_history.py .
    python scripts/audit_git_media_history.py . --extra-media-ext .aseprite,.kra
    python scripts/audit_git_media_history.py . --max-paths-per-blob 50
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterable, Sequence

IMAGE_EXTENSIONS = frozenset(
    {
        ".apng",
        ".avif",
        ".bmp",
        ".gif",
        ".heic",
        ".heif",
        ".ico",
        ".jpeg",
        ".jpg",
        ".jxl",
        ".png",
        ".psb",
        ".psd",
        ".raw",
        ".svg",
        ".tga",
        ".tif",
        ".tiff",
        ".webp",
    }
)

AUDIO_EXTENSIONS = frozenset(
    {
        ".aac",
        ".aif",
        ".aiff",
        ".flac",
        ".m4a",
        ".mid",
        ".midi",
        ".mp3",
        ".oga",
        ".ogg",
        ".opus",
        ".wav",
        ".wma",
    }
)

VIDEO_EXTENSIONS = frozenset(
    {
        ".avi",
        ".m4v",
        ".mkv",
        ".mov",
        ".mp4",
        ".mpeg",
        ".mpg",
        ".ogv",
        ".webm",
        ".wmv",
    }
)

DEFAULT_MEDIA_EXTENSIONS = IMAGE_EXTENSIONS | AUDIO_EXTENSIONS | VIDEO_EXTENSIONS


class GitError(RuntimeError):
    """Raised when a git subprocess fails."""


def human_size(num_bytes: int) -> str:
    """Return a binary human-readable size string."""

    amount = float(num_bytes)
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if amount < 1024 or unit == "TiB":
            if unit == "B":
                return f"{int(amount)} B"
            return f"{amount:.1f} {unit}"
        amount /= 1024
    raise AssertionError("unreachable")


def run_git(repo: Path, args: Sequence[str], input_text: str | None = None) -> str:
    """Run git in ``repo`` and return stdout."""

    proc = subprocess.run(
        ["git", "-C", os.fspath(repo), *args],
        input=input_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        command = "git " + " ".join(args)
        stderr = proc.stderr.strip()
        raise GitError(f"{command} failed with exit code {proc.returncode}:\n{stderr}")
    return proc.stdout


def parse_extensions(values: Iterable[str]) -> frozenset[str]:
    """Normalize user-provided extensions."""

    extensions: set[str] = set()
    for raw_value in values:
        for part in raw_value.split(","):
            part = part.strip().lower()
            if not part:
                continue
            if not part.startswith("."):
                part = "." + part
            extensions.add(part)
    return frozenset(extensions)


def collect_historical_paths(repo: Path) -> dict[str, set[str]]:
    """Return object SHA -> historical paths from all reachable refs."""

    output = run_git(repo, ["rev-list", "--objects", "--all"])
    object_paths: dict[str, set[str]] = defaultdict(set)
    for line in output.splitlines():
        if not line:
            continue
        # Format is either "<sha>" or "<sha> <path>". Split only once so paths
        # containing spaces remain intact.
        parts = line.split(" ", 1)
        if len(parts) != 2:
            continue
        sha, path = parts
        object_paths[sha].add(path)
    return object_paths


def batch_object_info(
    repo: Path, object_names: Iterable[str]
) -> dict[str, tuple[str, int]]:
    """Return object SHA -> (type, size) using one cat-file batch call."""

    object_list = list(object_names)
    if not object_list:
        return {}
    batch_input = "\n".join(object_list) + "\n"
    output = run_git(
        repo,
        ["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)"],
        input_text=batch_input,
    )
    info: dict[str, tuple[str, int]] = {}
    for line in output.splitlines():
        sha, object_type, size_text = line.split()
        info[sha] = (object_type, int(size_text))
    return info


def path_extension(path: str) -> str:
    """Return the lowercase suffix used for media checks."""

    return Path(path).suffix.lower()


def find_media_blobs(
    repo: Path,
    *,
    media_extensions: frozenset[str],
) -> list[dict[str, object]]:
    """Find historical blobs whose paths have media extensions."""

    object_paths = collect_historical_paths(repo)
    object_info = batch_object_info(repo, object_paths.keys())
    findings: list[dict[str, object]] = []

    for sha, paths in object_paths.items():
        object_type, size = object_info.get(sha, ("", -1))
        if object_type != "blob":
            continue

        media_paths = sorted(
            path for path in paths if path_extension(path) in media_extensions
        )
        if media_paths:
            matched_extensions = sorted({path_extension(path) for path in media_paths})
            findings.append(
                {
                    "sha": sha,
                    "size": size,
                    "extensions": matched_extensions,
                    "paths": media_paths,
                }
            )

    findings.sort(key=lambda item: (-int(item["size"]), str(item["paths"])))
    return findings


def build_argparser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Audit all reachable Git history for checked-in media assets.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "repo",
        nargs="?",
        default=".",
        type=Path,
        help="Repository path to audit.",
    )
    parser.add_argument(
        "--extra-media-ext",
        action="append",
        default=[],
        metavar="EXT[,EXT...]",
        help="Additional extension(s) to treat as media, e.g. .aseprite,.kra.",
    )
    parser.add_argument(
        "--max-paths-per-blob",
        default=25,
        type=int,
        help="Maximum number of historical paths to print for one blob.",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_argparser()
    args = parser.parse_args(argv)

    repo = args.repo.resolve()
    media_extensions = DEFAULT_MEDIA_EXTENSIONS | parse_extensions(args.extra_media_ext)

    try:
        git_dir = run_git(repo, ["rev-parse", "--git-dir"]).strip()
    except GitError as ex:
        print(ex, file=sys.stderr)
        return 2

    try:
        findings = find_media_blobs(repo, media_extensions=media_extensions)
    except GitError as ex:
        print(ex, file=sys.stderr)
        return 2

    if not findings:
        print(f"OK: no historical media assets found in {repo}")
        return 0

    print(f"Repository: {repo}")
    print(f"Git dir:    {git_dir}")
    print(f"Findings:   {len(findings)} historical media blob(s)\n")

    for finding in findings:
        size = int(finding["size"])
        sha = str(finding["sha"])
        extensions = ",".join(str(ext) for ext in finding["extensions"])
        paths = list(finding["paths"])
        print(f"{human_size(size):>10}  {sha}  media extension {extensions}")
        for path in paths[: args.max_paths_per_blob]:
            print(f"            {path}")
        remaining = len(paths) - args.max_paths_per_blob
        if remaining > 0:
            print(f"            ... {remaining} more path(s)")
        print()

    print(
        "These media blobs may no longer exist in the current checkout but are still reachable from Git history."
    )
    print(
        "If they should be removed, use a history rewrite tool such as git-filter-repo or BFG Repo-Cleaner."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
