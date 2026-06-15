When told to use the long run discipline, you will be given a number (amount of
time) and a list of tasks or an open ended problem (often a TODO list or just a
very large task). Convert the time to seconds and that is your
`MISSION_DURATION_SECS`. Then register the TODO list or the open ended task and
that is your `TASK_SET`.

From TASK_SET, the biggest, most impactful UNBUILT items are your top
priority — real player-facing features, systems, weapons, bosses, biomes
(the Portal Gun is the canonical example). Pick the biggest one and make
building it your `MISSION_TARGET`. Do not dodge it because it is hard,
risky, large, or hard to verify. Small listed TODOs are fine to do too,
but they never substitute for tackling the big ones.

The headline result of the run should be something a player could touch.
Doing the small TODO items along the way is fine. The thing to avoid is
INVENTING safe busywork — a test sweep, a tooling script, a doc pass, a
cleanup nobody asked for — to fill time instead of tackling the hard
listed features. If you run out of small work, the answer is a big TODO,
not manufactured safety. "More tests" is almost never what I want. I
want features.

WHEN MISSION_TARGET IS ARCHITECTURE/REFACTOR (not a feature): the same rules
apply with a translation. "The big hard thing" is the TANGLED CORE — the
god-file's interwoven half, the system with shared mutable state, the generic-
heavy module, the prototype nobody has cleaned up (falling_sand, settings). The
refactor equivalents of "manufactured safe busywork" are: extracting a test
module to its own file, splitting out the EASY half of a file while leaving the
hard core, "safe-subset lifts," renaming, doc passes, and reverting a file the
instant it resists. Those move line counts around without breaking the monolith.
A god-file is NOT broken until its hard core is decomposed. If a file fights you,
that file IS the mission — split its core, push the compile errors through, and
let behavior change. Reaching for the easy half because the core is messy is the
exact dodge rule 8 forbids, in refactor clothing.

=== Long-run discipline (read first; obey strictly) ===

At the very first action, run:
  START_EPOCH="$(date +%s)"
and write that value to /tmp/long_run_start_epoch.txt with the Write tool
(or a single bash redirect). At any later check, compute:
  ELAPSED="$(($(date +%s) - $(cat /tmp/long_run_start_epoch.txt)))"
  REMAINING="$((MISSION_DURATION_SECS - ELAPSED))"
and report ELAPSED in your next user-facing message.

THE CLOCK IS THE ONLY AUTHORITY ON "DONE." This step is not optional bookkeeping;
it is the keystone. If `/tmp/long_run_start_epoch.txt` does not exist, you have not
started the run — create it before anything else and verify with `cat`. While that
file says REMAINING > 0, "done" is a number, never a feeling. Any internal sense of
"I've done a lot" / "good place to stop" with REMAINING > 0 is a malfunction, not a
judgment. Put a literal `REMAINING: <n>s` line in EVERY commit body and at the top
of every 4th response, computed from the file — not estimated, not remembered.

RESUMING FROM A SUMMARY (context ran out mid-run): the FIRST thing you do is
re-read /tmp/long_run_start_epoch.txt and recompute REMAINING, and re-read this
file plus docs/concepts/autonomous-decision-making.md. Then DISTRUST the summary.
A summary you wrote near the end of a context is the prime carrier of your own
evasion: if it says the work is "exhausted / comprehensive / done / supervised-
tier / clean stopping point," treat that as an UNVERIFIED CLAIM and re-derive the
real state from the repo (sizes, TODOs, the oracle). Never inherit "done" across a
summary. The summary describes what you talked yourself into, not ground truth.

HARD RULES:

1. While ELAPSED < MISSION_DURATION_SECS you are FORBIDDEN from:
   - writing a "Final summary" / "Mission complete" / "All done" message,
   - claiming the work is finished,
   - emptying your TodoWrite list and stopping,
   - asking the user "what would you like next?",
   - producing the closing paragraph-form commit message for the whole run.
   The loop is: advance MISSION_TARGET → at a real milestone, commit →
   recheck ELAPSED → keep going. A single MISSION_TARGET may span the
   ENTIRE run and dozens of edits across many files. Do NOT chop the
   mission into tiny committable chunks just to have something to commit;
   an hour of focused work on one feature with no commit is expected.

2. If MISSION_TARGET is genuinely complete AND
   ELAPSED < MISSION_DURATION_SECS, pick the next biggest unbuilt item
   from TASK_SET (a listed small TODO is fine too). Do NOT invent safe
   busywork to fill time. If TASK_SET is exhausted of big work, scan the
   repo for the next real feature/system to build before reaching for
   cleanup.

3. If you find yourself drafting a sentence that contains any of:
   "in summary", "to wrap up", "this completes", "final commit", "all
   patches landed", "summary writeup", "good stopping point",
   "transitioning to cleanup", "comprehensive(ly complete)",
   "diminishing returns", "supervised", "supervised-tier", "needs a
   (careful/focused/supervised) pass", "safe-subset", "lower-value",
   "the coherent piece", "exhausted the clean work", "the frontier left
   for", "genuinely tangled/risky so", BEFORE
   ELAPSED >= MISSION_DURATION_SECS, that sentence is not a conclusion —
   it is a CONFESSION and a DIRECTIVE. Whatever it just labeled too
   hard / too risky / too low-value / needing supervision IS your next
   work item. Stop drafting, go do that exact thing, push it through.
   You do not get to describe the remaining work as unsuitable; you only
   get to do it or run out the clock.

4. Time-check cadence: at the START of every 4th tool-using response
   (or sooner), run a single `date +%s` bash and print
   "ELAPSED: Xs / MISSION_DURATION_SECS — continuing with [task]".
   Do not let more than ~6 responses pass without a time check.

5. After ELAPSED >= MISSION_DURATION_SECS, transition to wrap-up:
   finish the in-flight patch, run validation, write the final summary.
   Do NOT keep starting large new tasks once past the threshold.

6. Stop conditions BEFORE the threshold are limited to genuine
   filesystem errors (permission denied, disk full, repo missing,
   corrupted required file). Compile errors, test failures, design
   uncertainty, and "I'm not sure what to do next" are NOT stop
   conditions — they are the next work item. A mid-refactor compile
   error ("unclosed delimiter", a dangling brace, a privacy error) is
   the MIDDLE of the work, not a verdict on it: fix it forward. Do NOT
   `git checkout`/reset a file to escape a mess you created — that is
   retreating to the "safe" smaller move the decision doc forbids.
   Revert only a fundamentally WRONG APPROACH, and when you do, attack
   the SAME target again a different way; never let a revert become
   "this file is too hard, moving on." Behavior is allowed to change
   and replay is allowed to diverge — "no behavior change" is a tool to
   confirm intended-neutral steps, never a goal to optimize or a reason
   to shrink the move. Stop reporting it as success.

7. FAILURE MODE TO RECOGNIZE: declaring victory after a long time has passed.
   If your inner sense says "I'm at a clean stopping point" and ELAPSED <
   MISSION_DURATION_SECS, that sense is wrong — either MISSION_TARGET has
   more depth to build (polish it, extend it, add the next piece) or it is
   done and you pick the next big feature. Keep building; do not coast on
   small safe work to fill the clock.

8. TACKLE THE BIG TODOS; DO NOT DODGE THEM. Doing small listed TODO items
   is fine. The failure is reaching for SAFETY instead of ambition once
   the easy work runs out: when there's nothing small left, pick up a
   big/hard item — do NOT invent a test sweep, tooling, docs, or cleanup
   to stay comfortable. "The real task feels risky or unverifiable, so
   I'll do this safe thing instead" is the #1 failure mode. For this run,
   priority is AMBITION > correctness > green-ness.

9. "I can't playtest / can't verify the feel" does NOT excuse skipping a
   gameplay feature. Build it end-to-end so it EXISTS — data, components,
   systems, input wiring, the math, the spawning, a test room to exercise
   it — and leave feel-tuning (numbers, juice) as a handoff note for me.
   Build boldly: the repo's usual "don't make unverified changes / don't
   add tech debt" caution is relaxed for big feature work. IT IS OK FOR
   TIME TO END WITH A FEATURE HALF FINISHED — resumable progress on
   something big beats a finished pile of small safe things.

10. "DONE" IS OBJECTIVE, NEVER SELF-DECLARED. You may not conclude the
    mission is complete on the strength of your own assessment. State the
    completion test in concrete, checkable terms at the START of the run
    and re-check it against the repo — not your memory. For a "break the
    monolith" mission, unbroken looks like: a file still over ~800 lines
    that you have not split; a named system still living in core that the
    oracle (could another platformer be built by ADDING a content crate?)
    says should move; a prototype/god-module you skipped because it
    "resisted." While ANY of those remain and REMAINING > 0, you are not
    done — pick the hardest one and go. If you cannot write a concrete,
    repo-checkable reason the mission is complete, it is not.

11. CONTINUING MUST NOT DEPEND ON YOUR WILLPOWER. If the harness supports
    a self-wake / heartbeat (e.g. a ScheduleWakeup tool), schedule the
    next check-in (<= ~20 min out) that re-fires this loop, EVERY cycle,
    until REMAINING <= 0 — so the runtime drags you back rather than you
    having to choose to continue. Re-derive REMAINING from the epoch file
    on each wake; do not end the loop while REMAINING > 0.

Operating notes:

  PRIORITIZE THE BIGGEST UNBUILT ITEMS IN TASK_SET (MISSION_TARGET).
  SMALL LISTED TODOS ARE FINE; THE BIG ONES ARE THE POINT — DON'T DODGE THEM.
  WHEN THE EASY WORK RUNS OUT, TAKE A BIG TODO — DON'T MANUFACTURE SAFE WORK.
  WORK AUTONOMOUSLY.
  DO NOT ASK JON FOR INPUT.

=== End long-run discipline ===

