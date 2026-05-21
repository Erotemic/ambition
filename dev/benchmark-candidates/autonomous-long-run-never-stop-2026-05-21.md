# Never stop to ask the user during an autonomous long-run session

## Q: What should an agent do when an autonomous long-run session (`/loop`, mission-duration prompt) hits a transient blocker that ordinarily would warrant asking the user?

### Context

Some repo memories tell the agent to **stop and ask the user** when certain conditions appear. Two real Ambition memories like this:

- `feedback_filesystem_errors.md`: EMFILE / EIO from virtiofs is a host-side issue; "stop and ask for a reset, don't /tmp around it".
- `feedback_no_sudo_apt.md`: never sudo apt without confirmation.

These rules are safe-by-default for interactive sessions. They are dangerous-by-default for autonomous long-running sessions invoked via `/loop`, `/schedule`, or a multi-hour mission-duration prompt: the user is *not at the keyboard*. A "stop and ask for a reset" message in mid-run can burn the remaining hours of compute while waiting for a response that may never arrive in time.

Concrete 2026-05-21 incident: in the middle of an 8+ hour intro-v1 polish session, a transient EMFILE error during a Python tool invocation surfaced. The agent (Claude) wrote "I should stop and ask for a reset" and proceeded cautiously to alternate work. The user's later feedback: *"You stopped to ask me a question during an autonomous long session. NEVER do that. You wasted all the time you could have spent working."*

### Failure mode

The agent treats long-running autonomous sessions like interactive sessions, applying interactive safety rules verbatim. Result: hours of compute pause for ambiguity resolution that the agent should have answered itself.

### Distractor moves that look reasonable

- **Quote the rule back at the user**: "Memory says stop and ask, so I'm stopping." The memory rule is interactive-mode default; the long-run prompt is the override.
- **Wait for a "transient" error to clear before retrying**: leaves the agent idle. Better: retry once, then SWITCH TO OTHER WORK.
- **Treat a failed dependency / test as a stop**: it's the next work item, not a stop condition.

### Correct moves

1. **Define the long-run mode explicitly**: prompts that say "work for N hours", `/loop`, `/schedule`-fired routines, and any explicit MISSION_DURATION_SECS gate are long-run mode. Interactive defaults are suspended.

2. **For transient filesystem errors (EMFILE, EIO from virtiofs)**: retry the operation once. If it still fails, switch to a different task that doesn't touch the same file. Document the transient in the next commit message. Only stop if the repo is *durably* corrupted (`.git` gone, working tree wiped).

3. **For design uncertainty / missing dependency / failing test**: pick the safest interpretation and execute. Record the assumption in the commit message or playtest-handoff so it can be revisited. Long-run mode prefers *forward motion with a recorded assumption* over *paused certainty*.

4. **For destructive actions normally needing confirmation**: if the action isn't strictly required for the queued work, route around it. If it IS required (e.g. the queued work explicitly asks for a rebase / force-push), document why in the commit and proceed.

5. **Never paste the long-run rule back to the user mid-session**: that's another form of "asking" — it consumes their attention. Act on it silently.

### Adjacent rules

- `MISSION_DURATION_SECS` wrap-up is a SHORT closing window (commit in-flight, run validation, write summary), not "stop early because something is uncertain". Keep working until the threshold.

- If the queue exhausts before the threshold, ADD MORE WORK — don't stop. The discipline message from a typical long-run prompt explicitly enumerates fallback work (validation, polish, adjacent tasks).

### See also

- `dev/journals/lessons_learned.md` (search "never-stop-during-long-run").
- Memory: `feedback_never_stop_during_long_run.md`.
