# `ambition_test_support` — sequester harness boilerplate and make behavioral tests cheap

**Status:** triage. Strong candidate; the dependency layering is not decided.

## Finding

Behavioral tests are the repository's main architectural proof, but a new test
often needs a large amount of setup unrelated to its behavior: `MinimalPlugins`,
session-world insertion, fixed-time setup, room fixtures, catalog construction
and manual schedule stepping. Copied setup drifts from the real lifecycle,
hides the assertion, gets command flushing and fixed-tick boundaries subtly
wrong, and pushes agents to test a nearby value instead of the real consumer.

What exists:

- `crates/ambition_sim_harness` (`Platformer2dSimHarness`) already covers
  engine/session fixtures: build, `step`/`step_n`/`step_frame`,
  `set_timestep`, `reset_episode`, `drive_seat`, rollback setup, and world/app
  access. Do not build that layer again.
- Generic Bevy test boilerplate (named app profiles, exact stepping, command
  flush, message capture) has no shared home. `ambition_test_support` does not
  exist.

The open decision is a dependency question: does the generic layer go inside
`ambition_sim_harness` or in a crate below it, so that low-level domain crates
can dev-depend on it without a cycle?

## Laws for a shared fixture

1. No hidden production behavior: a fixture may install production plugins but
   does not replace the behavior under test with a fake.
2. No incidental completeness: a minimal fixture installs only named
   requirements.
3. Deterministic by default: time, ids, provider order and random sources are
   controlled.
4. Assert world behavior, messages, traces or snapshots, not source shape.
5. Keep `update`, fixed schedules and deferred-command application distinct.
6. Make it easy to omit a consumer, so a test can prove that its assertion
   discriminates.
7. Report the unexpected state, not only `left != right`.
8. No global mutable fixture state.
9. No game-name branches in generic support.
10. A reader can tell what a fixture installs without running it.

Keep game-specific fixtures (boss scripts, Mary-O/Sanic levels, one-off
catalogs, golden worlds) with their owner. A fixture that imports the whole game
is not an independent fixture, and external profiles must not depend on an
omnibus support package that installs flagship defaults.

## Evidence command

```bash
grep -rn 'App::new()' --include=*.rs crates game | wc -l
grep -rln 'App::new()' --include=*.rs crates game | wc -l
```

The count grows over time (1,004 sites on 2026-07-22, 1,346 on 2026-10-02).

## Owner

None.

## Trigger to promote

Promote to [`../tracks.md`](../tracks.md) when a pilot names:

- the dependency-safe base layer, and a statement that production crates do
  not depend on it;
- three test clusters with different needs (a low-level ECS cluster, a
  session/room cluster, a headless acceptance cluster) and a deletion target
  for their copied setup;
- the exact app profiles and stepping semantics;
- how fixtures expose installed behavior, and the policy for domain adapters;
- tests for the support code itself.

Success is not fewer lines. A new behavioral test must be easier to write,
harder to make vacuous, and give better failure output. Do not perform a
workspace-wide mechanical rewrite in the pilot.
