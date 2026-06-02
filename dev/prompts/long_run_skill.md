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

=== Long-run discipline (read first; obey strictly) ===

At the very first action, run:
  START_EPOCH="$(date +%s)"
and write that value to /tmp/long_run_start_epoch.txt with the Write tool
(or a single bash redirect). At any later check, compute:
  ELAPSED="$(($(date +%s) - $(cat /tmp/long_run_start_epoch.txt)))"
  REMAINING="$((MISSION_DURATION_SECS - ELAPSED))"
and report ELAPSED in your next user-facing message.

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
   "transitioning to cleanup", BEFORE
   ELAPSED >= MISSION_DURATION_SECS, stop drafting that sentence,
   pick the next queue item, and execute it instead.

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
   conditions — they are the next work item.

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

Operating notes:

  PRIORITIZE THE BIGGEST UNBUILT ITEMS IN TASK_SET (MISSION_TARGET).
  SMALL LISTED TODOS ARE FINE; THE BIG ONES ARE THE POINT — DON'T DODGE THEM.
  WHEN THE EASY WORK RUNS OUT, TAKE A BIG TODO — DON'T MANUFACTURE SAFE WORK.
  WORK AUTONOMOUSLY.
  DO NOT ASK JON FOR INPUT.

=== End long-run discipline ===

