#!/usr/bin/env python3
"""Collect an optimization baseline report for the Ambition repo.

This script is intentionally stdlib-only. It runs a curated set of Cargo,
git, and platform commands, captures stdout/stderr/timing/status, copies Cargo
timing HTML reports, summarizes source and binary sizes, writes a Markdown
LLM-friendly report, and zips the bundle.

Version 2 changes:
- stdout/stderr logs are raw payloads, so summary extraction is reliable.
- combined logs still include command metadata for human review.
- cargo check probes use short diagnostics by default.
- slower/noisier deep probes live behind --long-tests.
- the Markdown report includes concise failure excerpts instead of huge dumps.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as _dt
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import textwrap
import time
import zipfile
from pathlib import Path
from typing import Iterable, Sequence

REPORT_TOOL_VERSION = "2"
DEFAULT_FAILURE_EXCERPT_LINES = 80
DEFAULT_FAILURE_EXCERPT_BYTES = 12_000


@dataclasses.dataclass
class CommandResult:
    label: str
    argv: list[str]
    cwd: str
    returncode: int
    seconds: float
    stdout_path: str
    stderr_path: str
    combined_path: str
    meta_path: str
    stdout_bytes: int
    stderr_bytes: int
    stdout_sha256: str
    stderr_sha256: str

    @property
    def ok(self) -> bool:
        return self.returncode == 0


def utc_now_iso() -> str:
    return _dt.datetime.now(tz=_dt.timezone.utc).isoformat(timespec="seconds")


def shell_quote(argv: Sequence[str]) -> str:
    # Good enough for display. We do not execute through the shell.
    out = []
    for arg in argv:
        if not arg:
            out.append("''")
        elif all(ch.isalnum() or ch in "_@%+=:,./-" for ch in arg):
            out.append(arg)
        else:
            out.append("'" + arg.replace("'", "'\\''") + "'")
    return " ".join(out)


def safe_label(label: str) -> str:
    cleaned = []
    for ch in label.lower():
        if ch.isalnum():
            cleaned.append(ch)
        else:
            cleaned.append("_")
    s = "".join(cleaned).strip("_")
    while "__" in s:
        s = s.replace("__", "_")
    return s or "command"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()


def command_env() -> dict[str, str]:
    env = os.environ.copy()
    # Keep logs deterministic and avoid noisy color escapes/backtraces unless
    # the caller explicitly overrides them.
    env.setdefault("CARGO_TERM_COLOR", "never")
    env.setdefault("RUST_BACKTRACE", "0")
    return env


def run_command(
    label: str,
    argv: Sequence[str],
    repo: Path,
    logs_dir: Path,
    env: dict[str, str] | None = None,
) -> CommandResult:
    logs_dir.mkdir(parents=True, exist_ok=True)
    slug = safe_label(label)
    stdout_path = logs_dir / f"{slug}.stdout.txt"
    stderr_path = logs_dir / f"{slug}.stderr.txt"
    combined_path = logs_dir / f"{slug}.combined.txt"
    meta_path = logs_dir / f"{slug}.meta.json"

    started_utc = utc_now_iso()
    started = time.perf_counter()
    print(f"[optimization-report] RUN {label}: {shell_quote(list(argv))}", flush=True)
    run_env = command_env()
    if env is not None:
        run_env.update(env)
    try:
        proc = subprocess.run(
            list(argv),
            cwd=str(repo),
            env=run_env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        stdout = proc.stdout
        stderr = proc.stderr
        returncode = proc.returncode
    except FileNotFoundError as ex:
        stdout = ""
        stderr = f"command not found: {argv[0]}\n{ex}\n"
        returncode = 127
    except Exception as ex:  # keep collecting after unexpected command failures
        stdout = ""
        stderr = f"failed to execute command: {type(ex).__name__}: {ex}\n"
        returncode = 126
    seconds = time.perf_counter() - started
    finished_utc = utc_now_iso()

    stdout_path.write_text(stdout, encoding="utf-8", errors="replace")
    stderr_path.write_text(stderr, encoding="utf-8", errors="replace")

    meta = {
        "label": label,
        "cwd": str(repo),
        "command": list(argv),
        "command_display": shell_quote(list(argv)),
        "started_utc": started_utc,
        "finished_utc": finished_utc,
        "returncode": returncode,
        "seconds": seconds,
        "stdout_bytes": len(stdout.encode("utf-8", errors="replace")),
        "stderr_bytes": len(stderr.encode("utf-8", errors="replace")),
        "stdout_sha256": sha256_text(stdout),
        "stderr_sha256": sha256_text(stderr),
    }
    meta_path.write_text(json.dumps(meta, indent=2), encoding="utf-8")

    header = (
        f"# label: {label}\n"
        f"# cwd: {repo}\n"
        f"# command: {shell_quote(list(argv))}\n"
        f"# started_utc: {started_utc}\n"
        f"# finished_utc: {finished_utc}\n"
        f"# returncode: {returncode}\n"
        f"# seconds: {seconds:.3f}\n"
        f"# stdout_bytes: {meta['stdout_bytes']}\n"
        f"# stderr_bytes: {meta['stderr_bytes']}\n"
        f"\n"
    )
    combined_path.write_text(
        header + "## STDOUT\n\n" + stdout + "\n\n## STDERR\n\n" + stderr,
        encoding="utf-8",
        errors="replace",
    )
    status = "OK" if returncode == 0 else f"FAIL({returncode})"
    print(f"[optimization-report] {status} {label} in {seconds:.1f}s", flush=True)
    return CommandResult(
        label=label,
        argv=list(argv),
        cwd=str(repo),
        returncode=returncode,
        seconds=seconds,
        stdout_path=str(stdout_path.relative_to(logs_dir.parent)),
        stderr_path=str(stderr_path.relative_to(logs_dir.parent)),
        combined_path=str(combined_path.relative_to(logs_dir.parent)),
        meta_path=str(meta_path.relative_to(logs_dir.parent)),
        stdout_bytes=int(meta["stdout_bytes"]),
        stderr_bytes=int(meta["stderr_bytes"]),
        stdout_sha256=str(meta["stdout_sha256"]),
        stderr_sha256=str(meta["stderr_sha256"]),
    )


def git_tracked_files(repo: Path) -> list[Path]:
    try:
        proc = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=str(repo),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if proc.returncode == 0:
            raw = proc.stdout.split(b"\0")
            return [repo / p.decode("utf-8", errors="replace") for p in raw if p]
    except Exception:
        pass

    ignored = {".git", "target"}
    files: list[Path] = []
    for path in repo.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(repo).parts
        if any(part in ignored for part in rel_parts):
            continue
        files.append(path)
    return files


def file_kind(path: Path) -> str:
    try:
        data = path.read_bytes()[:4096]
    except Exception:
        return "unknown"
    if b"\0" in data:
        return "binary"
    try:
        data.decode("utf-8")
        return "text"
    except UnicodeDecodeError:
        return "binary-ish"


def collect_source_stats(repo: Path, out: Path) -> dict[str, object]:
    files = [p for p in git_tracked_files(repo) if p.exists()]
    by_ext: dict[str, dict[str, int]] = {}
    largest = []
    total_bytes = 0
    text_files = 0
    binary_files = 0
    for path in files:
        try:
            size = path.stat().st_size
        except OSError:
            continue
        total_bytes += size
        rel = path.relative_to(repo).as_posix()
        ext = path.suffix.lower() or "[no extension]"
        entry = by_ext.setdefault(ext, {"files": 0, "bytes": 0})
        entry["files"] += 1
        entry["bytes"] += size
        kind = file_kind(path)
        if kind == "text":
            text_files += 1
        elif kind.startswith("binary"):
            binary_files += 1
        largest.append({"path": rel, "bytes": size, "kind": kind})
    largest.sort(key=lambda item: item["bytes"], reverse=True)
    ext_rows = [
        {"extension": ext, "files": vals["files"], "bytes": vals["bytes"]}
        for ext, vals in by_ext.items()
    ]
    ext_rows.sort(key=lambda item: item["bytes"], reverse=True)
    stats = {
        "tracked_file_count": len(files),
        "tracked_total_bytes": total_bytes,
        "tracked_text_file_count": text_files,
        "tracked_binary_file_count": binary_files,
        "largest_tracked_files": largest[:80],
        "extensions_by_size": ext_rows[:80],
    }
    (out / "source_stats.json").write_text(json.dumps(stats, indent=2), encoding="utf-8")
    return stats


def human_bytes(num: int | float | None) -> str:
    if num is None:
        return "n/a"
    n = float(num)
    for unit in ["B", "KiB", "MiB", "GiB", "TiB"]:
        if abs(n) < 1024.0 or unit == "TiB":
            if unit == "B":
                return f"{int(n)} {unit}"
            return f"{n:.1f} {unit}"
        n /= 1024.0
    return f"{n:.1f} TiB"


def collect_binary_sizes(repo: Path, out: Path) -> list[dict[str, object]]:
    candidates = []
    for profile in ["debug", "release", "distribution"]:
        profile_dir = repo / "target" / profile
        if not profile_dir.exists():
            continue
        for name in [
            "ambition_sandbox",
            "headless",
            "rl_random_walker",
            "rl_smoke",
            "trace_replay",
        ]:
            path = profile_dir / name
            if path.exists() and path.is_file():
                candidates.append(
                    {
                        "profile": profile,
                        "name": name,
                        "path": path.relative_to(repo).as_posix(),
                        "bytes": path.stat().st_size,
                    }
                )
    candidates.sort(key=lambda item: (str(item["profile"]), -int(item["bytes"])))
    (out / "binary_sizes.json").write_text(json.dumps(candidates, indent=2), encoding="utf-8")
    return candidates


def copy_cargo_timings(repo: Path, out: Path) -> list[str]:
    timing_src = repo / "target" / "cargo-timings"
    timing_dst = out / "cargo_timings"
    copied: list[str] = []
    if not timing_src.exists():
        return copied
    timing_dst.mkdir(parents=True, exist_ok=True)
    for path in sorted(timing_src.glob("*")):
        if path.is_file() and path.suffix.lower() in {".html", ".json"}:
            dst = timing_dst / path.name
            try:
                shutil.copy2(path, dst)
                copied.append(dst.relative_to(out).as_posix())
            except OSError:
                pass
    return copied


def cargo_profile_exists(repo: Path, profile_name: str) -> bool:
    root = repo / "Cargo.toml"
    try:
        text = root.read_text(encoding="utf-8")
    except OSError:
        return False
    return f"[profile.{profile_name}]" in text


def cargo_bloat_available(repo: Path, logs_dir: Path) -> bool:
    result = run_command("probe_cargo_bloat", ["cargo", "bloat", "--version"], repo, logs_dir)
    return result.returncode == 0


def llvm_lines_available(repo: Path, logs_dir: Path) -> bool:
    result = run_command("probe_cargo_llvm_lines", ["cargo", "llvm-lines", "--version"], repo, logs_dir)
    return result.returncode == 0


def write_json(path: Path, data: object) -> None:
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def markdown_table(headers: Sequence[str], rows: Iterable[Sequence[str]]) -> str:
    rows = list(rows)
    if not rows:
        return "_None._"
    lines = []
    lines.append("| " + " | ".join(headers) + " |")
    lines.append("| " + " | ".join(["---"] * len(headers)) + " |")
    for row in rows:
        lines.append("| " + " | ".join(str(cell).replace("\n", "<br>") for cell in row) + " |")
    return "\n".join(lines)


def first_nonempty_line(path: Path) -> str:
    try:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            s = line.strip()
            if s:
                return s
    except OSError:
        pass
    return ""


def tail_excerpt(text: str, max_lines: int, max_bytes: int) -> str:
    if not text:
        return ""
    encoded = text.encode("utf-8", errors="replace")
    truncated_by_bytes = False
    if len(encoded) > max_bytes:
        encoded = encoded[-max_bytes:]
        text = encoded.decode("utf-8", errors="replace")
        truncated_by_bytes = True
    lines = text.splitlines()
    truncated_by_lines = False
    if len(lines) > max_lines:
        lines = lines[-max_lines:]
        truncated_by_lines = True
    prefix = []
    if truncated_by_bytes or truncated_by_lines:
        prefix.append(
            f"[excerpt: last {len(lines)} lines, capped at {human_bytes(max_bytes)}; see full log in logs/]"
        )
    return "\n".join(prefix + lines)


def failure_excerpts(
    out: Path,
    commands: list[CommandResult],
    max_lines: int,
    max_bytes: int,
) -> str:
    chunks = []
    for cmd in commands:
        if cmd.returncode == 0:
            continue
        stdout = ""
        stderr = ""
        try:
            stdout = (out / cmd.stdout_path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            pass
        try:
            stderr = (out / cmd.stderr_path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            pass
        stderr_excerpt = tail_excerpt(stderr, max_lines=max_lines, max_bytes=max_bytes)
        stdout_excerpt = tail_excerpt(stdout, max_lines=max_lines, max_bytes=max_bytes)
        body_parts = []
        if stderr_excerpt:
            body_parts.append("stderr excerpt:\n\n```text\n" + stderr_excerpt + "\n```")
        if stdout_excerpt and not stderr_excerpt:
            body_parts.append("stdout excerpt:\n\n```text\n" + stdout_excerpt + "\n```")
        if not body_parts:
            body_parts.append("No stdout/stderr payload captured. See command metadata.")
        chunks.append(
            f"### {cmd.label}\n\n"
            f"- status: `{cmd.returncode}`\n"
            f"- full log: `{cmd.combined_path}`\n"
            f"- command: `{shell_quote(cmd.argv)}`\n\n"
            + "\n\n".join(body_parts)
        )
    return "\n\n".join(chunks) if chunks else "_No command failures._"


def write_report(
    repo: Path,
    out: Path,
    args: argparse.Namespace,
    commands: list[CommandResult],
    source_stats: dict[str, object],
    binaries: list[dict[str, object]],
    timings: list[str],
) -> Path:
    failed = [cmd for cmd in commands if cmd.returncode != 0]
    rows = []
    for cmd in commands:
        status = "OK" if cmd.returncode == 0 else f"FAIL {cmd.returncode}"
        rows.append(
            [
                cmd.label,
                status,
                f"{cmd.seconds:.1f}s",
                f"out {human_bytes(cmd.stdout_bytes)} / err {human_bytes(cmd.stderr_bytes)}",
                f"`{cmd.combined_path}`",
                f"`{shell_quote(cmd.argv)}`",
            ]
        )

    largest_rows = []
    for item in source_stats.get("largest_tracked_files", [])[:25]:
        largest_rows.append([
            f"`{item['path']}`",
            human_bytes(int(item["bytes"])),
            str(item.get("kind", "")),
        ])

    ext_rows = []
    for item in source_stats.get("extensions_by_size", [])[:25]:
        ext_rows.append([
            f"`{item['extension']}`",
            str(item["files"]),
            human_bytes(int(item["bytes"])),
        ])

    bin_rows = []
    for item in binaries:
        bin_rows.append([
            str(item["profile"]),
            str(item["name"]),
            human_bytes(int(item["bytes"])),
            f"`{item['path']}`",
        ])

    timing_lines = "\n".join(f"- `{p}`" for p in timings) if timings else "- No Cargo timing files found."

    git_head = "unknown"
    for cmd in commands:
        if cmd.label == "git_rev_parse_head" and cmd.returncode == 0:
            git_head = first_nonempty_line(out / cmd.stdout_path) or "see log"

    failed_table_rows = []
    for cmd in failed:
        failed_table_rows.append([
            cmd.label,
            str(cmd.returncode),
            f"`{cmd.combined_path}`",
        ])

    failure_block = failure_excerpts(
        out,
        commands,
        max_lines=args.max_failure_lines,
        max_bytes=args.max_failure_bytes,
    )

    md = f"""# Ambition optimization baseline report

Generated: {utc_now_iso()}  
Repository: `{repo}`  
Git HEAD: `{git_head}`  
Report directory: `{out}`  
Report tool version: `{REPORT_TOOL_VERSION}`

## How to use this report with an LLM

Give the LLM this Markdown file plus the zipped report bundle from the same directory. Ask it to inspect the command logs, Cargo timing HTML, Cargo trees, binary sizes, and source-size summary before proposing changes. Prefer recommendations that preserve the current feature-gated architecture and produce measurable deltas against this baseline.

The command table intentionally stays concise. Full stdout/stderr payloads are in `logs/*.stdout.txt` and `logs/*.stderr.txt`; `logs/*.combined.txt` includes metadata plus both streams.

## Run options

- quick mode: `{args.quick}`
- clean first: `{args.clean}`
- long tests/deep probes: `{args.long_tests}`
- short cargo diagnostics: `{not args.long_tests}`
- strict failure exit: `{args.strict}`
- failure excerpt cap: `{args.max_failure_lines}` lines / `{human_bytes(args.max_failure_bytes)}`

## High-level status

- Commands run: {len(commands)}
- Failed commands: {len(failed)}
- Tracked files: {source_stats.get('tracked_file_count')}
- Tracked source/data size: {human_bytes(int(source_stats.get('tracked_total_bytes', 0)))}
- Text files: {source_stats.get('tracked_text_file_count')}
- Binary-ish files: {source_stats.get('tracked_binary_file_count')}

## Failed commands

{markdown_table(['label', 'status', 'full log'], failed_table_rows)}

## Failure excerpts

{failure_block}

## Command results

{markdown_table(['label', 'status', 'duration', 'output size', 'log', 'command'], rows)}

## Binary sizes found after builds

{markdown_table(['profile', 'binary', 'size', 'path'], bin_rows) if bin_rows else 'No known Ambition binaries were found under `target/{{debug,release,distribution}}`.'}

## Cargo timing artifacts copied

{timing_lines}

## Largest tracked files

{markdown_table(['path', 'size', 'kind'], largest_rows)}

## Largest tracked extensions

{markdown_table(['extension', 'files', 'bytes'], ext_rows)}

## Suggested first questions for optimization review

1. Which crates dominate `cargo build --timings -p ambition_sandbox`?
2. Which dependencies remain in the `--no-default-features --features rl,headless,ldtk_runtime` tree, and which should drop out of a truly headless build?
3. How much smaller is release versus distribution, if a distribution profile exists?
4. Are large generated assets externalized, or are any accidentally embedded / tracked?
5. If `--long-tests` was enabled, do `cargo bloat` or `cargo llvm-lines` identify a few dominant symbols or monomorphizations?

## Notes

This report intentionally lives under `target/optimization_reports`, which should not be checked in. The zip bundle is intended to be attached to an LLM conversation or issue.
"""
    report_path = out / "llm_optimization_report.md"
    report_path.write_text(md, encoding="utf-8")
    return report_path


def zip_report(out: Path) -> Path:
    zip_path = out / f"ambition_optimization_report_{out.name}.zip"
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(out.rglob("*")):
            if path == zip_path:
                continue
            if path.is_file():
                zf.write(path, path.relative_to(out.parent).as_posix())
    return zip_path


def cargo_check_cmd(*extra: str, long_tests: bool) -> list[str]:
    cmd = ["cargo", "check"]
    if not long_tests:
        cmd.append("--message-format=short")
    cmd.extend(extra)
    return cmd


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Collect an optimization baseline report for Ambition.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent(
            """
            Examples:
              ./run_optimization_report.sh
              ./run_optimization_report.sh --quick
              ./run_optimization_report.sh --clean
              ./run_optimization_report.sh --long-tests
              AMBITION_OPT_REPORT_DIR=/tmp/ambition-report ./run_optimization_report.sh
            """
        ),
    )
    parser.add_argument("--repo", required=True, type=Path, help="Repository root")
    parser.add_argument("--out", required=True, type=Path, help="Output report directory")
    parser.add_argument("--quick", action="store_true", help="Skip release/distribution builds and optional deep probes")
    parser.add_argument("--clean", action="store_true", help="Run cargo clean before measuring")
    parser.add_argument("--long-tests", action="store_true", help="Run slower/noisier deep probes, including optional bloat tools and full diagnostics")
    parser.add_argument("--strict", action="store_true", help="Exit non-zero if any measured command fails")
    parser.add_argument("--max-failure-lines", type=int, default=DEFAULT_FAILURE_EXCERPT_LINES, help="Max lines per failed-command excerpt in the Markdown report")
    parser.add_argument("--max-failure-bytes", type=int, default=DEFAULT_FAILURE_EXCERPT_BYTES, help="Max bytes per failed-command excerpt in the Markdown report")
    args = parser.parse_args(argv)

    repo = args.repo.resolve()
    out = args.out.resolve()
    logs_dir = out / "logs"
    out.mkdir(parents=True, exist_ok=True)
    logs_dir.mkdir(parents=True, exist_ok=True)

    if not (repo / "Cargo.toml").exists():
        print(f"error: {repo} does not look like a Cargo repo root", file=sys.stderr)
        return 2

    meta = {
        "generated_utc": utc_now_iso(),
        "report_tool_version": REPORT_TOOL_VERSION,
        "repo": str(repo),
        "out": str(out),
        "python": sys.version,
        "platform": platform.platform(),
        "args": vars(args) | {"repo": str(repo), "out": str(out)},
    }
    write_json(out / "report_metadata.json", meta)

    commands: list[CommandResult] = []

    def run(label: str, cmd: Sequence[str]) -> CommandResult:
        result = run_command(label, cmd, repo, logs_dir)
        commands.append(result)
        write_json(out / "commands_so_far.json", [dataclasses.asdict(c) for c in commands])
        return result

    run("git_rev_parse_head", ["git", "rev-parse", "HEAD"])
    run("git_status_short", ["git", "status", "--short", "--branch"])
    run("git_submodule_status", ["git", "submodule", "status", "--recursive"])
    run("rustc_version_verbose", ["rustc", "-Vv"])
    run("cargo_version_verbose", ["cargo", "-Vv"])
    run("cargo_metadata_no_deps", ["cargo", "metadata", "--format-version", "1", "--no-deps"])
    if args.long_tests:
        run("cargo_metadata_full", ["cargo", "metadata", "--format-version", "1"])

    source_stats = collect_source_stats(repo, out)

    if args.clean:
        run("cargo_clean", ["cargo", "clean"])

    # Dependency / feature graph probes. These are intentionally captured even
    # if later builds fail.
    run("cargo_tree_workspace_duplicates", ["cargo", "tree", "-d"])
    run("cargo_tree_sandbox_features_default", ["cargo", "tree", "-e", "features", "-p", "ambition_sandbox"])
    run(
        "cargo_tree_sandbox_features_headlessish",
        [
            "cargo",
            "tree",
            "-e",
            "features",
            "-p",
            "ambition_sandbox",
            "--no-default-features",
            "--features",
            "rl_sim,headless,ldtk_runtime",
        ],
    )

    check_results = []
    check_results.append(run("cargo_check_sfx_crates", cargo_check_cmd("-p", "ambition_sfx", "-p", "ambition_sfx_bank", long_tests=args.long_tests)))
    # The former ambition_engine crate is gone; its code lives in
    # ambition_sandbox/src/engine_core and is covered by the sandbox check below.
    check_results.append(run("cargo_check_sandbox_default", cargo_check_cmd("-p", "ambition_sandbox", long_tests=args.long_tests)))
    check_results.append(
        run(
            "cargo_check_sandbox_headlessish",
            cargo_check_cmd(
                "-p",
                "ambition_sandbox",
                "--no-default-features",
                "--features",
                "rl_sim,headless,ldtk_runtime",
                long_tests=args.long_tests,
            ),
        )
    )

    # Build/timing baseline. cargo --timings writes HTML under target/cargo-timings.
    if not args.quick:
        run("cargo_build_timings_sandbox_default", ["cargo", "build", "--timings", "-p", "ambition_sandbox"])

        run("cargo_build_timings_sandbox_release", ["cargo", "build", "--timings", "-p", "ambition_sandbox", "--release"])
        if cargo_profile_exists(repo, "distribution"):
            run(
                "cargo_build_timings_sandbox_distribution",
                ["cargo", "build", "--timings", "-p", "ambition_sandbox", "--profile", "distribution"],
            )
        else:
            (logs_dir / "cargo_build_timings_sandbox_distribution.skipped.txt").write_text(
                "Skipped: no [profile.distribution] found in root Cargo.toml.\n",
                encoding="utf-8",
            )
    else:
        (logs_dir / "quick_mode.skipped.txt").write_text(
            "Quick mode enabled: skipped build --timings release/distribution builds and optional deep probes.\n",
            encoding="utf-8",
        )

    # Optional deep binary-size tools. These are slow/noisy, so they are only
    # run when explicitly requested.
    if args.long_tests and not args.quick:
        if cargo_bloat_available(repo, logs_dir):
            run(
                "cargo_bloat_release_ambition_sandbox_top50",
                ["cargo", "bloat", "--release", "-p", "ambition_sandbox", "--bin", "ambition_sandbox", "-n", "50"],
            )
        else:
            (logs_dir / "cargo_bloat_release_ambition_sandbox_top50.skipped.txt").write_text(
                "Skipped: cargo-bloat is not installed. Install with `cargo install cargo-bloat`.\n",
                encoding="utf-8",
            )
        if llvm_lines_available(repo, logs_dir):
            run(
                "cargo_llvm_lines_release_ambition_sandbox_top50",
                ["cargo", "llvm-lines", "--release", "-p", "ambition_sandbox", "--bin", "ambition_sandbox", "--lines", "50"],
            )
        else:
            (logs_dir / "cargo_llvm_lines_release_ambition_sandbox_top50.skipped.txt").write_text(
                "Skipped: cargo-llvm-lines is not installed. Install with `cargo install cargo-llvm-lines`.\n",
                encoding="utf-8",
            )
    else:
        (logs_dir / "long_tests.skipped.txt").write_text(
            "Skipped: --long-tests not enabled; cargo-bloat, cargo-llvm-lines, and full cargo metadata were not run.\n",
            encoding="utf-8",
        )

    # Platform binary introspection after builds. Failure is fine; logs explain missing files.
    for profile_name in ["debug", "release", "distribution"]:
        candidate = repo / "target" / profile_name / "ambition_sandbox"
        if candidate.exists():
            run(f"file_{profile_name}_ambition_sandbox", ["file", str(candidate)])
            run(f"size_{profile_name}_ambition_sandbox", ["size", str(candidate)])

    timings = copy_cargo_timings(repo, out)
    binaries = collect_binary_sizes(repo, out)
    write_json(out / "commands.json", [dataclasses.asdict(c) for c in commands])
    report_path = write_report(repo, out, args, commands, source_stats, binaries, timings)
    zip_path = zip_report(out)

    print("", flush=True)
    print(f"[optimization-report] wrote report: {report_path}")
    print(f"[optimization-report] wrote bundle: {zip_path}")
    print("[optimization-report] hand the Markdown file or zip bundle to an LLM for review")

    if args.strict and any(cmd.returncode != 0 for cmd in commands):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
