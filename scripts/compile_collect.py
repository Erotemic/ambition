#!/usr/bin/env python3
"""Collect per-unit compile telemetry under named build configurations.

Unlike `compile_ratchet.py`, this script performs real Cargo builds. Each
configuration owns a separate target directory and runs serially so fingerprint
changes cannot invalidate another measurement. `cold` records the full graph;
`first-party` applies temporary source edits to measure the rebuild Ambition
itself pays.

Configuration facts such as optimization and incremental mode are read from or
cross-checked against the rustc invocation rather than inferred from manifest or
parent-environment defaults. Temporary edits are restored from saved bytes, and
the script refuses to touch dirty probe files.

Usage::

    python3 scripts/compile_collect.py --config dev
    python3 scripts/compile_collect.py --config dev --config release
    python3 scripts/compile_collect.py --config dev --phase first-party
    python3 scripts/compile_collect.py --list"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import compile_ratchet as ratchet  # noqa: E402
import measurement_paths  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
UNIT_LEDGER = ratchet.UNIT_LEDGER

# Outside the repo and outside the shared `.cargo/config.toml` target dir on
# purpose: a collector that writes into the dev loop's target dir invalidates
# the dev loop, and the dev loop invalidates the measurement. Same VOLUME as it
# though — the comparison is only valid on one filesystem, and `~` here is on
# the same device the configured `target-dir` is.
DEFAULT_TARGET_ROOT = Path.home() / "ambition-telemetry-target"

# The marker is a real edit, not a `touch`.
MARKER = "\n#[allow(dead_code)]\nfn _compile_collect_probe(x: u32) -> u32 {{ x.wrapping_add({salt}) }}\n"


@dataclass(frozen=True)
class Config:
    name: str
    profile: str  # what the cargo timing report's `Profile:` header will say
    incremental: bool
    cargo_args: list[str] = field(default_factory=list)
    why: str = ""
    manifest_edits: tuple[tuple[str, str, str], ...] = ()
    # Which target dir to build in. A variant that differs only by a manifest
    # knob shares its base configuration's dir on purpose: the comparison is
    # only meaningful warm, and a cold variant measures the dependency tree
    # instead of the knob.
    target_key: str = ""


# **deliberately three, not a matrix.** Each cold phase is a full build of the workspace and its
# dependency tree; a 2x2x2 nobody finishes is worse than three configurations that land.
CONFIGS: dict[str, Config] = {
    c.name: c
    for c in (
        Config(
            name="dev",
            profile="test",
            incremental=False,
            why="what `run_tests.py` builds under — it forces CARGO_INCREMENTAL=0",
        ),
        Config(
            name="dev-incremental",
            profile="test",
            incremental=True,
            why="what the edit loop builds under — `.cargo/config.toml` sets incremental=true",
        ),
        Config(
            name="release",
            profile="release",
            incremental=False,
            cargo_args=["--release"],
            why="the optimization-mode axis: opt-level 3 everywhere, no debug-assertions",
        ),
        # prices ONE manifest knob against the `dev` configuration it shares a target dir with.
        # `ambition_app` is the only crate in the workspace declaring a `cdylib` (it is how the
        # Android build gets its `.so`), and a unit that emits a cdylib emits no rmeta — so cargo
        # cannot pipeline it in either direction.
        Config(
            name="dev-app-rlib-only",
            profile="test",
            incremental=False,
            target_key="dev",
            manifest_edits=(
                (
                    "game/ambition_app/Cargo.toml",
                    'crate-type = ["rlib", "cdylib"]',
                    'crate-type = ["rlib"]',
                ),
            ),
            why="prices the app's Android cdylib, which is what disables pipelining for it",
        ),
    )
}

# The command every configuration runs.
BASE_COMMAND = ["cargo", "test", "-p", "ambition_app", "--test", "app_it", "--no-run"]


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    ).stdout.strip()


# ---------------------------------------------------------------------------
# reading what cargo ACTUALLY did
# ---------------------------------------------------------------------------

_CRATE_NAME = re.compile(r"--crate-name (\S+)")
_OPT_LEVEL = re.compile(r"-C opt-level=(\S+?)['\"]?(?:\s|$)")
_CODEGEN_UNITS = re.compile(r"-C codegen-units=(\d+)")
_INCREMENTAL_FLAG = re.compile(r"-C incremental=")
_SOURCE_PATH = re.compile(r"\s(\S+\.rs)\s")


def rustc_invocations(stderr: str, member_dirs: dict[str, Path]) -> dict[tuple, dict]:
    """What `cargo -v` says rustc was actually told, keyed to a compile unit.

    ⛔ **this is the whole reason `-v` is passed.** Every other route to
    `opt-level` is a model of cargo's profile resolution, and this repo has
    already shipped one that was backwards. The rustc command line is the
    ground truth and cargo prints it for free.

    Keyed `(package_or_None, crate_name)`. First-party units resolve by the
    source path's owning workspace member, which is exact; dependencies fall
    back to the crate name, which can collide across packages (`build_script_build`
    collides for every package that has one). A collision is recorded as such
    rather than resolved by guessing.
    """
    by_dir = {str(path.resolve()): name for name, path in member_dirs.items()}
    found: dict[tuple, list[dict]] = {}
    for line in stderr.splitlines():
        if "--crate-name" not in line:
            continue
        name_match = _CRATE_NAME.search(line)
        if not name_match:
            continue
        crate_name = name_match.group(1)

        package = None
        source = _SOURCE_PATH.search(line)
        if source:
            # cargo prints a workspace member's source path RELATIVE to the
            # workspace root and a registry crate's absolutely. Resolving both
            # against ROOT is right for the first and harmless for the second.
            path = Path(source.group(1))
            path = path if path.is_absolute() else (ROOT / path)
            for parent in path.parents:
                hit = by_dir.get(str(parent))
                if hit:
                    package = hit
                    break

        opt = _OPT_LEVEL.search(line)
        units = _CODEGEN_UNITS.search(line)
        found.setdefault((package, crate_name), []).append(
            {
                "opt_level": opt.group(1).strip("\"'") if opt else None,
                "codegen_units": int(units.group(1)) if units else None,
                "incremental": bool(_INCREMENTAL_FLAG.search(line)),
            }
        )

    resolved: dict[tuple, dict] = {}
    for key, seen in found.items():
        levels = {entry["opt_level"] for entry in seen}
        entry = dict(seen[0])
        entry["opt_level_source"] = (
            "rustc-argv" if len(levels) == 1 else "rustc-argv-ambiguous"
        )
        resolved[key] = entry
    return resolved


def unit_crate_name(unit: dict) -> str:
    """The `--crate-name` cargo would pass for this timing-report unit.

    The report's `target` is `""` for a lib and `app_it "test" (test)` /
    `ambition_game_bin "bin"` otherwise; the first token is the target name,
    and cargo turns `-` into `_` for the crate name.
    """
    target = (unit.get("target") or "").strip()
    token = target.split()[0] if target else unit["name"]
    return token.replace("-", "_")


# ---------------------------------------------------------------------------
# the probe edit
# ---------------------------------------------------------------------------


def first_party_roots(member_dirs: dict[str, Path], graph_crates: set[str]) -> dict[str, Path]:
    """`lib.rs` for every first-party crate in the consumer's resolved graph.

    Only crates the consumer actually links: perturbing a crate outside the
    graph costs a build and teaches nothing about the build being measured.
    """
    roots: dict[str, Path] = {}
    for name, directory in member_dirs.items():
        if name not in graph_crates:
            continue
        candidate = directory / "src" / "lib.rs"
        if candidate.exists():
            roots[name] = candidate
    return roots


def apply_manifest_edits(config: Config) -> dict[Path, bytes]:
    """Exact-text substitutions for a configuration, with the bytes kept to undo."""
    original: dict[Path, bytes] = {}
    for relative, old, new in config.manifest_edits:
        path = ROOT / relative
        if git("status", "--porcelain", "--", relative):
            raise SystemExit(
                f"⛔ {relative} has uncommitted changes and this configuration "
                "rewrites it; refusing rather than risking your work."
            )
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise SystemExit(
                f"⛔ {relative} does not contain {old!r}, so this configuration "
                "would measure something other than what it claims. Fix the "
                "edit before trusting the number."
            )
        original[path] = path.read_bytes()
        path.write_text(text.replace(old, new, 1), encoding="utf-8")
        print(f"  manifest: {relative}  {old!r} -> {new!r}", flush=True)
    return original


def perturb(paths: dict[str, Path]) -> dict[Path, bytes]:
    dirty = git("status", "--porcelain", "--", *[str(p) for p in paths.values()])
    if dirty:
        raise SystemExit(
            "⛔ these files already have uncommitted changes and this script "
            "rewrites and restores them; refusing rather than risking your work:\n"
            + dirty
        )
    original = {path: path.read_bytes() for path in paths.values()}
    for index, path in enumerate(paths.values()):
        path.write_bytes(original[path] + MARKER.format(salt=index + 1).encode("utf-8"))
    return original


def restore(original: dict[Path, bytes]) -> None:
    for path, data in original.items():
        path.write_bytes(data)


# ---------------------------------------------------------------------------
# the build
# ---------------------------------------------------------------------------


def foreign_cargo(own_target: Path) -> list[str]:
    """Return other cargo processes and the target directory each uses.

    Separate target directories avoid fingerprint corruption but still contend
    for CPU, so overlapping builds are recorded with measurements. Match
    `/proc/<pid>/comm` to identify the executable without self-matching probes.
    """
    found: list[str] = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            if (entry / "comm").read_text().strip() != "cargo":
                continue
            raw = (entry / "environ").read_bytes().decode("utf-8", "replace")
            env = dict(kv.split("=", 1) for kv in raw.split("\0") if "=" in kv)
        except (OSError, ValueError):
            continue
        target = env.get("CARGO_TARGET_DIR", "(config default)")
        if Path(target) == own_target:
            continue
        found.append(f"pid {entry.name} -> {target}")
    return found


def newest_timing_report(target_dir: Path) -> Path:
    reports = sorted(
        (target_dir / "cargo-timings").glob("cargo-timing-*.html"),
        key=lambda p: p.stat().st_mtime,
    )
    if not reports:
        raise SystemExit(
            f"⛔ no timing report under {target_dir / 'cargo-timings'} — the build "
            "produced no `--timings` output, which means the measurement did not "
            "happen even if cargo exited 0."
        )
    return reports[-1]


def run_build(config: Config, target_dir: Path, *, verbose_argv: bool) -> tuple[float, str, dict]:
    command = [*BASE_COMMAND, *config.cargo_args, "--timings"]
    if verbose_argv:
        command.append("-v")
    env = {
        **os.environ,
        "CARGO_TARGET_DIR": str(target_dir),
        # SET, never inherited. `run_tests.py` copies the env and then
        # setdefaults this to "0" for its children, so a collector that read its
        # own environment would report `true` for exactly the runs that are off.
        "CARGO_INCREMENTAL": "1" if config.incremental else "0",
    }
    print(f"    $ CARGO_INCREMENTAL={env['CARGO_INCREMENTAL']} "
          f"CARGO_TARGET_DIR={target_dir} {' '.join(command)}", flush=True)

    intruders = foreign_cargo(target_dir)
    if intruders:
        print("  ⚠ ANOTHER CARGO IS ALREADY BUILDING on this box: "
              + "; ".join(intruders)
              + "\n    Its target dir differs, so nothing gets invalidated — but it "
                "is competing for the same cores and every duration below is "
                "inflated by the overlap. Recorded on the rows.", flush=True)

    samples: list[float] = []
    peak_foreign = len(intruders)
    start = time.monotonic()
    proc = subprocess.Popen(
        command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, env=env,
    )
    # A sampling thread rather than a before/after pair: contention that starts
    # halfway through is invisible to two endpoint readings and is exactly the
    # case that happened while this script was being written.
    import threading

    stop = threading.Event()

    def sample() -> None:
        nonlocal peak_foreign
        while not stop.wait(10.0):
            samples.append(os.getloadavg()[0])
            peak_foreign = max(peak_foreign, len(foreign_cargo(target_dir)))

    watcher = threading.Thread(target=sample, daemon=True)
    watcher.start()
    stdout, stderr = proc.communicate()
    stop.set()
    watcher.join(timeout=1.0)
    elapsed = time.monotonic() - start

    if proc.returncode != 0:
        tail = "\n".join((stderr or "").strip().splitlines()[-25:])
        raise SystemExit(f"⛔ build failed, so its timing is meaningless:\n{tail}")
    contention = {
        "load_mean": round(sum(samples) / len(samples), 2) if samples else None,
        "load_max": round(max(samples), 2) if samples else None,
        "cores": os.cpu_count(),
        "foreign_cargo_peak": peak_foreign,
    }
    return elapsed, stderr, contention


def collect(
    config: Config,
    *,
    target_root: Path,
    phases: list[str],
    record: bool,
) -> list[dict]:
    target_dir = target_root / (config.target_key or config.name)
    target_dir.mkdir(parents=True, exist_ok=True)

    member_dirs = ratchet.workspace_dirs()
    baseline = json.loads(ratchet.BASELINE.read_text(encoding="utf-8"))
    graph_crates = set(baseline["crates"])
    roots = first_party_roots(member_dirs, graph_crates)

    run_id = uuid.uuid4().hex[:12]
    rows: list[dict] = []

    manifest_original = apply_manifest_edits(config)
    try:
        rows = _phases(config, target_dir, phases, roots, member_dirs, run_id, record)
    finally:
        # written back from memory, never `git checkout --`. Same rule as
        # `compile_cost.py`: a checkout here deletes whatever uncommitted work
        # was in the manifest.
        restore(manifest_original)
    return rows


def _phases(config, target_dir, phases, roots, member_dirs, run_id, record) -> list[dict]:
    rows: list[dict] = []
    for phase in phases:
        print(f"\n▶ {config.name} / {phase}   ({config.why})", flush=True)
        head_before = git("rev-parse", "HEAD")
        original: dict[Path, bytes] = {}
        if phase == "first-party":
            print(f"  editing {len(roots)} first-party lib roots …", flush=True)
            original = perturb(roots)
        try:
            elapsed, stderr, contention = run_build(config, target_dir, verbose_argv=True)
        finally:
            if original:
                restore(original)
        print(f"  {phase} build: {elapsed:.1f}s   "
              f"load mean {contention['load_mean']} / max {contention['load_max']} on "
              f"{contention['cores']} cores, foreign cargo peak "
              f"{contention['foreign_cargo_peak']}", flush=True)

        moved = [
            path
            for path in git("diff", "--name-only", f"{head_before}..HEAD").splitlines()
            if path.endswith((".rs", "Cargo.toml", "Cargo.lock"))
        ] if git("rev-parse", "HEAD") != head_before else []
        if moved:
            print(f"  ⚠ {len(moved)} compiled input(s) changed under this build "
                  f"(first: {moved[0]}); rows are marked backfilled=true because "
                  f"`lines` no longer describes the tree that was compiled.",
                  flush=True)

        # Keep the raw `-v` transcript beside the target dir. It is the only
        # record of what rustc was actually told, it is where `--crate-type`,
        # `--emit` and every other flag this ledger does not have a column for
        # can still be read, and it is worthless the moment the build is gone.
        transcript = target_dir / f"cargo-verbose-{config.name}-{phase}.log"
        transcript.write_text(stderr, encoding="utf-8")

        actual = rustc_invocations(stderr, member_dirs)
        report = newest_timing_report(target_dir)

        def unit_extra(unit: dict, _actual=actual, _members=member_dirs) -> dict:
            key = (
                unit["name"] if unit["name"] in _members else None,
                unit_crate_name(unit),
            )
            hit = _actual.get(key) or _actual.get((None, unit_crate_name(unit)))
            if not hit or hit["opt_level"] is None:
                return {}
            return {
                "opt_level": hit["opt_level"],
                "opt_level_source": hit["opt_level_source"],
                "codegen_units": hit["codegen_units"],
                "incremental": hit["incremental"],
            }

        label = f"collector: {config.name}/{phase}"
        phase_rows = ratchet.ingest_timings(
            report,
            label=label,
            record=record,
            run_id=run_id,
            profile=config.profile,
            extra={
                "config": config.name,
                "phase": phase,
                "incremental": config.incremental,
                "opt_level_source": "manifest",
                "codegen_units": None,
                "backfilled": bool(moved),
                "build_wall_seconds": round(elapsed, 2),
                "build_target_dir": str(target_dir),
                **{f"build_{k}": v for k, v in contention.items()},
            },
            unit_extra=unit_extra,
        )
        rows.extend(phase_rows)

        measured = sum(1 for r in phase_rows if r.get("opt_level_source") == "rustc-argv")
        print(f"  {len(phase_rows)} unit row(s); opt-level read from rustc argv for "
              f"{measured} of them", flush=True)
        mismatch = [
            r["unit"] for r in phase_rows
            if r.get("opt_level_source", "").startswith("rustc-argv")
            and bool(r.get("incremental")) != config.incremental
        ]
        if mismatch:
            print(f"  ⚠ {len(mismatch)} unit(s) disagree with the configured "
                  f"incremental setting (first: {mismatch[0]}) — a unit compiled "
                  f"without `-C incremental=` under an incremental config is "
                  f"cargo's decision, not a bug in this script, but it is a "
                  f"finding worth reading.", flush=True)

    return rows


# ---------------------------------------------------------------------------
# reading the ledger back
# ---------------------------------------------------------------------------


def _rank(values: list[float]) -> list[float]:
    order = sorted(range(len(values)), key=lambda i: values[i])
    ranks = [0.0] * len(values)
    index = 0
    while index < len(order):
        stop = index
        while stop + 1 < len(order) and values[order[stop + 1]] == values[order[index]]:
            stop += 1
        shared = (index + stop) / 2 + 1
        for position in range(index, stop + 1):
            ranks[order[position]] = shared
        index = stop + 1
    return ranks


def _pearson(xs: list[float], ys: list[float]) -> float:
    n = len(xs)
    if n < 3:
        return float("nan")
    mx, my = sum(xs) / n, sum(ys) / n
    num = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    dx = sum((x - mx) ** 2 for x in xs) ** 0.5
    dy = sum((y - my) ** 2 for y in ys) ** 0.5
    return num / (dx * dy) if dx and dy else float("nan")


def _spearman(xs: list[float], ys: list[float]) -> float:
    return _pearson(_rank(xs), _rank(ys))


def load_units() -> list[dict]:
    if not UNIT_LEDGER.exists():
        raise SystemExit(f"⛔ {UNIT_LEDGER} does not exist; run a collection first")
    return [json.loads(line) for line in UNIT_LEDGER.read_text().splitlines() if line.strip()]


def analyze() -> int:
    """What the recorded seconds say. **Builds nothing.**

    Three questions, in the order they were asked:

    1. **Does `compile_ratchet.py`'s theory hold?** It guards four numbers
       derived from the graph on the premise that they predict compile cost, and
       nothing had ever tested that against seconds. Now both halves exist, so
       the correlation is printed whether it flatters the guard or not.
    2. **Where does the time go?** frontend / codegen / unattributed, per run.
    3. **Which crate costs more than its size predicts?** The residual against
       the run's own median ms/line — the number that says where to look next.
    """
    rows = load_units()
    baseline = json.loads(ratchet.BASELINE.read_text(encoding="utf-8"))
    table = baseline["crates"]

    runs: dict[tuple, list[dict]] = {}
    for row in rows:
        runs.setdefault(
            (row.get("config") or (row.get("label", "") or "?")[:24], row.get("phase", "-"),
             row["run_id"]), []
        ).append(row)

    print("═ runs in the ledger ═\n")
    print(f"  {'config':<26} {'phase':<12} {'n':>4} {'unit-s':>9} {'wall-s':>8} "
          f"{'prof':<8} {'inc':<6} commit")
    for (config, phase, _), group in runs.items():
        total = sum(r["seconds"] for r in group)
        wall = group[0].get("build_wall_seconds")
        print(f"  {str(config):<26} {str(phase):<12} {len(group):>4} {total:>9.1f} "
              f"{(f'{wall:.1f}' if wall else '-'):>8} "
              f"{str(group[0].get('build_profile')):<8} "
              f"{str(group[0].get('incremental')):<6} {group[0]['commit'][:8]}")

    print("\n═ where the seconds go ═\n")
    print(f"  {'config':<26} {'phase':<12} {'total':>9} {'frontend':>16} "
          f"{'codegen':>16} {'unattributed':>16}")
    for (config, phase, _), group in runs.items():
        total = sum(r["seconds"] for r in group)
        front = sum(r.get("frontend_seconds") or 0.0 for r in group)
        codegen = sum(r.get("codegen_seconds") or 0.0 for r in group)
        rest = total - front - codegen
        if not total:
            continue
        print(f"  {str(config):<26} {str(phase):<12} {total:>9.1f} "
              f"{front:>9.1f} ({front / total:>4.0%}) {codegen:>9.1f} "
              f"({codegen / total:>4.0%}) {rest:>9.1f} ({rest / total:>4.0%})")

    # The ratchet's two structural claims, tested against seconds rather than
    # against lines. `direct_dependents` is the only edge list the baseline
    # carries, and it is the one this needs: reverse edges.
    dependents = {name: set(entry.get("direct_dependents", ())) for name, entry in table.items()}

    def closure(start: str) -> set[str]:
        seen, queue = {start}, [start]
        while queue:
            for parent in dependents.get(queue.pop(), ()):
                if parent not in seen:
                    seen.add(parent)
                    queue.append(parent)
        return seen

    print("\n═ do the ratchet's graph numbers predict seconds? ═\n")
    print("  ⚠ first-party LIB units only, joined to the frozen baseline's per-crate")
    print("  table. `backfilled` rows are dropped: their `lines` describe the tree at")
    print("  ingest, not the tree that was built.\n")
    predictors = [
        ("lines", lambda e: e["lines"], "largest_unit_lines is the max of this"),
        ("edit_cost_lines", lambda e: e["edit_cost_lines"], "worst/watched_edit_cost_lines"),
        ("edit_cost_crates", lambda e: e["edit_cost_crates"], "blast radius, in crates"),
        ("depth", lambda e: e["depth"], "critical_path_crates is the max of this"),
    ]
    for (config, phase, _), group in runs.items():
        points = [
            (table[r["unit"]], r)
            for r in group
            if r.get("first_party") and not r.get("target") and not r.get("backfilled")
            and r["unit"] in table and r["seconds"] > 0
        ]
        if len(points) < 5:
            continue
        seconds = [r["seconds"] for _, r in points]
        print(f"  {config} / {phase}   n={len(points)} crates, "
              f"{sum(seconds):.1f} unit-seconds")
        for name, get, note in predictors:
            xs = [float(get(entry)) for entry, _ in points]
            print(f"     seconds vs {name:<18} r={_pearson(xs, seconds):+.2f}  "
                  f"rho={_spearman(xs, seconds):+.2f}   ({note})")
        ms_per_line = sorted(
            (r["seconds"] * 1000.0 / entry["lines"], r["unit"])
            for entry, r in points if entry["lines"]
        )
        spread = ms_per_line[-1][0] / ms_per_line[0][0] if ms_per_line[0][0] else float("inf")
        print(f"     ms/line spans {ms_per_line[0][0]:.2f} ({ms_per_line[0][1]}) to "
              f"{ms_per_line[-1][0]:.2f} ({ms_per_line[-1][1]}) — {spread:.0f}x\n")

    print("═ blast radius: the ratchet ranks it in LINES, the build pays it in SECONDS ═\n")
    print("  `edit_cost_lines` is guarded on the premise that it orders crates by what")
    print("  an edit to them costs. The same closure, weighted by MEASURED seconds, is")
    print("  the thing it claims to stand in for. If the two orderings disagree the")
    print("  guard is guarding the wrong crate.\n")
    for (config, phase, _), group in runs.items():
        measured = {
            r["unit"]: r["seconds"]
            for r in group
            if r.get("first_party") and not r.get("target") and not r.get("backfilled")
            and r["unit"] in table
        }
        if len(measured) < 5:
            continue
        cost = {
            name: (
                table[name]["edit_cost_lines"],
                sum(measured.get(x, 0.0) for x in closure(name)),
            )
            for name in measured
        }
        xs = [v[0] for v in cost.values()]
        ys = [v[1] for v in cost.values()]
        print(f"  {config} / {phase}   n={len(cost)}")
        print(f"     edit_cost_lines vs edit_cost_seconds   r={_pearson(xs, ys):+.2f}  "
              f"rho={_spearman(xs, ys):+.2f}")
        by_lines = sorted(cost, key=lambda n: -cost[n][0])[:8]
        by_seconds = sorted(cost, key=lambda n: -cost[n][1])[:8]
        print(f"     {'ranked by edit_cost_LINES':<44} {'ranked by edit_cost_SECONDS':<44}")
        for left, right in zip(by_lines, by_seconds):
            print(f"     {left:<34}{cost[left][0]:>9,}  {right:<34}{cost[right][1]:>8.1f}s")

        # `critical_path_crates` counts hops. What a build waits on is the
        # longest chain in SECONDS, and the two need not be the same chain.
        chain: dict[str, tuple[float, list[str]]] = {}

        def longest(name: str) -> tuple[float, list[str]]:
            if name in chain:
                return chain[name]
            chain[name] = (0.0, [])  # cycle guard; the graph is acyclic
            best = max(
                (longest(parent) for parent in dependents.get(name, ())),
                key=lambda item: item[0],
                default=(0.0, []),
            )
            chain[name] = (measured.get(name, 0.0) + best[0], [name, *best[1]])
            return chain[name]

        heaviest = max((longest(name) for name in measured), key=lambda item: item[0])
        print(f"\n     critical_path_crates guards {baseline['critical_path_crates']} hops.")
        print(f"     The heaviest chain by seconds is {len(heaviest[1])} crates and "
              f"{heaviest[0]:.1f}s of the {sum(measured.values()):.1f} unit-seconds:")
        print("       " + " -> ".join(heaviest[1]))
        print()

    print("═ pipelining: the chain is NOT serial, and that changes what to optimise ═\n")
    print("  ⭐ rustc releases a dependent when the predecessor's METADATA lands, not")
    print("  when it finishes. So on a pipelined edge only the FRONTEND is serial and")
    print("  the predecessor's codegen overlaps everything downstream. That is why a")
    print("  chain whose durations sum past the wall clock is not a contradiction —")
    print("  and why `critical_path_crates`, which counts hops, prices the wrong half.\n")
    for (config, phase, _), group in runs.items():
        def key_of(row: dict) -> str:
            target = (row.get("target") or "").strip()
            return f"{row['unit']}{' ' + target.split()[0] if target else ''}"

        dag = {key_of(r): r for r in group if "unblocks_at_rmeta" in r}
        if not dag:
            # rows written before `unblocks_*` existed can still be analysed, because every row
            # names the report it came from.
            source = Path(group[0].get("build_source") or "")
            if not source.exists():
                continue
            reparsed = ratchet.ingest_timings(source, record=False,
                                              profile=group[0].get("build_profile") or "dev")
            byname = {key_of(r): r for r in group}
            dag = {
                key_of(r): {**byname[key_of(r)], **{
                    k: r[k] for k in ("unblocks_at_rmeta", "unblocks_at_completion")}}
                for r in reparsed if key_of(r) in byname
            }
            print(f"  ⚠ {config} / {phase}: DAG re-read from {source.name} — these rows "
                  f"predate the `unblocks_*` columns.")
            if not dag:
                continue

        def parts(row: dict) -> tuple[float, float]:
            """`(frontend, codegen)`. A unit with no sections emitted no rmeta —
            a proc-macro, build script, bin, test, or a lib declaring a cdylib —
            so all of its time is counted as the unserialisable half."""
            front = row.get("frontend_seconds")
            if front is None:
                return row["seconds"], 0.0
            return front, max(row["seconds"] - front, 0.0)

        preds: dict[str, list[tuple[str, bool]]] = {name: [] for name in dag}
        for name, row in dag.items():
            for successor in row.get("unblocks_at_rmeta") or []:
                if successor in preds:
                    preds[successor].append((name, True))
            for successor in row.get("unblocks_at_completion") or []:
                if successor in preds:
                    preds[successor].append((name, False))

        def makespan(front_weight: float, codegen_weight: float) -> float:
            memo: dict[str, float] = {}

            def finish(name: str) -> float:
                if name in memo:
                    return memo[name]
                memo[name] = 0.0  # the DAG is acyclic; this only guards a bad report
                front, codegen = parts(dag[name])
                start = 0.0
                for parent, pipelined in preds[name]:
                    pfront, pcodegen = parts(dag[parent])
                    start = max(
                        start,
                        finish(parent) - (pcodegen * codegen_weight if pipelined else 0.0),
                    )
                memo[name] = start + front * front_weight + codegen * codegen_weight
                return memo[name]

            return max(finish(name) for name in dag)

        def naive() -> tuple[float, int]:
            memo: dict[str, tuple[float, int]] = {}

            def chain(name: str) -> tuple[float, int]:
                if name in memo:
                    return memo[name]
                memo[name] = (0.0, 0)
                best = max((chain(p) for p, _ in preds[name]), default=(0.0, 0))
                memo[name] = (best[0] + dag[name]["seconds"], best[1] + 1)
                return memo[name]

            return max((chain(name) for name in dag), key=lambda item: item[0])

        total = sum(row["seconds"] for row in dag.values())
        front = sum(parts(row)[0] for row in dag.values())
        # edges are recorded as unit NAMES, and a cold build compiles some
        # packages twice (host vs target, or two feature sets). Those collapse
        # onto one key, so a cold DAG is approximate and says by how much. The
        # first-party phase has no collisions and is exact.
        dropped = len(group) - len(dag)
        if dropped:
            print(f"  ⚠ {dropped} of {len(group)} units share a name+target key with "
                  f"another and collapse; this DAG covers "
                  f"{total / sum(r['seconds'] for r in group):.0%} of the unit-seconds.")
        cores = group[0].get("build_cores") or os.cpu_count() or 1
        wall = group[0].get("build_wall_seconds")
        floor = makespan(1.0, 1.0)
        serial, hops = naive()
        print(f"  {config} / {phase}   {len(dag)} units, {total:.1f} unit-seconds "
              f"({front:.1f} frontend, {total - front:.1f} codegen)")
        print(f"     actual wall clock                          {wall if wall else '?':>8}s"
              f"   load mean {group[0].get('build_load_mean')} on {cores} cores")
        print(f"     perfect packing on {cores} cores                 {total / cores:8.1f}s"
              f"   (work / cores; cannot go below this)")
        print(f"     dependency floor, INFINITE cores           {floor:8.1f}s"
              f"   (cannot go below this either)")
        print(f"     naive serial chain, {hops:>2} units             {serial:8.1f}s"
              f"   ⚠ what counting hops models — and it exceeds the wall clock")
        print(f"     ... floor if CODEGEN were free             {makespan(1.0, 0.0):8.1f}s")
        print(f"     ... floor if the FRONTEND were free        {makespan(0.0, 1.0):8.1f}s")
        # **both bounds, not just the floor.** Halving one half also halves
        # the work, so the packing bound moves too and can become the binding
        # one. Quoting the floor alone overstates the win — which is the shape
        # of mistake this whole file exists to avoid.
        base = max(floor, total / cores)
        for name, wf, wc in (("codegen", 1.0, 0.5), ("the frontend", 0.5, 1.0)):
            work = sum(
                parts(r)[0] * wf + parts(r)[1] * wc for r in dag.values()
            )
            achievable = max(makespan(wf, wc), work / cores)
            print(f"     halving {name:<13} would save        {base - achievable:8.1f}s"
                  f"   (floor {makespan(wf, wc):.1f}s vs packing {work / cores:.1f}s)")
        blind = [n for n, r in dag.items() if r.get("frontend_seconds") is None]
        if blind:
            print(f"     ⚠ {len(blind)} unit(s) emit no metadata, so they pipeline nothing and "
                  f"have no frontend/codegen split:\n        "
                  + ", ".join(sorted(blind)[:6]) + (" …" if len(blind) > 6 else ""))
        print()

    print("═ codegen-bound or frontend-bound, and who costs more than their size ═\n")
    for (config, phase, _), group in runs.items():
        points = [
            r for r in group
            if r.get("first_party") and not r.get("target") and not r.get("backfilled")
            and r["unit"] in table and (r.get("codegen_seconds") or r.get("frontend_seconds"))
        ]
        if len(points) < 5:
            continue
        total_seconds = sum(r["seconds"] for r in points)
        total_lines = sum(table[r["unit"]]["lines"] for r in points) or 1
        rate = total_seconds * 1000.0 / total_lines
        print(f"  {config} / {phase}   the run's own average is {rate:.2f} ms/line\n")
        print(f"     {'crate':<44} {'sec':>7} {'lines':>8} {'ms/ln':>7} "
              f"{'cg%':>5} {'opt':>4} {'excess vs average':>18}")
        scored = sorted(
            points,
            key=lambda r: r["seconds"] - table[r["unit"]]["lines"] * rate / 1000.0,
            reverse=True,
        )
        for row in scored[:12]:
            entry = table[row["unit"]]
            predicted = entry["lines"] * rate / 1000.0
            codegen = row.get("codegen_seconds") or 0.0
            share = codegen / row["seconds"] if row["seconds"] else 0.0
            print(f"     {row['unit']:<44} {row['seconds']:>7.1f} {entry['lines']:>8,} "
                  f"{row['seconds'] * 1000 / max(entry['lines'], 1):>7.2f} "
                  f"{share:>5.0%} {str(row.get('opt_level')):>4} "
                  f"{row['seconds'] - predicted:>+13.1f}s")
        print()
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--analyze", action="store_true",
                        help="read dev/ambition_dev_measurements/compile_units.jsonl and print what it says; builds nothing")
    parser.add_argument("--config", action="append", choices=sorted(CONFIGS),
                        help="repeatable; default: dev")
    parser.add_argument("--phase", action="append", choices=["cold", "first-party"],
                        help="repeatable; default: both, cold first")
    parser.add_argument("--target-root", default=str(DEFAULT_TARGET_ROOT),
                        help="parent of the per-configuration CARGO_TARGET_DIRs")
    parser.add_argument("--no-record", action="store_true",
                        help="measure without appending to dev/ambition_dev_measurements/compile_units.jsonl")
    parser.add_argument("--list", action="store_true", help="print the configurations and exit")
    args = parser.parse_args(argv)

    if args.analyze:
        return analyze()

    if args.list:
        for config in CONFIGS.values():
            print(f"  {config.name:<16} profile={config.profile:<8} "
                  f"incremental={str(config.incremental):<5}  {config.why}")
        return 0

    if shutil.which("cargo") is None:
        raise SystemExit("⛔ cargo not on PATH; this measures cargo and cannot proxy it")

    # checked HERE, before a cold build that costs 9 minutes. `ingest_timings`
    # checks again at the append — this one exists so the refusal arrives before
    # the wait rather than after it.
    if not args.no_record:
        measurement_paths.require_writable(ratchet.UNIT_LEDGER)

    configs = [CONFIGS[name] for name in (args.config or ["dev"])]
    phases = args.phase or ["cold", "first-party"]
    target_root = Path(args.target_root)

    started = time.monotonic()
    rows: list[dict] = []
    for config in configs:
        rows.extend(
            collect(config, target_root=target_root, phases=phases, record=not args.no_record)
        )

    print(f"\n{len(rows)} unit row(s) in {time.monotonic() - started:.0f}s across "
          f"{len(configs)} configuration(s)")
    if not args.no_record:
        print(f"  file://{ratchet.UNIT_LEDGER}\n  file://{ratchet.UNIT_LEDGER.parent}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
