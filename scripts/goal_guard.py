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

And every word of that assumes the guard RAN. Nothing in this file can establish
that — a guard that is never invoked is indistinguishable, from inside, from a
guard whose checks all passed. Both failures found on 2026-08-05 were of exactly
that kind and neither was visible in the checks: the wiring could not find the
script, and `repo_root()` found the wrong repository. The one symptom available
to a human is `.goal/state.json` going stale while the session works on.

## The four ways out, none of which needs the repository to be healthy

A guard that can wedge a session is worse than no guard, and this one has had
that bug twice (a SessionStart hook that ran `cargo check` before the session
was ready; a stall counter that reset itself whenever `git` failed). The escape
hatches are therefore listed here rather than inferred from the code:

1. `deadline_utc` passes — the intended release. `--arm` REFUSES a deadline it
   cannot parse, because a deadline that silently becomes `None` is a run with
   no end.
2. `max_stalled_blocks` blocks with no new commit. An unreadable `HEAD` counts
   as a stall, not as progress: not knowing whether work happened must never
   read as "work happened".
3. `max_run_hours` since the FIRST block (default 36). Depends on nothing but
   the clock — not on git, not on the checks, not on this file being correct
   past the point of parsing.
4. `MAX_CONSECUTIVE_CRASHES` crashes of this script. A crashed guard blocks,
   because a broken instrument is not a passing one, but it stands down before
   a typo here can cost somebody their whole session.

Every one of those says out loud that the goal was NOT met when it fires.

## Wiring (`.claude/settings.json`) — and why the command is not one word long

⛔ **The hook command must not contain a RELATIVE path.** A hook inherits the
session's working directory, which follows the agent around: one `cd` into a
subdirectory and `python3 scripts/goal_guard.py` resolves to nothing. The
runtime treats a hook script it cannot find as NON-BLOCKING and says so only in
a suggestion-level note, so the guard does not fail — it silently ceases to
exist. That released a 72-hour run on 2026-08-05 and the session idled 4h12m.

So the command walks UP from the working directory to find the guard, and, if it
cannot, emits a block of its own rather than vanishing:

    d="${CLAUDE_PROJECT_DIR:-$PWD}"
    until [ -f "$d/scripts/goal_guard.py" ] || [ "$d" = / ]; do d=$(dirname "$d"); done
    if [ -f "$d/scripts/goal_guard.py" ]; then exec python3 "$d/scripts/goal_guard.py"
    else echo '{"decision":"block","reason":"the guard could not be located..."}'; fi

`$CLAUDE_PROJECT_DIR` is preferred when the runtime sets it; the walk is the
fallback that does not depend on a variable this file cannot verify. The
SessionStart copy is the same finder with `--inject`, and stays SILENT when the
walk fails — blocking startup is the one thing that can lock you out (see below).

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

## A goal belongs to ONE session

The goal is armed in the REPOSITORY, but a run belongs to one session. Without
scoping, every other session sharing the working directory — a quick question in
a second window, another agent, a fresh terminal — is held by a goal it was
never given, and the only way out is to disarm the run that IS working.

So the guard records an owner. `session_id` arrives in the hook payload the
runtime already sends, and:

* an UNCLAIMED goal is claimed by the first Stop hook that sees it, so `--arm`
  still needs no session id and the arming run keeps the goal by default;
* any other session's Stop hook returns immediately and has NO side effects on
  the run — it does not run the checks, count a block, or touch the stall
  counter;
* `--inject` stays silent in non-owner sessions, so a second window does not get
  told to work somebody else's queue;
* a payload with NO session id at all (a manual run, an older client) is treated
  as the owner. "I cannot tell whose this is" must fail toward blocking — the
  one thing this must never do by accident is RELEASE a run.

    python3 scripts/goal_guard.py --own <session-id>   # bind it explicitly
    python3 scripts/goal_guard.py --disown             # next session claims it

Ownership lives in `.goal/owner`, which `--arm` and `--clear` remove, so every
fresh run starts unclaimed.

⚠ **`/clear` gives the session a new id**, which orphans the goal: the old owner
will never stop again, so every session is released and the guard is silently
doing nothing. It fails OPEN rather than locking anybody out, which is the right
direction, but nothing announces it — the new session is not the owner, so
SessionStart stays silent and the agent never learns a goal exists.

The way back is one command, and it is what to tell an agent after a `/clear`:

    python3 scripts/goal_guard.py --resume

That releases ownership AND prints the goal with its open items, so the agent
knows what it is resuming; this session claims it at the end of the turn.
`--status` prints the current owner if you need to check.

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
import time
from pathlib import Path

DEFAULT_CHECK_TIMEOUT = 120
DEFAULT_MAX_STALLED_BLOCKS = 3
# The wall-clock ceiling on how long an armed goal may block, counted from its
# FIRST block. A backstop under `deadline_utc`, not a replacement for it: a goal
# with no deadline (or one armed before `--arm` validated deadlines) would
# otherwise block forever if a check could never pass.
DEFAULT_MAX_RUN_HOURS = 36.0
# How many consecutive crashes of this script are treated as "the guard is
# broken" rather than "the work is not done". A crashed guard keeps blocking —
# deliberately, because a broken instrument must not read as success — but it
# cannot do so forever, or a typo in this file wedges every session in the repo.
MAX_CONSECUTIVE_CRASHES = 3


def as_int(value, fallback: int) -> int:
    """A malformed number in the goal file must not crash the guard.

    Every one of these used to be a bare `int(...)` on JSON a human hand-edits at
    2am. A `TypeError` there lands in the crash handler, which keeps blocking —
    so a stray string in `max_stalled_blocks` could wedge the session with no way
    out but deleting the file (GPT 5.6, 2026-07-28)."""
    try:
        return int(value)
    except (TypeError, ValueError):
        return fallback


def as_float(value, fallback: float) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return fallback


def repo_root() -> Path:
    """The root of the repository THIS FILE lives in — never the one cwd is in.

    This asked `git rev-parse --show-toplevel`, and its docstring claimed that
    made the guard work "from any cwd a hook happens to have". It did not. A hook
    inherits the session's working directory, and this tree contains a NESTED git
    repository (`tools/ambition_sprite2d_renderer`). One `cd` into it and
    `--show-toplevel` answered with the sub-repo, where `.goal/active.json` does
    not exist — so `mode_stop` took its "not armed: ordinary sessions are
    untouched" path and released a 72-hour run (2026-08-05, 4h12m idle).

    `git` was never the right authority for this. The guard is armed relative to
    the repo it is COMMITTED IN, and `__file__` says which one that is without
    asking anybody. A worktree gets its own copy and so resolves to itself, which
    is also correct.
    """
    return Path(__file__).resolve().parent.parent


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


def session_id_of(hook_input: dict) -> str:
    """The calling session's id, from either field the runtime provides.

    `session_id` is the documented one. `transcript_path` is read as a fallback
    because its stem IS the session id, and a guard that decides who owns a run
    should not hinge on one key surviving a schema change.
    """
    session = hook_input.get("session_id")
    if isinstance(session, str) and session.strip():
        return session.strip()
    transcript = hook_input.get("transcript_path")
    if isinstance(transcript, str) and transcript.strip():
        return Path(transcript.strip()).stem
    return ""


def owner_path(root: Path) -> Path:
    """Ownership gets its OWN file, deliberately, not a key in `state.json`.

    `mode_stop` loads `state.json`, spends minutes in `run_checks`, then writes
    the whole dict back. A second session claiming ownership during that window
    would have its key silently dropped by the owning session's write — and the
    symptom is the goal quietly reverting to unclaimed, which is the failure that
    releases runs. A separate file has no writer to lose a race with.
    """
    return goal_dir(root) / "owner"


def owner_session(root: Path) -> str:
    try:
        return owner_path(root).read_text().strip()
    except OSError:
        return ""


def set_owner(root: Path, session: str) -> None:
    goal_dir(root).mkdir(parents=True, exist_ok=True)
    try:
        if session:
            owner_path(root).write_text(session + "\n")
        else:
            owner_path(root).unlink(missing_ok=True)
    except OSError:
        # Not fatal: an unrecorded owner reads as unclaimed, which blocks. The
        # failure direction is "holds the run", never "releases it".
        pass


def session_owns_goal(root: Path, hook_input: dict, *, claim: bool) -> bool:
    """Does the armed goal belong to the session calling this hook?

    `claim` is True only for the Stop hook: an unclaimed goal binds to the first
    session that finishes a turn under it, which is the arming run. SessionStart
    must NOT claim — it fires for every new session, so the first window opened
    after arming would steal a run it is not doing.
    """
    session = session_id_of(hook_input)
    if not session:
        return True  # Cannot tell. Fail toward blocking, never toward release.
    owner = owner_session(root)
    if not owner:
        if claim:
            set_owner(root, session)
        return True
    return session == owner


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
    def __init__(self, name: str, cmd: str, ok: bool, detail: str, seconds: float = 0.0):
        self.name = name
        self.cmd = cmd
        self.ok = ok
        self.detail = detail
        self.seconds = seconds


def _foreign_build_processes() -> int:
    """How many cargo/rustc processes are running that are not this check.

    Read from `/proc` rather than shelling out to `pgrep -f`, which MATCHES ITS
    OWN SHELL and so can never report zero (a tighter regex does not fix it).
    """
    count = 0
    try:
        for entry in os.listdir("/proc"):
            if not entry.isdigit():
                continue
            try:
                with open(f"/proc/{entry}/comm", "r") as handle:
                    comm = handle.read().strip()
            except OSError:
                continue
            if comm in ("cargo", "rustc"):
                count += 1
    except OSError:
        return -1
    return count


def record_check_cost(
    root: Path, results: list[CheckResult], load_before: float | None = None
) -> None:
    """Append one row per Stop-hook invocation to `.goal/check_cost.jsonl`.

    ⭐ **Jon, 2026-08-08: measure compile and test time in the context of real
    work.** This is that loop — the goal guard runs `cargo check -p ambition_app`
    and the whole `app_it` suite every time a turn ends. It had run **114 times
    in 11h17m** before anything timed it, while three instruments built the same
    day measured only synthetic builds staged for measurement.

    ⛔ **it writes under `.goal/`, which the "nothing is left uncommitted" check
    already excludes.** A recorder that dirties the tree would fail the very run
    it measures — the instrument becoming the defect.

    ⚠ **`foreign_builds` is a POINT-IN-TIME sample and is noisy** — build
    processes come and go between samples, and a zero here has been verified
    against `/proc` to be real rather than a broken counter. `load_before` /
    `load_after` (1-minute averages) are the sturdier contention signal; prefer
    them when the two disagree.

    ⚠ **and none of this is decoration.** The same
    688-unit build measured 833.9s and 540.0s on this machine depending on what
    else was running, and the biggest contender IS this guard. A duration with no
    contention stamp records the supervisor's reporting cadence as if it were the
    code. Never fails the run: recording is best-effort by construction.
    """
    try:
        payload = {
            "schema": 1,
            "kind": "goal_check",
            "recorded_at": now_utc().isoformat(),
            "head": head_sha(root),
            "total_seconds": round(sum(r.seconds for r in results), 3),
            "load_before": None if load_before is None else round(load_before, 2),
            "load_after": round(os.getloadavg()[0], 2),
            "foreign_builds": _foreign_build_processes(),
            "checks": [
                {
                    "name": r.name,
                    "ok": r.ok,
                    "seconds": round(r.seconds, 3),
                }
                for r in results
            ],
        }
        path = root / ".goal" / "check_cost.jsonl"
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a") as handle:
            handle.write(json.dumps(payload) + "\n")
    except Exception:
        # A measurement must never be able to break the thing it measures.
        pass


def run_checks(goal: dict, root: Path) -> list[CheckResult]:
    """Every check, always — a partial answer would hide the second open item."""
    results: list[CheckResult] = []
    try:
        load_before = os.getloadavg()[0]
    except OSError:
        load_before = None
    for raw in goal.get("checks", []):
        name = str(raw.get("name") or raw.get("cmd") or "unnamed check")
        cmd = raw.get("cmd")
        if not cmd:
            results.append(CheckResult(name, "", False, "check has no `cmd`"))
            continue
        timeout = raw.get("timeout", DEFAULT_CHECK_TIMEOUT)
        started = time.monotonic()
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
                CheckResult(
                    name, cmd, False, f"timed out after {timeout}s",
                    time.monotonic() - started,
                )
            )
            continue
        except OSError as exc:
            results.append(
                CheckResult(
                    name, cmd, False, f"could not run: {exc}",
                    time.monotonic() - started,
                )
            )
            continue
        elapsed = time.monotonic() - started
        tail = (proc.stderr.strip() or proc.stdout.strip() or "").splitlines()
        detail = "" if proc.returncode == 0 else " / ".join(tail[-3:])[:400]
        results.append(
            CheckResult(name, cmd, proc.returncode == 0, detail, elapsed)
        )
    record_check_cost(root, results, load_before)
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
        owner_path(root).unlink(missing_ok=True)
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

    # Whose run is this? Checked FIRST, so a session that does not own the goal
    # has no effect on it at all — it does not run the checks (which can take
    # minutes), count a block, or advance the stall counter toward a release
    # somebody else's work would be judged by.
    if not session_owns_goal(root, hook_input, claim=True):
        return 0

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
    # An UNREADABLE head is not progress.
    #
    # This read `stalled + 1 if sha and sha == last_head else 0`, so a failing
    # `git rev-parse` — a permission problem, a lock file, a repo the hook cannot
    # see — returned "" and reset the counter to zero on every single block. The
    # stall release could then never fire, and the one escape hatch from a run
    # that is going nowhere was disabled by exactly the infrastructure failures
    # most likely to make a run go nowhere (GPT 5.6, 2026-07-28).
    #
    # So the three cases are named: a NEW commit is progress and resets; the same
    # commit is a stall; and not knowing is a stall too, because a guard that
    # cannot see progress must not assume it.
    if not sha:
        stalled = as_int(state.get("stalled"), 0) + 1
    elif sha == state.get("last_head"):
        stalled = as_int(state.get("stalled"), 0) + 1
    else:
        stalled = 0
    max_stalled = as_int(
        goal.get("max_stalled_blocks"), DEFAULT_MAX_STALLED_BLOCKS
    )
    blocks = as_int(state.get("blocks"), 0) + 1
    first_block_at = state.get("first_block_at") or now_utc().isoformat()

    state.update(
        {
            "last_head": sha,
            "stalled": stalled,
            "blocks": blocks,
            "first_block_at": first_block_at,
            "last_block_at": now_utc().isoformat(),
            "last_open": [r.name for r in results if not r.ok],
            # A clean run: the crash fuse in `main` counts consecutive crashes,
            # so reaching here at all means the guard is working.
            "crashes": 0,
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

    # The WALL-CLOCK fuse, which depends on nothing but the clock.
    #
    # `deadline_utc` is the intended release and every armed goal should carry
    # one — but it is optional, and an unparseable one used to become silently
    # no deadline at all. A goal with no deadline and a check that can never pass
    # is an unbounded block, so the guard keeps its own ceiling: `--arm` now
    # rejects a malformed deadline outright, and this catches goals armed before
    # that existed, or edited by hand afterwards.
    fuse_h = as_float(goal.get("max_run_hours"), DEFAULT_MAX_RUN_HOURS)
    started = parse_deadline(first_block_at)
    if fuse_h > 0 and started and now_utc() - started >= _dt.timedelta(hours=fuse_h):
        clear_goal(root, f"wall-clock fuse: blocked for over {fuse_h}h")
        emit(
            {
                "systemMessage": (
                    f"Goal guard: RELEASED by the {fuse_h}h wall-clock fuse — this "
                    f"goal has been blocking since {first_block_at} and is being "
                    f"cleared so the session is usable. It was NOT met. Still "
                    f"open: " + ", ".join(state["last_open"])
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

    # Someone else's run: say nothing. `claim=False` because SessionStart fires
    # for every new session, and the first window opened after arming would
    # otherwise take ownership of a run it is not doing.
    if not session_owns_goal(root, hook_input, claim=False):
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


def mode_resume(root: Path) -> int:
    """Hand an orphaned goal to the session running this command.

    `/clear` gives a session a new id, so a goal stays bound to an id that will
    never stop again: the run is silently unguarded, and SessionStart says
    nothing because the new session is not the owner. That silence is the reason
    this prints the goal rather than only re-pointing it — an agent told to
    "resume the goal" otherwise has no idea what the goal IS.

    Ownership is RELEASED rather than reassigned, because a command line has no
    session id to assign. The next Stop hook claims it, which in practice is this
    session at the end of this turn. The gap is real but small: another session
    stopping in between would take it instead, and `--status` shows who did.
    """
    goal = load_json(active_path(root))
    if not goal:
        print("no goal armed — nothing to resume")
        return 0

    previous = owner_session(root)
    set_owner(root, "")

    print(f"GOAL: {goal.get('goal', '(unnamed goal)')}")
    deadline = parse_deadline(goal.get("deadline_utc"))
    if deadline:
        hours = max(0.0, (deadline - now_utc()).total_seconds() / 3600.0)
        print(f"deadline: {deadline.isoformat()} ({hours:.1f}h remain)")

    state = load_json(state_path(root)) or {}
    last_open = state.get("last_open")
    if isinstance(last_open, list) and last_open:
        print(f"open as of the last Stop check ({state.get('last_block_at', '?')}):")
        for name in last_open:
            print(f"  ▢ {name}")
    else:
        print("checks that will run at the end of every turn:")
        for raw in goal.get("checks", []):
            print(f"  ▢ {raw.get('name') or raw.get('cmd') or 'unnamed check'}")

    print(
        f"\nreleased from session {previous or '(unclaimed)'}; THIS session claims "
        f"it when the turn ends. The goal is enforced by this script reading the "
        f"repository, not by a judge reading a summary — keep working it."
    )
    return 0


def mode_status(root: Path) -> int:
    goal = load_json(active_path(root))
    if not goal:
        print("no goal armed")
        return 0
    print(f"goal: {goal.get('goal', '')}")
    owner = owner_session(root)
    print(
        f"owner session: {owner}"
        if owner
        else "owner session: (unclaimed — the next session to stop claims it)"
    )
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
        # The only symptom of a guard that is not running at all. Every other
        # line here is about whether the WORK is done; this one is about whether
        # the instrument is plugged in, which is the failure that costs hours
        # because nothing else reports it.
        last = parse_deadline(state.get("last_block_at"))
        if last:
            idle_h = (now_utc() - last).total_seconds() / 3600.0
            note = "  ⚠ THE GUARD MAY NOT BE RUNNING" if idle_h >= 1.0 else ""
            print(f"last Stop check: {idle_h:.1f}h ago{note}")
    return 0 if results and all(r.ok for r in results) else 1


def validate_goal(goal: dict) -> list[str]:
    """Every way a goal file can wedge a session, checked BEFORE it is armed.

    Arming used to check one thing — that checks exist — and everything else was
    read later, in a hook, where a bad value either silently disabled a release
    (an unparseable `deadline_utc` became NO deadline) or raised inside the Stop
    hook, which keeps blocking by design (GPT 5.6, 2026-07-28). Both failures
    land on a human who has to work out why the session will not end.

    Arming is the one moment a person is watching, so it is where these belong.
    """
    problems: list[str] = []

    checks = goal.get("checks")
    if not isinstance(checks, list) or not checks:
        problems.append(
            "no `checks` — a goal with nothing to verify is exactly the "
            "judged-by-vibes hook this replaces"
        )
    else:
        for index, raw in enumerate(checks):
            label = f"check {index}"
            if not isinstance(raw, dict):
                problems.append(f"{label}: not an object")
                continue
            label = f"check {index} ({raw.get('name') or 'unnamed'})"
            if not raw.get("cmd"):
                problems.append(f"{label}: has no `cmd`")
            if "timeout" in raw:
                try:
                    timeout = float(raw["timeout"])
                except (TypeError, ValueError):
                    problems.append(f"{label}: `timeout` is not a number")
                else:
                    if timeout <= 0:
                        problems.append(f"{label}: `timeout` must be positive")

    raw_deadline = goal.get("deadline_utc")
    if raw_deadline and parse_deadline(raw_deadline) is None:
        # The dangerous one: this used to degrade to "no deadline", which is
        # indistinguishable from "run until somebody notices".
        problems.append(
            f"`deadline_utc` {raw_deadline!r} is not an ISO-8601 timestamp — a "
            "deadline that does not parse is silently NO deadline"
        )

    for field, kind in (("max_stalled_blocks", int), ("max_run_hours", float)):
        if field in goal:
            try:
                kind(goal[field])
            except (TypeError, ValueError):
                problems.append(f"`{field}` is not a number")

    return problems


def mode_arm(root: Path, source: str) -> int:
    src = Path(source)
    if not src.is_absolute():
        src = root / source
    goal = load_json(src)
    if goal is None:
        print(f"cannot read a goal from {src}", file=sys.stderr)
        return 2
    problems = validate_goal(goal)
    if problems:
        print(f"refusing to arm {src}:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 2
    goal_dir(root).mkdir(parents=True, exist_ok=True)
    if src.resolve() != active_path(root).resolve():
        shutil.copyfile(src, active_path(root))
    state_path(root).unlink(missing_ok=True)
    set_owner(root, "")  # A fresh run is unclaimed; its first Stop takes it.
    print(f"armed: {goal.get('goal', '')}")
    return mode_status(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inject", action="store_true", help="SessionStart mode")
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--arm", metavar="GOAL_JSON")
    parser.add_argument("--clear", action="store_true")
    parser.add_argument(
        "--own",
        metavar="SESSION_ID",
        help="bind the armed goal to one session id (transcript basename)",
    )
    parser.add_argument(
        "--disown",
        action="store_true",
        help="unbind the goal; the next session to stop claims it",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="take over an orphaned goal (after /clear) and print what it is",
    )
    args = parser.parse_args()

    root = repo_root()

    if args.own:
        set_owner(root, args.own.strip())
        print(f"goal bound to session {args.own.strip()}")
        return 0
    if args.disown:
        set_owner(root, "")
        print("goal unbound — the next session to stop will claim it")
        return 0
    if args.resume:
        return mode_resume(root)
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


def note_crash(root: Path) -> int:
    """Count consecutive crashes, so a broken guard is bounded.

    Returns the new count; any clean `mode_stop` resets it to zero.
    """
    state = load_json(state_path(root)) or {}
    crashes = as_int(state.get("crashes"), 0) + 1
    state["crashes"] = crashes
    state["last_crash_at"] = now_utc().isoformat()
    try:
        goal_dir(root).mkdir(parents=True, exist_ok=True)
        state_path(root).write_text(json.dumps(state, indent=2) + "\n")
    except OSError:
        # Cannot even record it. Treat that as the fuse blowing: a guard that can
        # neither run nor remember that it failed has nothing left to be trusted
        # with, and the alternative is an unbreakable block.
        return MAX_CONSECUTIVE_CRASHES
    return crashes


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        # A crashed guard must never silently release a run: say so loudly and
        # keep blocking, because "the guard broke" is not "the work is done".
        #
        # But not forever. Blocking on a crash means a typo in THIS file wedges
        # every session in the repo, with the only escape being to find and move
        # `.goal/active.json` by hand — which is exactly the lock-out this file
        # claims at the top it cannot cause (GPT 5.6, 2026-07-28). After
        # MAX_CONSECUTIVE_CRASHES it releases and says why, loudly enough that
        # nobody mistakes it for the goal being met.
        root = repo_root()
        if active_path(root).exists():
            crashes = note_crash(root)
            if crashes >= MAX_CONSECUTIVE_CRASHES:
                emit(
                    {
                        "systemMessage": (
                            f"Goal guard: RELEASED after crashing {crashes} times "
                            f"in a row ({exc!r}). The goal is NOT met — the guard "
                            f"itself is broken and is standing down so the session "
                            f"is usable. Fix scripts/goal_guard.py, then re-arm."
                        )
                    }
                )
                sys.exit(0)
            emit(
                {
                    "decision": "block",
                    "reason": (
                        f"goal_guard.py crashed ({exc!r}) while a goal was armed "
                        f"({crashes} of {MAX_CONSECUTIVE_CRASHES} consecutive "
                        f"crashes before it stands down). Treating this as NOT "
                        f"MET. FIX THE GUARD FIRST — it is the instrument this "
                        f"run is judged by — then continue the goal."
                    ),
                }
            )
        else:
            print(f"goal_guard: {exc!r}", file=sys.stderr)
        sys.exit(0)
