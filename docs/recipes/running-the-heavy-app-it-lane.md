# Running the heavy `app_it` lane

`ambition_app`'s `app_it` binary assembles the real Ambition host. Use it to
investigate schedule composition, simulation, room transitions and rollback.
The suite is expensive. Prefer a relevant test filter over the entire suite.

```bash
./run_tests.sh --list
./run_tests.sh -k <relevant_test_name>
# When a specific app_it test is known:
cargo test -p ambition_app --test app_it -- <relevant_test_name>
```

A full run is an integration checkpoint, not a routine response to every edit.
See [cheapest sufficient check](cheapest-sufficient-check.md).

## Interpreting a result

- `cargo test` can stop launching additional targets after a failure. A red
  workspace run may count a different set of tests from a green run.
- Report the source commit, selected features, machine and relevant local
  generated assets. Gitignored art can affect test availability.
- Tests that read published rigs or sheets may require
  [sprite publication](../tools/sprite-publication.md) or
  [asset mirroring](adding-an-asset.md). Verify the missing product before
  diagnosing the simulation.
- Local GGRS SyncTest proves local resimulation properties. The peer tests in
  `game/ambition_app/tests/two_peers.rs` use an in-process link; neither is a
  real network-transport acceptance test.

## Schedule and rollback failures

A Bevy system set determines both scheduling position and membership. Before
combining `.in_set` with cross-phase `.before`/`.after` constraints, check the
phase ordering. A cycle can prevent an app from stepping and exhaust memory;
`cargo check` does not execute schedules. One focused host-step check is
appropriate after changing those relationships.

Likewise, a generic simulation run condition does not guarantee that an
optional resource, such as `BossCatalog`, exists. Gate resource-dependent
systems on the actual required capability.

When diagnosing rollback, distinguish a write inside the rewind window from a
write that no rewind crosses, and determine whether a rewind can erase its
trigger. Fix ownership or replay semantics instead of installing a test-only
process guard.

`scripts/measure/measure_test_arm_rss.py` can limit the memory of a suspected
runaway arm. Inspect and terminate processes by PID; broad `pkill -f` patterns
can match their invoking shell.

## Demo hosts have separate integration tests

Passing `ambition_app`'s `app_it` does not run the Mary-O, Sanic, Smash or
TwinTrack demo host tests. The test runner has a focused lane:

```bash
./run_tests.sh -p ambition_demo_mary_o_app -p ambition_demo_sanic_app \
  -p ambition_demo_smash_app -p ambition_demo_twintrack_app \
  --only-job ambition_demo
```

Use the affected demo's test when its session lifecycle, restore, rollback or
observable host behavior changes. Some older test fixtures assert event counts
rather than gameplay effects: inspect what an assertion measures before
changing production behavior to satisfy it.

The repository's **Avoid bullshit guardrails** policy applies. Do not make
large suites or permanent tests-of-tests an automatic price of small fixes.
