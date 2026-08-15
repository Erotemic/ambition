"""Poison tests for the deterministic goal guard.

A guardrail that has never been shown to FAIL proves nothing (this repo has the
scar tissue: three green room-replay proofs of a beat that had no consumer). So
every test here first establishes the guard can say "no", and the interesting
ones are the cases where a plausible implementation would wrongly say "yes":

* a transcript that claims the work is finished (the actual 2026-07-25 bug),
* a check that times out rather than answering,
* a guard that crashes while a goal is armed.

Each of those must resolve to STILL OPEN. "We could not tell" is not "done".
"""

from __future__ import annotations

import datetime as _dt
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

GUARD_SOURCE = Path(__file__).resolve().parents[1] / "goal_guard.py"


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def guard_in(repo: Path) -> Path:
    return repo / "scripts" / "goal_guard.py"


@pytest.fixture()
def repo(tmp_path: Path) -> Path:
    """A real git repo — the guard's progress oracle is HEAD, not a counter.

    The guard is COPIED IN at `scripts/goal_guard.py` rather than run from the
    checkout, because the guard locates its own `.goal/` from `__file__`. Running
    the checkout's copy against a tmp repo would aim every test in this file at
    the REAL `.goal/` — and `test_all_checks_passing_clears_the_goal` would then
    disarm whatever run is live. Copying it also means these tests exercise the
    production resolution rule instead of a test-only seam.
    """
    root = tmp_path / "repo"
    (root / "scripts").mkdir(parents=True)
    shutil.copyfile(GUARD_SOURCE, guard_in(root))
    git(root, "init", "-q")
    git(root, "config", "user.email", "guard@test")
    git(root, "config", "user.name", "guard")
    (root / "seed.txt").write_text("seed\n")
    git(root, "add", "seed.txt", "scripts/goal_guard.py")
    git(root, "commit", "-qm", "seed")
    return root


def arm(repo: Path, **goal) -> None:
    goal.setdefault("goal", "the test goal")
    (repo / ".goal").mkdir(exist_ok=True)
    (repo / ".goal" / "active.json").write_text(json.dumps(goal))


def run(repo: Path, *args: str, stdin: dict | None = None, cwd: Path | None = None) -> dict:
    proc = subprocess.run(
        [sys.executable, str(guard_in(repo)), *args],
        cwd=str(cwd or repo),
        input=json.dumps(stdin or {}),
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert proc.returncode == 0, f"the guard must never fail the hook: {proc.stderr}"
    out = proc.stdout.strip()
    return json.loads(out) if out else {}


PASS = {"name": "always true", "cmd": "true"}
FAIL = {"name": "the queue still has open items", "cmd": "false"}


# ── It can say no ─────────────────────────────────────────────────────────────


def test_a_failing_check_blocks_the_stop(repo: Path) -> None:
    arm(repo, checks=[FAIL])
    out = run(repo)
    assert out.get("decision") == "block"


def test_the_block_names_the_open_item(repo: Path) -> None:
    """A block that says only "not done" sends the agent hunting. Name it."""
    arm(repo, checks=[PASS, FAIL])
    reason = run(repo)["reason"]
    assert "the queue still has open items" in reason
    assert "always true" in reason, "and say what is already satisfied"


def test_it_tells_the_agent_a_summary_will_not_end_the_turn(repo: Path) -> None:
    """The 2026-07-25 failure was a status report read as completion. The block
    text has to name that move specifically, or the next run reinvents it."""
    arm(repo, checks=[FAIL])
    reason = run(repo)["reason"].lower()
    assert "status report" in reason and "does not close an item" in reason


# ── The poison cases: a plausible guard would wrongly release here ────────────


def test_a_transcript_claiming_success_does_not_release_it(repo: Path) -> None:
    """THE bug this file exists for.

    `/goal`'s judge reads the conversation, so a well-written "everything is
    done" message clears it. This guard is handed the same claim — through the
    hook payload, including a transcript path whose contents shout success — and
    must not care, because it never reads either.
    """
    transcript = repo / "transcript.jsonl"
    transcript.write_text(
        json.dumps(
            {
                "role": "assistant",
                "text": "All queue items are complete. A1 done, A2 done. "
                "Nothing remains. The goal has been fully met.",
            }
        )
        + "\n"
    )
    arm(repo, checks=[FAIL])
    out = run(
        repo,
        stdin={
            "session_id": "abc",
            "transcript_path": str(transcript),
            "stop_hook_active": False,
        },
    )
    assert out.get("decision") == "block", "prose must not be able to close a goal"


def test_it_keeps_blocking_even_when_stop_hook_active_is_set(repo: Path) -> None:
    """Claude Code's docs tell hooks to return success while `stop_hook_active`
    is true. A goal hook that obeys blocks exactly once and then lets go — the
    same hole in a different shape. The runtime's own consecutive-block cap is
    the loop guard; this one holds."""
    arm(repo, checks=[FAIL])
    out = run(repo, stdin={"stop_hook_active": True})
    assert out.get("decision") == "block"


def test_a_check_that_times_out_is_not_a_pass(repo: Path) -> None:
    """Silence is not consent. A hung check is an unanswered question, and an
    unanswered question leaves the goal open."""
    arm(repo, checks=[{"name": "slow check", "cmd": "sleep 30", "timeout": 1}])
    out = run(repo)
    assert out.get("decision") == "block"
    assert "timed out" in out["reason"]


def test_a_goal_with_no_checks_cannot_be_armed(repo: Path) -> None:
    """A goal with nothing to verify IS the judged-by-vibes hook this replaces."""
    (repo / "empty.json").write_text(json.dumps({"goal": "vibes", "checks": []}))
    proc = subprocess.run(
        [sys.executable, str(guard_in(repo)), "--arm", "empty.json"],
        cwd=str(repo),
        capture_output=True,
        text=True,
    )
    assert proc.returncode != 0
    assert not (repo / ".goal" / "active.json").exists()


def test_a_crash_while_armed_blocks_rather_than_releases(repo: Path) -> None:
    """`.goal/active.json` exists but is not valid JSON for a goal: the guard
    must not treat its own confusion as permission to stop."""
    (repo / ".goal").mkdir(exist_ok=True)
    (repo / ".goal" / "active.json").write_text('{"goal": "x", "checks": "not-a-list"}')
    out = run(repo)
    assert out.get("decision") == "block"


# ── And it can say yes, exactly once, and then get out of the way ─────────────


def test_an_unarmed_repo_is_completely_untouched(repo: Path) -> None:
    """This config is committed, so it runs in Jon's ordinary sessions too. If
    it emitted anything at all when no goal is armed, it would be a nuisance
    that gets deleted, and then it protects nothing."""
    assert run(repo) == {}


def test_all_checks_passing_clears_the_goal(repo: Path) -> None:
    arm(repo, checks=[PASS, PASS])
    out = run(repo)
    assert "decision" not in out
    assert "MET" in out.get("systemMessage", "")
    assert not (repo / ".goal" / "active.json").exists(), "a met goal disarms itself"


def test_a_met_goal_is_archived_not_deleted(repo: Path) -> None:
    """What a run was asked to do is evidence for the post-mortem."""
    arm(repo, checks=[PASS])
    run(repo)
    archived = list((repo / ".goal").glob("done-*.json"))
    assert len(archived) == 1
    assert json.loads(archived[0].read_text())["_cleared_because"] == "all checks passed"


def test_the_deadline_releases_the_run(repo: Path) -> None:
    """"24 hours" has to mean 24 hours in BOTH directions, or an unattended
    session is pinned against the wall forever."""
    arm(repo, checks=[FAIL], deadline_utc="2000-01-01T00:00:00Z")
    out = run(repo)
    assert "decision" not in out
    assert "deadline passed" in out.get("systemMessage", "")


def test_a_far_deadline_does_not_release_it(repo: Path) -> None:
    arm(repo, checks=[FAIL], deadline_utc="2999-01-01T00:00:00Z")
    out = run(repo)
    assert out.get("decision") == "block"
    assert "remain before this goal releases" not in out["reason"], (
        "a deadline 900 years out is noise in a message the agent reads every turn"
    )


def test_a_near_deadline_says_how_long_is_left(repo: Path) -> None:
    """Within the week it IS decision-shaping: how much is left changes whether
    you start the big item or close the small ones."""
    soon = (
        _dt.datetime.now(_dt.timezone.utc) + _dt.timedelta(hours=6)
    ).isoformat()
    arm(repo, checks=[FAIL], deadline_utc=soon)
    assert "remain before this goal releases" in run(repo)["reason"]


# ── The stall escape hatch, and its poison ───────────────────────────────────


def test_it_gives_up_after_blocking_with_no_new_commit(repo: Path) -> None:
    """Blocking forever against an agent that has stopped committing is not
    enforcement, it is a hang. Three strikes and it says so out loud."""
    arm(repo, checks=[FAIL], max_stalled_blocks=3)
    decisions = [run(repo).get("decision") for _ in range(4)]
    assert decisions[:3] == ["block", "block", "block"]
    assert decisions[3] is None, "the fourth must release"
    out = run(repo)
    assert "stuck, not finished" in out.get("systemMessage", "")


def test_a_new_commit_resets_the_stall_counter(repo: Path) -> None:
    """The poison for the test above: an agent that IS working must never be
    released early just because the goal is taking a while."""
    arm(repo, checks=[FAIL], max_stalled_blocks=2)
    assert run(repo).get("decision") == "block"
    assert run(repo).get("decision") == "block"

    (repo / "work.txt").write_text("real work\n")
    git(repo, "add", "work.txt")
    git(repo, "commit", "-qm", "did something")

    assert run(repo).get("decision") == "block", "progress buys more time"
    assert run(repo).get("decision") == "block"


# ── The one-shot pause: it must be a pause and not a door ────────────────────


def state_of(repo: Path) -> dict:
    return json.loads((repo / ".goal" / "state.json").read_text())


def cli(repo: Path, *args: str) -> str:
    """The text-printing modes — `run` above parses stdout as hook JSON."""
    proc = subprocess.run(
        [sys.executable, str(guard_in(repo)), *args],
        cwd=str(repo),
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    return proc.stdout


def test_a_pause_lets_exactly_this_turn_end(repo: Path) -> None:
    arm(repo, checks=[FAIL], max_stalled_blocks=0)
    assert run(repo).get("decision") == "block"
    cli(repo, "--pause", "Jon asked me to wait")
    out = run(repo)
    assert out.get("decision") is None, "the paused turn must be allowed to end"
    assert "PAUSED" in out.get("systemMessage", "")
    assert "Jon asked me to wait" in out["systemMessage"], "say whose idea it was"


def test_the_goal_is_still_armed_after_a_pause(repo: Path) -> None:
    """THE poison. A pause that quietly behaves like `--clear` is the 2026-07-25
    bug with better manners: the run ends and nobody is told it ended."""
    arm(repo, checks=[FAIL], max_stalled_blocks=0)
    cli(repo, "--pause")
    run(repo)  # spends it
    assert (repo / ".goal" / "active.json").exists(), "a pause must not disarm"
    assert run(repo).get("decision") == "block", "the very next turn is guarded"
    assert run(repo).get("decision") == "block"


def test_a_pause_is_spent_once_and_cannot_be_re_used(repo: Path) -> None:
    """One ask, one turn. An agent that could re-spend one token would have an
    unbounded exit and the run would end whenever it got tired."""
    arm(repo, checks=[FAIL], max_stalled_blocks=0)
    cli(repo, "--pause")
    assert run(repo).get("decision") is None
    assert "pause_once" not in state_of(repo), "the token is consumed, not kept"
    assert state_of(repo)["pauses"] == 1, "and counted where a human can see it"
    assert run(repo).get("decision") == "block"


def test_an_expired_pause_does_not_release_the_turn(repo: Path) -> None:
    """A token armed at hour 2 and cashed at hour 40 is not "waiting for input",
    it is a delayed release — exactly the shape this guard exists to refuse."""
    arm(repo, checks=[FAIL], max_stalled_blocks=0)
    cli(repo, "--pause")
    stale = state_of(repo)
    past = _dt.datetime.now(_dt.timezone.utc) - _dt.timedelta(minutes=5)
    stale["pause_once"]["expires_at"] = past.isoformat()
    (repo / ".goal" / "state.json").write_text(json.dumps(stale))

    assert run(repo).get("decision") == "block"
    assert "pause_once" not in state_of(repo), "and the dead token is cleared"


def test_pausing_an_unarmed_repo_does_nothing(repo: Path) -> None:
    """No goal, no state file: a pause must not leave a token lying around for
    the NEXT run to spend before it has done any work."""
    cli(repo, "--pause")
    assert not (repo / ".goal" / "state.json").exists()


# ── Context injection, the compaction half ───────────────────────────────────


def test_inject_restates_the_open_items(repo: Path) -> None:
    """`PostCompact` has no `additionalContext` in its schema, so the goal
    cannot be re-injected at the compaction boundary itself. SessionStart plus
    the Stop hook's own reason are the channels that survive."""
    arm(repo, checks=[FAIL])
    out = run(repo, "--inject", stdin={"hook_event_name": "SessionStart"})
    specific = out["hookSpecificOutput"]
    assert specific["hookEventName"] == "SessionStart"
    assert "the queue still has open items" in specific["additionalContext"]


def test_inject_never_runs_the_checks(repo: Path) -> None:
    """The wedge fix, pinned.

    This used to assert that inject was SILENT when every check passed — which
    it could only know by RUNNING them. On 2026-07-27 a goal whose checks
    included a 900s `cargo check` did exactly that at SessionStart, startup hung
    until the IDE gave up at 60s, and the only way back into the repository was
    to move `.goal/active.json` aside. A guard that can lock you out of the repo
    it guards is worse than no guard, so inject now reads the Stop hook's cache
    and nothing else.

    The consequence is that "the goal is met" is no longer a state inject can
    observe, and the old test was asserting an ability that was removed on
    purpose. What replaces it is the property the fix actually bought: a check
    that would take forever, or fail loudly, is never executed here.
    """
    forever = {"name": "would wedge startup", "cmd": "sleep 600"}
    arm(repo, checks=[forever])

    out = run(repo, "--inject", stdin={"hook_event_name": "SessionStart"})

    context = out["hookSpecificOutput"]["additionalContext"]
    assert "would wedge startup" in context, "the armed check is NAMED, not run"


def test_inject_restates_an_armed_goal_that_has_never_been_checked(repo: Path) -> None:
    """With no Stop hook run yet there is no cache, and silence would read as
    'nothing is armed' — the one thing a fresh session must not conclude."""
    arm(repo, checks=[PASS])
    out = run(repo, "--inject", stdin={"hook_event_name": "SessionStart"})
    assert "GOAL ARMED" in out["hookSpecificOutput"]["additionalContext"]


def test_inject_is_silent_when_unarmed(repo: Path) -> None:
    assert run(repo, "--inject", stdin={"hook_event_name": "SessionStart"}) == {}


# ── Which repository is this? (the 2026-08-05 four-hour hole) ─────────────────


def test_a_nested_git_repo_does_not_hide_the_goal(repo: Path) -> None:
    """The one that cost 4h12m on 2026-08-05.

    `repo_root()` asked `git rev-parse --show-toplevel`, under a docstring
    claiming that made the guard work "from any cwd a hook happens to have". This
    tree contains a nested repository (`tools/ambition_sprite2d_renderer`), so one
    `cd` into it and the guard resolved to the SUB-repo, where `.goal/active.json`
    does not exist. `mode_stop` then took its "not armed: ordinary sessions are
    untouched" path and released a 72-hour run, silently, having verified nothing.

    A guard that answers "no goal is armed" because of where it was standing is
    the same failure as a judge persuaded by prose, with fewer words.
    """
    nested = repo / "tools" / "sub_repo"
    nested.mkdir(parents=True)
    git(nested, "init", "-q")
    git(nested, "config", "user.email", "sub@test")
    git(nested, "config", "user.name", "sub")
    (nested / "f.txt").write_text("x\n")
    git(nested, "add", "f.txt")
    git(nested, "commit", "-qm", "sub seed")

    arm(repo, checks=[FAIL])
    assert run(repo, cwd=nested).get("decision") == "block", (
        "the guard must belong to the repository it is COMMITTED IN, "
        "not to whichever one the working directory happens to be inside"
    )
    # Two companions to this were written and deleted: "found from a
    # subdirectory" and "the checks run at the root". Both passed against the OLD
    # `git rev-parse` implementation, because from a plain subdirectory of the
    # same repo it answered correctly — so neither could ever have caught the bug
    # they were written for. A test that is green through its own motivating case
    # is not coverage, it is furniture.
