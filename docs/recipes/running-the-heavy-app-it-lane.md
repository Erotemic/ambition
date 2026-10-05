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

⛔ **A P2P session is built in one process only, over an in-memory link.**
`start_peer_session` (`crates/ambition_platformer2d_rollback_ggrs/src/peer.rs`)
starts one over any GGRS socket, and `LoopbackTransport` in the same file is the
only transport in the tree. `game/ambition_app/tests/two_peers.rs` is the only
caller: two Apps, one process. All other rollback arms use a sync test
(`start_synctest_session()` in
`crates/ambition_platformer2d_rollback_ggrs/src/session.rs`), which compares one
App with itself. A green rollback lane clears a LOCAL RESIMULATION defect, and
the peer questions that the arms of `two_peers.rs` name. It says nothing about a
network transport. Always write "local" beside "the rollback suite is green".

⛔ **`app_it` does not run the demo apps.** See the next section.

## Arms that read published art

Published sprites are gitignored. An arm that reads a published product fails
on a checkout that did not publish it, and its failure names the remedy. Run
the remedy and build again. Do not record the red as environmental.

| Arm | Reads | Remedy |
| --- | --- | --- |
| `a_pet_hand_meets_the_contact_point::the_petting_hand_is_on_the_place_the_dog_is_petted` | the part flipbooks of the robot and the dog (the build embeds their landmark tables from them) | `scripts/regen/sprites.sh --target player_robot_v3` and `--target companion_dog` |
| `admiral_gun_sword` (the rig arms) | the pirates' published body rigs | `scripts/regen/sprites.sh` |
| `boss_sheet_wiring`, `declared_art_resolves` | each declared sheet | `scripts/regen/sprites.sh` |

The table is the arms whose failure text names `scripts/regen` (grep of
`game/ambition_app/tests`, 2026-10-05). `enemy_body_scale` and
`hall_scale_spread` print `[skip]` and pass without baked sheets, so a green
there on such a checkout proves nothing.

## The demo host apps have their own lane

`app_it` composes the shipped game. Each demo host app composes a different
host, and has its own integration binary:

| package | binary |
|---|---|
| `ambition_demo_mary_o_app` | `mary_o_it` |
| `ambition_demo_sanic_app` | `sanic_it` |
| `ambition_demo_smash_app` | `smash_it` |
| `ambition_demo_twintrack_app` | `twintrack_it` |

`app_it` and `python3 -m pytest scripts/tests` do not run them. The default
`./run_tests.sh` does, in its `workspace (default features)` job. The narrow
command is:

```
./run_tests.sh -p ambition_demo_mary_o_app -p ambition_demo_sanic_app \
  -p ambition_demo_smash_app -p ambition_demo_twintrack_app \
  --only-job ambition_demo
```

Without `--only-job`, a `-p` filter also plans the three external-consumer
jobs. Run `--list` to see the plan.

**When this lane is required.** Run it before you push a change to one of
these, in the engine or in a demo:

- session death, or a checkpoint restore
- room replay, or a sandbox reset
- the rollback host (`ambition_platformer2d_rollback_ggrs`), or a rollback
  registration
- an instrument that a demo test reads (a message, a counter, an outcome)

The last row is how the gap was found. From `f7ecfc019` until `69d29caa0`
(2026-10-05), three `mary_o_it` arms and one `sanic_it` arm were red on main
while `app_it` and the pytest lane were green. A restore stopped writing a
message, and the demo fixtures counted that message. The behaviour held; the
demo counters were blind. No test in `app_it` reads those fixtures.

**The lane tolerates no red.** There is no waiver list for it. Measured
2026-10-05 on `18ae4b133`, host `aivm-2404`: `mary_o_it` 65 passed and 3
ignored, `sanic_it` 41 passed, `smash_it` 69 passed and 4 ignored,
`twintrack_it` 27 passed.

- The `body_rig_trial` arms of `mary_o_it` read Mary-O's published rig. Publish
  output is gitignored, so on a checkout that did not publish it they fail, and
  the failure names the remedy: `scripts/regen/sprites.sh --target mary_o_v2`.
  Run the remedy. Do not record the red as environmental.
- An `#[ignore]`d arm in these binaries is a print-only probe, a diagnostic
  census, or a route that a fixture course replaced. Each one says which in its
  `ignore` reason.
- A red arm needs a control before you name its cause: run the same arm with
  your change switched off. The four arms above were first attributed to the
  wrong commit from the messages alone.

**`SimTick` advances 1:1 with `sim.step()` in the fixed-tick harness.** "The sim
stopped" and "my writer stopped" look the same downstream; only the tick tells
them apart. A rollback composition is a different host; re-measure there.

## Counting what an arm proves

Counting calls to a safety API measures vigilance. Counting assertions that a
broken world fails measures safety. `scripts/check_headless_arms_can_fail.py`
checks that headless arms assert something a non-stepping engine cannot satisfy.
When you poison an arm, check that the poison changed the scope the arm uses (for
example, a shared helper that does the stepping).
