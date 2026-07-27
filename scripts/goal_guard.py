#!/usr/bin/env python3
"""Deterministic goal guard for long agent runs.

`/goal` installs a Stop hook whose condition is judged by a model reading the
conversation. On 2026-07-25 that judge released a 24-hour run after 84 minutes:
the agent wrote a completion-shaped status report ("A1 done, A2 done, three
findings...") and the judge, reading prose, called the goal met and auto-cleared
itself. The session then idled for nine hours with nobody to restart it.

The lesson is not "the hook was too weak". It is that the ARBITER was the wrong
kind. A judge that reads prose can be persuaded by prose, and a model that is
tired of an item is exactly the thing most motivated to write persuasive prose.

So this guard is a `command` hook. It reads the repository, never the
conversation. Its answer is an exit status from a list of shell checks the
human wrote in advance, and no amount of eloquence at 2am moves it.

## What it is NOT

It is not unfoolable. The checks it runs are commands, and the agent can edit
the files those commands read. The honest claim is narrower: it converts
"convince a judge" into "falsify a specific artifact", which is a much louder
thing to do and leaves a diff. Write checks that name a TEST, not a doc marker,
and the bar goes up again.

## Wiring (`.claude/settings.json`)

    "hooks": {
      "Stop": [{"hooks": [{"type": "command", "timeout": 900,
                "command": "python3 scripts/goal_guard.py"}]}],
      "SessionStart": [{"hooks": [{"type": "command", "timeout": 15,
                "command": "python3 scripts/goal_guard.py --inject"}]}]
    }

Only the Stop hook runs the checks, and only the Stop hook gets a long timeout.
`--inject` reads `.goal/state.json` and returns immediately: SessionStart blocks
session startup, so a hook that shells out to `cargo check` there can lock you
out of the repository entirely. The short timeout is a second line of defence,
not the fix.

Both are INERT unless `.goal/active.json` exists, so committing that config does
not change ordinary interactive sessions. Arming is an explicit act:

    python3 scripts/goal_guard.py --arm .goal/my-run.json
    python3 scripts/goal_guard.py --status
    python3 scripts/goal_guard.py --clear

## The goal file

    {
      "goal": "complete the 24h queue",
      "deadline_utc": "2026-07-26T04:51:00Z",
      "max_stalled_blocks": 3,
      "checks": [
        {"name": "queue has no open items",
         "cmd": "! grep -q '▢' docs/planning/queue-24h-2026-07-25.md"},
        {"name": "the app still compiles",
         "cmd": "cargo check -p ambition_app -q", "timeout": 900}
      ]
    }

`deadline_utc` is what makes "24 hours" mean 24 hours in both directions: the
guard holds the run open until then, and releases it afterwards rather than
wedging the session forever.

## Why it does not follow the documented `stop_hook_active` advice

Claude Code's hook docs say a Stop hook should "check stop_hook_active in the
input and return success while it's true". A hook that obeys that blocks exactly
ONCE and then lets the agent go — which is the same hole in a different shape.

The runtime already has the loop guard that advice exists to provide:
`stopHookBlockingCount` is capped (default 8, `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`
raises it) and — the part that matters — it RESETS to zero on any turn that runs
tools. So consecutive blocks only accumulate while the agent is doing nothing,
which is precisely when a run should be released.

This guard keeps its own, stricter version of that: it releases if it has
blocked `max_stalled_blocks` times with no new commit on HEAD. Not "no new
turn" — no new COMMIT, because in this repo finished work is committed work.
An agent thrashing without committing is not making progress, and the guard
says so out loud instead of pinning the session against the wall.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

DEFAULT_CHECK_TIMEOUT = 120
DEFAULT_MAX_STALLED_BLOCKS = 3


def repo_root() -> Path:
    """The git root, so the guard works from any cwd a hook happens to have."""
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            timeout=15,
        )
        if out.returncode == 0 and out.stdout.strip():
            return Path(out.stdout.strip())
    except (OSError, subprocess.SubprocessError):
        pass
    return Path.cwd()


def goal_dir(root: Path) -> Path:
    return root / ".goal"


def active_path(root: Path) -> Path:
    return goal_dir(root) / "active.json"


def state_path(root: Path) -> Path:
    return goal_dir(root) / "state.json"


def load_json(path: Path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return None


def head_sha(root: Path) -> str:
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=15,
        )
        return out.stdout.strip() if out.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError):
        return ""


def now_utc() -> _dt.datetime:
    return _dt.datetime.now(_dt.timezone.utc)


def parse_deadline(raw) -> _dt.datetime | None:
    if not raw:
        return None
    try:
        text = str(raw).replace("Z", "+00:00")
        parsed = _dt.datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=_dt.timezone.utc)
    return parsed


# ── Running the checks ────────────────────────────────────────────────────────


class CheckResult:
    def __init__(self, name: str, cmd: str, ok: bool, detail: str):
        self.name = name
        self.cmd = cmd
        self.ok = ok
        self.detail = detail


def run_checks(goal: dict, root: Path) -> list[CheckResult]:
    """Every check, always — a partial answer would hide the second open item."""
    results: list[CheckResult] = []
    for raw in goal.get("checks", []):
        name = str(raw.get("name") or raw.get("cmd") or "unnamed check")
        cmd = raw.get("cmd")
        if not cmd:
            results.append(CheckResult(name, "", False, "check has no `cmd`"))
            continue
        timeout = raw.get("timeout", DEFAULT_CHECK_TIMEOUT)
        try:
            proc = subprocess.run(
                ["bash", "-o", "pipefail", "-c", cmd],
                cwd=str(root),
                capture_output=True,
                text=True,
                timeout=timeout,
            )
        except subprocess.TimeoutExpired:
            # A timeout is NOT a pass. The whole point of this file is that
            # "we could not tell" never resolves to "done".
            results.append(
                CheckResult(name, cmd, False, f"timed out after {timeout}s")
            )
            continue
        except OSError as exc:
            results.append(CheckResult(name, cmd, False, f"could not run: {exc}"))
            continue
        tail = (proc.stderr.strip() or proc.stdout.strip() or "").splitlines()
        detail = "" if proc.returncode == 0 else " / ".join(tail[-3:])[:400]
        results.append(CheckResult(name, cmd, proc.returncode == 0, detail))
    return results


def clear_goal(root: Path, reason: str) -> None:
    """Archive rather than delete: what a finished run was asked to do is
    evidence, and the next run's post-mortem wants it."""
    active = active_path(root)
    if not active.exists():
        return
    stamp = now_utc().strftime("%Y%m%dT%H%M%SZ")
    archive = goal_dir(root) / f"done-{stamp}.json"
    payload = load_json(active) or {}
    payload["_cleared_at"] = now_utc().isoformat()
    payload["_cleared_because"] = reason
    try:
        archive.write_text(json.dumps(payload, indent=2) + "\n")
        active.unlink()
        state_path(root).unlink(missing_ok=True)
    except OSError:
        pass


# ── Hook output shapes ────────────────────────────────────────────────────────


def emit(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def open_items_text(goal: dict, results: list[CheckResult]) -> str:
    failed = [r for r in results if not r.ok]
    lines = [f"GOAL STILL OPEN: {goal.get('goal', '(unnamed goal)')}", ""]
    lines.append(f"{len(failed)} of {len(results)} checks are still failing:")
    for r in failed:
        detail = f" — {r.detail}" if r.detail else ""
        lines.append(f"  ▢ {r.name}{detail}")
    passed = [r for r in results if r.ok]
    if passed:
        lines.append("")
        lines.append("Already satisfied: " + ", ".join(r.name for r in passed))
    return "\n".join(lines)


def block_reason(goal: dict, results: list[CheckResult], deadline) -> str:
    parts = [open_items_text(goal, results)]
    if deadline:
        hours = max(0.0, (deadline - now_utc()).total_seconds() / 3600.0)
        # Only worth saying when it is close enough to shape what to do next; a
        # far deadline printed to one decimal place is just noise in a message
        # the agent reads at the end of every single turn.
        if hours <= 168.0:
            parts.append(f"\n{hours:.1f}h remain before this goal releases on its own.")
    parts.append(
        "\nThis is a command hook reading the repository, not a judge reading "
        "your summary. Writing a status report does not close an item and will "
        "not end this turn. Resume the first open item above now — no recap, no "
        "hand-off, pick up mid-thought."
    )
    return "\n".join(parts)


# ── Modes ─────────────────────────────────────────────────────────────────────


def mode_stop(root: Path, hook_input: dict) -> int:
    goal = load_json(active_path(root))
    if not goal:
        return 0  # Not armed: ordinary sessions are untouched.

    deadline = parse_deadline(goal.get("deadline_utc"))
    if deadline and now_utc() >= deadline:
        clear_goal(root, "deadline passed")
        emit(
            {
                "systemMessage": (
                    f"Goal guard: deadline passed, releasing the run. "
                    f"Goal was: {goal.get('goal', '')}"
                )
            }
        )
        return 0

    results = run_checks(goal, root)
    if results and all(r.ok for r in results):
        clear_goal(root, "all checks passed")
        emit({"systemMessage": f"Goal guard: MET — {goal.get('goal', '')}"})
        return 0

    # Still open. Decide whether blocking is still useful, or whether this run
    # is simply stuck and a human should hear about it.
    state = load_json(state_path(root)) or {}
    sha = head_sha(root)
    stalled = int(state.get("stalled", 0))
    stalled = stalled + 1 if sha and sha == state.get("last_head") else 0
    max_stalled = int(goal.get("max_stalled_blocks", DEFAULT_MAX_STALLED_BLOCKS))

    state.update(
        {
            "last_head": sha,
            "stalled": stalled,
            "blocks": int(state.get("blocks", 0)) + 1,
            "last_block_at": now_utc().isoformat(),
            "last_open": [r.name for r in results if not r.ok],
        }
    )
    goal_dir(root).mkdir(parents=True, exist_ok=True)
    try:
        state_path(root).write_text(json.dumps(state, indent=2) + "\n")
    except OSError:
        pass

    if max_stalled > 0 and stalled >= max_stalled:
        emit(
            {
                "systemMessage": (
                    f"Goal guard: released after {stalled} blocks with no new "
                    f"commit — the run is stuck, not finished. Still open: "
                    + ", ".join(state["last_open"])
                )
            }
        )
        return 0

    emit({"decision": "block", "reason": block_reason(goal, results, deadline)})
    return 0


def mode_inject(root: Path, hook_input: dict) -> int:
    """Re-state the goal wherever context can be injected — from CACHE only.

    Compaction is the reason this exists. `PostCompact` cannot inject context
    (its schema has no `additionalContext`), so the goal has to re-enter through
    a channel that fires anyway — SessionStart on resume, and the Stop hook's own
    block reason after every single turn.

    It must NOT run the checks. SessionStart fires before the session is ready,
    and the caller waits: on 2026-07-27 a goal whose checks include a 900s
    `cargo check` wedged startup until the IDE gave up at 60s and the only way
    in was `mv .goal/active.json .goal/active.json.paused`. A guard that can
    lock you out of the repository it guards is worse than no guard.

    So this reads `.goal/state.json` — the open items the last Stop hook wrote —
    and says so. That is a cache, and it can be stale, but it is stale in the
    safe direction: it restates work as OPEN, and the Stop hook is still the
    authority that decides anything is finished.
    """
    goal = load_json(active_path(root))
    if not goal:
        return 0

    deadline = parse_deadline(goal.get("deadline_utc"))
    if deadline and now_utc() >= deadline:
        clear_goal(root, "deadline passed before this session started")
        emit({"systemMessage": f"Goal guard: deadline passed, releasing the run. Goal was: {goal.get('goal', '')}"})
        return 0

    state = load_json(state_path(root)) or {}
    last_open = state.get("last_open")
    if isinstance(last_open, list) and last_open:
        lines = [
            f"GOAL STILL OPEN: {goal.get('goal', '(unnamed goal)')}",
            "",
            f"Open as of the last Stop check ({state.get('last_block_at', 'unknown time')}):",
        ]
        lines += [f"  ▢ {name}" for name in last_open]
    else:
        lines = [
            f"GOAL ARMED: {goal.get('goal', '(unnamed goal)')}",
            "",
            "Checks that will be run at the end of every turn:",
        ]
        lines += [
            f"  ▢ {raw.get('name') or raw.get('cmd') or 'unnamed check'}"
            for raw in goal.get("checks", [])
        ]

    if deadline:
        hours = max(0.0, (deadline - now_utc()).total_seconds() / 3600.0)
        lines += ["", f"{hours:.1f}h remain before this goal releases on its own."]
    lines += [
        "",
        "This goal is enforced by a command hook (scripts/goal_guard.py) reading "
        "the repository, not by a judge reading your summary. The list above is "
        "the last recorded state, not a fresh run — the Stop hook re-checks for "
        "real when this turn ends. Continue working it.",
    ]

    event = hook_input.get("hook_event_name") or "SessionStart"
    emit(
        {
            "hookSpecificOutput": {
                "hookEventName": event,
                "additionalContext": "\n".join(lines),
            }
        }
    )
    return 0


def mode_status(root: Path) -> int:
    goal = load_json(active_path(root))
    if not goal:
        print("no goal armed")
        return 0
    print(f"goal: {goal.get('goal', '')}")
    deadline = parse_deadline(goal.get("deadline_utc"))
    if deadline:
        print(f"deadline: {deadline.isoformat()} ({deadline - now_utc()} remaining)")
    results = run_checks(goal, root)
    for r in results:
        mark = "✅" if r.ok else "▢"
        detail = f"  ({r.detail})" if r.detail and not r.ok else ""
        print(f"  {mark} {r.name}{detail}")
    state = load_json(state_path(root)) or {}
    if state:
        print(f"blocks so far: {state.get('blocks', 0)}, stalled: {state.get('stalled', 0)}")
    return 0 if results and all(r.ok for r in results) else 1


def mode_arm(root: Path, source: str) -> int:
    src = Path(source)
    if not src.is_absolute():
        src = root / source
    goal = load_json(src)
    if goal is None:
        print(f"cannot read a goal from {src}", file=sys.stderr)
        return 2
    if not goal.get("checks"):
        print(
            "refusing to arm a goal with no checks — a goal with nothing to "
            "verify is exactly the judged-by-vibes hook this replaces",
            file=sys.stderr,
        )
        return 2
    goal_dir(root).mkdir(parents=True, exist_ok=True)
    if src.resolve() != active_path(root).resolve():
        shutil.copyfile(src, active_path(root))
    state_path(root).unlink(missing_ok=True)
    print(f"armed: {goal.get('goal', '')}")
    return mode_status(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inject", action="store_true", help="SessionStart mode")
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--arm", metavar="GOAL_JSON")
    parser.add_argument("--clear", action="store_true")
    args = parser.parse_args()

    root = repo_root()

    if args.arm:
        return mode_arm(root, args.arm)
    if args.clear:
        clear_goal(root, "cleared by hand")
        print("goal cleared")
        return 0
    if args.status:
        return mode_status(root)

    hook_input = {}
    if not sys.stdin.isatty():
        try:
            raw = sys.stdin.read()
            if raw.strip():
                hook_input = json.loads(raw)
        except (OSError, ValueError):
            hook_input = {}

    if args.inject:
        return mode_inject(root, hook_input)
    return mode_stop(root, hook_input)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        # A crashed guard must never silently release a run: say so loudly and
        # keep blocking, because "the guard broke" is not "the work is done".
        root = repo_root()
        if active_path(root).exists():
            emit(
                {
                    "decision": "block",
                    "reason": (
                        f"goal_guard.py crashed ({exc!r}) while a goal was armed. "
                        "Treating this as NOT MET. Fix the guard, then continue "
                        "the goal."
                    ),
                }
            )
        else:
            print(f"goal_guard: {exc!r}", file=sys.stderr)
        sys.exit(0)
