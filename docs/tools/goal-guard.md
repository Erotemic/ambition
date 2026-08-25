# goal_guard — the incidents behind the rules

`scripts/goal_guard.py` holds the rules. This file holds the failures that
produced them, and exists so the rules can travel to another repository without
carrying Ambition's history, and without becoming arbitrary once they arrive.

Read this when a rule in that file looks like it could be simplified. Every one
of them already was simpler, once.

## Why the arbiter is a command and not a model

**2026-07-25.** `/goal` installed a Stop hook whose condition was judged by a
model reading the conversation. It released a 24-hour run after **84 minutes**:
the agent wrote a completion-shaped status report — *"A1 done, A2 done, three
findings…"* — and the judge, reading prose, called the goal met and cleared
itself. The session then idled for nine hours with nobody to restart it.

The lesson is not "the hook was too weak." It is that the arbiter was the wrong
*kind*. A judge that reads prose can be persuaded by prose, and a model tired of
an item is exactly the thing most motivated to write persuasive prose.

This is why the block text names that specific move — *"writing a status report
does not close an item"* — and why `test_a_transcript_claiming_success_does_not_release_it`
exists.

## The guard that silently ceased to exist

**2026-08-05, two failures in one day, neither visible from outside.**

The hook command contained a relative path. A hook inherits the session's
working directory, so one `cd` into a subdirectory made `python3
scripts/goal_guard.py` resolve to nothing. The runtime treats a hook script it
cannot find as **non-blocking**, and says so only in a suggestion-level note — so
the guard did not fail, it stopped existing. A 72-hour run was released and the
session idled 4h12m.

Separately, `repo_root()` asked `git rev-parse --show-toplevel`. This tree
contains a nested git repository at `tools/ambition_sprite2d_renderer`; one `cd`
into it and `--show-toplevel` answered with the sub-repo, where
`.goal/active.json` does not exist. `mode_stop` took its "not armed, ordinary
sessions are untouched" path and released the run.

Hence: the hook command walks *up* from `$CLAUDE_PROJECT_DIR` and emits a block
of its own if it cannot find the guard, and `repo_root()` is `__file__` rather
than anything git knows. The only symptom available to a human for either
failure was `.goal/state.json` going stale while the session worked on.

## The stall counter that reset itself on infrastructure failure

`stalled` counted blocks with no new commit, and read
`stalled + 1 if sha and sha == last_head else 0`. A failing `git rev-parse` — a
permission problem, a lock file, a repo the hook could not see — returned `""`
and reset the counter to **zero on every block**. The one escape hatch from a run
going nowhere was disabled by exactly the infrastructure failures most likely to
make a run go nowhere (GPT 5.6, 2026-07-28).

Three cases are named separately now: a new commit is progress and resets; the
same commit is a stall; and **not knowing is a stall too**, because a guard that
cannot see progress must not assume it.

## The checks that ran for 66 hours and decided nothing

**2026-08-25 — the feature is gone.** A goal used to carry a `checks` list of
shell commands, run on every Stop, and the run released when they all passed.
Two years of good reasons built it: a check must name a test rather than a doc
marker, because a doc marker is a file the agent can edit; a check must fail
when its subject cannot be read; a timeout is not a failure.

Every one of those is sound, and together they answered the wrong question.
Anti-gaming is about the TERMINAL verdict — is this really done. Pacing is about
EVERY Stop — is there work left. One list served both, so a check built to make
completion hard to fake was billed once per turn.

`.goal/check_cost.jsonl` had been recording the price since 2026-08-08. Read
back on 2026-08-25: **972 runs, 66.0 hours of cargo, median 192s and rising to
395s over the last 40 runs. Not one run ever passed. In 972 of 972 the verdict
was already sealed by a sub-second `docs/planning` grep** — the ledger has open
rows, so the guard was always going to block — and `run_checks` deliberately did
not short-circuit, so the build ran anyway.

Jon: *"I do not want programmatic checks to run there at all. The goal guard
should just be firing a hook that tells the agent to continue because there is
work left in docs/planning. There is so much backlog we are never going to
complete it even if we ran for an entire month nonstop."*

So there is no completion condition to test. `checks` is refused at `--arm`,
which is the only thing standing between this file and the next agent that reads
the anti-gaming argument and finds it convincing. A goal is text and clocks;
release comes from a deadline, a fuse, or Jon. Removed with it: the per-check
timeout, `.goal-guard.json`, `check_cost.jsonl`, and `--status --quick`, which
existed only because `--status` used to trigger a build.

Asked once before why a turn took so long, this repo built the cost ledger
instead of deleting the checks. The ledger is what eventually made the case —
but measuring a thing you should not be doing is not the same as stopping.

## Arming destroyed the goal it replaced

**2026-08-15.** `mode_arm` was a bare `shutil.copyfile` over `active.json`.
`.goal/` is gitignored, so re-arming erased a live 72-hour goal that existed in
no other place: no archive, no git object, nothing. `clear_goal` had written a
`done-<stamp>.json` receipt on every *other* exit from a run since the beginning
— replacement was the one door out with no receipt.

The goal was reconstructed from a verbatim dump that happened to be in the
session transcript. That is luck, not a recovery procedure.

## Waiting on subagents is not stopping — and then it was not a condition at all

**2026-08-15, Jon.** A coordinator that spawns subagents and yields the turn to
wait for them is *ending a turn*, which is the only event a Stop hook can
observe. The guard blocked that yield and told the agent to resume — every
yield, each block injecting the whole preamble.

So the guard learned to read the transcript for work that went async and never
reported back, and to stand down while any was outstanding. Two parser bugs were
caught by tests rather than by reading: matching `running in background with ID`
without the trailing id counted a `grep` whose own output quoted the phrase, and
keying the compact boundary on a literal JSON fragment saw nothing once the
separator had a space in it.

**2026-08-23, Jon, deleting all of it:** *"I often see rando shells that you
often just forget about and they just exist and are never killed. That pattern
happens ALL THE TIME. So the goal guard should never be using that as a
condition."*

He is right, and the failure it produced was measured before he said it: an
abandoned poll loop or a superseded gate run never sends a completion, so the
outstanding set was almost never empty and the run was **unguarded by default**.
Twenty-one consecutive waits, a nineteen-hour-old clock under a four-hour
ceiling, zero blocks. Every fix applied to it — a clock that only restarted when
something reported, a ceiling read before every stand-down — made a mechanism
that should not have existed slightly less wrong.

⭐ **the general form: before making X a condition, ask how often X is true when
nothing is wrong.** A stand-down that always applies is the inverse of a guard
that cannot fail, and it costs the same thing — the whole instrument.

⇒ `--pause` (one turn) and `--hold` (until lifted) remain. Both are explicit,
both say so loudly, and neither can be entered by accident.

## "Extend the timer" was two edits, and one of them was invisible

Jon asked for 48 more hours on a live run (2026-08-16). There is no single field
that holds "when this ends": `deadline_utc` is an absolute time, `max_run_hours`
is a fuse counted from the **first block**, and the run ends on whichever comes
first. Editing the deadline alone — the obvious move, and the one the file
invites — leaves the fuse to release the run on the old schedule from a field
the editor never looked at.

Worse, answering *"how long is left?"* meant reading `.goal/active.json` and
`.goal/state.json` and doing the arithmetic, because the only command that knew
was `--status`, which then ran the goal's checks — a cargo build — to print a
date.

```bash
python3 scripts/goal_guard.py --extend 48h   # also 2d, 90m, or an ISO timestamp
python3 scripts/goal_guard.py --extend       # just the clocks
```

It moves both clocks by the same amount, refuses to write a goal that would no
longer arm, and records what it did in an `extended` list in the goal file. It
deliberately does **not** touch `max_stalled_blocks`: that one counts blocks with
no new commit, so resetting it would quietly convert "give it another day" into
"forgive the silence". It prints the stall count instead — the number a human
extending a quiet run actually needs.

## One run, several sessions — and why the default stayed single-owner

**2026-08-20.** The arrangement grew a second lane: an architecture session on
`main` and a feature session in a worktree, both against one repository. The
guard held exactly one of them, and not the one committing — ownership had been
claimed by a third session that had since gone quiet, so the lane doing the work
never had a Stop blocked and the stall counter read **8 of 40 with no new commit**
while `main` gained a dozen commits. A run can be armed, alive, and enforcing on
nobody.

⭐ **the fix is a ROSTER, not a second goal file.** `.goal/owner` holds one
session id per line; a pre-2026-08-20 single-line file is a roster of one, so
nothing armed before this reads differently.

⛔ **and `--share` is a flag rather than the default, deliberately.** The
single-owner property is worth keeping: an unshared goal does not reach out and
hold the window somebody opened to ask one thing. A shared goal does exactly
that. Making the capability free would have spent a property nobody asked to
spend.

```sh
python3 scripts/goal_guard.py --share      # every session that stops here JOINS
python3 scripts/goal_guard.py --unshare    # no NEW session joins; the roster stays held
python3 scripts/goal_guard.py --own <id>   # ADD one session
python3 scripts/goal_guard.py --disown     # release EVERY session
```

Three details that are each a defect avoided rather than a preference:

- **the roster is APPENDED to, never read-modify-written.** `owner_path`'s own
  docstring records why ownership is not a key in `state.json`: a Stop hook
  reads that dict and writes the whole thing back, so a concurrent claim in the
  window is silently dropped and the goal quietly reverts to unclaimed. A roster
  read-modify-written has the same defect against itself.
- **`--own` ADDS.** It used to replace, so binding a second lane released the
  first without saying so.
- **the share marker dies with `--clear`.** A stale one would make the next goal
  armed here hold every window in the repository without anybody asking.

⚠ **the block and stall counters stay shared across the roster.** *"40 blocks
with no new commit"* is a fact about the repository, not about a session, and
HEAD moving for either lane resets it for both. Two lanes therefore reach the
stall fuse in half the wall clock — which is a real cost, and better than a
per-session fuse that would let one idle lane sit forever while the other carried
the run.

⭐ **and two lanes in two worktrees need none of this.** `repo_root()` resolves
through `__file__`, so a worktree's copy of the script resolves to the worktree
and reads its own `.goal/`. Two worktrees can hold two DIFFERENT goals today.
⇒ `--share` when the lanes are working ONE goal; a goal per worktree when they
are not.

⚠ **what the tests do NOT prove:** the append-vs-rewrite property is argued, not
executed — the six new cases in `scripts/tests/test_goal_guard.py` drive sessions
one at a time, so a read-modify-write roster would pass all of them. The
concurrency claim rests on `O_APPEND`, and the reason it is written down here is
that a later edit "tidying" `join_owner` into a read/modify/write would be
invisible to the suite.

## Armed, alive, and enforcing on nobody — twice in two days

**2026-08-23, Jon, three times: "the goal is not firing."** Both causes were the
guard standing down *silently*, and silence is the one failure nothing reports.

**The wait ceiling was unreachable for the caller that needs it most.**
`handle_wait` returned from its changed-key branch before it ever read
`waiting_since`, so a coordinator that launches a background command most turns
— growing the pending set every turn — bought unlimited quiet. On this
repository: 21 waits, a 19-hour-old clock, a 4-hour ceiling, zero blocks. The
fix three commits earlier had stopped the *clock* from restarting and left the
*escape* in place, so the clock read 19h and nothing read the clock.

⛔ the existing ceiling test held the pending set **fixed**, which is exactly the
variable the defect needs. A test that holds the suspect constant cannot see it
— the same shape as a bisect that can only vary the code half.

**A session id rotates and the roster does not follow it.** A compact or a
resume opens a new transcript under a new id, so `.goal/owner` named a session
that would never stop again: `mode_stop` returned before doing anything,
`--inject` said nothing because the new id was a stranger, and the run enforced
on nobody. `--resume` had been the manual answer since 2026-08-01 — but the only
symptom is silence, so nobody knows to run it.

The runtime injects the previous transcript's path into the first records of the
new one. That is *proof* of continuation rather than a guess at one, so the
session inherits the run — and only from an id already on the roster. A window
that merely mentions somebody else's transcript inherits nothing; taking over a
run you are not doing is what `--share` is for. SessionStart now names the holder
and points at `--resume` instead of saying nothing.

**What both had in common was a bare `return 0`.** No state changed, so from
outside "the guard decided to stand down" and "the hook never ran" were the same
observation. Every path out of `mode_stop` now records a one-line verdict, and
`--status` reads it back.

## What this file cannot do

It cannot make an agent work. It blocks a Stop and prints the goal; everything
after that is the agent's. The honest claim is narrow — it removes *"I have
decided I am finished"* as a way for a turn to end, and nothing more.

And every word of that assumes the guard **ran**. It cannot prove that from
inside, but it no longer has to be guessed at from outside: every Stop records
what it decided, and `--status` prints that verdict with its age, or says
`NEVER` when no turn has been checked at all.
