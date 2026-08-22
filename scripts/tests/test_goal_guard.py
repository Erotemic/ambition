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

# Goal guard is a self-contained maintainer tool, not an Ambition runtime
# feature. Repo-wide validation excludes detached-tool tests; run them
# explicitly after editing the tool with `./run_tests.sh --tool-tests`.
pytestmark = pytest.mark.detached_tool

GUARD_SOURCE = Path(__file__).resolve().parents[1] / "goal_guard.py"


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def guard_in(repo: Path) -> Path:
    return repo / "scripts" / "goal_guard.py"


@pytest.fixture(scope="session")
def repo_template(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """One committed tiny repo, copied per test instead of rebuilt per test.

    The guard's progress oracle genuinely is Git, so these remain real Git
    repositories. What was expensive was paying five process launches (`init`,
    two `config`s, `add`, `commit`) for every assertion even though every test
    begins from the same commit. A byte-for-byte copy of this tiny repository
    gives each test independent refs/index/worktree while preserving the real
    Git behavior the fixture exists to exercise.
    """
    root = tmp_path_factory.mktemp("goal-guard-template") / "repo"
    (root / "scripts").mkdir(parents=True)
    shutil.copyfile(GUARD_SOURCE, guard_in(root))
    git(root, "init", "-q")
    git(root, "config", "user.email", "guard@test")
    git(root, "config", "user.name", "guard")
    (root / "seed.txt").write_text("seed\n")
    git(root, "add", "seed.txt", "scripts/goal_guard.py")
    git(root, "commit", "-qm", "seed")
    return root


@pytest.fixture()
def repo(tmp_path: Path, repo_template: Path) -> Path:
    """An independent real Git repo starting from the shared committed template."""
    root = tmp_path / "repo"
    shutil.copytree(repo_template, root)
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

# These tests prove timeout semantics, not the passage of human-scale time.
# Keep the subprocess genuinely hung, but let the guard cut it off quickly so
# three timeout assertions do not spend three wall-clock seconds per suite run.
TEST_TIMEOUT_SECONDS = 0.1


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
    arm(
        repo,
        checks=[
            {"name": "slow check", "cmd": "sleep 30", "timeout": TEST_TIMEOUT_SECONDS}
        ],
    )
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


# ── Extending a live run ─────────────────────────────────────────────────────


def cli_raw(repo: Path, *args: str) -> subprocess.CompletedProcess:
    """`cli` asserts rc 0; `--extend` REFUSES bad input with a non-zero rc, and
    refusing is half of what these tests are about."""
    return subprocess.run(
        [sys.executable, str(guard_in(repo)), *args],
        cwd=str(repo),
        capture_output=True,
        text=True,
        timeout=120,
    )


def goal_of(repo: Path) -> dict:
    return json.loads((repo / ".goal" / "active.json").read_text())


def test_extending_moves_both_clocks(repo: Path) -> None:
    """⛔⛔ THE WHOLE REASON THIS IS A COMMAND. Two independent releases end a
    run on time — an absolute deadline and a fuse counted in hours from the first
    block — and the earlier one wins. A hand edit moves the one you can see, and
    the run then releases itself on the old schedule from the one you cannot."""
    arm(repo, checks=[FAIL], deadline_utc="2999-01-01T00:00:00Z", max_run_hours=74)
    assert cli_raw(repo, "--extend", "48h").returncode == 0
    after = goal_of(repo)
    assert after["deadline_utc"] == "2999-01-03T00:00:00Z"
    assert after["max_run_hours"] == 122, "the fuse must move by the same 48 hours"


def test_extending_does_not_forgive_a_stall(repo: Path) -> None:
    """The poison. `--extend` buys TIME, and time is not evidence that work is
    landing. A version that also reset the stall counter would turn every "give
    it another day" into an unbounded silence, which is the failure the stall
    fuse exists for."""
    arm(repo, checks=[FAIL], deadline_utc="2999-01-01T00:00:00Z", max_stalled_blocks=2)
    assert run(repo).get("decision") == "block"
    assert run(repo).get("decision") == "block"
    assert cli_raw(repo, "--extend", "48h").returncode == 0
    assert run(repo).get("decision") is None, "the stall still releases it"
    assert state_of(repo)["stalled"] >= 2, "and the count was never rewritten"


def test_a_dead_deadline_extends_from_now_not_from_itself(repo: Path) -> None:
    """+48h onto a deadline that lapsed three days ago is still in the past, so
    "extend" would hand back a run that is already over — the shape where the
    command reports success and changes nothing."""
    stale = (_dt.datetime.now(_dt.timezone.utc) - _dt.timedelta(days=3)).isoformat()
    arm(repo, checks=[FAIL], deadline_utc=stale)
    assert cli_raw(repo, "--extend", "48h").returncode == 0
    extended = _dt.datetime.fromisoformat(goal_of(repo)["deadline_utc"].replace("Z", "+00:00"))
    assert extended > _dt.datetime.now(_dt.timezone.utc) + _dt.timedelta(hours=47)


def test_a_nonsense_duration_changes_nothing(repo: Path) -> None:
    """A guard file half-written by a rejected command is an UNARMED run."""
    arm(repo, checks=[FAIL], deadline_utc="2999-01-01T00:00:00Z", max_run_hours=74)
    before = (repo / ".goal" / "active.json").read_text()
    proc = cli_raw(repo, "--extend", "next tuesday")
    assert proc.returncode != 0
    assert "ISO-8601" in proc.stdout, "say what it would have accepted"
    assert (repo / ".goal" / "active.json").read_text() == before


def test_a_bare_number_of_hours_is_echoed_as_such(repo: Path) -> None:
    """`--extend 2` is the one input a caller can mean two ways. It means hours,
    and the answer says so rather than leaving them to check the file."""
    arm(repo, checks=[FAIL], deadline_utc="2999-01-01T00:00:00Z")
    proc = cli_raw(repo, "--extend", "2")
    assert goal_of(repo)["deadline_utc"] == "2999-01-01T02:00:00Z"
    assert "2h" in proc.stdout


def test_extending_reads_the_clocks_without_running_the_checks(repo: Path) -> None:
    """With no argument it is the cheap read. `--status` answers the same
    question by running every check, which in a real repository is a build."""
    marker = repo / "ran.txt"
    arm(
        repo,
        checks=[{"name": "an expensive check", "cmd": f"touch {marker}"}],
        deadline_utc="2999-01-01T00:00:00Z",
    )
    before = (repo / ".goal" / "active.json").read_text()
    proc = cli_raw(repo, "--extend")
    assert proc.returncode == 0
    assert not marker.exists(), "printing the clocks must not run the checks"
    assert "2999-01-01T00:00:00Z" in proc.stdout
    assert (repo / ".goal" / "active.json").read_text() == before


def test_extending_an_unarmed_repo_arms_nothing(repo: Path) -> None:
    proc = cli_raw(repo, "--extend", "48h")
    assert proc.returncode != 0
    assert not (repo / ".goal" / "active.json").exists()


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
    # A test that is green through its own motivating case is not coverage, it is furniture.


# ── Waiting on background work is not stopping ────────────────────────────────
#
# A coordinator that spawns subagents and yields to wait for them ENDS A TURN,
# which is the only thing a Stop hook can see. Blocking it there is the guard
# pushing an agent through the exact behaviour it wanted. These tests pin the
# stand-down AND its three bounds, because "I am waiting" is the cheapest
# sentence an agent that wants out can write — so the wait is read from the
# TRANSCRIPT, never from anything the agent claims.


def transcript(repo: Path, *records: dict, name: str = "t.jsonl") -> str:
    path = repo / name
    path.write_text("\n".join(json.dumps(r) for r in records))
    return str(path)


def launched(tool_use_id: str, task_id: str = "task1") -> dict:
    return {
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": f"Command running in background with ID: {task_id}",
                }
            ]
        },
    }


def finished(tool_use_id: str) -> dict:
    return {
        "type": "queue-operation",
        "note": f"<task-notification><tool-use-id>{tool_use_id}</tool-use-id>"
        "<status>completed</status></task-notification>",
    }


def test_work_still_in_flight_stands_the_guard_down(repo: Path) -> None:
    arm(repo, checks=[FAIL])
    out = run(repo, stdin={"transcript_path": transcript(repo, launched("toolu_A"))})
    assert out.get("decision") != "block", "a coordinator waiting is not a coordinator stopping"
    assert "standing down" in out.get("systemMessage", "")


def test_a_stand_down_says_the_goal_is_still_armed(repo: Path) -> None:
    """Silence that looks like a release is how a run gets abandoned."""
    arm(repo, checks=[FAIL])
    out = run(repo, stdin={"transcript_path": transcript(repo, launched("toolu_A"))})
    assert "STILL ARMED" in out["systemMessage"]
    assert json.loads((repo / ".goal" / "active.json").read_text())["checks"]


def test_finished_work_does_not_stand_it_down(repo: Path) -> None:
    arm(repo, checks=[FAIL])
    tp = transcript(repo, launched("toolu_A"), finished("toolu_A"))
    assert run(repo, stdin={"transcript_path": tp})["decision"] == "block"


def test_prose_about_backgrounding_is_not_work_in_flight(repo: Path) -> None:
    """The poison: a grep whose OUTPUT quotes the launch phrase. The first cut of
    this parser counted it as a live task, which would have bought silence for a
    coordinator that had launched nothing at all."""
    arm(repo, checks=[FAIL])
    tp = transcript(
        repo,
        {
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_Z",
                        "content": "grep output: running in background with ID",
                    }
                ]
            },
        },
    )
    assert run(repo, stdin={"transcript_path": tp})["decision"] == "block"


def test_an_unreadable_transcript_blocks_rather_than_stands_down(repo: Path) -> None:
    """Not being able to tell whether work is in flight is not evidence that it
    is. Every failure in this parser has to fail toward the old behaviour."""
    arm(repo, checks=[FAIL])
    assert run(repo, stdin={"transcript_path": "/nope/nope.jsonl"})["decision"] == "block"


def test_a_stalled_wait_is_asked_about_on_the_heartbeat(repo: Path) -> None:
    """In-flight work hangs, and from here a hung task and a slow one look
    identical. Jon, 2026-08-15: ask how the subagents are doing."""
    arm(repo, checks=[FAIL])
    tp = transcript(repo, launched("toolu_A"))
    run(repo, stdin={"transcript_path": tp})
    state = state_of(repo)
    stale = (_dt.datetime.now(_dt.timezone.utc) - _dt.timedelta(hours=2)).isoformat()
    state["last_wait_nudge_at"] = stale
    (repo / ".goal" / "state.json").write_text(json.dumps(state))
    out = run(repo, stdin={"transcript_path": tp})
    assert out["decision"] == "block"
    assert "HAS NOT MOVED" in out["reason"]
    assert "GOAL STILL OPEN" not in out["reason"], "the nudge is short, not the preamble"


def test_a_wait_that_never_ends_stops_buying_silence(repo: Path) -> None:
    """A task that was KILLED never sends a completion, so without a ceiling one
    `TaskStop` would stand the guard down for the rest of the session."""
    arm(repo, checks=[FAIL])
    tp = transcript(repo, launched("toolu_A"))
    run(repo, stdin={"transcript_path": tp})
    state = state_of(repo)
    old = (_dt.datetime.now(_dt.timezone.utc) - _dt.timedelta(hours=9)).isoformat()
    state["waiting_since"] = old
    (repo / ".goal" / "state.json").write_text(json.dumps(state))
    assert run(repo, stdin={"transcript_path": tp})["decision"] == "block"


# ── The full goal text is worth tokens sometimes, not every turn ──────────────


def test_a_repeat_block_drops_the_goal_preamble(repo: Path) -> None:
    arm(repo, goal="THE WHOLE PREAMBLE, " + "thousands of tokens of it. " * 80, checks=[FAIL])
    first = run(repo)["reason"]
    second = run(repo)["reason"]
    assert "THE WHOLE PREAMBLE" in first, "the first block carries the goal"
    assert "THE WHOLE PREAMBLE" not in second, "the second does not repeat it"
    assert len(second) < len(first)


def test_a_repeat_block_still_names_the_open_item(repo: Path) -> None:
    """Shorter must not mean vaguer: what CHANGES between blocks is the checks."""
    arm(repo, checks=[PASS, FAIL])
    run(repo)
    second = run(repo)["reason"]
    assert "the queue still has open items" in second
    assert "does not close an item" in second, "and the closing push survives"


def test_a_compact_brings_the_whole_goal_back(repo: Path) -> None:
    """The one case that matters. A compact is precisely when the agent has lost
    the goal, so that is when reprinting it is worth the tokens."""
    arm(repo, goal="THE WHOLE PREAMBLE, " + "thousands of tokens of it. " * 80, checks=[FAIL])
    tp = transcript(repo, {"type": "assistant", "message": {"content": []}})
    run(repo, stdin={"transcript_path": tp})
    assert "THE WHOLE PREAMBLE" not in run(repo, stdin={"transcript_path": tp})["reason"]
    later = (_dt.datetime.now(_dt.timezone.utc) + _dt.timedelta(minutes=1)).isoformat()
    tp = transcript(
        repo,
        {"type": "system", "subtype": "compact_boundary", "timestamp": later},
    )
    assert "THE WHOLE PREAMBLE" in run(repo, stdin={"transcript_path": tp})["reason"]


# ── Replacing a goal is not the same as having no receipt for it ──────────────


def test_arming_over_a_live_goal_archives_the_one_it_replaces(repo: Path) -> None:
    """The 2026-08-15 data loss. `--arm` was a bare copy over `active.json`, and
    `.goal/` is gitignored, so re-arming erased a live 72-hour goal that existed
    nowhere else. Every OTHER exit from a run had written a receipt for years."""
    arm(repo, goal="THE GOAL THAT WAS ALREADY RUNNING", checks=[FAIL])
    replacement = repo / "next.json"
    replacement.write_text(
        json.dumps({"goal": "the new one", "checks": [PASS], "deadline_utc": "2099-01-01T00:00:00Z"})
    )
    cli(repo, "--arm", str(replacement))
    archived = [p for p in (repo / ".goal").glob("done-*.json")]
    assert archived, "the replaced goal must leave a receipt"
    assert any(
        "THE GOAL THAT WAS ALREADY RUNNING" in json.loads(p.read_text())["goal"]
        for p in archived
    )


def test_arming_still_installs_the_new_goal(repo: Path) -> None:
    """Archiving the outgoing goal must not disarm the incoming one."""
    arm(repo, goal="the old one", checks=[FAIL])
    replacement = repo / "next.json"
    replacement.write_text(
        json.dumps({"goal": "the new one", "checks": [FAIL], "deadline_utc": "2099-01-01T00:00:00Z"})
    )
    # NOT `cli`, which asserts exit 0: `--arm` finishes by printing `--status`,
    # and status exits 1 while anything is open — which is the whole point of
    # arming a goal with an open check.
    subprocess.run(
        [sys.executable, str(guard_in(repo)), "--arm", str(replacement)],
        cwd=str(repo), capture_output=True, text=True, timeout=120,
    )
    assert json.loads((repo / ".goal" / "active.json").read_text())["goal"] == "the new one"
    assert run(repo)["decision"] == "block", "and the new goal is live"


# ── Repo-specific settings live in the repo, not in the guard ────────────────


def test_a_repo_with_no_config_still_works(repo: Path) -> None:
    """A port with no `.goal-guard.json` is a working port."""
    assert not (repo / ".goal-guard.json").exists()
    arm(repo, checks=[FAIL])
    assert run(repo)["decision"] == "block"


def test_the_default_check_timeout_comes_from_the_repo(repo: Path) -> None:
    (repo / ".goal-guard.json").write_text(
        json.dumps({"default_check_timeout": TEST_TIMEOUT_SECONDS})
    )
    arm(repo, checks=[{"name": "a slow check", "cmd": "sleep 30"}])
    reason = run(repo)["reason"]
    expected = f"timed out after {TEST_TIMEOUT_SECONDS:g}s"
    assert expected in reason, "the repo's number, not the built-in one"


def test_a_timeout_says_it_is_the_clock_and_not_a_failure(repo: Path) -> None:
    """This repo read a green integration suite as red for days, because the
    block said only "timed out after 120s" and that looks exactly like a red
    test. A port must not lose the same days to the same sentence."""
    (repo / ".goal-guard.json").write_text(
        json.dumps({"default_check_timeout": TEST_TIMEOUT_SECONDS})
    )
    arm(repo, checks=[{"name": "the integration suite", "cmd": "sleep 30"}])
    reason = run(repo)["reason"]
    assert "NOT RUN" in reason
    assert "timeout" in reason and ".goal-guard.json" in reason, "and say how to fix it"


def test_an_undeclared_build_is_not_a_quiet_machine(repo: Path) -> None:
    """`foreign_builds: 0` in a repo that never said what a build looks like is a
    measurement that cannot fail. It records null instead."""
    arm(repo, checks=[PASS])
    run(repo)
    rows = [
        json.loads(line)
        for line in (repo / ".goal" / "check_cost.jsonl").read_text().splitlines()
        if line.strip()
    ]
    assert rows[-1]["foreign_builds"] is None
    assert rows[-1]["build_processes"] == []


def test_the_cost_row_says_what_it_counted(repo: Path) -> None:
    """A contention number whose subject is unrecorded cannot be compared."""
    (repo / ".goal-guard.json").write_text(
        json.dumps({"build_processes": ["definitely_not_a_real_process"]})
    )
    arm(repo, checks=[PASS])
    run(repo)
    row = json.loads(
        (repo / ".goal" / "check_cost.jsonl").read_text().splitlines()[-1]
    )
    assert row["build_processes"] == ["definitely_not_a_real_process"]
    assert row["foreign_builds"] == 0, "declared and genuinely absent IS zero"


# ── More than one session can be held by one run ──────────────────────────────


def _stop(repo: Path, session: str) -> dict:
    return run(repo, stdin={"session_id": session, "hook_event_name": "Stop"})


def _cli_raw(repo: Path, *args: str) -> subprocess.CompletedProcess:
    """Like `_cli` but tolerates a non-zero exit — a REFUSAL is a real outcome
    with its own status, and asserting success would make it untestable."""
    return subprocess.run(
        [sys.executable, str(guard_in(repo)), *args],
        cwd=str(repo),
        input="{}",
        capture_output=True,
        text=True,
        timeout=120,
    )


def _roster(repo: Path) -> list[str]:
    owner = repo / ".goal" / "owner"
    return owner.read_text().split() if owner.exists() else []


def test_a_bare_clear_refuses_to_disarm_a_roster_it_does_not_own(repo: Path) -> None:
    """⛔⛔ **One lane standing down used to release every other lane.**

    On 2026-08-20 an agent finished its work, typed `--clear`, and silently
    disarmed a second session that was still running — its Stops stopped being
    blocked and nobody noticed until the human asked why it had gone quiet. The
    guard cannot know which window typed the command, so the only safe answer
    when several sessions hold the goal is to refuse and make the caller say
    which one it is.
    """
    arm(repo, goal="shared work", checks=[])
    _cli(repo, "--own", "sess-A")
    _cli(repo, "--own", "sess-B")

    proc = _cli_raw(repo, "--clear")

    assert proc.returncode != 0, "a bare --clear on a shared roster must not succeed"
    assert "REFUS" in proc.stdout.upper()
    assert "sess-A" in proc.stdout and "sess-B" in proc.stdout, (
        "the refusal has to name who is holding it, or the caller cannot act on it"
    )
    assert (repo / ".goal" / "active.json").exists(), "the goal was disarmed anyway"
    assert _roster(repo) == ["sess-A", "sess-B"], "the roster was disturbed"


def test_clearing_by_session_leaves_the_other_holders_armed(repo: Path) -> None:
    """The per-session stand-down: one name leaves, the goal does not."""
    arm(repo, goal="shared work", checks=[])
    _cli(repo, "--own", "sess-A")
    _cli(repo, "--own", "sess-B")

    _cli(repo, "--clear", "sess-A")

    assert _roster(repo) == ["sess-B"]
    assert (repo / ".goal" / "active.json").exists(), (
        "a session leaving is not the goal ending — sess-B is still working"
    )


def test_the_last_holder_leaving_disarms_the_goal(repo: Path) -> None:
    """⚠ the other half, and without it the fix would strand goals forever: a
    goal nobody holds is not armed, it is litter."""
    arm(repo, goal="shared work", checks=[])
    _cli(repo, "--own", "sess-A")
    _cli(repo, "--own", "sess-B")

    _cli(repo, "--clear", "sess-A")
    assert (repo / ".goal" / "active.json").exists()
    _cli(repo, "--clear", "sess-B")

    assert not (repo / ".goal" / "active.json").exists(), (
        "the last holder left and the goal stayed armed with nobody to enforce it"
    )
    assert _roster(repo) == []


def test_clear_all_disarms_a_shared_roster_on_purpose(repo: Path) -> None:
    """The escape hatch the refusal points at. Named separately so that ending a
    run for everybody is a thing you TYPE, not a thing you get by accident."""
    arm(repo, goal="shared work", checks=[])
    _cli(repo, "--own", "sess-A")
    _cli(repo, "--own", "sess-B")

    _cli(repo, "--clear-all")

    assert not (repo / ".goal" / "active.json").exists()
    assert _roster(repo) == []


def test_help_stays_short_enough_for_an_agent_to_read(repo: Path) -> None:
    """⛔ `--help` used to print the module docstring — 296 lines of design
    history — to an agent that was mid-task and wanted to know which flag stops
    the blocking. A reference nobody can skim is a reference nobody reads.

    ⚠ the bound is deliberately loose: this guards the ORDER OF MAGNITUDE, not a
    line count somebody has to keep updating. The docstring itself is untouched
    and still in the file for whoever opens it.
    """
    proc = _cli_raw(repo, "--help")

    lines = proc.stdout.splitlines()
    assert proc.returncode == 0
    assert len(lines) < 90, f"--help is {len(lines)} lines again"
    assert "--clear-all" in proc.stdout, "the way out has to be IN the way out"
    assert "--status" in proc.stdout


def _cli(repo: Path, *args: str) -> str:
    """A management flag prints prose, not a hook decision, so it does not go
    through `run` (which parses stdout as JSON)."""
    proc = subprocess.run(
        [sys.executable, str(guard_in(repo)), *args],
        cwd=str(repo),
        input="{}",
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert proc.returncode == 0, f"{args} failed: {proc.stderr}"
    return proc.stdout


def test_an_unshared_goal_still_holds_exactly_one_session(repo: Path) -> None:
    """⛔ **the property sharing must not spend by accident.** A goal one session
    claimed does not reach out and hold every other window in the repository —
    a quick question in a second terminal is untouched by a run it is not doing.
    This is the behaviour every goal armed before 2026-08-20 had, and it is
    still the default."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    assert _stop(repo, "first")["decision"] == "block"
    assert _stop(repo, "second") == {}, "an unshared goal held a session that never claimed it"


def test_a_shared_goal_holds_every_session_that_stops(repo: Path) -> None:
    """The capability itself: two sessions, one run, both held."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    assert _stop(repo, "first")["decision"] == "block"
    _cli(repo, "--share")
    assert _stop(repo, "second")["decision"] == "block", "a shared goal let a second session go"
    assert _stop(repo, "third")["decision"] == "block"
    roster = (repo / ".goal" / "owner").read_text().split()
    assert roster == ["first", "second", "third"], f"the roster is not in claim order: {roster}"


def test_sharing_is_an_explicit_act_and_is_reversible(repo: Path) -> None:
    """⚠ a shared run holds windows nobody meant to enlist, so it must be
    possible to narrow it again WITHOUT releasing the sessions already working."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    _stop(repo, "first")
    _cli(repo, "--share")
    assert _stop(repo, "second")["decision"] == "block"
    _cli(repo, "--unshare")
    assert _stop(repo, "third") == {}, "a narrowed goal still enlisted a new session"
    assert _stop(repo, "second")["decision"] == "block", "narrowing released a session already held"
    assert _stop(repo, "first")["decision"] == "block"


def test_disown_removes_only_the_session_that_runs_it(repo: Path) -> None:
    """⛔ the multi-session failure that would be worst: one lane finishing and
    silently releasing the other."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    _stop(repo, "first")
    _cli(repo, "--share")
    _stop(repo, "second")
    _cli(repo, "--own", "second")  # idempotent; `second` is already on the roster
    roster = (repo / ".goal" / "owner").read_text().split()
    assert roster == ["first", "second"], f"--own duplicated or replaced: {roster}"
    assert _stop(repo, "first")["decision"] == "block"
    assert _stop(repo, "second")["decision"] == "block"


def test_own_adds_a_second_session_rather_than_replacing_the_first(repo: Path) -> None:
    """`--own` used to REPLACE, so binding a second lane released the first
    without saying so."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    _stop(repo, "first")
    _cli(repo, "--own", "second")
    assert _stop(repo, "first")["decision"] == "block", "--own released the session already held"
    assert _stop(repo, "second")["decision"] == "block", "--own did not bind the new session"


def test_clearing_a_run_takes_its_share_marker_with_it(repo: Path) -> None:
    """⛔ a stale marker would make the NEXT goal armed here hold every window in
    the repository without anybody asking for it."""
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    _stop(repo, "first")
    _cli(repo, "--share")
    _cli(repo, "--clear")
    assert not (repo / ".goal" / "shared").exists(), "the share marker outlived its run"
    arm(repo, checks=[FAIL], max_stalled_blocks=99)
    assert _stop(repo, "a")["decision"] == "block"
    assert _stop(repo, "b") == {}, "a cleared run's sharing leaked into the next goal"
