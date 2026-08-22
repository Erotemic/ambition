#!/usr/bin/env python3
"""Measure what an edit COSTS to rebuild, and record it so the number moves.

⛔ **This exists because "compile time is too long" is not a measurement.**
Jon, 2026-08-07: *"Currently compile time is too long even after a warm
recompile… We are wasting so much time on compiles and links, especially when
agents run tests."* ADR 0013 prescribes `cargo build --timings` "quarterly",
which is a thing nobody does and which answers a different question anyway — it
profiles ONE build rather than tracking the edit→feedback loop over time.

## What it measures

An EDIT, not a build. Each scenario appends a tiny function to a real source
file, runs a real cargo command, and reverts. That is the loop a person or an
agent actually pays for, and it is the only number that falls when the monolith
is carved.

⚠ **the scenarios differ by more than their name suggests**, and the differences
are the finding rather than noise:

* `check` is the AGENTS.md gate, and it is frontend-only — no codegen, no link.
* `test-build` is what an agent pays before a single test runs. Measured
  2026-08-07 it was **12x** the check for the same edit, because `opt-level = 1`
  on workspace crates makes codegen the dominant cost.
* `relink` isolates the link step, which turned out NOT to be the problem
  (9.3s to relink a 769 MB binary with mold) despite being the obvious suspect.

## Honesty rules this obeys

⛔ **Never `git checkout` to revert.** A probe that reverts with git deletes
whatever uncommitted work was in the file. This writes the original bytes back
from memory, and refuses to run at all if the target file is already dirty.

⚠ **A cold cache measures the wrong thing.** Every scenario warms first with the
identical command, so what is timed is the marginal cost of the edit rather than
whatever the tree happened to owe. Without this the same scenario measured 4m52s
and then 9.3s, and only the second was the answer.

⛔ **ONE cargo build at a time, per target dir.** Any flag that changes a
fingerprint — `CARGO_INCREMENTAL`, `CARGO_PROFILE_*`, a feature set — makes a
second concurrent build rebuild everything the first one just built, and then
the first rebuilds it back. Measured 2026-08-07: a warm no-op that should have
been under a second reported **222s** because an incremental-on build was
running beside an incremental-off one in the same target dir. The reading looks
like a slow machine, not like a mistake, which is what makes it dangerous.
Use a separate `CARGO_TARGET_DIR` when comparing two configurations, or run
them strictly in sequence (this repo already knows the worktree version of this
rule: never share a target dir across worktrees).

Usage:
    python scripts/compile_cost.py                 # every scenario, append to dev/
    python scripts/compile_cost.py --scenario check
    python scripts/compile_cost.py --no-record     # measure without recording
    python scripts/compile_cost.py --env CARGO_INCREMENTAL=1 --label incremental
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
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


@dataclass(frozen=True)
class Scenario:
    name: str
    edit: str  # repo-relative file to perturb
    command: list[str]
    why: str
    env: dict[str, str] = field(default_factory=dict)


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


def run_timed(command: list[str], env: dict[str, str]) -> float:
    merged = {**os.environ, **env}
    start = time.monotonic()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, env=merged)
    elapsed = time.monotonic() - start
    if proc.returncode != 0:
        tail = "\n".join((proc.stderr or "").strip().splitlines()[-12:])
        raise SystemExit(f"⛔ `{' '.join(command)}` failed, so its timing is meaningless:\n{tail}")
    return elapsed


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
        warm = run_timed(scenario.command, merged_env)

        if verbose:
            print(f"  editing {scenario.edit} and rebuilding …", flush=True)
        target.write_bytes(original + MARKER.format(salt=17).encode("utf-8"))
        edited = run_timed(scenario.command, merged_env)
    finally:
        target.write_bytes(original)

    # Leave the tree in the state the caller handed us: the revert above changes
    # content back, but the rebuild artifacts now describe the probe. One more
    # build makes the next `cargo` invocation honest instead of surprising.
    if verbose:
        print("  restoring build state …", flush=True)
    settle = run_timed(scenario.command, merged_env)

    return {
        "scenario": scenario.name,
        "why": scenario.why,
        "edited_file": scenario.edit,
        "command": " ".join(scenario.command),
        "warm_noop_seconds": round(warm, 2),
        "after_edit_seconds": round(edited, 2),
        "restore_seconds": round(settle, 2),
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


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--scenario", action="append", choices=sorted(BY_NAME), help="default: all")
    ap.add_argument("--label", default="", help="free-text tag for the run (e.g. 'incremental')")
    ap.add_argument("--env", action="append", default=[], metavar="K=V",
                    help="extra environment for the cargo invocations")
    ap.add_argument("--no-record", action="store_true", help="print only; do not append to the ledger")
    args = ap.parse_args(argv)

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
        print(
            f"  warm no-op {row['warm_noop_seconds']:>7.2f}s"
            f"   AFTER EDIT {row['after_edit_seconds']:>7.2f}s"
        )

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
