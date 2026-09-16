#!/usr/bin/env python3
"""Measure the marginal rebuild cost of a source edit.

Each scenario warms its Cargo command, makes a temporary real edit to a clean
source file, times the rebuild, then restores the original bytes from memory.
The script refuses to probe a target file that is already dirty and never uses
Git to revert user work. Builds sharing a target directory must run serially.

Results can be appended to the compile telemetry ledger or printed without
recording.

Usage::

    python scripts/compile_cost.py
    python scripts/compile_cost.py --scenario check
    python scripts/compile_cost.py --no-record
    python scripts/compile_cost.py --env CARGO_INCREMENTAL=1 --label incremental"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import measurement_paths  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

LEDGER = measurement_paths.SCENARIO_LEDGER

# The marker is appended and removed; it is a plain private fn so it compiles in any Rust module and
# triggers a real recompile rather than an mtime-only one.
MARKER = "\n#[allow(dead_code)]\nfn _compile_cost_probe(x: u32) -> u32 {{ x.wrapping_add({salt}) }}\n"

#: What `MARKER` does to a file, in the vocabulary of
#: `docs/planning/engine/extension-iteration-evidence.md` M0 (`edit_class`).
#:
#: ⛔ EVERY scenario below uses the SAME marker, so every row this script has
#: ever written carries this one class. The ledger therefore cannot answer "does
#: a signature change cost more than a body change" — not because the column is
#: missing, but because the corpus has one value in it. Recording the class is
#: what makes that limit visible instead of assumed.
MARKER_EDIT_CLASS = "append-private-fn"


@dataclass(frozen=True)
class Scenario:
    name: str
    edit: str  # repo-relative file to perturb
    command: list[str]
    why: str
    env: dict[str, str] = field(default_factory=dict)
    #: Overridable so a future scenario that edits differently must SAY so.
    edit_class: str = MARKER_EDIT_CLASS


SCENARIOS: list[Scenario] = [
    Scenario(
        name="check",
        edit="crates/ambition_platformer2d_actor_monolith/src/lib.rs",
        command=["cargo", "check", "-p", "ambition_app"],
        why="the AGENTS.md gate, after an edit to the crate most work touches",
    ),
    Scenario(
        name="check-leaf",
        edit="crates/ambition_platformer2d_core/src/lib.rs",
        command=["cargo", "check", "-p", "ambition_app"],
        why="the same gate from the BOTTOM of the graph — the worst-case fan-out",
    ),
    Scenario(
        name="test-build",
        edit="crates/ambition_platformer2d_actor_monolith/src/lib.rs",
        command=["cargo", "test", "-p", "ambition_app", "--test", "app_it", "--no-run"],
        why="what an agent pays before one test runs; codegen, not frontend",
    ),
    Scenario(
        name="relink",
        edit="game/ambition_app/tests/app_it.rs",
        command=["cargo", "test", "-p", "ambition_app", "--test", "app_it", "--no-run"],
        why="link + one crate only; isolates the link step from the graph",
    ),
]

BY_NAME = {scenario.name: scenario for scenario in SCENARIOS}


def git(*args: str) -> str:
    proc = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    )
    return proc.stdout.strip()


def run_timed(command: list[str], env: dict[str, str]) -> tuple[float, int | None]:
    """Wall seconds, and the peak RSS of the LARGEST SINGLE PROCESS in the tree.

    ⚠⚠ `ru_maxrss` is a HIGH-WATER MARK, NOT A TOTAL. `os.wait4` reports the
    maximum over the reaped child and every descendant it waited for, so this is
    the biggest single `rustc`/linker — never the sum of the ones resident at
    once. A 30-job build whose largest unit held 1.1 GB reports 1.1 GB while the
    host was holding far more. It answers *"does one unit still fit"*, which is
    the question that decides whether a link OOMs; it does NOT answer *"what did
    this build cost the machine"*, and no column here does.

    ⛔ Do not "improve" this by polling RSS on a timer. A sampler misses a peak
    between samples and reports a number that looks measured — this session lost
    hours to exactly that class of instrument. `wait4` is the kernel's own
    accounting for this child and cannot miss.
    """
    merged = {**os.environ, **env}
    with tempfile.TemporaryFile() as sink:
        start = time.monotonic()
        # `Popen` for `cwd`, then `os.wait4` for the rusage that `Popen.wait()`
        # discards. Reaping behind Popen's back is safe ONLY because
        # `returncode` is assigned below: a Popen that still believes its child
        # is running will wait on that pid again at GC time, and pids get reused.
        proc = subprocess.Popen(command, cwd=ROOT, stdout=sink, stderr=sink, env=merged)
        try:
            _, status, usage = os.wait4(proc.pid, 0)
        except BaseException:
            # Otherwise the child OUTLIVES us: an orphaned `cargo` keeps writing
            # into the shared target dir while the caller believes the probe
            # stopped, and the next measurement times a build racing a ghost.
            proc.kill()
            _, killed_status, _ = os.wait4(proc.pid, 0)
            # Told for the SAME reason as the success path below: a Popen that
            # still believes its child is running waits on that pid at GC time.
            proc.returncode = os.waitstatus_to_exitcode(killed_status)
            raise
        elapsed = time.monotonic() - start
        proc.returncode = os.waitstatus_to_exitcode(status)
        if proc.returncode != 0:
            sink.seek(0)
            captured = sink.read().decode("utf-8", errors="replace")
            tail = "\n".join(captured.strip().splitlines()[-12:])
            raise SystemExit(
                f"⛔ `{' '.join(command)}` failed, so its timing is meaningless:\n{tail}"
            )

    # `ru_maxrss` is KiB on Linux and BYTES on macOS. Guessing wrong silently
    # publishes a number off by 1024, which is worse than publishing none.
    peak = usage.ru_maxrss * 1024 if sys.platform.startswith("linux") else None
    return elapsed, peak


def job_limit(command: list[str], env: dict[str, str]) -> int | None:
    """The `-j` cap actually in force, or `None` for "uncapped; `machine_cores` applies".

    ⛔ A capped run's wall clock is NOT the machine's cost, and a row that omits
    the cap is a number nobody can compare to any other row. The cargo flag beats
    the environment because that is cargo's own precedence.
    """
    for index, token in enumerate(command):
        if token in ("-j", "--jobs") and index + 1 < len(command):
            token = command[index + 1]
        elif token.startswith("--jobs="):
            token = token.split("=", 1)[1]
        else:
            continue
        return int(token) if token.lstrip("-").isdigit() else None

    raw = {**os.environ, **env}.get("CARGO_BUILD_JOBS", "")
    return int(raw) if raw.isdigit() else None


def measure(scenario: Scenario, env: dict[str, str], *, verbose: bool = True) -> dict:
    target = ROOT / scenario.edit
    if not target.exists():
        raise SystemExit(f"⛔ {scenario.edit} does not exist; fix the scenario before trusting it")

    # refuse on a dirty target. Reverting would otherwise mean choosing
    # between restoring the probe's baseline and keeping somebody's live edit.
    if git("status", "--porcelain", "--", scenario.edit):
        raise SystemExit(
            f"⛔ {scenario.edit} has uncommitted changes. This script rewrites and "
            "restores that file; refusing rather than risking your work."
        )

    original = target.read_bytes()
    merged_env = {**scenario.env, **env}
    try:
        if verbose:
            print(f"  warming ({' '.join(scenario.command)}) …", flush=True)
        warm, warm_peak = run_timed(scenario.command, merged_env)

        if verbose:
            print(f"  editing {scenario.edit} and rebuilding …", flush=True)
        target.write_bytes(original + MARKER.format(salt=17).encode("utf-8"))
        edited, edited_peak = run_timed(scenario.command, merged_env)
    finally:
        target.write_bytes(original)

    # Leave the tree in the state the caller handed us: the revert above changes
    # content back, but the rebuild artifacts now describe the probe. One more
    # build makes the next `cargo` invocation honest instead of surprising.
    if verbose:
        print("  restoring build state …", flush=True)
    settle, settle_peak = run_timed(scenario.command, merged_env)

    return {
        "scenario": scenario.name,
        "why": scenario.why,
        "edited_file": scenario.edit,
        "command": " ".join(scenario.command),
        "edit_class": scenario.edit_class,
        "warm_noop_seconds": round(warm, 2),
        "after_edit_seconds": round(edited, 2),
        "restore_seconds": round(settle, 2),
        # Peak RSS of the largest single process in each build — see `run_timed`
        # for why this is a high-water mark and not a total. `null` off Linux.
        "warm_noop_peak_rss_bytes": warm_peak,
        "after_edit_peak_rss_bytes": edited_peak,
        "restore_peak_rss_bytes": settle_peak,
        # The `-j` cap this row was measured under; `null` means uncapped, in
        # which case `machine_cores` is the parallelism.
        "job_limit": job_limit(scenario.command, merged_env),
        # ⛔ NULL BECAUSE NOTHING COLLECTS IT, NOT BECAUSE IT IS ZERO.
        # `docs/planning/engine/extension-iteration-evidence.md` M0 names this
        # field, and counting it honestly needs a shim on the linker path — which
        # would perturb the very timings in this row. The column lands now so a
        # future collector back-fills nothing: rows written before it exist say
        # `null` and mean "unknown", which is true.
        "host_link_invocations": None,
    }


def build_config() -> dict:
    """The dimensions a measurement is only comparable WITHIN — as columns.

    ⛔ **this used to be a stringly-typed side effect of how the run was
    invoked**, and the four schema-0 rows in the ledger disagree with each other
    because of it: `machine_cargo_incremental` reads `"1"` in two of them and
    `"(config default)"` in two, and `"(config default)"` meant OFF before
    `.cargo/config.toml` turned incremental on and ON after. A dimension encoded
    that way cannot be regressed against. The normalisation for those four rows
    is written out in `dev/compile_telemetry_schema.md`; they are not rewritten,
    because this ledger is append-only.

    ⚠ `opt_level` is the WORKSPACE default. It is not uniform — `runtime`,
    `render` and `app` are pinned to 0 in `Cargo.toml` — and per-crate opt-levels
    belong on the per-unit rows in `dev/ambition_dev_measurements/compile_units.jsonl`, where there is a
    crate to attach them to. A scenario row covers a whole command.
    """
    import tomllib

    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    dev = manifest.get("profile", {}).get("dev", {})

    env_flag = os.environ.get("CARGO_INCREMENTAL")
    if env_flag is not None:
        incremental = env_flag not in ("0", "")
    else:
        config = ROOT / ".cargo" / "config.toml"
        text = config.read_text(encoding="utf-8") if config.exists() else ""
        incremental = bool(re.search(r"^\s*incremental\s*=\s*true", text, re.M))
    return {
        "incremental": incremental,
        "opt_level": str(dev.get("opt-level", 0)),
    }


def machine_facts() -> dict:
    linker = "unknown"
    config = ROOT / ".cargo" / "config.toml"
    if config.exists():
        text = config.read_text(encoding="utf-8")
        if "mold" in text:
            linker = "mold"
        elif "lld" in text:
            linker = "lld"
    return {
        "cores": os.cpu_count(),
        "linker": linker,
        # The env var beats the config file, so record what was in force rather
        # than what the config says. A run that forgets this is not comparable.
        "cargo_incremental": os.environ.get("CARGO_INCREMENTAL", "(config default)"),
        "platform": platform.platform(),
        "cargo": subprocess.run(
            ["cargo", "--version"], capture_output=True, text=True, check=False
        ).stdout.strip(),
    }


def _abort_on_signal(signum: int, _frame) -> None:
    """Turn a signal into an exception so `measure`'s `finally` actually runs.

    ⛔ THE BUG THIS CLOSES: default `SIGTERM` does not unwind, so `finally` never
    fires and the restore never happens. This script's whole contract is that it
    perturbs a clean source file and puts the original bytes back — and under
    `timeout(1)`, which is how any bounded or unattended run must invoke it, that
    contract was silently void. The residue is a `_compile_cost_probe` function
    left inside a real crate, in a file nobody edited on purpose, which then
    trips this script's OWN dirty-target refusal on the next run.
    """
    raise SystemExit(
        f"⛔ signal {signum} received mid-measurement; "
        "restoring the probed file before exiting"
    )


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--scenario", action="append", choices=sorted(BY_NAME), help="default: all")
    ap.add_argument("--label", default="", help="free-text tag for the run (e.g. 'incremental')")
    ap.add_argument("--env", action="append", default=[], metavar="K=V",
                    help="extra environment for the cargo invocations")
    ap.add_argument("--no-record", action="store_true", help="print only; do not append to the ledger")
    args = ap.parse_args(argv)

    # Installed BEFORE any file is touched, so there is no window in which a
    # signal can strand the marker.
    for caught in (signal.SIGTERM, signal.SIGINT):
        signal.signal(caught, _abort_on_signal)

    if shutil.which("cargo") is None:
        raise SystemExit("⛔ cargo not on PATH; this measures cargo and cannot proxy it")

    if not args.no_record:
        measurement_paths.require_writable(LEDGER)

    env: dict[str, str] = {}
    for pair in args.env:
        if "=" not in pair:
            raise SystemExit(f"--env expects K=V, got {pair!r}")
        key, value = pair.split("=", 1)
        env[key] = value

    chosen = [BY_NAME[name] for name in (args.scenario or sorted(BY_NAME))]
    facts = machine_facts()
    config = build_config()
    if "CARGO_INCREMENTAL" in env:
        config["incremental"] = env["CARGO_INCREMENTAL"] not in ("0", "")
    run_id = uuid.uuid4().hex[:12]
    if env.get("CARGO_INCREMENTAL"):
        facts["cargo_incremental"] = env["CARGO_INCREMENTAL"]

    rows = []
    for scenario in chosen:
        print(f"▶ {scenario.name}: {scenario.why}")
        row = measure(scenario, env)
        row.update(
            {
                # The shared envelope — `dev/compile_telemetry_schema.md` §1.
                # Copied by hand rather than imported: this script is the one
                # that must keep working when everything else is mid-edit.
                "schema": 1,
                "kind": "scenario",
                "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
                "commit": git("rev-parse", "--short=12", "HEAD") or "unknown",
                "dirty": bool(git("status", "--porcelain")),
                "run_id": run_id,
                "label": args.label,
                # `test` when the command builds a test target, else `dev`. The
                # cargo timing report calls the same thing `Profile:`.
                "profile": "test" if "test" in scenario.command else "dev",
                **config,
                "env": env,
                **{f"machine_{k}": v for k, v in facts.items()},
            }
        )
        rows.append(row)
        peak = row["after_edit_peak_rss_bytes"]
        largest = f"   largest proc {peak / 1e9:>5.2f} GB" if peak is not None else ""
        print(
            f"  warm no-op {row['warm_noop_seconds']:>7.2f}s"
            f"   AFTER EDIT {row['after_edit_seconds']:>7.2f}s"
            + largest
        )
        if args.no_record:
            # ⛔ A DRY RUN THAT HIDES THE ROW CANNOT VERIFY THE ROW. Without
            # this, the first sight of a newly added column is inside an
            # append-only file, where a mistake cannot be taken back.
            print(json.dumps(row, indent=2, sort_keys=True))

    if not args.no_record:
        LEDGER.parent.mkdir(parents=True, exist_ok=True)
        with LEDGER.open("a", encoding="utf-8") as handle:
            for row in rows:
                handle.write(json.dumps(row, sort_keys=True) + "\n")
        print(f"\nappended {len(rows)} row(s) to {LEDGER.relative_to(ROOT)}")

    print("\n⚠ compare rows only within one machine and linker — "
          f"this run: {facts['cores']} cores, {facts['linker']}, "
          f"incremental={facts['cargo_incremental']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
