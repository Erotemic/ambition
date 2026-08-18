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

## Pausing for one turn (`--pause`) — the exception that is not an exit

Those four END a run. There is a fifth thing a human wants that none of them
serves: *"finish that and then wait for me."* Without a way to say it, the only
lever is `--clear`, which kills a 72-hour run to ask a question — so the run
gets cleared, or the agent talks itself out of the guard instead. Neither is
what was asked for.

    python3 scripts/goal_guard.py --pause "Jon asked me to stop and wait"

That arms a ONE-SHOT token. The Stop hook at the end of this turn spends it
(skipping the checks entirely, since the point is to hand the turn back now) and
lets the session idle. The token is gone by then, so the NEXT Stop blocks
normally: the pause costs exactly one turn, and pausing again needs asking
again. It expires after `PAUSE_TTL_MINUTES` unused, so it cannot be armed early
and cashed in at 3am, and every spend prints its reason and increments
`pauses` in `.goal/state.json`, where `--status` shows it.

⛔ **The agent arms this only when the human asks for it in that turn.** The
guard cannot check intent — it makes the act visible instead, which is the same
bargain the rest of this file makes.

## Holding indefinitely (`--hold` / `--unhold`) — when one turn is not enough

⭐ **added 2026-08-18, because the one-shot was the wrong shape for working on
something together.** `--pause` buys exactly one turn, so a conversation — Jon
and the agent iterating on Mary-O's art, a render and a reaction per exchange —
costs a re-arm every single turn. Jon: *"if we need to implement a real pause
option for the goal script, then do it."*

    python3 scripts/goal_guard.py --hold "we're working on the art"
    python3 scripts/goal_guard.py --unhold

A hold is NOT spent by reading it. Every Stop while held skips the checks and
hands the turn back, until a human lifts it. Three properties keep that from
being a hole:

* ⭐ **the deadline still runs.** A held goal releases on `deadline_utc` /
  `max_run_hours` exactly as an unheld one does. Holding buys quiet, not
  immortality, and a run held past its deadline simply ends.
* ⭐ **it is loud on every turn, not just the first.** Each Stop prints that the
  goal is held, for how long, and how to lift it — so unlike a silent hold it
  cannot be forgotten, and the age in the message is the nag.
* ⭐ **a new session is told before it reads the open items.** `--inject`
  announces the hold and returns, instead of listing work that would read as
  pressure to resume something the human deliberately stopped.

⛔ **this one is a HUMAN's instrument, more than `--pause` is.** A one-shot armed
unasked costs a turn; a hold armed unasked costs the run. Nothing here can
prevent that — what it does is make it impossible to hide.

## Giving a live run more time (`--extend`)

    python3 scripts/goal_guard.py --extend 48h      # 2d, 90m, or 2026-08-20T20:15Z
    python3 scripts/goal_guard.py --extend          # just print the clocks

⛔ **Do not hand-edit `deadline_utc`.** Two of the four releases above are
clocks, they are stored in different units against different origins, and the
EARLIER one wins: `deadline_utc` is absolute, `max_run_hours` counts from the
first block. Moving one is not extending the run — it leaves the other to fire
on the old schedule, from a field the editor was not looking at. `--extend`
moves both and keeps the gap between them.

It will not touch `max_stalled_blocks`. That is a progress oracle, not a clock;
buying hours is not evidence that work is landing, and resetting it silently
would turn "extend the timer" into "forgive the stall" — so the stall count is
printed instead, which is the number a human extending a quiet run needs to see.

With no argument it changes nothing and prints the three releases. `--status`
answers the same question, but it RUNS EVERY CHECK to do it, and in a repository
whose checks build the workspace that is minutes of wall clock to learn a date.

## Coordinator mode: waiting on background work is not stopping

A coordinator that spawns subagents and yields the turn to wait for them is
ENDING A TURN, and a turn ending is the only event this hook can observe. Left
alone, the guard blocks that yield and tells the agent to resume — every time,
forever, each block injecting the whole preamble. That is the guard pushing an
agent through precisely the behaviour it wanted.

So the Stop path checks the TRANSCRIPT for work that went async and never
reported back, and stands down while any is outstanding. The wait is read from
the transcript rather than from anything the agent says, because "I am waiting
for my subagents" is the cheapest sentence an agent that wants out can write.
Three bounds, and each one exists because of a specific way this could go wrong:

* every `WAIT_HEARTBEAT_MINUTES` a still-unmoved wait is BLOCKED anyway, with a
  short message asking how the subagents are doing. Work in flight hangs, and
  from inside a Stop hook a hung task and a slow one are indistinguishable
  (Jon, 2026-08-15);
* the wait resets whenever the outstanding SET changes, so a coordinator whose
  subagents are landing keeps earning quiet and one whose set is frozen does not;
* after `WAIT_CEILING_HOURS` the stand-down stops entirely. A task that was
  KILLED never sends a completion, so without a ceiling one `TaskStop` buys
  silence for the rest of the session.

⚠ The launch/completion pairing is VERIFIED for background `Bash` tasks. Agent
tool subagents are documented to report through the same channel and the join key
is not tool-specific, but no transcript available when this was written contained
one — every `isSidechain` count was zero. If subagents are not standing the guard
down, that is the first thing to check.

## The full goal text is worth tokens sometimes, not every turn

The preamble is several thousand tokens and byte-identical every turn, so
reprinting it at every block spends the context the agent needs to do the work in
order to say nothing new. Repeats get a short form — the open items, the time
left, and the closing push — and the full text comes back on the first block, on
`FULL_REASON_INTERVAL_MINUTES`, and whenever the transcript shows a COMPACT since
the last full print. That last one is the case that matters: a compact is exactly
when the agent has lost the goal. A goal shorter than
`SHORT_FORM_MIN_GOAL_CHARS` is always printed whole, because abbreviating it
costs more than it saves.

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

## The goal file — per RUN

    {
      "goal": "complete the queue",
      "deadline_utc": "2026-07-26T04:51:00Z",
      "max_stalled_blocks": 3,
      "checks": [
        {"name": "the ledger has no open rows",
         "cmd": "grep -q 'TODO' docs/ledger.md; test $? -eq 1"},
        {"name": "the project still builds",
         "cmd": "<your build command>", "timeout": 900}
      ]
    }

`deadline_utc` is what makes "24 hours" mean 24 hours in both directions: the
guard holds the run open until then, and releases it afterwards rather than
wedging the session forever.

⛔ **Write a file-scoped check as `grep -q PAT F; test $? -eq 1`, never as
`! grep -q PAT F`.** `grep` exits 2 when it cannot READ the file, and `!` turns
that into a pass — so a check whose subject gets renamed or archived reports
satisfied forever, about a file that no longer exists. Three checks in this repo
did exactly that for two days. `test $? -eq 1` passes only on "read it, found
nothing", so a vanished subject goes RED and names itself.

## `.goal-guard.json` — per REPOSITORY

Two things vary by repository rather than by run, and they were constants in
this file until 2026-08-15, which is what stopped it being portable:

    {
      "build_processes": ["cargo", "rustc"],
      "default_check_timeout": 900
    }

`build_processes` names the processes whose presence means something else is
competing for the machine while a check is timed; leave it out and the cost
recorder writes `null` rather than claiming an idle machine it never observed.
`default_check_timeout` is the per-check ceiling when a check does not set its
own. Both are optional — a repo with no config file at all is a working install.

It is COMMITTED, unlike `.goal/`, which is gitignored per-run state.

## Porting this to another repository

Copy `goal_guard.py`, its tests, the two hook entries in `.claude/settings.json`,
and add `.goal/` to `.gitignore`. Then write a `.goal-guard.json`. There is
nothing else: the file is stdlib-only, `git` is soft-required (an unreadable
HEAD counts as a stall, never as progress), and the checks are ordinary shell.

Three assumptions come with it, and a new repo should agree to them rather than
discover them:

* **the progress oracle is a new commit on HEAD.** `max_stalled_blocks` releases
  a run that has blocked N times without one. That suits a repo where finished
  work is committed work; a repo with long-lived branches and rare commits would
  be released for making steady progress;
* **the guard must sit one directory below the repo root** — `repo_root()` is
  `__file__.parent.parent` — and the up-walk in the hook command must name that
  directory;
* **`/proc` is how contention is sampled.** Elsewhere the cost recorder writes
  `-1`, which is honest and useless; the rest of the guard is unaffected.

The narrative behind these rules — which incident produced which line — is in
`docs/tools/goal-guard.md`. Rules travel; stories stay where they happened.

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
import re
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
# How long a `--pause` token stays valid. A pause is armed DURING a turn and
# spent when that turn ends, so this only has to cover one turn's work — and it
# has to expire, because a token that lives forever is a release the agent can
# arm at hour 2 and cash in at hour 40, long after the human who asked for it
# has gone to bed.
PAUSE_TTL_MINUTES = 30.0

# ── Waiting on background work (coordinator mode) ─────────────────────────────
#
# A coordinator that spawns background subagents and yields the turn to wait for
# them is ENDING A TURN, which is the only thing this hook can see. Without the
# stand-down below it gets blocked and told to resume — every yield, forever,
# each block injecting the whole goal. That is not a stall the guard should push
# through; it is the agent doing exactly the right thing.
#
# So: while launched work has not reported back, the guard stands down. Three
# bounds keep that from becoming a hole, because "I am waiting" is the easiest
# sentence in the world for an agent that wants out to write:
#
# * the wait is derived from the TRANSCRIPT, not from anything the agent says —
#   a real `tool_use` that really went async and really has not returned;
# * every WAIT_HEARTBEAT_MINUTES the guard blocks anyway to ask how the subagents
#   are doing, because in-flight work hangs and a silent coordinator waiting on a
#   dead task looks exactly like a healthy one (Jon, 2026-08-15);
# * after WAIT_CEILING_HOURS with nothing returning, standing down stops. A task
#   that was killed never sends a completion, so without this one `TaskStop` buys
#   permanent silence.
WAIT_HEARTBEAT_MINUTES = 60.0
WAIT_CEILING_HOURS = 4.0

# How often the FULL goal text is reprinted in a block reason. The preamble runs
# to several thousand tokens and it is identical every turn, so repeating it at
# every block spends context to say nothing new. Repeats get the short form; the
# full text comes back on the first block, on this interval, and — the case that
# actually matters — whenever the transcript shows a COMPACT since the last full
# print, because that is precisely when the agent has lost it.
FULL_REASON_INTERVAL_MINUTES = 120.0

# How much of the transcript tail to read when looking for those signals. The
# transcripts in this project reach tens of megabytes, and this runs at the end
# of every turn. A launch older than this window is not a live wait, and a
# compact older than it is covered by FULL_REASON_INTERVAL_MINUTES anyway.
TRANSCRIPT_TAIL_BYTES = 2_000_000

# Below this, a goal is reprinted in full every time. The short form carries a
# sentence explaining where the full text went, so abbreviating a small goal
# makes the block BIGGER — the saving only exists for the multi-thousand-token
# preambles this repo actually arms.
SHORT_FORM_MIN_GOAL_CHARS = 1200


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

    This asked `git rev-parse --show-toplevel`, and claimed that made the guard
    work "from any cwd a hook happens to have". It did not: a hook inherits the
    session's working directory, and one `cd` into a NESTED git repository made
    `--show-toplevel` answer with the sub-repo, where `.goal/active.json` does
    not exist — so `mode_stop` took its "not armed: ordinary sessions are
    untouched" path and released a 72-hour run (2026-08-05; see
    `docs/tools/goal-guard.md`).

    `git` was never the right authority for this. The guard is armed relative to
    the repo it is COMMITTED IN, and `__file__` says which one that is without
    asking anybody. A worktree gets its own copy and so resolves to itself, which
    is also correct.

    ⛔ This is why a port must keep the guard exactly one directory below the
    repo root.
    """
    return Path(__file__).resolve().parent.parent


def config_path(root: Path) -> Path:
    return root / ".goal-guard.json"


def guard_config(root: Path) -> dict:
    """Per-REPOSITORY settings, as opposed to per-RUN settings.

    The goal JSON is already the config layer for a run: what to check, how long
    to run, when to release. But two things vary by REPOSITORY rather than by
    run, and they were constants in this file until 2026-08-15 — which is the
    coupling that stopped it being portable. A Rust repo and a Python repo want
    different numbers here and neither should have to fork the guard to say so.

    It lives at the repo root and is COMMITTED, unlike `.goal/`, which is
    gitignored per-run state and would take this with it on a fresh clone.

    Absent or unreadable, the defaults apply and the guard behaves exactly as it
    did before this existed — a port with no config file is a working port.

        {
          "build_processes": ["cargo", "rustc"],
          "default_check_timeout": 120
        }

    `build_processes` names the processes whose presence means something ELSE is
    competing for this machine while a check is timed. Empty (the default) makes
    the recorder write `null` rather than `0`: a repo that never said what a
    build looks like has not observed an idle machine, and recording that as
    "zero contention" would be a measurement that cannot fail.
    """
    raw = load_json(config_path(root))
    if not isinstance(raw, dict):
        raw = {}
    names = raw.get("build_processes")
    if not isinstance(names, list):
        names = []
    return {
        "build_processes": [str(n) for n in names if str(n).strip()],
        "default_check_timeout": as_float(
            raw.get("default_check_timeout"), DEFAULT_CHECK_TIMEOUT
        ),
    }


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


_DURATION_UNIT_HOURS = {
    "": 1.0,
    "h": 1.0,
    "hr": 1.0,
    "hrs": 1.0,
    "hour": 1.0,
    "hours": 1.0,
    "m": 1.0 / 60.0,
    "min": 1.0 / 60.0,
    "mins": 1.0 / 60.0,
    "minute": 1.0 / 60.0,
    "minutes": 1.0 / 60.0,
    "d": 24.0,
    "day": 24.0,
    "days": 24.0,
    "w": 168.0,
    "week": 168.0,
    "weeks": 168.0,
}


def parse_duration_hours(raw) -> float | None:
    """`48h`, `48`, `2d`, `90m`, `1.5h`, `2 days` → hours. Anything else, None.

    A BARE NUMBER MEANS HOURS, which is the one thing a caller might get wrong
    silently, so the unit is echoed back by every command that uses this.
    """
    text = str(raw).strip().lower().replace(" ", "")
    if not text:
        return None
    digits = 0
    while digits < len(text) and (text[digits].isdigit() or text[digits] == "."):
        digits += 1
    unit = _DURATION_UNIT_HOURS.get(text[digits:])
    if unit is None:
        return None
    try:
        value = float(text[:digits])
    except ValueError:
        return None
    return value * unit if value > 0 else None


def format_hours(hours: float) -> str:
    """`122.0` → `5d 2h`. For humans reading a release time, not for parsing."""
    if hours <= 0:
        return "0h"
    days, rest = divmod(round(hours * 60), 24 * 60)
    part_h, minutes = divmod(rest, 60)
    parts = [f"{days}d" if days else "", f"{part_h}h" if part_h else "", f"{minutes}m" if minutes else ""]
    return " ".join(p for p in parts if p) or "0h"


def stamp_utc(when: _dt.datetime) -> str:
    """The `deadline_utc` spelling this file already uses, so `--extend` writes
    what a hand edit would have written."""
    return when.astimezone(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# ── Running the checks ────────────────────────────────────────────────────────


class CheckResult:
    def __init__(self, name: str, cmd: str, ok: bool, detail: str, seconds: float = 0.0):
        self.name = name
        self.cmd = cmd
        self.ok = ok
        self.detail = detail
        self.seconds = seconds


def _foreign_build_processes(names: list[str]) -> int | None:
    """How many of `names` are running — the machine's contention at sample time.

    Read from `/proc` rather than shelling out to `pgrep -f`, which MATCHES ITS
    OWN SHELL and so can never report zero (a tighter regex does not fix it).

    The names were `("cargo", "rustc")` in the source until 2026-08-15. That made
    the recorder silently meaningless in any repo that does not build Rust — it
    would count zero forever and the row would read as a quiet machine. They come
    from `.goal-guard.json` now.

    Three distinguishable answers, and keeping them apart is the whole point:
    a COUNT, `None` for "this repo never said what a build looks like", and `-1`
    for "`/proc` could not be read" (a Mac, a container). Collapsing any of those
    into `0` would report an idle machine that was never observed.
    """
    if not names:
        return None
    wanted = set(names)
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
            if comm in wanted:
                count += 1
    except OSError:
        return -1
    return count


def record_check_cost(
    root: Path, results: list[CheckResult], load_before: float | None = None
) -> None:
    """Append one row per Stop-hook invocation to `.goal/check_cost.jsonl`.

    ⭐ **Jon, 2026-08-08: measure compile and test time in the context of REAL
    work.** This is that loop. A guard that runs the build and the suite at the
    end of every turn is the heaviest recurring job on the machine, and it had
    run 114 times in 11h17m before anything timed it — while purpose-built
    instruments measured only synthetic builds staged for measurement.

    ⛔ **it writes under `.goal/`, which the "nothing is left uncommitted" check
    already excludes.** A recorder that dirties the tree would fail the very run
    it measures — the instrument becoming the defect.

    ⚠ **`foreign_builds` is a POINT-IN-TIME sample and is noisy** — build
    processes come and go between samples. `load_before` / `load_after`
    (1-minute averages) are the sturdier contention signal; prefer them when the
    two disagree. `build_processes` records WHAT was counted, because a
    contention number whose subject is unstated cannot be compared to anything.

    ⚠ **and none of this is decoration.** A duration with no contention stamp
    records the supervisor's cadence as if it were the code, and the biggest
    contender for the machine IS this guard. Recording never fails the run: it is
    best-effort by construction.

    The measurements that motivated this, including one whose headline number
    turned out to be overstated, are written up in `docs/tools/goal-guard.md`.
    """
    try:
        watched = guard_config(root)["build_processes"]
        payload = {
            # SCHEMA 2 (2026-08-15): `foreign_builds` may now be `null`, meaning
            # this repo never declared what a build process looks like, and the
            # new `build_processes` field says what was actually counted. A
            # contention number whose subject is unrecorded cannot be compared
            # across repos, which is the same reason a duration needs a
            # contention stamp in the first place.
            "schema": 2,
            "kind": "goal_check",
            "recorded_at": now_utc().isoformat(),
            "head": head_sha(root),
            "total_seconds": round(sum(r.seconds for r in results), 3),
            "load_before": None if load_before is None else round(load_before, 2),
            "load_after": round(os.getloadavg()[0], 2),
            "build_processes": watched,
            "foreign_builds": _foreign_build_processes(watched),
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
    fallback_timeout = guard_config(root)["default_check_timeout"]
    for raw in goal.get("checks", []):
        name = str(raw.get("name") or raw.get("cmd") or "unnamed check")
        cmd = raw.get("cmd")
        if not cmd:
            results.append(CheckResult(name, "", False, "check has no `cmd`"))
            continue
        timeout = raw.get("timeout", fallback_timeout)
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
            #
            # But it is not a FAILURE either, and saying only "timed out after
            # 120s" let this repo read a green suite as red for days: the check
            # `cargo test -p ambition_app --test app_it` cannot finish a cold
            # compile inside the default, so the block reported an integration
            # suite that was never actually run. The line has to say which of the
            # two it is and what to do about it, or the next repo loses the same
            # days to the same sentence.
            results.append(
                CheckResult(
                    name, cmd, False,
                    f"NOT RUN — timed out after {timeout:g}s. This is the "
                    "clock, not a failure: give this check its own `timeout` "
                    "in the goal, or raise `default_check_timeout` in "
                    ".goal-guard.json",
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


def clear_goal(root: Path, reason: str, remove: bool = True) -> None:
    """Archive rather than delete: what a finished run was asked to do is
    evidence, and the next run's post-mortem wants it.

    `remove=False` archives the outgoing goal WITHOUT disarming, which is what
    `--arm` needs: replacing a goal is not clearing one, but the thing being
    replaced is still evidence and still gone forever if nobody writes it down.
    """
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
        if not remove:
            return
        active.unlink()
        state_path(root).unlink(missing_ok=True)
        owner_path(root).unlink(missing_ok=True)
    except OSError:
        pass


# ── Reading the transcript for what the hook input does not say ───────────────

# A tool result announcing that its work went async. BOTH forms matter and only
# one of them was obvious: a call made with `run_in_background` says "running in
# background with ID", and a FOREGROUND call that outlived its timeout says
# "moved to the background (ID". Matching only the first misses every task that
# went async by timing out — which is the shape a HUNG task arrives in, so the
# miss would land exactly where the heartbeat is needed most.
#
# ⛔ THE ID IS PART OF THE PATTERN, and leaving it out cost a false positive on
# the first run: a `grep` whose OUTPUT contained the phrase "running in
# background with ID" registered as a launch, so the guard believed a task was
# in flight that had never existed. Requiring `: <id>` is what separates the
# announcement from prose about the announcement — the same distinction
# `check_absence_contracts.py` learned three times.
_ASYNC_LAUNCH = re.compile(
    r"running in background with ID: (\w+)|moved to the background \(ID: (\w+)\)"
)
# The completion notification. `<tool-use-id>` is the join key because it is on
# BOTH sides; the human-readable task id is only in prose on the launch side.
_TASK_DONE = re.compile(
    r"<tool-use-id>(\w+)</tool-use-id>|<task-id>(\w+)</task-id>"
)
# Matched as a bare token and then CONFIRMED by parsing, because the first
# version keyed on the literal `"subtype":"compact_boundary"` and silently saw
# nothing the moment the separator had a space in it. A transcript writer's
# whitespace is not part of the contract.
_COMPACT_BOUNDARY = "compact_boundary"


def transcript_tail(hook_input: dict) -> list[str]:
    """The last TRANSCRIPT_TAIL_BYTES of the session transcript, as lines.

    Returns [] for anything unreadable. Every caller treats an empty result as
    "no signal", and every signal here can only make the guard QUIETER, so
    failing to read must fail toward the existing behaviour: block as before.
    """
    path = hook_input.get("transcript_path")
    if not isinstance(path, str) or not path.strip():
        return []
    try:
        with open(path.strip(), "rb") as handle:
            handle.seek(0, os.SEEK_END)
            size = handle.tell()
            handle.seek(max(0, size - TRANSCRIPT_TAIL_BYTES))
            raw = handle.read()
    except OSError:
        return []
    text = raw.decode("utf8", errors="replace")
    lines = text.split("\n")
    # A window that starts mid-record leaves a fragment that will not parse.
    return lines[1:] if size > TRANSCRIPT_TAIL_BYTES and len(lines) > 1 else lines


def transcript_signals(hook_input: dict) -> dict:
    """What the transcript knows that the Stop payload does not.

    `pending`: tool_use ids whose work went async and never reported back.
    `last_compact_at`: when this session was last compacted, if it was.

    ⚠ VERIFIED FOR BACKGROUND `Bash` TASKS ONLY. Agent-tool subagents are
    documented to report through the same completion channel, and the join key
    used here (`tool_use_id`) is not tool-specific, but no transcript available
    when this was written contained one — every `isSidechain` count was zero. If
    a coordinator's subagents are NOT standing the guard down, that is the first
    thing to check, and the check is: grep the transcript for the launch phrases
    above and see which one the Agent tool actually emits.
    """
    launched: set[str] = set()
    done: set[str] = set()
    # A launch announces a human-readable task id; a completion carries both that
    # and the tool_use_id. Keeping the alias means EITHER key clears the wait,
    # which is the difference between a guard that works and one that nags about
    # a task that finished under a name it did not recognise.
    alias: dict[str, str] = {}
    last_compact = None
    for line in transcript_tail(hook_input):
        if not line.strip():
            continue
        if _COMPACT_BOUNDARY in line:
            try:
                record = json.loads(line)
                confirmed = record.get("subtype") == _COMPACT_BOUNDARY
            except (ValueError, TypeError, AttributeError):
                confirmed, record = False, {}
            stamp = parse_deadline(record.get("timestamp")) if confirmed else None
            if stamp and (last_compact is None or stamp > last_compact):
                last_compact = stamp
        for match in _TASK_DONE.finditer(line):
            done.add(match.group(1) or match.group(2))
        if "tool_result" not in line:
            continue
        try:
            record = json.loads(line)
        except ValueError:
            continue
        message = record.get("message")
        content = message.get("content") if isinstance(message, dict) else None
        if not isinstance(content, list):
            continue
        for block in content:
            if not isinstance(block, dict) or block.get("type") != "tool_result":
                continue
            tool_use_id = block.get("tool_use_id")
            if not isinstance(tool_use_id, str):
                continue
            # Match the ANNOUNCEMENT, not the word "background" — a grep whose
            # own output mentions backgrounding would otherwise register as a
            # launch, and this file has spent three separate incidents on
            # patterns that matched prose about the thing instead of the thing.
            match = _ASYNC_LAUNCH.search(json.dumps(block.get("content")))
            if match:
                launched.add(tool_use_id)
                task_id = match.group(1) or match.group(2)
                if task_id:
                    alias[tool_use_id] = task_id
    pending = {
        t for t in launched if t not in done and alias.get(t) not in done
    }
    return {"pending": pending, "last_compact_at": last_compact}


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


def short_open_items_text(results: list[CheckResult]) -> str:
    """The open items WITHOUT the goal preamble, for a repeat block.

    The preamble is several thousand tokens and is byte-identical every turn, so
    reprinting it at every block buys nothing and costs the context the agent
    needs to actually do the work. What changes between blocks is the check
    results, so that is what a repeat says. The full text comes back on the
    schedule in `wants_full_reason` — above all, after a compact.
    """
    failed = [r for r in results if not r.ok]
    lines = [
        "GOAL STILL OPEN (short form — the full goal text is unchanged in "
        ".goal/active.json, and is reprinted here after a compact).",
        "",
        f"{len(failed)} of {len(results)} checks are still failing:",
    ]
    for r in failed:
        detail = f" — {r.detail}" if r.detail else ""
        lines.append(f"  ▢ {r.name}{detail}")
    passed = len(results) - len(failed)
    if passed:
        lines.append("")
        lines.append(f"The other {passed} are satisfied.")
    return "\n".join(lines)


def wants_full_reason(state: dict, signals: dict, goal: dict | None = None) -> bool:
    """Whether this block should carry the whole goal text.

    Four ways to earn it. The first is that there is nothing to save: a SHORT
    goal costs less to reprint than the sentence explaining why it was not
    reprinted, and abbreviating it makes the block longer. Then: nothing has been
    printed yet; the transcript shows a COMPACT newer than the last full print,
    which is the case that matters because a compact is precisely when the agent
    lost the goal; or enough time has passed that a re-read is worth the tokens.
    """
    text = (goal or {}).get("goal") or ""
    if len(text) < SHORT_FORM_MIN_GOAL_CHARS:
        return True
    last_full = parse_deadline(state.get("last_full_reason_at"))
    if last_full is None:
        return True
    compact_at = signals.get("last_compact_at")
    if compact_at and compact_at > last_full:
        return True
    return now_utc() - last_full >= _dt.timedelta(
        minutes=FULL_REASON_INTERVAL_MINUTES
    )


def block_reason(
    goal: dict, results: list[CheckResult], deadline, full: bool = True
) -> str:
    parts = [
        open_items_text(goal, results) if full else short_open_items_text(results)
    ]
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


def write_state(root: Path, state: dict) -> None:
    goal_dir(root).mkdir(parents=True, exist_ok=True)
    try:
        state_path(root).write_text(json.dumps(state, indent=2) + "\n")
    except OSError:
        pass


def current_hold(root: Path) -> dict | None:
    """The SUSTAINED pause, if one is in force.

    Unlike [`take_pause`] this is not spent by reading it: a hold stays until a
    human lifts it with `--unhold`. That is the whole difference, and it is the
    difference Jon asked for — a one-shot pause has to be re-armed every turn,
    which makes working through a problem together cost a re-arm per exchange.

    ⚠ **it does not expire, and the deadline is what stops that being a hole.**
    A held goal still releases on `deadline_utc` / `max_run_hours` exactly as an
    unheld one does; holding buys quiet, not immortality.
    """
    state = load_json(state_path(root)) or {}
    hold = state.get("hold")
    return hold if isinstance(hold, dict) else None


def hold_age_text(hold: dict) -> str:
    """How long this hold has been in force, for the message that says so."""
    started = hold.get("held_at")
    try:
        began = _dt.datetime.fromisoformat(started) if started else None
    except (TypeError, ValueError):
        began = None
    if began is None:
        return ""
    mins = (now_utc() - began).total_seconds() / 60.0
    if mins < 90:
        return f" (held {mins:.0f}m)"
    return f" (held {mins / 60.0:.1f}h)"


def take_pause(root: Path) -> dict | None:
    """Spend a pending one-shot pause, if there is a live one.

    Returns the pause record it consumed, or None. Consuming is the whole point:
    the token is removed from `.goal/state.json` before this returns, so the very
    next Stop hook — the end of the turn after the human's reply — blocks again.
    There is no way to pause TWO turns except by asking twice.

    An expired token is dropped rather than honoured. A pause armed at hour 2 and
    spent at hour 40 is not "waiting for input", it is a release with a delay
    fuse, and this guard's whole subject is releases that happen for reasons
    other than the work being done.
    """
    state = load_json(state_path(root)) or {}
    pending = state.get("pause_once")
    if not isinstance(pending, dict):
        return None

    state.pop("pause_once", None)
    expires = parse_deadline(pending.get("expires_at"))
    if expires and now_utc() >= expires:
        # Say nothing to the agent: an expired token is not a decision anybody
        # made, and announcing it invites treating the next one as a release.
        state["last_pause_expired_at"] = now_utc().isoformat()
        write_state(root, state)
        return None

    state["last_pause_at"] = now_utc().isoformat()
    state["pauses"] = as_int(state.get("pauses"), 0) + 1
    write_state(root, state)
    return pending


def handle_wait(root: Path, signals: dict) -> int | None:
    """Stand down while launched background work has not reported back.

    Returns an exit code when this turn was handled as a WAIT, or None to carry
    on into the checks. Three outcomes:

    * nothing pending, or the wait has run past WAIT_CEILING_HOURS — None, and
      the ordinary block path runs. The ceiling is what stops a KILLED task from
      buying silence forever: a task that was stopped never sends a completion,
      so it stays "pending" for the rest of the session.
    * pending and the heartbeat is not due — stand down silently.
    * pending and the heartbeat IS due — block, but with a SHORT reason asking
      about the subagents rather than the goal preamble. Work in flight hangs,
      and a coordinator waiting on a dead task is indistinguishable from a
      healthy one until somebody asks.

    The wait resets whenever the outstanding SET changes, so a coordinator whose
    subagents are landing one by one keeps earning fresh quiet, while one whose
    set has not moved gets asked about it on the hour.
    """
    pending = signals.get("pending") or set()
    if not pending:
        return None
    state = load_json(state_path(root)) or {}
    key = ",".join(sorted(pending))
    if state.get("wait_key") != key:
        # A new or changed wait: something returned, or something new launched.
        state["wait_key"] = key
        state["waiting_since"] = now_utc().isoformat()
        state["last_wait_nudge_at"] = now_utc().isoformat()
        state["waits"] = as_int(state.get("waits"), 0) + 1
        write_state(root, state)
        emit(
            {
                "systemMessage": (
                    f"Goal guard: standing down — {len(pending)} background "
                    "task(s) launched from this session have not reported back. "
                    "The goal is STILL ARMED. It will ask how they are doing in "
                    f"{WAIT_HEARTBEAT_MINUTES:.0f} minutes if none of them return."
                )
            }
        )
        return 0

    since = parse_deadline(state.get("waiting_since"))
    if since and now_utc() - since >= _dt.timedelta(hours=WAIT_CEILING_HOURS):
        # No longer a credible wait. Fall through and block as normal.
        return None

    last_nudge = parse_deadline(state.get("last_wait_nudge_at")) or since
    due = last_nudge is None or now_utc() - last_nudge >= _dt.timedelta(
        minutes=WAIT_HEARTBEAT_MINUTES
    )
    if not due:
        state["waits"] = as_int(state.get("waits"), 0) + 1
        write_state(root, state)
        return 0

    state["last_wait_nudge_at"] = now_utc().isoformat()
    write_state(root, state)
    held = (now_utc() - since).total_seconds() / 3600.0 if since else 0.0
    emit(
        {
            "decision": "block",
            "reason": (
                f"BACKGROUND WORK HAS NOT MOVED IN {held:.1f}h. The same "
                f"{len(pending)} task(s) have been outstanding since this wait "
                "began, and nothing has reported back since. That is either "
                "long-running work or a HUNG one, and from here they look "
                "identical.\n\nCheck on them NOW — read each one's output, and "
                "kill and relaunch anything wedged. A task that was stopped "
                "never sends a completion, so it stays outstanding forever and "
                "this will keep asking.\n\nIf they are all healthy, say so in "
                "one line and go back to waiting; do NOT start unrelated work "
                "to fill the time. The goal is still armed and unchanged in "
                ".goal/active.json."
            ),
        }
    )
    return 0


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

    # A SUSTAINED pause the human asked for. Checked before everything else for
    # the same reason the one-shot is: the point is to hand the turn back now.
    # ⚠ it is deliberately LOUD — every single Stop says the goal is held, names
    # the reason and says how to lift it, so a hold cannot be quietly forgotten
    # the way a silent one would be.
    hold = current_hold(root)
    if hold is not None:
        note = hold.get("reason") or ""
        emit(
            {
                "systemMessage": (
                    "Goal guard: ON HOLD at the human's request"
                    + hold_age_text(hold)
                    + (f" — {note}" if note else "")
                    + ". The goal is STILL ARMED and was not checked; it stays "
                    "held until somebody runs `python3 scripts/goal_guard.py "
                    "--unhold`. Its deadline still releases it on time. Goal: "
                    + goal.get("goal", "")
                )
            }
        )
        return 0

    # A pause the human asked for, spent BEFORE the checks run: the point is to
    # hand the turn back now, and the checks can take minutes. It is not a
    # release — the goal stays armed, this consumes the token, and the next Stop
    # blocks as if nothing happened.
    paused = take_pause(root)
    if paused is not None:
        note = paused.get("reason") or ""
        emit(
            {
                "systemMessage": (
                    "Goal guard: PAUSED for this turn at the agent's request"
                    + (f" — {note}" if note else "")
                    + ". The goal is STILL ARMED and was not checked; it blocks "
                    "again at the end of the next turn. Goal: "
                    + goal.get("goal", "")
                )
            }
        )
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

    # Waiting on background work is not stopping. Decided BEFORE the checks for
    # the same reason the pause is: the point is to hand the turn back now, and
    # the checks take minutes. A coordinator that yields ten times an hour must
    # not pay a full `cargo check --all-targets` for each yield.
    signals = transcript_signals(hook_input)
    waited = handle_wait(root, signals)
    if waited is not None:
        return waited

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
            # Reaching an ordinary block means nothing is in flight (or the wait
            # ran past its ceiling). Dropping the key here keeps `--status`
            # honest and makes the NEXT wait start its clock fresh.
            "wait_key": None,
        }
    )
    write_state(root, state)

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

    full = wants_full_reason(state, signals, goal)
    if full:
        state["last_full_reason_at"] = now_utc().isoformat()
        write_state(root, state)
    emit(
        {
            "decision": "block",
            "reason": block_reason(goal, results, deadline, full=full),
        }
    )
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
    # ⚠ a HELD goal must say so at session start, or a fresh session reads the
    # open items as pressure and starts working on something the human paused.
    held = state.get("hold")
    if isinstance(held, dict):
        note = held.get("reason") or ""
        emit(
            {
                "systemMessage": (
                    "GOAL ON HOLD"
                    + hold_age_text(held)
                    + (f" — {note}" if note else "")
                    + ". It is still armed and its deadline still runs, but no "
                    "turn will be blocked until somebody runs `python3 "
                    "scripts/goal_guard.py --unhold`. ⛔ do NOT resume the goal's "
                    "work on your own; the human paused it deliberately. Goal: "
                    + goal.get("goal", "")
                )
            }
        )
        return 0

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
        # The one place the answer to "extend it" belongs: an agent asked for
        # more time reads this, and without the pointer it goes reading the goal
        # file, finds ONE clock, and edits the wrong half. Injected once per
        # session, not per block, so it costs nothing per turn.
        lines += [
            "",
            f"{hours:.1f}h remain before this goal releases on its own. "
            "⛔ To give it more time, run `python3 scripts/goal_guard.py --extend 48h` "
            "— never hand-edit deadline_utc, there are two clocks and it moves both.",
        ]
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


def mode_pause(root: Path, reason: str) -> int:
    """Arm a ONE-SHOT pause: the current turn may end, the next one may not.

    This is the only sanctioned way for an agent to hand the turn back mid-run,
    and it exists because the alternative Jon was left with — clearing the goal —
    ends the run. "Finish up and wait for me" should cost one turn, not the run.

    Three properties make it a pause rather than a hole:

    * it is SPENT by the next Stop hook and gone (`take_pause`), so the goal is
      live again the moment the human's reply is answered;
    * it EXPIRES (`PAUSE_TTL_MINUTES`), so it cannot be armed early and cashed
      in hours later when nobody is watching;
    * it is LOUD — `.goal/state.json` counts pauses, and spending one prints a
      `systemMessage` naming the reason into the transcript the human reads.

    None of that stops an agent that wants out from calling it unasked. Nothing
    in this file can (see the module docstring): the guard converts persuasion
    into a visible act, and this is one of the visible acts. A pause nobody asked
    for shows up in the terminal, in the state file, and in `--status`.
    """
    if not load_json(active_path(root)):
        print("no goal armed — nothing to pause")
        return 0
    state = load_json(state_path(root)) or {}
    expires = now_utc() + _dt.timedelta(minutes=PAUSE_TTL_MINUTES)
    state["pause_once"] = {
        "armed_at": now_utc().isoformat(),
        "expires_at": expires.isoformat(),
        "reason": reason.strip(),
    }
    write_state(root, state)
    print(
        f"one-shot pause armed — THIS turn may end; the goal blocks again at the "
        f"end of the next turn. Expires {expires.isoformat()} if unused."
    )
    return 0


def mode_hold(root: Path, reason: str) -> int:
    """Hold the goal until a human lifts it — the SUSTAINED sibling of `--pause`.

    ⭐ **why this exists.** `--pause` is a one-shot: it buys exactly one turn and
    has to be re-armed for the next. That is right for *"finish up and wait"*, and
    wrong for *"stop, we are going to work on this together"* — which is what Jon
    asked for while iterating on Mary-O's art, where a re-arm per exchange is
    noise in the middle of a conversation.

    ⚠ **it does not expire, and that is safe for one reason only: the DEADLINE
    still runs.** A held goal releases on `deadline_utc` / `max_run_hours`
    exactly as an unheld one does. Holding buys quiet, not immortality, and a run
    that is held past its deadline simply ends.

    ⛔ **this is a HUMAN's instrument.** The one-shot pause is self-limiting, so an
    agent arming it unasked costs a turn; a hold armed unasked would cost the run.
    Nothing in this file can stop that (see the module docstring) — what it does
    instead is make it impossible to hide: every Stop while held prints that it is
    held, for how long, and why, into the transcript the human reads.
    """
    if not load_json(active_path(root)):
        print("no goal armed — nothing to hold")
        return 0
    state = load_json(state_path(root)) or {}
    if state.get("hold"):
        print("already on hold — `--unhold` lifts it")
        return 0
    state["hold"] = {
        "held_at": now_utc().isoformat(),
        "reason": reason.strip(),
    }
    state["holds"] = as_int(state.get("holds"), 0) + 1
    write_state(root, state)
    print(
        "goal HELD — every turn may now end until `--unhold`. The goal stays "
        "armed and its deadline still releases it on time."
        + (f" Reason: {reason.strip()}" if reason.strip() else "")
    )
    return 0


def mode_unhold(root: Path) -> int:
    """Lift a `--hold`; the next Stop checks the goal normally again."""
    state = load_json(state_path(root)) or {}
    hold = state.get("hold")
    if not hold:
        print("not on hold — nothing to lift")
        return 0
    state.pop("hold", None)
    state["last_unhold_at"] = now_utc().isoformat()
    write_state(root, state)
    print(
        "hold lifted"
        + (hold_age_text(hold) if isinstance(hold, dict) else "")
        + " — the goal blocks again at the end of this turn."
    )
    return 0


def timer_lines(root: Path, goal: dict) -> list[str]:
    """ALL THREE RELEASES, in one place, computed the same way `mode_stop` does.

    ⭐ this exists because "how long is left?" had no cheap answer. `--status`
    knows, but it RUNS THE CHECKS to say so, and in this repository that is a
    cargo build — minutes of wall clock and a screenful of output to learn a
    date. Reading the two files by hand instead is what `--extend` was added to
    stop, and it would be silly to fix the write and leave the read expensive.

    Naming the stall fuse here is the load-bearing part: it is the release
    `--extend` CANNOT move (it counts blocks, not seconds), so a run that is
    about to be let go for lack of commits looks exactly like a healthy one from
    the deadline alone.
    """
    now = now_utc()
    state = load_json(state_path(root)) or {}
    lines: list[str] = []

    deadline = parse_deadline(goal.get("deadline_utc"))
    if deadline:
        left = (deadline - now).total_seconds() / 3600.0
        when = "PASSED" if left <= 0 else f"in {format_hours(left)}"
        lines.append(f"  deadline    {stamp_utc(deadline)}  ({when})")
    else:
        lines.append("  deadline    (none) — ⚠ this run has no intended end")

    fuse_h = as_float(goal.get("max_run_hours"), DEFAULT_MAX_RUN_HOURS)
    first_block = parse_deadline(state.get("first_block_at"))
    if fuse_h <= 0:
        lines.append("  run fuse    disabled (max_run_hours = 0)")
    elif first_block:
        fires = first_block + _dt.timedelta(hours=fuse_h)
        left = (fires - now).total_seconds() / 3600.0
        when = "PASSED" if left <= 0 else f"in {format_hours(left)}"
        lines.append(
            f"  run fuse    {stamp_utc(fires)}  ({when})"
            f" — {fuse_h:g}h from the first block {stamp_utc(first_block)}"
        )
    else:
        lines.append(f"  run fuse    {fuse_h:g}h from the first block (not blocked yet)")

    max_stalled = as_int(goal.get("max_stalled_blocks"), DEFAULT_MAX_STALLED_BLOCKS)
    stalled = as_int(state.get("stalled"), 0)
    close = "  ⚠ CLOSE" if max_stalled > 0 and stalled >= max_stalled - 2 else ""
    lines.append(
        f"  stall fuse  {stalled} of {max_stalled} blocks with no new commit"
        f"{close} — ⛔ NOT movable by --extend; commit something instead"
    )
    return lines


def mode_extend(root: Path, raw: str) -> int:
    """Move BOTH clocks that end a run, in one command, and show the result.

    ⛔⛔ **THE DEADLINE IS NOT THE ONLY CLOCK, AND IT IS NOT EVEN THE ONE THAT
    FIRES FIRST.** `max_run_hours` counts from the FIRST BLOCK, not from the
    arming, so a hand edit that moves `deadline_utc` alone leaves the run to
    release itself on the old schedule through a backstop the editor was not
    looking at. That is the entire reason this is a command: the operation is
    two coupled edits, and the coupling is invisible in the file. Extending
    keeps the gap between them, so whichever one Jon armed as the real end stays
    the real end.

    The stall fuse is deliberately NOT touched. It is a progress oracle, not a
    clock; buying more time is not evidence that work is landing, and silently
    resetting it would turn "extend the timer" into "forgive the stall".

    Called with no argument it changes nothing and just prints the clocks, which
    makes the read cheap too — `--status` answers the same question but runs
    every check to do it.
    """
    goal = load_json(active_path(root))
    if not goal:
        print("no goal armed — nothing to extend")
        return 1

    if not str(raw).strip():
        print(f"goal: {str(goal.get('goal', ''))[:90]}…")
        for line in timer_lines(root, goal):
            print(line)
        print("\nextend with:  python3 scripts/goal_guard.py --extend 48h")
        return 0

    now = now_utc()
    old = parse_deadline(goal.get("deadline_utc"))
    live_base = old if (old and old > now) else now
    hours = parse_duration_hours(raw)
    if hours is not None:
        new_deadline = live_base + _dt.timedelta(hours=hours)
    else:
        # An absolute timestamp is the other thing a human means by "extend to".
        new_deadline = parse_deadline(raw)
        if new_deadline is None:
            print(
                f"cannot read {raw!r} — give a duration (48h, 2d, 90m; a bare "
                f"number is HOURS) or an ISO-8601 timestamp (2026-08-20T20:15Z)"
            )
            return 2
        if new_deadline <= now:
            print(f"{stamp_utc(new_deadline)} is in the past — that would end the run, not extend it")
            return 2
        hours = (new_deadline - live_base).total_seconds() / 3600.0

    fuse_h = as_float(goal.get("max_run_hours"), DEFAULT_MAX_RUN_HOURS)
    if fuse_h > 0:
        # Written even when it was ABSENT: the default applies either way, and a
        # default that fires early is the failure this command exists to prevent.
        goal["max_run_hours"] = round(fuse_h + hours, 3)
    goal["deadline_utc"] = stamp_utc(new_deadline)

    problems = validate_goal(goal)
    if problems:
        print("refusing to write — the extended goal would not arm:")
        for problem in problems:
            print(f"  ⛔ {problem}")
        return 2

    log = goal.get("extended")
    if not isinstance(log, list):
        log = []
    log.append(
        {
            "at": stamp_utc(now),
            "by": f"+{format_hours(hours)}",
            "deadline_utc": goal["deadline_utc"],
            "max_run_hours": goal.get("max_run_hours"),
        }
    )
    goal["extended"] = log[-12:]

    # Written through a temp file: a Stop hook in another session may be reading
    # this exact path, and a half-written goal is an unarmed one.
    target = active_path(root)
    temp = target.with_suffix(".json.tmp")
    try:
        temp.write_text(json.dumps(goal, indent=2, ensure_ascii=False) + "\n")
        temp.replace(target)
    except OSError as exc:
        print(f"could not write {target}: {exc}")
        return 2

    print(f"extended by {format_hours(hours)}" + (f" (from {stamp_utc(old)})" if old else ""))
    for line in timer_lines(root, goal):
        print(line)
    return 0


def mode_status(root: Path) -> int:
    goal = load_json(active_path(root))
    if not goal:
        print("no goal armed")
        return 0
    print(f"goal: {goal.get('goal', '')}")
    held = current_hold(root)
    if held is not None:
        note = held.get("reason") or ""
        print(
            "⏸ ON HOLD"
            + hold_age_text(held)
            + (f" — {note}" if note else "")
            + "   (`--unhold` lifts it; the deadline still releases the goal)"
        )
    owner = owner_session(root)
    print(
        f"owner session: {owner}"
        if owner
        else "owner session: (unclaimed — the next session to stop claims it)"
    )
    for line in timer_lines(root, goal):
        print(line)
    results = run_checks(goal, root)
    for r in results:
        mark = "✅" if r.ok else "▢"
        detail = f"  ({r.detail})" if r.detail and not r.ok else ""
        print(f"  {mark} {r.name}{detail}")
    state = load_json(state_path(root)) or {}
    pending = state.get("pause_once")
    if isinstance(pending, dict):
        live = "expired" if (parse_deadline(pending.get("expires_at")) or now_utc()) <= now_utc() else "pending"
        print(
            f"one-shot pause: {live} (armed {pending.get('armed_at', '?')}"
            + (f", {pending['reason']}" if pending.get("reason") else "")
            + ")"
        )
    if state.get("pauses"):
        print(f"pauses spent: {state['pauses']}, last {state.get('last_pause_at', '?')}")
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
    # ⛔ ARMING USED TO DESTROY THE GOAL IT REPLACED. This was a bare
    # `copyfile` over `active.json`, and `.goal/` is gitignored, so on
    # 2026-08-15 a re-arm erased a live 72-hour goal that existed in no other
    # place — no archive, no git object, nothing. `clear_goal` had written a
    # `done-<stamp>.json` on every OTHER exit from a run since the beginning;
    # replacement was the one door out with no receipt.
    #
    # `remove=False` because this is not a release: the outgoing goal is
    # archived and then immediately overwritten by the incoming one.
    if src.resolve() != active_path(root).resolve():
        clear_goal(root, "replaced by --arm", remove=False)
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
    parser.add_argument(
        "--extend",
        nargs="?",
        const="",
        metavar="DURATION",
        help=(
            "give the armed goal more time: --extend 48h (also 2d, 90m, or an "
            "ISO-8601 timestamp; a bare number is HOURS). Moves deadline_utc AND "
            "max_run_hours together — they are two independent releases and the "
            "earlier one wins. With no argument, just prints the clocks."
        ),
    )
    parser.add_argument(
        "--pause",
        nargs="?",
        const="",
        metavar="REASON",
        help=(
            "let THIS turn end and wait for the human; the goal stays armed and "
            "blocks again at the end of the next turn. Only when asked."
        ),
    )
    parser.add_argument(
        "--hold",
        nargs="?",
        const="",
        metavar="REASON",
        help=(
            "hold the goal until --unhold: every turn may end, the goal stays "
            "armed, and its deadline still releases it. For working on something "
            "together, where a one-shot --pause would need re-arming every turn. "
            "Only when the human asks."
        ),
    )
    parser.add_argument(
        "--unhold",
        action="store_true",
        help="lift a --hold; the goal blocks normally again",
    )
    args = parser.parse_args()

    root = repo_root()

    if args.extend is not None:
        return mode_extend(root, args.extend)
    if args.hold is not None:
        return mode_hold(root, args.hold)
    if args.unhold:
        return mode_unhold(root)
    if args.pause is not None:
        return mode_pause(root, args.pause)
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
