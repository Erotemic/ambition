# Running the heavy `app_it` lane

`cargo test -p ambition_app --test app_it` is the only instrument in this
repository that can see a schedule cycle, a composition that does not step, or a
rollback defect in a real host. It can also exhaust the machine's memory.

## A suite total belongs to a tree and a machine

Gitignored sprite-sheet publish output differs between machines, so arm counts
differ too. Name the machine and the commit beside every lane result.

## A red lane reports a smaller population than a green one

`cargo test` is fail-fast across targets: after one test binary fails it stops
launching the rest. A red `--workspace` run can skip dozens of crates and still
report a total close to a green run's.

- Quote a pass count only from a green run, or pass `--no-fail-fast`. The
  feature-union job in `scripts/run_tests.py` already passes it.
- Two machines with different reds measure different populations.

The open work is in
[TEST-LANES](../planning/queue.md#test-lanes--keep-required-test-lanes-executable).

## Standing prohibitions from one schedule cycle

A system that was put in an early set (`.in_set(WorldPrep)`) while it also ran
`.after(...)` a system in a later set was ordered both before and after the same
phase. The fixed loop retried the broken schedule forever and leaked memory until
the machine went down. Guard:
`the_boss_animator_takes_the_gate_and_not_a_phase` in
`crates/ambition_platformer2d_actor_monolith/src/features/mod.rs`.

1. **A set carries a POSITION as well as a gate.** Before you add `.in_set(X)` to
   a system with cross-phase `.after`/`.before` edges, check which set every
   edge target lives in.
2. **A session gate is not a capability check.** `run_if(simulation_authorized)`
   is true in a host with no boss catalog. Guard a system that needs a resource
   with that resource's existence, for example
   `.run_if(resource_exists::<BossCatalog>)`.
3. **`cargo check` cannot see a schedule cycle, and neither can an arm that does
   not step.** A schedule change needs an `app_it` run.

## Operational rules

- `scripts/measure_test_arm_rss.py` bounds a runaway: one process per arm,
  `RssAnon` (not `VmRSS` or cgroup `memory.current`), kill by process group at a
  hard cap. It refuses a row where libtest ran zero tests.
- ⛔ **`pkill -f <pattern>` is not a safe cleanup.** The shell that runs it has
  the pattern in its own argv, so the first `pkill` can kill that shell and the
  rest of the line does not run. Use `pgrep -af` to list, kill by PID, and run
  `pgrep` again in a separate call.
- When a build fails in a crate you did not touch, check free space before you
  read the diagnostic. ENOSPC can appear as ordinary compile errors.

## A rollback-mutator red has three independent questions

Answering one is not a verdict:

1. Is the write inside the rewind window?
2. Is the write at a point no rewind crosses? This satisfies the guard without
   moving anything.
3. Can a rollback erase the TRIGGER? This closes the waiver route, and (2) cannot
   rescue it.

Name which question a green answers.

## What a green lane does and does not clear

⛔ **No P2P session is built in this workspace.** The only session construction
is `start_synctest_session()` in
`crates/ambition_platformer2d_rollback_ggrs/src/session.rs`. A green rollback lane
clears a LOCAL RESIMULATION defect. It says nothing about two peers agreeing.
Always write "local" beside "the rollback suite is green".

**`SimTick` advances 1:1 with `sim.step()` in the fixed-tick harness.** "The sim
stopped" and "my writer stopped" look the same downstream; only the tick tells
them apart. A rollback composition is a different host; re-measure there.

## Counting what an arm proves

Counting calls to a safety API measures vigilance. Counting assertions that a
broken world fails measures safety. `scripts/check_headless_arms_can_fail.py`
checks that headless arms assert something a non-stepping engine cannot satisfy.
When you poison an arm, check that the poison changed the scope the arm uses (for
example, a shared helper that does the stepping).
