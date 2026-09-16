# Running the heavy `app_it` lane

`cargo test -p ambition_app --test app_it` is the only instrument in this
repository that can see a schedule cycle, a composition that does not step, or a
rollback defect in a real host. It is also the one that can take the box down.

⛔ **Everything here was learned by getting it wrong on this machine.** It lived
in `queue.md`'s TEST-LANES row until 2026-09-16 and moved here because the queue
is for open executable work, not for the rules a closed investigation left.

## A suite total is stamped to a TREE and a MACHINE

Two agents disagreed by 98 arms for an hour because one checkout's gitignored
sprite-sheet publish output was ~90 files short. **Name the box beside the
number, and the commit.** A lane result quoted without its commit is a number for
a program that may no longer exist.

## Three standing prohibitions, from one schedule cycle

`b9f2ece18` gave `drive_boss_animators` `.in_set(WorldPrep)` to buy a capability
gate, while that system also runs `.after(project_boss_attack_state_from_move)`,
which is `.in_set(CombatSet::Playback)` — LATER in the sim schedule. One system
ordered both before and after Playback; the fixed loop retried the broken
schedule forever, allocating ~27 MB/s for the first minute and ~234 MB/s after.
One orphaned arm reached anon-rss 64,629,160 kB in 316 s and took a 62 GB box
down. Bisected over `770ac4bff..ee3d0852e`, fixed at `23f786757`, guarded from
both sides by `the_boss_animator_takes_the_gate_and_not_a_phase`.

1. **A set carries a POSITION as well as a gate.** Adding `.in_set(X)` to a
   system that already has cross-phase `.after`/`.before` edges can contradict
   them. Check which set every existing edge target lives in.
2. **A session gate is not a capability check.** `run_if(simulation_authorized)`
   answers *"is this session authorized"*, which is TRUE in a host with no boss
   catalog. A system needing a resource is guarded by that resource's existence:
   `.run_if(resource_exists::<BossCatalog>)`.
3. **`cargo check` cannot see a schedule cycle, and neither can an arm that never
   steps.** That commit shipped on `cargo check` alone with an explicit "NO
   `app_it` run", justified by two facts that were exactly what made `app_it` the
   only instrument that could see it.

## Operational rules, kept because they cost a night

- `scripts/measure_test_arm_rss.py` bounds a runaway: ONE PROCESS PER ARM (peak
  RSS is a property of a process), `RssAnon` rather than `VmRSS` or cgroup
  `memory.current`, kill by process group at a hard cap, and it refuses a row
  where libtest ran zero tests.
- ⛔⛔ **`pkill -f <pattern>` IS NOT A SAFE CLEANUP.** The shell running it is a
  `bash -c '<whole line>'`, so its own argv contains the pattern: the first
  `pkill` kills the shell, the second — aimed at the binary — never runs, and
  neither does the verifying `pgrep`. That is how a 61.6 GB orphan escaped a
  sampler whose cap was working. ⇒ **`pgrep -af` to LIST, kill by PID, re-`pgrep`
  in a SEPARATE call.**
- ⚠ When a build fails in a crate you did not touch, **check free space BEFORE
  reading the diagnostic.** ENOSPC arrives as `error: could not compile <crate>`
  with the cause one line above, and has been seen as six ordinary-looking
  compile errors with no `os error 28` anywhere.

## What a green lane does and does not clear

⛔⛤ **NO P2P SESSION IS EVER BUILT IN THIS WORKSPACE.** `Session::P2P` appears
EXACTLY ONCE — `crates/ambition_platformer2d_rollback_ggrs/src/session.rs:910`, a
match arm reading `confirmed_frame()` — and there is ONE construction site for a
session at all, `AmbitionGgrsSession::SyncTest` at `:220`. ⇒ **A green rollback
lane clears a row of a LOCAL RESIMULATION defect and says nothing about two peers
agreeing.** The sync test saves, rewinds and resimulates in one process against
itself; a desync needing two hosts with different local state has no instrument
here.

⚠ So "the rollback suite is green" must never be written without the word LOCAL.
Ten S7 rows were redirected away from being measured this way, on the grounds
that ten more *"clean under local resimulation"* verdicts would have read on the
page as ten rows CLEARED.

⭐ **`SimTick` ADVANCES 1:1 WITH `sim.step()`, and it is the discriminator nobody
reaches for.** `fixed_60hz_room_sim("blink_run")` sampled every 40 steps:
`[(0,0), (40,40), … (240,240)]`. A peer read a derived count freezing flat over
240 frames as *"the simulation stops advancing ticks"*; it does not. **"The sim
stopped" and "my writer stopped" produce identical evidence downstream, and only
the TICK tells them apart.** ⚠ Scope: the fixed-tick harness. A rollback
composition is a different host in a different schedule — re-measure there rather
than quoting this.

## Counting what an arm proves

⭐ **Counting calls to a safety API measures VIGILANCE; counting assertions a
broken world fails measures SAFETY.** An arm demanding a room change has a
stronger liveness guarantee than one reading a health API once at the end,
because its check is load-bearing for its own subject rather than bolted on
beside it.

⚠ `scripts/check_headless_arms_can_fail.py`'s 17 arms were audited for exactly
this inflation and **0 were exposed**: 10 pass on assertions alone and every one
asserts something a non-stepping engine cannot satisfy. Two that looked like
composition-only assertions POISON RED when the two `app.update()` calls are
removed from their shared `presentation_shell` helper. ⚠ The first poison at
those two removed ZERO calls — they step through a helper — and a poison that
edits the wrong scope is a finding about the poison.
